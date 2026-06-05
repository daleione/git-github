use clap::Parser;
use git_github::ai::{self, CommitMode};
use git_github::report;
use std::process::ExitCode;

/// AI commit. Commits the staged changes with an AI-generated message.
/// Pass `-a` to stage all changes first. Usable as `git ac`.
#[derive(Parser, Debug)]
#[clap(name = "git-ac", version)]
struct Cli {
    /// Stage all changes before committing (like `git add -A`)
    #[clap(short, long)]
    all: bool,

    /// Open the editor to review/edit the message before committing
    #[clap(short, long, conflicts_with = "preview")]
    edit: bool,

    /// Only preview the message; do not stage or commit
    #[clap(short, long)]
    preview: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stage = cli.all && !cli.preview;

    let mode = if cli.preview {
        CommitMode::Preview
    } else if cli.edit {
        CommitMode::Editor
    } else {
        CommitMode::Apply
    };

    report(ai::run(stage, mode))
}
