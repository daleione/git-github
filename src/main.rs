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

    /// Generate a commit message with AI. Bare `commit` previews only.
    Commit {
        /// Stage all changes (like `git add -A`) before committing
        #[clap(short = 'a', long)]
        all: bool,

        /// Open the editor to review/edit the message before committing
        #[clap(short = 'e', long)]
        edit: bool,
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
            Commands::Commit { all, edit } => {
                let result = if *edit {
                    // -e: generate, then open editor (optionally staging first with -a)
                    llm::ai_commit_with_editor(*all)
                } else if *all {
                    // -a: stage everything, generate, and commit
                    llm::ai_commit(true, true)
                } else {
                    // bare: preview the message only, no staging, no commit
                    llm::ai_commit(false, false)
                };
                if let Err(msg) = result {
                    eprintln!("{}", msg);
                }
            }
        }
    }
}
