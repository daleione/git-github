use std::error::Error;
use std::process::ExitCode;

pub mod ai;
mod config;
pub mod issue;
pub mod open;
mod remote;
mod repo;

/// Print an error in the conventional `error: …` form (as cargo and ripgrep do)
/// and turn it into a process failure code. Binaries route their top-level
/// result through this so every command reports failures the same way.
pub fn report(result: Result<(), Box<dyn Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
