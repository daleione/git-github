use clap::{Parser, ValueEnum};
use git_github::{issue, report};
use std::process::ExitCode;

/// List the repository's GitHub issues. Usable as `git issues`.
#[derive(Parser, Debug)]
#[clap(name = "git-issues", version)]
struct Cli {
    /// Which issues to list
    #[clap(short, long, value_enum, default_value_t = State::Open)]
    state: State,

    /// Remote name
    #[clap(short, long, default_value = "origin")]
    remote: String,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum State {
    Open,
    Closed,
    All,
}

impl From<State> for octocrab::params::State {
    fn from(state: State) -> Self {
        match state {
            State::Open => octocrab::params::State::Open,
            State::Closed => octocrab::params::State::Closed,
            State::All => octocrab::params::State::All,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    report(issue::list_issues(&cli.remote, cli.state.into()))
}
