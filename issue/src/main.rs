use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing,
    AddExtensionLayer, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let app = Router::new()
        .route("/projects/:project_id/issues", routing::post(issues_create))
        .route("/projects/:project_id/issues/:issue_id", routing::get(issues_get))
        .layer(AddExtensionLayer::new(client));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn issues_create(
    Path(project_id): Path<String>,
    Json(input): Json<Issue>,
    Extension(client): Extension<reqwest::Client>,
) -> Result<impl IntoResponse, StatusCode> {
    let create_issue = CreateIssue {
        fields: Fields {
            project: Project {
                key: project_id.clone(),
            },
            issuetype: IssueType {
                name: "Task".to_owned(),
            },
            summary: input.title.clone(),
            description: input.body.clone().unwrap(),
        },
    };

    let resp: CreateResponse = client
        .post(format!(
            "{}/rest/api/latest/issue",
            env::var("JIRA_HOST").unwrap()
        ))
        .basic_auth(
            env::var("JIRA_USER").unwrap(),
            Some(env::var("JIRA_PASS").unwrap()),
        )
        .json(&create_issue)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let issue = Issue {
        name: Some(format!("projects/{}/issues/{}", project_id, resp.key)),
        title: input.title,
        body: input.body,
        owner: input.owner,
        assignee: input.assignee,
        labels: input.labels,
    };

    Ok((StatusCode::CREATED, Json(issue)))
}

async fn issues_get(
    Path((project_id, issue_id)): Path<(String, String)>,
    Extension(client): Extension<reqwest::Client>,
) -> Result<impl IntoResponse, StatusCode> {
    let resp: GetResponse = client
        .get(format!(
            "{}/rest/api/latest/issue/{}",
            env::var("JIRA_HOST").unwrap(),
            issue_id.clone(),
        ))
        .basic_auth(
            env::var("JIRA_USER").unwrap(),
            Some(env::var("JIRA_PASS").unwrap()),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let issue = Issue {
        name: Some(format!("projects/{}/issues/{}", project_id, resp.key)),
        title: resp.fields.summary,
        body: Option::from(resp.fields.description),
        owner: None,
        assignee: Option::from(resp.fields.assignee.name),
        labels: Option::from(resp.fields.labels),
    };

    Ok((StatusCode::OK, Json(issue)))
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
struct Issue {
    name: Option<String>,
    title: String,
    body: Option<String>,
    owner: Option<String>,
    assignee: Option<String>,
    labels: Option<Vec<String>>,
}

#[derive(Serialize)]
struct CreateIssue {
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
