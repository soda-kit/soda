use axum::{
    async_trait,
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    AddExtensionLayer, Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    if let (Ok(host), Ok(user), Ok(pass)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_PASS"),
    ) {
        let issue_service = Arc::new(JiraIssueService::new(host, Credentials::Basic(user, pass)))
            as DynIssueService;

        let app = Router::new()
            .route("/projects/:project_id/issues", post(issues_create))
            .route("/projects/:project_id/issues/:issue_id", get(issues_get))
            .layer(AddExtensionLayer::new(issue_service));

        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        println!("Listening on http://{}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

async fn issues_create(
    Path(project_id): Path<String>,
    Json(create_issue): Json<CreateIssue>,
    Extension(issue_service): Extension<DynIssueService>,
) -> Result<Json<Issue>, AppError> {
    let issue = issue_service.create_issue(project_id, create_issue).await?;
    Ok(issue.into())
}

async fn issues_get(
    Path((project_id, issue_id)): Path<(String, String)>,
    Extension(issue_service): Extension<DynIssueService>,
) -> Result<Json<Issue>, AppError> {
    let issue = issue_service.get_issue(project_id, issue_id).await?;
    Ok(issue.into())
}

#[derive(Deserialize)]
struct GetResponse {
    id: String,
    key: String,
    #[serde(rename = "self")]
    url: String,
    fields: GetFields,
}

#[derive(Serialize, Deserialize, Debug)]
enum AppError {
    BadRequest,
    Unauthorized,
    NotFound,
}

impl From<IssueError> for AppError {
    fn from(inner: IssueError) -> Self {
        match inner {
            IssueError::BadRequest => AppError::BadRequest,
            IssueError::Unauthorized => AppError::Unauthorized,
            IssueError::NotFound => AppError::NotFound,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_mesage) = match self {
            AppError::BadRequest => (StatusCode::BAD_REQUEST, "Bad Request"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not Found"),
        };

        let body = Json(json!({ "error": error_mesage }));

        (status, body).into_response()
    }
}

#[async_trait]
trait IssueService {
    async fn create_issue(
        &self,
        project_id: String,
        create_issue: CreateIssue,
    ) -> Result<Issue, IssueError>;
    async fn get_issue(&self, project_id: String, issue_id: String) -> Result<Issue, IssueError>;
}

type DynIssueService = Arc<dyn IssueService + Send + Sync>;

#[derive(Serialize)]
struct Issue {
    name: String,
    title: String,
    body: Option<String>,
    owner: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CreateIssue {
    title: String,
    body: Option<String>,
    owner: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
}

enum IssueError {
    BadRequest,
    Unauthorized,
    NotFound,
}

enum Credentials {
    Basic(String, String),
}

struct JiraIssueService {
    host: String,
    credentials: Credentials,
    client: Client,
}

impl JiraIssueService {
    fn new<H>(host: H, credentials: Credentials) -> Self
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
        let resp: GetResponse = self
            .client
            .get(format!("{}/rest/api/latest/issue/{}", self.host, issue_id))
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

#[derive(Serialize)]
struct CreateRequest {
    fields: Fields,
}

#[derive(Deserialize)]
struct GetFields {
    project: Project,
    issuetype: IssueType,
    summary: String,
    description: String,
    assignee: Assignee,
    labels: Vec<String>,
}

#[derive(Serialize)]
struct Fields {
    project: Project,
    issuetype: IssueType,
    summary: String,
    description: String,
}

#[derive(Deserialize)]
struct Assignee {
    #[serde(rename = "self")]
    key: String,
    name: String,
    #[serde(rename = "emailAddress")]
    email_address: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Serialize, Deserialize)]
struct Project {
    key: String,
}

#[derive(Serialize, Deserialize)]
struct IssueType {
    name: String,
}

#[derive(Deserialize)]
struct CreateResponse {
    id: String,
    key: String,
    #[serde(rename = "self")]
    url: String,
}
