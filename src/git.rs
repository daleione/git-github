use std::error::Error;
use std::path::PathBuf;

use git2::Repository;


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

        Self {
            repository,
        }
    }

    pub fn remote_url(&self) -> Result<String, Box<dyn Error>> {
        let remote = self.repository.find_remote("origin").unwrap();
        let remote_url = remote.url().unwrap();
        return Ok(remote_url.into());
    }
}
