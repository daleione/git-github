use clap::{Parser, Subcommand};

use git_github::{focus, llm, open};

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
    /// issues
    Issue {
        #[command(subcommand)]
        issue_command: IssueCommands,
    },

    /// auto commit with message gened by ai
    Commit {
        /// apply the message to the new commit
        #[clap(short, long, default_value_t = false)]
        apply: bool,
    },
}

#[derive(Subcommand, Debug)]
enum IssueCommands {
    /// Focus on a specific issue
    Focus {
        #[clap(short, long, value_parser)]
        issue_id: i64,
    },
    /// List all issues
    List,
}

fn main() {
    let cli = Cli::parse();
    if let Some(command) = &cli.command {
        match command {
            Commands::Open {
                commit,
                branch,
                remote,
            } => {
                if let Some(commit) = commit {
                    open::open(remote, open::OpenTarget::Commit(commit.to_string()));
                } else if let Some(branch) = branch {
                    open::open(remote, open::OpenTarget::Branch(branch.to_string()));
                } else {
                    open::open(remote, open::OpenTarget::Remote);
                }
            }
            Commands::Issue {
                issue_command: focus_command,
            } => match focus_command {
                IssueCommands::Focus { issue_id } => {
                    println!("Focusing on issue {}", issue_id);
                }
                IssueCommands::List => {
                    println!("Listing all issues...");
                    let _ = focus::list_issues("origin");
                }
            },
            Commands::Commit { apply } => {
                if let Err(msg) = llm::ai_commit(*apply) {
                    eprint!("{:?}", msg);
                }
            }
        }
    }
}
