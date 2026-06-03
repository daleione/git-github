use std::error::Error;
use std::path::PathBuf;

use crate::url::Remote;
use git2::{Delta, IndexAddOption, Patch, Repository};
use octocrab::models::issues::Issue;
use octocrab::Page;

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

        for (idx, delta) in diff.deltas().enumerate() {
            let old_path = delta.old_file().path();
            let new_path = delta.new_file().path();

            match delta.status() {
                Delta::Added => {
                    if let Some(p) = new_path {
                        changes.push_str(&format!("Staged (new): {}\n", p.display()));
                    }
                }
                Delta::Modified => {
                    if let Some(p) = new_path {
                        changes.push_str(&format!("Staged (modified): {}\n", p.display()));
                    }
                }
                Delta::Deleted => {
                    if let Some(p) = old_path {
                        changes.push_str(&format!("Staged (deleted): {}\n", p.display()));
                    }
                }
                Delta::Renamed => {
                    if let (Some(old), Some(new)) = (old_path, new_path) {
                        changes
                            .push_str(&format!("Renamed: {} -> {}\n", old.display(), new.display()));
                    }
                }
                _ => continue,
            }

            // Print only this file's patch, not the whole diff.
            if let Ok(Some(mut patch)) = Patch::from_diff(&diff, idx) {
                let buf = patch.to_buf()?;
                let text = String::from_utf8_lossy(&buf);
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
