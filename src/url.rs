use nom::branch::alt;
use nom::bytes::complete::{tag, take_till, take_until, take_while};
use nom::sequence::{terminated, tuple};
use nom::IResult;

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
    take_until(".git")(input)
}

enum Platform {
    Github,
    Gitlab,
    Other(String),
}

/// Contains the address of the git repository.
/// git remote url can be two format:
/// * git@xxx.com:user/repo.git
//  * https://xxx.com/user/repo.git
#[derive(Debug, Default)]
pub struct Remote {
    schema: String,
    host: String,
    user: String,
    repo: String,
}

impl Remote {
    pub fn parse(url_str: &str) -> Option<Remote> {
        if let Ok((_, (schema, host, user, repo))) =
            tuple((schema_parser, host_parser, user_parser, repo_parser))(url_str)
        {
            Some(Remote {
                schema: schema.to_string(),
                host: host.to_string(),
                user: user.to_string(),
                repo: repo.to_string(),
            })
        } else {
            None
        }
    }

    pub fn is_git(&self) -> bool {
        self.schema == "git"
    }

    pub fn is_http(&self) -> bool {
        let http_schemas: [&str; 2] = ["http", "https"];
        if http_schemas.iter().any(|&s| s == self.schema) {
            return true;
        }
        false
    }

    pub fn get_platform(&self) -> Platform {
        match self.host.as_str() {
            "github.com" => Platform::Github,
            "gitlab.com" => Platform::Gitlab,
            _ => Platform::Other(self.host.clone()),
        }
    }

    pub fn get_repo_url(&self) -> String {
        format!(
            "https://{}/{}/{}",
            self.host.as_str(),
            self.user.as_str(),
            self.repo.as_str(),
        )
    }

    pub fn get_commit_url(&self, commit: &str) -> String {
        format!("{}/commit/{}", self.get_repo_url(), commit)
    }

    pub fn get_branch_url(&self, branch: &str) -> String {
        format!("{}/tree/{}", self.get_repo_url(), branch)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn repo_name() {
        assert_eq!(repo_parser("repo_name.git"), Ok((".git", "repo_name")));
        assert_eq!(repo_parser("repo_name.rs.git"), Ok((".git", "repo_name.rs")));
    }

    #[test]
    fn remote_git() {
        let remote = Remote::parse("git@xxx.com:user/repo.git");
        assert!(remote.is_some());
        if let Some(remote) = remote {
            assert!(remote.is_git());
            assert_eq!(remote.schema, "git");
            assert_eq!(remote.repo, "repo");
            assert_eq!(remote.host, "xxx.com");
            assert_eq!(remote.user, "user");
            assert!(matches!(remote.get_platform(), Platform::Other(..)));
        }
    }

    #[test]
    fn remote_http() {
        let remote = Remote::parse("http://github.com/user/repo.git");
        assert!(remote.is_some());
        if let Some(remote) = remote {
            assert!(remote.is_http());
            assert_eq!(remote.schema, "http");
            assert_eq!(remote.repo, "repo");
            assert_eq!(remote.host, "github.com");
            assert_eq!(remote.user, "user");
            assert!(matches!(remote.get_platform(), Platform::Github));
        }
    }

    #[test]
    fn remote_https() {
        let remote = Remote::parse("https://xxx.com/user/repo.git");
        assert!(remote.is_some());
        if let Some(remote) = remote {
            assert!(remote.is_http());
            assert_eq!(remote.schema, "https");
            assert_eq!(remote.repo, "repo");
            assert_eq!(remote.host, "xxx.com");
            assert_eq!(remote.user, "user");
        }
    }

    #[test]
    fn remote_url() {
        let remote = Remote::parse("https://github.com/user/repo.git");
        if let Some(remote) = remote {
            assert_eq!(remote.get_repo_url(), "https://github.com/user/repo");
            assert_eq!(
                remote.get_commit_url("commit_id"),
                "https://github.com/user/repo/commit/commit_id",
            );
            assert_eq!(
                remote.get_branch_url("test"),
                "https://github.com/user/repo/tree/test",
            );
        }
    }
}
