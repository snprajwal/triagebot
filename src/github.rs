use failure::{Error, ResultExt};
use reqwest::header::USER_AGENT;
use reqwest::Error as HttpError;
use reqwest::{Client, RequestBuilder, Response};

#[derive(Debug, serde::Deserialize)]
pub struct User {
    pub login: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Label {
    pub name: String,
}

impl Label {
    fn exists(&self, repo_api_prefix: &str, client: &GithubClient) -> bool {
        match client
            .get(&format!("{}/labels/{}", repo_api_prefix, self.name))
            .send_req()
        {
            Ok(_) => true,
            // XXX: Error handling if the request failed for reasons beyond 'label didn't exist'
            Err(_) => false,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Issue {
    number: u64,
    title: String,
    user: User,
    labels: Vec<Label>,
    assignees: Vec<User>,
    // API URL
    repository_url: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Comment {
    pub body: String,
    pub user: User,
}

impl Issue {
    pub fn set_labels(&self, client: &GithubClient, mut labels: Vec<Label>) -> Result<(), Error> {
        // PUT /repos/:owner/:repo/issues/:number/labels
        // repo_url = https://api.github.com/repos/Codertocat/Hello-World
        let url = format!(
            "{repo_url}/issues/{number}/labels",
            repo_url = self.repository_url,
            number = self.number
        );

        labels.retain(|label| label.exists(&self.repository_url, &client));

        #[derive(serde::Serialize)]
        struct LabelsReq {
            labels: Vec<String>,
        }
        client
            .put(&url)
            .json(&LabelsReq {
                labels: labels.iter().map(|l| l.name.clone()).collect(),
            })
            .send_req()
            .context("failed to set labels")?;

        Ok(())
    }

    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    pub fn add_assignee(&self, client: &GithubClient, user: &str) -> Result<(), Error> {
        let url = format!(
            "{repo_url}/issues/{number}/assignees",
            repo_url = self.repository_url,
            number = self.number
        );

        #[derive(serde::Serialize)]
        struct AssigneeReq<'a> {
            assignees: &'a [&'a str],
        }
        client
            .post(&url)
            .json(&AssigneeReq { assignees: &[user] })
            .send_req()
            .context("failed to add assignee")?;

        Ok(())
    }
}

trait RequestSend: Sized {
    fn configure(self, g: &GithubClient) -> Self;
    fn send_req(self) -> Result<Response, HttpError>;
    fn send(self) {} // cause conflicts to force users to `send_req`
}

impl RequestSend for RequestBuilder {
    fn configure(self, g: &GithubClient) -> RequestBuilder {
        self.header(USER_AGENT, "rust-lang-triagebot")
            .basic_auth("rust-highfive", Some(&g.token))
    }

    fn send_req(self) -> Result<Response, HttpError> {
        match self.send() {
            Ok(r) => r.error_for_status(),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone)]
pub struct GithubClient {
    token: String,
    client: Client,
}

impl GithubClient {
    pub fn new(c: Client, token: String) -> Self {
        GithubClient { client: c, token }
    }

    fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(url).configure(self)
    }

    fn post(&self, url: &str) -> RequestBuilder {
        self.client.post(url).configure(self)
    }

    fn put(&self, url: &str) -> RequestBuilder {
        self.client.put(url).configure(self)
    }
}
