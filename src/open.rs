use crate::git;
use std::env;

pub enum OpenTarget {
    Remote,
    Commit(String),
    Branch(String),
}

fn get_remote(remote_name: &str) -> Result<crate::url::Remote, String> {
    let path = env::current_dir().map_err(|_| "Failed to get current directory")?;
    let repo = git::Repo::new(&path);
    repo.remote(remote_name)
        .map_err(|_| format!("Error: Remote '{}' not found", remote_name))
}

pub fn open(remote_name: &str, target: OpenTarget) {
    let path = env::current_dir().unwrap_or_else(|_| {
        eprintln!("Failed to get current directory");
        Default::default()
    });
    let repo = git::Repo::new(&path);

    if let OpenTarget::Branch(branch_name) = &target {
        if !repo.exist(remote_name, branch_name) {
            eprintln!(
                "Error: Branch '{}' not found in remote '{}'",
                branch_name, remote_name
            );
            return;
        }
    }

    // An explicit -c/-b target is honored as-is. A bare `open` (Remote)
    // defaults to the current branch when on one, else the repo homepage.
    let target = match target {
        OpenTarget::Remote => match repo.current_branch() {
            Ok(current_branch) => OpenTarget::Branch(current_branch),
            Err(_) => OpenTarget::Remote,
        },
        other => other,
    };

    match get_remote(remote_name) {
        Ok(remote) => {
            let url = match target {
                OpenTarget::Remote => remote.get_repo_url(),
                OpenTarget::Commit(commit_id) => remote.get_commit_url(&commit_id),
                OpenTarget::Branch(branch_name) => remote.get_branch_url(&branch_name),
            };
            open::that(url).unwrap_or_else(|_| eprintln!("Failed to open URL"))
        }
        Err(e) => eprintln!("{}", e),
    }
}
