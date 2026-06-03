use git_github::focus;

/// List the repository's GitHub issues. Usable as `git issues`.
fn main() {
    if let Err(e) = focus::list_issues("origin") {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
