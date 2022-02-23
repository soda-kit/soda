use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    AddExtensionLayer, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let app = Router::new()
        .route("/projects/:project_id/issues", post(issues_create))
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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize)]
struct Fields {
    project: Project,
    issuetype: IssueType,
    summary: String,
    description: String,
}

#[derive(Serialize)]
struct Project {
    key: String,
}

#[derive(Serialize)]
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
