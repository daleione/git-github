use crate::error::{Error, Result};
use crate::repo::Repo;
use std::env;

pub fn list_issues(remote_name: &str) -> Result<()> {
    let path = env::current_dir().map_err(|_| Error::NoCurrentDir)?;
    let repo = Repo::new(&path)?;
    let remote = repo.remote(remote_name)?;

    let rt = tokio::runtime::Runtime::new()?;
    let issues = rt.block_on(async {
        octocrab::instance()
            .issues(remote.user, remote.repo)
            .list()
            .send()
            .await
    })?;

    for issue in issues {
        println!("#{}: {}", issue.number, issue.title);
    }
    Ok(())
}
