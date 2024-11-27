use clap::{Parser, Subcommand};

use git_github::open;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Open the repo website in your browser.
    Open {
        #[clap(short, long, value_parser, conflicts_with_all = &["branch"])]
        commit: Option<String>,

        #[clap(short, long, value_parser)]
        branch: Option<String>,

        #[clap(short, long, value_parser, default_value = "origin")]
        remote: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Open {
            commit,
            branch,
            remote,
        }) => {
            if let Some(commit) = commit {
                open::open(remote, open::OpenTarget::Commit(commit.to_string()));
            } else if let Some(branch) = branch {
                open::open(remote, open::OpenTarget::Branch(branch.to_string()));
            } else {
                open::open(remote, open::OpenTarget::Remote);
            }
        }
        None => {}
    }
}
