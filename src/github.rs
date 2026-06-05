use crate::error::Result;
use octocrab::Octocrab;
use std::env;
use std::process::Command;

/// A GitHub token, from `GITHUB_TOKEN`/`GH_TOKEN`, or the `gh` CLI as a
/// fallback. Enables private repos and lifts the unauthenticated rate limit
/// (60/hr).
pub fn token() -> Option<String> {
    env::var("GITHUB_TOKEN")
        .or_else(|_| env::var("GH_TOKEN"))
        .ok()
        .filter(|t| !t.is_empty())
        .or_else(gh_cli_token)
}

/// Ask the `gh` CLI for its stored token, so a machine authenticated with
/// `gh auth login` works without exporting an env var.
fn gh_cli_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// An authenticated client when a token is present, otherwise the shared
/// anonymous instance (sufficient for reading public data).
pub fn client() -> Result<Octocrab> {
    match token() {
        Some(token) => Ok(Octocrab::builder().personal_token(token).build()?),
        None => Ok((*octocrab::instance()).clone()),
    }
}
