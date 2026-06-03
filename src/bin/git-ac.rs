use clap::Parser;
use git_github::ai::{self, CommitMode};
use git_github::report;
use std::process::ExitCode;

/// AI commit. By default stages all changes and commits with an AI-generated
/// message. Usable as `git ac`.
#[derive(Parser, Debug)]
#[clap(name = "git-ac", version)]
struct Cli {
    /// Open the editor to review/edit the message before committing
    #[clap(short, long, conflicts_with = "preview")]
    edit: bool,

    /// Only preview the message; do not stage or commit
    #[clap(short, long)]
    preview: bool,

    /// Do not stage; commit only what is already staged
    #[clap(short = 'n', long)]
    no_stage: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stage = !cli.no_stage && !cli.preview;

    let mode = if cli.preview {
        CommitMode::Preview
    } else if cli.edit {
        CommitMode::Editor
    } else {
        CommitMode::Apply
    };

    report(ai::run(stage, mode))
}
