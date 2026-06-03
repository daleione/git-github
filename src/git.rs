use std::error::Error;
use std::path::PathBuf;

use crate::url::Remote;
use git2::{Delta, IndexAddOption, Patch, Repository};
use octocrab::models::issues::Issue;
use octocrab::Page;

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

pub struct Repo {
    repository: Repository,
}

impl Repo {
    pub fn new(path: &PathBuf) -> Self {
        let mut cwd = path.clone();

        let repository = loop {
            match Repository::open(&cwd) {
                Ok(r) => break r,
                Err(_e) => {
                    if !cwd.pop() {
                        panic!("Unable to open repository at path or parent: {:?}", path);
                    }
                }
            }
        };

        Self { repository }
    }

    pub fn remote(&self, name: &str) -> Result<Remote, Box<dyn Error>> {
        let repo_remote = self.repository.find_remote(name).unwrap();
        let remote_url = repo_remote.url().unwrap();
        if let Some(remote) = Remote::parse(remote_url) {
            return Ok(remote);
        }
        Err("nop".into())
    }

    pub fn exist(&self, remote: &str, branch: &str) -> bool {
        let reference_name = format!("refs/remotes/{}/{}", remote, branch);
        self.repository.find_reference(&reference_name).is_ok()
    }

    pub fn current_branch(&self) -> Result<String, Box<dyn Error>> {
        let head = self.repository.head()?;
        if let Some(name) = head.shorthand() {
            Ok(name.to_string())
        } else {
            Err("Could not get current branch name".into())
        }
    }

    pub async fn issues(&self) -> Result<Page<Issue>, Box<dyn Error>> {
        let octocrab = octocrab::instance();
        let remote = self.remote("origin")?;
        let issue_list = octocrab
            .issues(remote.user, remote.repo)
            .list()
            .send()
            .await?;
        Ok(issue_list)
    }

    /// Stage every change in the working tree (equivalent to `git add -A`):
    /// new and modified files via `add_all`, deletions via `update_all`.
    pub fn stage_all(&self) -> Result<(), Box<dyn Error>> {
        let mut index = self.repository.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.update_all(["*"].iter(), None)?;
        index.write()?;
        Ok(())
    }

    pub fn get_staged_git_changes(&self) -> Result<String, Box<dyn Error>> {
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

        for (idx, delta) in diff.deltas().enumerate() {
            let old_path = delta.old_file().path();
            let new_path = delta.new_file().path();
            let path = new_path.or(old_path).map(|p| p.display().to_string());

            // Header line is always kept so the AI knows the file changed,
            // even when we omit its diff body below.
            match delta.status() {
                Delta::Added => match &path {
                    Some(p) => changes.push_str(&format!("Staged (new): {}\n", p)),
                    None => continue,
                },
                Delta::Modified => match &path {
                    Some(p) => changes.push_str(&format!("Staged (modified): {}\n", p)),
                    None => continue,
                },
                Delta::Deleted => match old_path {
                    Some(p) => changes.push_str(&format!("Staged (deleted): {}\n", p.display())),
                    None => continue,
                },
                Delta::Renamed => match (old_path, new_path) {
                    (Some(o), Some(n)) => changes
                        .push_str(&format!("Renamed: {} -> {}\n", o.display(), n.display())),
                    _ => continue,
                },
                _ => continue,
            }

            let path = path.unwrap_or_default();

            // Decide whether to include the diff body, and why not if omitted.
            // Everything is still committed; this only trims the AI prompt.
            if delta.new_file().is_binary() || delta.old_file().is_binary() {
                changes.push_str("(binary file, diff omitted)\n\n");
                continue;
            }
            if is_excluded(&path) {
                changes.push_str("(generated/lock file, diff omitted)\n\n");
                continue;
            }

            // Print only this file's patch, not the whole diff.
            if let Ok(Some(mut patch)) = Patch::from_diff(&diff, idx) {
                let buf = patch.to_buf()?;

                // Guard against binary content libgit2 may emit as raw bytes.
                if buf.iter().any(|&b| b == 0) {
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
            return Err("No staged changes found.".into());
        }

        Ok(changes)
    }

    pub fn commit(&self, message: &str) -> Result<String, Box<dyn Error>> {
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

        return Ok(commit_id.to_string());
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
