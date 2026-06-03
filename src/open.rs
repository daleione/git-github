use crate::repo::Repo;
use std::env;
use std::error::Error;

pub enum OpenTarget {
    Remote,
    Commit(String),
    Branch(String),
}

pub fn open(remote_name: &str, target: OpenTarget) -> Result<(), Box<dyn Error>> {
    let path = env::current_dir().map_err(|_| "failed to get the current directory")?;
    let repo = Repo::new(&path)?;
    let remote = repo.remote(remote_name)?;

    // An explicit -b is validated against the remote; an explicit -c is taken
    // as-is. A bare `open` (Remote) defaults to the current branch when on one,
    // else the repo homepage.
    let target = match target {
        OpenTarget::Branch(branch_name) => {
            if !repo.exist(remote_name, &branch_name) {
                return Err(format!(
                    "branch '{}' not found in remote '{}'",
                    branch_name, remote_name
                )
                .into());
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
