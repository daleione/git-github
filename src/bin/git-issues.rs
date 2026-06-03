use git_github::issue;

/// List the repository's GitHub issues. Usable as `git issues`.
fn main() {
    if let Err(e) = issue::list_issues("origin") {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
