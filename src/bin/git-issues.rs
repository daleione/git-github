use git_github::{issue, report};
use std::process::ExitCode;

/// List the repository's GitHub issues. Usable as `git issues`.
fn main() -> ExitCode {
    report(issue::list_issues("origin"))
}
