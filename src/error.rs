use std::fmt;
use std::path::PathBuf;

/// Every error the CLI can surface, as one concrete type. Domain variants carry
/// their own message; the rest wrap a library error. The top-level reporter
/// renders any of these as a single `error: …` line.
#[derive(Debug)]
pub enum Error {
    // --- Domain errors (our own messages) ---
    NotARepo(PathBuf),
    RemoteNotFound(String),
    RemoteUrlNotUtf8,
    RemoteUrlParse(String),
    NoCurrentBranch,
    BranchNotFound { branch: String, remote: String },
    NoStagedChanges,
    NoApiKey,
    ApiError(String),
    CommitCancelled,
    NoCurrentDir,
    NoHomeDir,

    // --- Wrapped library errors ---
    Git(git2::Error),
    Io(std::io::Error),
    Http(reqwest::Error),
    Config(config::ConfigError),
    GitHub(octocrab::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotARepo(p) => {
                write!(f, "not a git repository (or any parent): {}", p.display())
            }
            Error::RemoteNotFound(name) => write!(f, "remote '{}' not found", name),
            Error::RemoteUrlNotUtf8 => write!(f, "remote URL is not valid UTF-8"),
            Error::RemoteUrlParse(url) => write!(f, "could not parse remote URL: {}", url),
            Error::NoCurrentBranch => write!(f, "could not determine the current branch"),
            Error::BranchNotFound { branch, remote } => {
                write!(f, "branch '{}' not found in remote '{}'", branch, remote)
            }
            Error::NoStagedChanges => write!(f, "no staged changes found"),
            Error::NoApiKey => write!(
                f,
                "no DeepSeek API key found; set `api_key` in ~/.config/git-github/config.toml"
            ),
            Error::ApiError(msg) => write!(f, "DeepSeek API error: {}", msg),
            Error::CommitCancelled => write!(f, "git commit was cancelled or failed"),
            Error::NoCurrentDir => write!(f, "failed to get the current directory"),
            Error::NoHomeDir => write!(f, "could not determine the home directory"),
            Error::Git(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::Http(e) => write!(f, "{}", e),
            Error::Config(e) => write!(f, "{}", e),
            Error::GitHub(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Git(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::Http(e) => Some(e),
            Error::Config(e) => Some(e),
            Error::GitHub(e) => Some(e),
            _ => None,
        }
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Error::Git(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}

impl From<config::ConfigError> for Error {
    fn from(e: config::ConfigError) -> Self {
        Error::Config(e)
    }
}

impl From<octocrab::Error> for Error {
    fn from(e: octocrab::Error) -> Self {
        Error::GitHub(e)
    }
}
