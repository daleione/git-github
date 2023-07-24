use clap::{Parser, Subcommand};

use git_github;

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
    /// does testing things
    Open {
        #[clap(short, long, value_parser, conflicts_with_all = &["branch"])]
        commit: bool,

        #[clap(short, long, value_parser)]
        branch: bool,

        #[clap(short, long, value_parser, default_value = "origin")]
        remote_name: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Open { .. }) => {
            git_github::open::open_url();
        }
        None => {}
    }
}
