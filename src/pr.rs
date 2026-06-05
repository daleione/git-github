use crate::config::load_config;
use crate::error::{Error, Result};
use crate::github;
use crate::llm::{self, ChatMessage};
use crate::repo::Repo;
use std::env;
use std::fs;
use std::future::Future;
use std::process::{Command, Stdio};

/// Cap the diff sent to the model so large branches don't blow the context.
const MAX_DIFF_BYTES: usize = 12 * 1024;

pub struct Options {
    pub remote: String,
    pub base: Option<String>,
    pub draft: bool,
    pub edit: bool,
}

/// Create a GitHub pull request for the current branch, with an AI-drafted
/// title and body generated from the commits and diff against the base branch.
pub fn create(opts: Options) -> Result<()> {
    let path = env::current_dir().map_err(|_| Error::NoCurrentDir)?;
    let repo = Repo::new(&path)?;
    let remote = repo.remote(&opts.remote)?;
    let head = repo.current_branch()?;

    let config = load_config()?;
    if config.deepseek.api_key.is_empty() {
        return Err(Error::NoApiKey);
    }
    // Creating a PR is a write; an anonymous client cannot do it.
    if github::token().is_none() {
        return Err(Error::NoGitHubToken);
    }
    let client = github::client()?;

    // Resolve the base branch: an explicit `--base`, else the repo's default.
    let base = match opts.base.clone() {
        Some(base) => base,
        None => block_on(client.repos(&remote.user, &remote.repo).get())??
            .default_branch
            .ok_or(Error::NoDefaultBranch)?,
    };
    if base == head {
        return Err(Error::NoCommitsForPr(base));
    }

    // Compare against the remote-tracking base when available; it reflects what
    // the PR will actually be diffed against on GitHub.
    let base_ref = if repo.exist(&opts.remote, &base) {
        format!("{}/{}", opts.remote, base)
    } else {
        base.clone()
    };

    let commits = git_capture(&[
        "log",
        "--reverse",
        "--pretty=format:- %s",
        &format!("{}..HEAD", base_ref),
    ])?;
    if commits.trim().is_empty() {
        return Err(Error::NoCommitsForPr(base));
    }
    let diff = truncate(&git_capture(&["diff", &format!("{}...HEAD", base_ref)])?);

    // Publish the branch so the PR head exists (and is up to date) on the remote.
    println!("Pushing {} to {}...", head, opts.remote);
    git_run(&["push", "-u", &opts.remote, &head])?;

    print_banner("AI Drafting Pull Request");
    let model = config.deepseek.model.as_deref().unwrap_or("deepseek-chat");
    let drafted = llm::stream_and_collect(
        &config.deepseek.api_key,
        model,
        build_prompt(&commits, &diff),
        config.deepseek.temperature,
    )?;

    let (mut title, mut body) = split_title_body(&drafted);
    if title.is_empty() {
        return Err(Error::EmptyMessage);
    }

    if opts.edit {
        let edited = edit_in_editor(&format!("{}\n\n{}", title, body))?;
        let (t, b) = split_title_body(&edited);
        title = t;
        body = b;
        if title.is_empty() {
            return Err(Error::EmptyMessage);
        }
    }

    let pull = block_on(
        client
            .pulls(&remote.user, &remote.repo)
            .create(&title, &head, &base)
            .body(body)
            .draft(opts.draft)
            .send(),
    )??;

    print_banner("✅ Pull Request Created");
    match pull.html_url {
        Some(url) => println!("{}", url),
        None => println!("Created pull request into {}", base),
    }

    Ok(())
}

/// Build the prompt: commit subjects plus the (truncated) diff.
fn build_prompt(commits: &str, diff: &str) -> Vec<ChatMessage> {
    let system = r###"You are an assistant that writes GitHub pull request descriptions.

Given the commit list and diff, produce:
1. A concise PR title on the FIRST line (max 72 characters, no type prefix and no markdown heading).
2. A blank line.
3. A markdown body: a short summary paragraph, then a "## Changes" section with a bullet list.

Be professional and concise. Output only the title and body, nothing else."###;

    vec![
        ChatMessage::system(system),
        ChatMessage::user(format!("Commits:\n{}\n\nDiff:\n{}", commits, diff)),
    ]
}

/// Split generated text into a title (first non-empty line) and body (the rest).
fn split_title_body(text: &str) -> (String, String) {
    let mut lines = text.trim().lines();
    let title = lines
        .by_ref()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_string();
    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    (title, body)
}

fn truncate(diff: &str) -> String {
    if diff.len() <= MAX_DIFF_BYTES {
        return diff.to_string();
    }
    let mut end = MAX_DIFF_BYTES;
    while !diff.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n…(diff truncated)…", &diff[..end])
}

/// Open `$VISUAL`/`$EDITOR` (falling back to `vi`) on the seeded text.
fn edit_in_editor(initial: &str) -> Result<String> {
    let temp = env::temp_dir().join(format!("git-github-pr-{}.md", std::process::id()));
    fs::write(&temp, initial)?;

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    // Split so editors configured with flags (e.g. "code --wait") still work.
    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or("vi");

    let status = Command::new(program)
        .args(parts)
        .arg(&temp)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let edited = fs::read_to_string(&temp).unwrap_or_default();
    let _ = fs::remove_file(&temp);

    if !status?.success() {
        return Err(Error::CommitCancelled);
    }
    Ok(edited)
}

/// Run a git command, capturing stdout; errors carry git's stderr.
fn git_capture(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(Error::GitCommand(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a git command with inherited stdio (so progress is visible).
fn git_run(args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err(Error::GitCommand(format!("git {}", args.join(" "))));
    }
    Ok(())
}

fn block_on<F: Future>(future: F) -> Result<F::Output> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(rt.block_on(future))
}

fn print_banner(title: &str) {
    let width = std::cmp::max(60, title.chars().count() + 8);
    let line = "=".repeat(width);
    let pad = (width - title.chars().count()) / 2;
    println!("{line}");
    println!("{}{}", " ".repeat(pad), title);
    println!("{line}");
}
