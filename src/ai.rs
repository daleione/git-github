use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::llm::{self, ChatMessage};
use crate::repo::Repo;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

use crate::style;

/// Report a successful commit as `✓ Committed <short-sha> on <branch>`.
fn report_commit(repo: &Repo) -> Result<()> {
    let short = repo.head_short_id()?;
    let line = match repo.current_branch() {
        Ok(branch) => format!("Committed {} {}", style::cyan(&short), style::dim(&format!("on {branch}"))),
        Err(_) => format!("Committed {}", style::cyan(&short)),
    };
    style::success(&line);
    Ok(())
}

/// Shared setup for every commit entry point: open the repo, optionally stage,
/// and load a config with a usable API key.
fn prepare(stage: bool) -> Result<(Repo, String, AppConfig)> {
    let path = env::current_dir().map_err(|_| Error::NoCurrentDir)?;
    let repo = Repo::new(&path)?;

    if stage {
        repo.stage_all()?;
    }

    let changes = repo.get_staged_git_changes()?;
    let config = crate::config::load_config()?;
    if config.deepseek.api_key.is_empty() {
        return Err(Error::NoApiKey);
    }

    Ok((repo, changes, config))
}

/// What to do with the AI-generated message once it has been produced.
pub enum CommitMode {
    Preview,
    Apply,
    Editor,
}

/// What the user chose to do with a generated message in interactive mode.
enum Action {
    Commit,
    Edit,
    Regenerate,
    Abort,
}

/// Generate a commit message from the staged changes and act on it per `mode`.
pub fn run(stage: bool, mode: CommitMode) -> Result<()> {
    let (repo, changes, config) = prepare(stage)?;
    let model = config.deepseek.model.as_deref().unwrap_or("deepseek-chat");

    // In Apply mode on a TTY, let the user review the message before it lands
    // (accept / edit / regenerate / abort). Piped input keeps the old
    // commit-immediately behavior so scripts are unaffected.
    let interactive = matches!(mode, CommitMode::Apply) && io::stdin().is_terminal();

    // Extra instructions accumulated from "regenerate" guidance.
    let mut guidance: Vec<String> = Vec::new();

    loop {
        let title = match mode {
            CommitMode::Editor => "Generating commit message",
            _ => "Suggested commit message",
        };

        let mut messages = build_prompt_messages(&changes, config.deepseek.prompt.clone());
        for hint in &guidance {
            messages.push(ChatMessage::user(format!(
                "Please revise the commit message: {hint}"
            )));
        }

        let message = llm::stream_and_collect(
            title,
            &config.deepseek.api_key,
            model,
            messages,
            config.deepseek.temperature,
        )?;

        if !matches!(mode, CommitMode::Preview) && message.trim().is_empty() {
            return Err(Error::EmptyMessage);
        }

        match mode {
            CommitMode::Preview => return Ok(()),
            CommitMode::Editor => {
                commit_via_git(&message, true)?;
                report_commit(&repo)?;
                return Ok(());
            }
            CommitMode::Apply if !interactive => {
                commit_via_git(&message, false)?;
                report_commit(&repo)?;
                return Ok(());
            }
            CommitMode::Apply => match prompt_action()? {
                Action::Commit => {
                    commit_via_git(&message, false)?;
                    report_commit(&repo)?;
                    return Ok(());
                }
                Action::Edit => {
                    commit_via_git(&message, true)?;
                    report_commit(&repo)?;
                    return Ok(());
                }
                Action::Regenerate => {
                    let hint = prompt_line("Any guidance for the rewrite? (optional): ")?
                        .unwrap_or_default();
                    if !hint.is_empty() {
                        guidance.push(hint);
                    }
                }
                Action::Abort => {
                    println!("Aborted; nothing committed.");
                    return Ok(());
                }
            },
        }
    }
}

/// Ask what to do with the generated message, repeating on invalid input.
fn prompt_action() -> Result<Action> {
    // The prompt is transient: a blank line sets it apart from the message
    // above, and we tally every line it draws so the whole block can be erased
    // once the user decides — leaving only the outcome on screen.
    println!();
    let mut drawn = 1;
    loop {
        // EOF (Ctrl-D) aborts; leave the prompt visible since there's no echo.
        let Some(choice) = prompt_line(&style::prompt(
            "Commit this message?",
            "[Y]es / [e]dit / [r]egenerate / [a]bort",
        ))?
        else {
            return Ok(Action::Abort);
        };
        drawn += 1; // the prompt line, with the user's echoed answer
        let action = match choice.to_lowercase().as_str() {
            "" | "y" | "yes" => Action::Commit,
            "e" | "edit" => Action::Edit,
            "r" | "regenerate" => Action::Regenerate,
            "a" | "abort" | "q" | "quit" => Action::Abort,
            _ => {
                println!("Please enter Y, e, r, or a.");
                drawn += 1;
                continue;
            }
        };
        style::erase_lines(drawn);
        return Ok(action);
    }
}

/// Print `prompt` and read a trimmed line from stdin. Returns `None` on EOF.
fn prompt_line(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Ok(None);
    }
    Ok(Some(input.trim().to_string()))
}

/// Commit the staged changes via `git commit -F`, so pre-commit/commit-msg
/// hooks and signing run (libgit2 would skip them). When `edit` is set, `-e`
/// opens the editor on the seeded message first.
fn commit_via_git(message: &str, edit: bool) -> Result<()> {
    // Process-unique name so concurrent runs don't clobber each other.
    let temp_file = env::temp_dir().join(format!("git-github-commit-{}.txt", std::process::id()));
    fs::write(&temp_file, message.trim())?;

    let mut command = Command::new("git");
    // `-q` suppresses git's own `[branch sha] summary` + diffstat; we print a
    // clean success line ourselves.
    command.args(["commit", "-q"]);
    if edit {
        style::header("Opening editor for review");
        // `-e` opens the editor to edit the seeded message; `-F` (below) does
        // not abort when the message is left unchanged, unlike `--template`.
        command.args(["-e", "-v"]);
    }
    let status = command
        .arg("-F")
        .arg(&temp_file)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    // Clean up regardless of how git exited.
    let _ = fs::remove_file(&temp_file);

    if !status?.success() {
        return Err(Error::CommitCancelled);
    }

    Ok(())
}

fn build_prompt_messages(changes: &str, prompt_opt: Option<String>) -> Vec<ChatMessage> {
    let prompt = prompt_opt.unwrap_or_else(|| {
        r#"
You are an AI commit message assistant.

Please generate a commit message with the following format:
1. Title (one short sentence, 50-72 characters max).
2. A clear bullet-point list of changes (start each line with "- ").
3. Each line, including bullets, should be under 100 characters.
4. Keep it concise, consistent, and professional.

Example:

Improve error handling in user authentication

- Add detailed error messages for login failures
- Handle timeout errors gracefully
- Refactor error propagation logic for clarity
"#
        .to_string()
    });

    vec![
        ChatMessage::system(prompt),
        ChatMessage::user(format!("Here are my current Git changes:\n{}", changes)),
    ]
}
