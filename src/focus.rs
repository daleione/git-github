use crate::git;
use std::env;
use std::error::Error;

pub fn list_issues(remote_name: &str) -> Result<(), Box<dyn Error>> {
    let path = env::current_dir().map_err(|_| "Failed to get current directory")?;
    let repo = git::Repo::new(&path);
    let rt = tokio::runtime::Runtime::new()?;
    let issues = rt.block_on(async { repo.issues().await })?;
    for issue in issues {
        println!("{:?}", issue.title);
    }
    Ok(())
}
