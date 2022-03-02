use std::sync::Arc;
use reqwest::Client;
use crate::error::IssueError;
use serde::{Deserialize, Serialize};
use axum::async_trait;

#[async_trait]
pub trait IssueService {
    async fn create_issue(
        &self,
        project_id: String,
        create_issue: CreateIssue,
    ) -> Result<Issue, IssueError>;
    async fn get_issue(
        &self,
        project_id: String,
        issue_id: String,
    ) -> Result<Issue, IssueError>;
}

pub type DynIssueService = Arc<dyn IssueService + Send + Sync>;

pub enum Credentials {
    Basic(String, String),
}

#[derive(Deserialize)]
pub struct GetResponse {
    id: String,
    key: String,
    #[serde(rename = "self")]
    url: String,
    fields: GetFields,
}


pub struct JiraIssueService {
    host: String,
    credentials: Credentials,
    client: Client,
}


#[derive(Serialize)]
pub struct Issue {
    name: String,
    title: String,
    body: Option<String>,
    owner: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct CreateIssue {
    title: String,
    body: Option<String>,
    owner: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
}


#[derive(Serialize, Deserialize)]
pub struct CreateRequest {
    fields: Fields,
}

#[derive(Deserialize)]
pub struct GetFields {
    project: Project,
    issuetype: IssueType,
    summary: String,
    description: String,
    assignee: Assignee,
    labels: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Fields {
    project: Project,
    issuetype: IssueType,
    summary: String,
    description: String,
}

#[derive(Deserialize)]
pub struct Assignee {
    #[serde(rename = "self")]
    key: String,
    name: String,
    #[serde(rename = "emailAddress")]
    email_address: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    key: String,
}

#[derive(Serialize, Deserialize)]
pub struct IssueType {
    name: String,
}

#[derive(Deserialize)]
pub struct CreateResponse {
    id: String,
    key: String,
    #[serde(rename = "self")]
    url: String,
}


impl JiraIssueService {
    pub fn new<H>(host: H, credentials: Credentials) -> Self
        where
            H: Into<String>,
    {
        Self {
            host: host.into(),
            credentials,
            client: Client::new(),
        }
    }
}


#[async_trait]
impl IssueService for JiraIssueService {
    async fn create_issue(
        &self,
        project_id: String,
        create_issue: CreateIssue,
    ) -> Result<Issue, IssueError> {
        let create_request = CreateRequest {
            fields: Fields {
                project: Project {
                    key: project_id.clone(),
                },
                issuetype: IssueType {
                    name: "Task".to_owned(),
                },
                summary: create_issue.title.clone(),
                description: create_issue.body.clone().unwrap(),
            },
        };

        let builder = self
            .client
            .post(format!("{}/rest/api/latest/issue", self.host));

        let builder = match self.credentials {
            Credentials::Basic(ref user, ref pass) => {
                builder.basic_auth(user.to_owned(), Some(pass.to_owned()))
            }
        };

        let resp: CreateResponse = builder
            .json(&create_request)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let issue = Issue {
            name: format!("projects/{}/issues/{}", project_id, resp.key),
            title: create_issue.title,
            body: create_issue.body,
            owner: create_issue.owner,
            assignee: create_issue.assignee,
            labels: create_issue.labels,
        };

        Ok(issue)
    }
    async fn get_issue(&self, project_id: String, issue_id: String) -> Result<Issue, IssueError> {
        let builder = self
            .client
            .get(format!("{}/rest/api/latest/issue/{}", self.host, issue_id));

        let builder = match self.credentials {
            Credentials::Basic(ref user, ref pass) => {
                builder.basic_auth(user.to_owned(), Some(pass.to_owned()))
            }
        };

        let resp: GetResponse = builder
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        let issue = Issue {
            name: format!("projects/{}/issues/{}", project_id, resp.key),
            title: resp.fields.summary,
            body: Some(resp.fields.description),
            owner: None,
            assignee: Some(resp.fields.assignee.name),
            labels: Some(resp.fields.labels),
        };

        Ok(issue)
    }
}