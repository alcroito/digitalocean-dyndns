mod domain_record_ip_changes;

use std::sync::Arc;

use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use axum::Extension;
use axum::Router;
#[cfg(debug_assertions)]
use hyper::http::Method;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use self::domain_record_ip_changes::list_domain_record_ip_changes_docs;

use super::docs::api_docs;
use super::docs::docs_routes;
use super::server::WebServerState;
use crate::web::routes::domain_record_ip_changes::list_domain_record_ip_changes;
use crate::web::static_server::serve_static_decisor;

const WEB_API_PATH_URL_PART: &str = "/api/v1";

pub fn get_final_router(state: WebServerState) -> Router {
    #[allow(unused_mut)]
    let mut cors = CorsLayer::new();

    #[cfg(debug_assertions)]
    {
        let origins = [
            // Default rust listening address, not really needed
            "http://localhost:8095"
                .parse()
                .expect("Failed to parse CORS origin into HeaderValue"),
            // Default vite dev listening address
            "http://localhost:5173"
                .parse()
                .expect("Failed to parse CORS origin into HeaderValue"),
        ];
        cors = cors
            .allow_methods(vec![Method::GET, Method::POST])
            .allow_origin(origins);
    }
    let cors_service = ServiceBuilder::new().layer(cors);

    let (router, api) = get_pure_router_and_open_api();

    router
        .layer(TraceLayer::new_for_http())
        .layer(cors_service)
        .layer(Extension(Arc::new(api)))
        .with_state(state)
}

fn get_pure_router_and_open_api() -> (Router<WebServerState>, OpenApi) {
    aide::generate::on_error(|gen_error| {
        panic!("Open API generation error: {}", gen_error);
    });

    aide::generate::extract_schemas(true);

    let mut api = OpenApi::default();

    let api_router = api_routes();

    let final_router = ApiRouter::new()
        .nest(WEB_API_PATH_URL_PART, api_router)
        .nest("/docs", docs_routes())
        // Explicitly set fallback on outer router, to avoid
        // https://github.com/tokio-rs/axum/discussions/2012
        .fallback(serve_static_decisor)
        .finish_api_with(&mut api, api_docs);
    (final_router, api)
}

fn api_routes() -> ApiRouter<WebServerState> {
    ApiRouter::new().api_route(
        "/domain_record_ip_changes",
        get_with(
            list_domain_record_ip_changes,
            list_domain_record_ip_changes_docs,
        ),
    )
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::ser::{PrettyFormatter, Serializer};

    use super::*;
    use std::path::Path;

    #[track_caller]
    fn ensure_file_contents(file: &Path, contents: &str) {
        if let Err(()) = try_ensure_file_contents(file, contents) {
            panic!("Open API schema file was not not up to date");
        }
    }

    fn try_ensure_file_contents(file: &Path, contents: &str) -> Result<(), ()> {
        match std::fs::read_to_string(file) {
            Ok(old_contents) if old_contents == contents => return Ok(()),
            _ => (),
        }
        let display_path = file;
        eprintln!(
            "\n\x1b[31;1merror\x1b[0m: {} was not up-to-date, updating\n",
            display_path.display()
        );
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(file, contents).unwrap();
        Err(())
    }

    #[test]
    fn generate_open_api_schema() {
        crate::logger::init_color_eyre();

        let (_, api) = get_pure_router_and_open_api();

        let mut buf = Vec::with_capacity(128);
        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut writer = Serializer::with_formatter(&mut buf, formatter);

        api.serialize(&mut writer)
            .unwrap_or_else(|_| panic!("could not serialize open api schema"));

        let json = String::from_utf8(buf).expect("invalid open api schema encoding");

        const OPEN_API_SCHEMA_PATH_REL_TO_WORKSPACE_DIR: &str = "generated/openapi.json";
        let schema_file_path = [
            env!("CARGO_MANIFEST_DIR"),
            OPEN_API_SCHEMA_PATH_REL_TO_WORKSPACE_DIR,
        ]
        .iter()
        .collect::<std::path::PathBuf>();

        ensure_file_contents(schema_file_path.as_path(), &json);
    }
}
