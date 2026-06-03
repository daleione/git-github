use clap::Parser;
use git_github::open::{self, OpenTarget};
use git_github::report;
use std::process::ExitCode;

/// Open the GitHub repo page in your browser. Usable as `git open`.
#[derive(Parser, Debug)]
#[clap(name = "git-open", version)]
struct Cli {
    /// Open a specific commit (conflicts with --branch)
    #[clap(short, long, conflicts_with = "branch")]
    commit: Option<String>,

    /// Open a specific branch
    #[clap(short, long)]
    branch: Option<String>,

    /// Remote name
    #[clap(short, long, default_value = "origin")]
    remote: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let target = if let Some(commit) = cli.commit {
        OpenTarget::Commit(commit)
    } else if let Some(branch) = cli.branch {
        OpenTarget::Branch(branch)
    } else {
        OpenTarget::Remote
    };

    report(open::open(&cli.remote, target))
}
