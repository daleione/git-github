use crate::git;
use std::env;

fn get_remote(remote_name: &str) -> Result<crate::url::Remote, String> {
    let path = env::current_dir().map_err(|_| "Failed to get current directory")?;
    let repo = git::Repo::new(&path);
    repo.remote(remote_name)
        .map_err(|_| format!("Error: Remote '{}' not found", remote_name))
}

pub fn open_remote(name: &str) {
    match get_remote(name) {
        Ok(remote) => open::that(remote.get_repo_url())
            .unwrap_or_else(|_| eprintln!("Failed to open URL")),
        Err(e) => eprintln!("{}", e),
    }
}

pub fn open_commit(remote_name: &str, commit_id: &str) {
    match get_remote(remote_name) {
        Ok(remote) => open::that(remote.get_commit_url(commit_id))
            .unwrap_or_else(|_| eprintln!("Failed to open URL")),
        Err(e) => eprintln!("{}", e),
    }
}

pub fn open_branch(remote_name: &str, branch_name: &str) {
    let path = env::current_dir().unwrap_or_else(|_| {
        eprintln!("Failed to get current directory");
        return Default::default();
    });
    let repo = git::Repo::new(&path);

    if !repo.exist(remote_name, branch_name) {
        eprintln!("Error: Branch '{}' not found in remote '{}'", branch_name, remote_name);
        return;
    }

    match get_remote(remote_name) {
        Ok(remote) => open::that(remote.get_branch_url(branch_name))
            .unwrap_or_else(|_| eprintln!("Failed to open URL")),
        Err(e) => eprintln!("{}", e),
    }
}
