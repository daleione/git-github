use crate::error::{Error, Result};
use crate::github;
use crate::repo::Repo;
use octocrab::params;
use std::env;

pub fn list_issues(remote_name: &str, state: params::State) -> Result<()> {
    let path = env::current_dir().map_err(|_| Error::NoCurrentDir)?;
    let repo = Repo::new(&path)?;
    let remote = repo.remote(remote_name)?;

    let client = github::client()?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let first = client
            .issues(&remote.user, &remote.repo)
            .list()
            .state(state)
            .per_page(100u8)
            .send()
            .await?;

        // Follow pagination so all issues are listed, not just the first page.
        let issues = client.all_pages(first).await?;

        // The issues endpoint also returns pull requests; drop them.
        for issue in issues.into_iter().filter(|i| i.pull_request.is_none()) {
            println!("#{}: {}", issue.number, issue.title);
        }

        Ok::<(), Error>(())
    })
}
