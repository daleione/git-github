use std::process::ExitCode;

pub mod ai;
mod config;
pub mod error;
mod github;
pub mod issue;
mod llm;
pub mod open;
pub mod pr;
mod remote;
mod repo;
mod style;

pub use error::{Error, Result};

/// Print an error as `error: …` (cargo/ripgrep style) and map it to a failure
/// exit code. Every binary funnels its top-level result through this.
pub fn report(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
