use std::path::Path;

use crate::error::{Error, Result};
use crate::remote::Remote;
use git2::{Delta, DiffDelta, IndexAddOption, Patch, Repository};

/// Per-file diff size cap sent to the AI. Larger diffs are omitted.
const MAX_FILE_DIFF_BYTES: usize = 16 * 1024;
/// Overall cap across all files in one commit.
const MAX_TOTAL_DIFF_BYTES: usize = 64 * 1024;

/// Files whose diff body is noise for a commit message (lock files, generated
/// or minified output). They are still committed; only the diff is omitted.
fn is_excluded(path: &str) -> bool {
    const SKIP_NAMES: &[&str] = &[
        "Cargo.lock",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "composer.lock",
        "Gemfile.lock",
        "poetry.lock",
        "go.sum",
    ];
    const SKIP_SUFFIXES: &[&str] = &[".lock", ".min.js", ".min.css", ".map"];

    let name = path.rsplit('/').next().unwrap_or(path);
    SKIP_NAMES.contains(&name) || SKIP_SUFFIXES.iter().any(|s| path.ends_with(s))
}

/// The header line for a staged file, or `None` for statuses we don't report.
fn status_header(delta: &DiffDelta) -> Option<String> {
    let old = delta.old_file().path();
    let new = delta.new_file().path();
    match delta.status() {
        Delta::Added => new.map(|p| format!("Staged (new): {}\n", p.display())),
        Delta::Modified => new.map(|p| format!("Staged (modified): {}\n", p.display())),
        Delta::Deleted => old.map(|p| format!("Staged (deleted): {}\n", p.display())),
        Delta::Renamed => match (old, new) {
            (Some(o), Some(n)) => Some(format!("Renamed: {} -> {}\n", o.display(), n.display())),
            _ => None,
        },
        _ => None,
    }
}

pub struct Repo {
    repository: Repository,
}

impl Repo {
    /// Open the repository at `path`, walking up to parent directories.
    pub fn new(path: &Path) -> Result<Self> {
        let mut cwd = path.to_path_buf();

        let repository = loop {
            match Repository::open(&cwd) {
                Ok(r) => break r,
                Err(_) => {
                    if !cwd.pop() {
                        return Err(Error::NotARepo(path.to_path_buf()));
                    }
                }
            }
        };

        Ok(Self { repository })
    }

    pub fn remote(&self, name: &str) -> Result<Remote> {
        let repo_remote = self
            .repository
            .find_remote(name)
            .map_err(|_| Error::RemoteNotFound(name.to_string()))?;
        let remote_url = repo_remote.url().ok_or(Error::RemoteUrlNotUtf8)?;
        Remote::parse(remote_url).ok_or_else(|| Error::RemoteUrlParse(remote_url.to_string()))
    }

    pub fn exist(&self, remote: &str, branch: &str) -> bool {
        let reference_name = format!("refs/remotes/{}/{}", remote, branch);
        self.repository.find_reference(&reference_name).is_ok()
    }

    pub fn current_branch(&self) -> Result<String> {
        let head = self.repository.head()?;
        if let Some(name) = head.shorthand() {
            Ok(name.to_string())
        } else {
            Err(Error::NoCurrentBranch)
        }
    }

    /// Stage every change in the working tree (equivalent to `git add -A`):
    /// new and modified files via `add_all`, deletions via `update_all`.
    pub fn stage_all(&self) -> Result<()> {
        let mut index = self.repository.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.update_all(["*"].iter(), None)?;
        index.write()?;
        Ok(())
    }

    pub fn get_staged_git_changes(&self) -> Result<String> {
        let head_tree = self
            .repository
            .head()
            .ok()
            .and_then(|h| h.peel_to_tree().ok());

        // Tree(HEAD) -> index is exactly the set of staged changes.
        let diff = self
            .repository
            .diff_tree_to_index(head_tree.as_ref(), None, None)?;

        let mut changes = String::new();
        let mut total_bytes = 0usize;
        let mut capped = false;

        for (idx, delta) in diff.deltas().enumerate() {
            // The header is kept even when the diff body is omitted below, so
            // the AI still sees that the file changed. Everything is committed
            // regardless; this only trims what we send to the model.
            let Some(header) = status_header(&delta) else {
                continue;
            };
            changes.push_str(&header);

            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            if delta.new_file().is_binary() || delta.old_file().is_binary() {
                changes.push_str("(binary file, diff omitted)\n\n");
                continue;
            }
            if is_excluded(&path) {
                changes.push_str("(generated/lock file, diff omitted)\n\n");
                continue;
            }
            if capped {
                changes.push_str("(diff omitted: total size limit reached)\n\n");
                continue;
            }

            if let Ok(Some(mut patch)) = Patch::from_diff(&diff, idx) {
                let buf = patch.to_buf()?;

                // Guard against binary content libgit2 may emit as raw bytes.
                if buf.contains(&0) {
                    changes.push_str("(binary file, diff omitted)\n\n");
                    continue;
                }

                let text = String::from_utf8_lossy(&buf);
                if text.len() > MAX_FILE_DIFF_BYTES {
                    changes.push_str(&format!(
                        "(diff omitted: {} bytes exceeds per-file limit)\n\n",
                        text.len()
                    ));
                    continue;
                }
                if total_bytes + text.len() > MAX_TOTAL_DIFF_BYTES {
                    capped = true;
                    changes.push_str("(diff omitted: total size limit reached)\n\n");
                    continue;
                }

                total_bytes += text.len();
                if !text.trim().is_empty() {
                    changes.push_str(&format!("\n{}\n", text));
                }
            }
        }

        if changes.trim().is_empty() {
            return Err(Error::NoStagedChanges);
        }

        Ok(changes)
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        let mut index = self.repository.index()?;
        let tree_id = index.write_tree()?;
        let tree = self.repository.find_tree(tree_id)?;

        let signature = self.repository.signature()?;

        let parent_commit = self.repository.head()?.peel_to_commit()?;

        let commit_id = self.repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(commit_id.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::is_excluded;

    #[test]
    fn excludes_lock_and_generated_files() {
        assert!(is_excluded("Cargo.lock"));
        assert!(is_excluded("frontend/package-lock.json"));
        assert!(is_excluded("dist/app.min.js"));
        assert!(is_excluded("dist/app.min.css"));
        assert!(is_excluded("bundle.js.map"));
        assert!(is_excluded("custom.lock"));
    }

    #[test]
    fn keeps_normal_source_files() {
        assert!(!is_excluded("src/main.rs"));
        assert!(!is_excluded("README.md"));
        assert!(!is_excluded("locksmith.rs"));
    }
}
