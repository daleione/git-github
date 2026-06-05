use crate::error::{Error, Result};
use crate::repo::Repo;
use std::env;
use std::path::Path;

pub enum OpenTarget {
    Remote,
    Commit(String),
    Branch(String),
    File {
        path: String,
        start: Option<u32>,
        end: Option<u32>,
    },
}

/// Parse a `path[:line[-line]]` argument into its parts. A trailing `:`-segment
/// is only treated as a line/range when it is fully numeric, so paths
/// containing a colon are left untouched.
pub fn parse_file_arg(arg: &str) -> (String, Option<u32>, Option<u32>) {
    if let Some((path, loc)) = arg.rsplit_once(':') {
        if let Some((start, end)) = loc.split_once('-') {
            if let (Ok(start), Ok(end)) = (start.parse(), end.parse()) {
                return (path.to_string(), Some(start), Some(end));
            }
        } else if let Ok(start) = loc.parse() {
            return (path.to_string(), Some(start), None);
        }
    }
    (arg.to_string(), None, None)
}

pub fn open(remote_name: &str, target: OpenTarget) -> Result<()> {
    let path = env::current_dir().map_err(|_| Error::NoCurrentDir)?;
    let repo = Repo::new(&path)?;
    let remote = repo.remote(remote_name)?;

    // An explicit -b is validated against the remote; an explicit -c is taken
    // as-is. A bare `open` (Remote) defaults to the current branch when on one,
    // else the repo homepage.
    let target = match target {
        OpenTarget::Branch(branch_name) => {
            if !repo.exist(remote_name, &branch_name) {
                return Err(Error::BranchNotFound {
                    branch: branch_name,
                    remote: remote_name.to_string(),
                });
            }
            OpenTarget::Branch(branch_name)
        }
        OpenTarget::Remote => match repo.current_branch() {
            Ok(current_branch) => OpenTarget::Branch(current_branch),
            Err(_) => OpenTarget::Remote,
        },
        passthrough => passthrough,
    };

    let url = match target {
        OpenTarget::Remote => remote.get_repo_url(),
        OpenTarget::Commit(commit_id) => remote.get_commit_url(&commit_id),
        OpenTarget::Branch(branch_name) => remote.get_branch_url(&branch_name),
        OpenTarget::File { path, start, end } => {
            // Anchor to the current branch (or commit when detached) so the link
            // points at what the user is looking at.
            let reference = repo
                .current_branch()
                .or_else(|_| repo.head_commit_id())?;
            let relative = repo.workdir_relative(Path::new(&path))?;
            remote.get_file_url(&reference, &relative, start.map(|s| (s, end)))
        }
    };

    open::that(url)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::parse_file_arg;

    #[test]
    fn parses_path_line_and_range() {
        assert_eq!(parse_file_arg("src/main.rs"), ("src/main.rs".into(), None, None));
        assert_eq!(
            parse_file_arg("src/main.rs:42"),
            ("src/main.rs".into(), Some(42), None)
        );
        assert_eq!(
            parse_file_arg("src/main.rs:40-50"),
            ("src/main.rs".into(), Some(40), Some(50))
        );
    }

    #[test]
    fn non_numeric_suffix_is_part_of_the_path() {
        // A colon followed by non-digits is not a line spec.
        assert_eq!(parse_file_arg("a:b.rs"), ("a:b.rs".into(), None, None));
        assert_eq!(parse_file_arg("src/main.rs:"), ("src/main.rs:".into(), None, None));
    }
}
