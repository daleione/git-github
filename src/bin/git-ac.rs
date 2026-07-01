use clap::Parser;
use git_github::ai::{self, CommitMode, StageMode};
use git_github::report;
use std::process::ExitCode;

/// AI commit. Commits the staged changes with an AI-generated message.
/// Pass `-a` to stage all changes first, or `-u` to stage tracked files only.
/// Usable as `git ac`.
#[derive(Parser, Debug)]
#[clap(name = "git-ac", version)]
struct Cli {
    /// Stage all changes before committing (like `git add -A`)
    #[clap(short, long)]
    all: bool,

    /// Stage tracked files only before committing (like `git add -u`)
    #[clap(short, long, conflicts_with = "all")]
    update: bool,

    /// Open the editor to review/edit the message before committing
    #[clap(short, long, conflicts_with = "preview")]
    edit: bool,

    /// Only preview the message; do not stage or commit
    #[clap(short, long)]
    preview: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let stage = if cli.preview {
        StageMode::None
    } else if cli.all {
        StageMode::All
    } else if cli.update {
        StageMode::Tracked
    } else {
        StageMode::None
    };

    let mode = if cli.preview {
        CommitMode::Preview
    } else if cli.edit {
        CommitMode::Editor
    } else {
        CommitMode::Apply
    };

    report(ai::run(stage, mode))
}
