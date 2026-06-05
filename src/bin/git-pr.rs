use clap::Parser;
use git_github::pr::{self, Options};
use git_github::report;
use std::process::ExitCode;

/// Create a GitHub pull request for the current branch with an AI-generated
/// title and description. Usable as `git pr`.
#[derive(Parser, Debug)]
#[clap(name = "git-pr", version)]
struct Cli {
    /// Base branch to merge into (defaults to the repo's default branch)
    #[clap(short, long)]
    base: Option<String>,

    /// Create the pull request as a draft
    #[clap(short, long)]
    draft: bool,

    /// Open the editor to review/edit the title and body before creating
    #[clap(short, long)]
    edit: bool,

    /// Remote name
    #[clap(short, long, default_value = "origin")]
    remote: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    report(pr::create(Options {
        remote: cli.remote,
        base: cli.base,
        draft: cli.draft,
        edit: cli.edit,
    }))
}
