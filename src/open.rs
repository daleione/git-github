use crate::git;
use std::env;

pub fn open_remote(name: &str) {
    let p = env::current_dir().unwrap();
    let remote = git::Repo::new(&p).remote(name);
    if let Ok(remote) = remote {
        open::that(remote.get_repo_url()).unwrap();
    } else {
        println!("url not exist");
    }
}

pub fn open_commit(name: &str, commit: &str) {
    let p = env::current_dir().unwrap();
    let remote = git::Repo::new(&p).remote(name);
    if let Ok(remote) = remote {
        open::that(remote.get_commit_url(commit)).unwrap();
    } else {
        println!("url not exist");
    }
}

pub fn open_branch(name: &str, branch: &str) {
    let p = env::current_dir().unwrap();
    let remote = git::Repo::new(&p).remote(name);
    if let Ok(remote) = remote {
        open::that(remote.get_branch_url(branch)).unwrap();
    } else {
        println!("url not exist");
    }
}
