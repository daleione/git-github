use clap::Parser;
use git_github::ai;

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

fn main() {
    let cli = Cli::parse();
    let stage = !cli.no_stage && !cli.preview;

    let result = if cli.preview {
        ai::ai_commit(false, false)
    } else if cli.edit {
        ai::ai_commit_with_editor(stage)
    } else {
        ai::ai_commit(stage, true)
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
