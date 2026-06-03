use crate::error::{Error, Result};
use crate::repo::Repo;
use std::env;

pub enum OpenTarget {
    Remote,
    Commit(String),
    Branch(String),
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
        commit => commit,
    };

    let url = match target {
        OpenTarget::Remote => remote.get_repo_url(),
        OpenTarget::Commit(commit_id) => remote.get_commit_url(&commit_id),
        OpenTarget::Branch(branch_name) => remote.get_branch_url(&branch_name),
    };

    open::that(url)?;
    Ok(())
}
