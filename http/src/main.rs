use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    AddExtensionLayer, Json, Router,
};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use issue::jira::{JiraIssueService, Credentials, DynIssueService, CreateIssue, Issue};
use issue::error::{AppError, IssueError};

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



