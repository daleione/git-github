use crate::error::Result;
use octocrab::Octocrab;
use std::env;

/// A GitHub token from the environment, if set. Enables private repos and
/// lifts the unauthenticated rate limit (60/hr).
pub fn token() -> Option<String> {
    env::var("GITHUB_TOKEN")
        .or_else(|_| env::var("GH_TOKEN"))
        .ok()
        .filter(|t| !t.is_empty())
}

/// An authenticated client when a token is present, otherwise the shared
/// anonymous instance (sufficient for reading public data).
pub fn client() -> Result<Octocrab> {
    match token() {
        Some(token) => Ok(Octocrab::builder().personal_token(token).build()?),
        None => Ok((*octocrab::instance()).clone()),
    }
}
