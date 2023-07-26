use std::error::Error;
use std::path::PathBuf;

use git2::Repository;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until, take_while};
use nom::sequence::{terminated, tuple};
use nom::IResult;

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

        Self { repository }
    }

    pub fn remote_url(&self) -> Result<String, Box<dyn Error>> {
        let remote = self.repository.find_remote("origin").unwrap();
        let remote_url = remote.url().unwrap();
        if let Some(remote) = Remote::parse(remote_url) {
            return Ok(remote.get_http_url());
        }
        return Err("nop".into())
    }
}

/// Contains the address of the git repository.
/// git remote url can be two format:
/// * git@xxx.com:user/repo.git
//  * https://xxx.com/user/repo.git
#[derive(Debug, Default)]
pub struct Remote {
    schema: String,
    host: String,
    username: String,
    reponame: String,
}

fn schema_parser(input: &str) -> IResult<&str, &str> {
    terminated(
        alt((tag("git"), tag("https"), tag("http"))),
        take_while(|c| c == '@' || c == ':' || c == '/'),
    )(input)
}

fn host_parser(input: &str) -> IResult<&str, &str> {
    terminated(
        take_till(|c| c == ':' || c == '/'),
        take_while(|c| c == ':' || c == '/'),
    )(input)
}

fn user_parser(input: &str) -> IResult<&str, &str> {
    terminated(take_until("/"), take_while(|c| c == ':' || c == '/'))(input)
}

fn repo_parser(input: &str) -> IResult<&str, &str> {
    take_till(|c| c == '.')(input)
}

impl Remote {
    fn parse(url_str: &str) -> Option<Remote> {
        if let Ok((_, (schema, host, username, reponame))) = tuple((schema_parser, host_parser, user_parser, repo_parser))(url_str) {
            Some(Remote {
                schema: schema.to_string(),
                host: host.to_string(),
                username: username.to_string(),
                reponame: reponame.to_string(),
            })
        } else {
            None
        }
    }

    fn is_git(&self) -> bool {
        return self.schema == "git";
    }

    fn is_http(&self) -> bool {
        let http_schemas: [&str; 2] = ["http", "https"];
        if http_schemas.iter().any(|&s| s == self.schema) {
            return true;
        }
        return false;
    }

    fn get_http_url(&self) -> String {
        return format!(
            "https://{}/{}/{}",
            self.host.as_str(),
            self.username.as_str(),
            self.reponame.as_str(),
        );
    }
}
