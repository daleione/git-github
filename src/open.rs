use std::env;
use crate::git;

pub fn open_url() {
    let p = env::current_dir().unwrap();
    let repo = git::Repo::new(&p);
    if let Ok(url) = repo.remote_url() {
        open::that(url).unwrap();
    } else {
        println!("url not exist");
    }
}
