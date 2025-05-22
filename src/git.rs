use std::error::Error;
use std::path::PathBuf;

use crate::url::Remote;
use git2::{DiffFormat, Repository, StatusOptions};
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

    pub fn get_staged_git_changes(&self) -> Result<String, Box<dyn Error>> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(false);
        opts.include_ignored(false);
        opts.include_unmodified(false);

        let statuses = self.repository.statuses(Some(&mut opts))?;
        let mut changes = String::new();

        let head_tree = self.repository.head().ok().and_then(|h| h.peel_to_tree().ok());

        let diff = self
            .repository
            .diff_tree_to_index(head_tree.as_ref(), None, None)?;

        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry.path().unwrap_or("");

            if !(status.is_index_new() || status.is_index_modified() || status.is_index_deleted()) {
                continue;
            }

            let status_desc = if status.is_index_new() {
                "Staged (new)"
            } else if status.is_index_modified() {
                "Staged (modified)"
            } else if status.is_index_deleted() {
                "Staged (deleted)"
            } else {
                continue;
            };

            changes.push_str(&format!("{}: {}\n", status_desc, path));

            for delta in diff.deltas() {
                let delta_path = delta.new_file().path().or_else(|| delta.old_file().path());
                if delta_path == Some(std::path::Path::new(path)) {
                    let mut diff_text = String::new();
                    diff.print(DiffFormat::Patch, |_, _, line| {
                        diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
                        true
                    })?;
                    if !diff_text.is_empty() {
                        changes.push_str(&format!("\n{}\n", diff_text));
                    }
                    break;
                }
            }
        }

        if changes.trim().is_empty() {
            changes = "No staged changes found.".to_string();
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
