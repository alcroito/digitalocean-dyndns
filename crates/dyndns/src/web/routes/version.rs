use aide::transform::TransformOperation;
use axum::Json;
use schemars::JsonSchema;
use serde::Serialize;

use crate::web::errors::WebApiResult;

#[derive(Debug, Serialize, JsonSchema)]
pub struct VersionResponse {
    pub version: String,
    pub git_sha: Option<String>,
    pub git_branch: Option<String>,
    pub build_date: String,
    pub build_timestamp: String,
}

impl VersionResponse {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_sha: option_env!("VERGEN_GIT_SHA").map(|s| s.to_string()),
            git_branch: option_env!("VERGEN_GIT_BRANCH").map(|s| s.to_string()),
            build_date: env!("VERGEN_BUILD_DATE").to_string(),
            build_timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
        }
    }
}

pub async fn get_version() -> WebApiResult<Json<VersionResponse>> {
    Ok(Json(VersionResponse::new()))
}

pub fn get_version_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.description("Get version information")
        .summary("Returns version, build date, and Git information for the running application")
        .tag("version")
}
