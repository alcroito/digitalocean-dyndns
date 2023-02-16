use std::sync::Arc;

use super::{errors::WebApiError, server::WebServerState};
use aide::{
    axum::{
        routing::{get, get_with},
        ApiRouter, IntoApiResponse,
    },
    openapi::OpenApi,
    redoc::Redoc,
};
use aide::{operation::OperationIo, transform::TransformOpenApi};
use axum::response::Response;
use axum::{response::IntoResponse, Extension, Json as AxumJson};
use axum_macros::FromRequest;
use serde::Serialize;

pub fn docs_routes() -> ApiRouter<WebServerState> {
    // We infer the return types for these routes
    // as an example.
    //
    // As a result, the `serve_redoc` route will
    // have the `text/html` content-type correctly set
    // with a 200 status.
    aide::gen::infer_responses(true);

    let router = ApiRouter::new()
        .api_route(
            "/",
            get_with(
                Redoc::new("/docs/private/api.json")
                    .with_title("Raw OpenAPI json")
                    .axum_handler(),
                |op| op.description("This documentation page."),
            ),
        )
        .route("/private/api.json", get(serve_docs));

    // Afterwards we disable response inference because
    // it might be incorrect for other routes.
    aide::gen::infer_responses(false);

    router
}

pub fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("ddns Open API")
        .summary("ddns Open API")
        .description("ddns Open API")
        .default_response_with::<AxumJson<WebApiError>, _>(|res| {
            res.example(WebApiError::generic())
        })
}

#[derive(FromRequest, OperationIo)]
#[from_request(via(axum_jsonschema::Json))]
#[aide(
    input_with = "axum_jsonschema::Json<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        AxumJson(self.0).into_response()
    }
}

async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api).into_response()
}
