use axum::{
    body::{boxed, Body},
    response::{IntoResponse, Response},
};
use http::Request;

#[cfg(any(feature = "debug_static_embedded", not(debug_assertions)))]
use axum::body::Full;
#[cfg(any(feature = "debug_static_embedded", not(debug_assertions)))]
use mime_guess;
#[cfg(any(feature = "debug_static_embedded", not(debug_assertions)))]
use rust_embed::RustEmbed;

#[cfg(debug_assertions)]
use tower::ServiceExt;
#[cfg(debug_assertions)]
use tower_http::services::ServeDir;
use tracing::{info, trace};

use super::errors::{WebError, WebResult};

#[allow(unused)]
enum ServeStaticFilesFrom {
    Filesystem,
    Embedded,
    FSOrEmbedded,
}

fn decide_where_files_are_served_from() -> ServeStaticFilesFrom {
    // First condition is not strictly true, but cfg_if does not allow nesting multiple
    // conditions.
    cfg_if::cfg_if! {
        if #[cfg(feature = "debug_static_embedded")] {
            ServeStaticFilesFrom::FSOrEmbedded
        } else if #[cfg(debug_assertions)] {
            ServeStaticFilesFrom::Filesystem
        } else {
            ServeStaticFilesFrom::Embedded
        }
    }
}

pub fn print_where_files_are_served_from() {
    match decide_where_files_are_served_from() {
        ServeStaticFilesFrom::Filesystem => info!("Serving web assets from filesystem"),
        ServeStaticFilesFrom::Embedded => info!("Serving web assets embedded in binary"),
        ServeStaticFilesFrom::FSOrEmbedded => {
            info!("Serving web assets from filesystem with fallback to assets embedded in binary")
        }
    }
}

pub async fn serve_static_decisor(req: Request<Body>) -> WebResult<Response> {
    // When building in debug mode, we serve from the file system,
    // or from the embedded content if the debug_static_embedded feature is set.
    // When building in release mode, serve from the embedded content.
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            serve_static_from_filesystem_and_maybe_embedded(req).await
        } else {
            serve_static_using_embedded(req).await
        }
    }
}

#[cfg(debug_assertions)]
async fn serve_static_from_filesystem_and_maybe_embedded(
    req: Request<Body>,
) -> WebResult<Response> {
    #![cfg_attr(not(feature = "debug_static_embedded"), allow(unused))]
    let path = req.uri().path().trim_start_matches('/').to_string();

    let static_files_dir = [
        env!("CARGO_MANIFEST_DIR"),
        WEB_CLIENT_BUILD_PATH_REL_TO_CARGO_MANIFEST_DIR,
    ]
    .iter()
    .collect::<std::path::PathBuf>();

    trace!("Trying to serve path: '{path}' using ServeDir");
    match ServeDir::new(static_files_dir).oneshot(req).await {
        Ok(res) if res.status().is_success() || res.status().is_redirection() => Ok(res.map(boxed)),
        Ok(_) | Err(_) => {
            trace!("Couldn't find: '{path}'");
            cfg_if::cfg_if! {
                if #[cfg(feature = "debug_static_embedded")] {
                    trace!("Trying to serve path: '{path}' using rust_embed");
                    serve_static_using_embedded_from_path(&path).await
                } else {
                    static_file_not_found()
                }
            }
        }
    }
}

fn static_file_not_found<T: IntoResponse>() -> WebResult<T> {
    Err(WebError::NotFound)
}

#[cfg(not(debug_assertions))]
async fn serve_static_using_embedded(req: Request<Body>) -> WebResult<Response> {
    let path = req.uri().path().trim_start_matches('/');
    serve_static_using_embedded_from_path(path).await
}

#[cfg(any(feature = "debug_static_embedded", not(debug_assertions)))]
async fn serve_static_using_embedded_from_path(path: &str) -> WebResult<Response> {
    // Handle special case of root dir.
    let mut lookup_path = path.to_owned();
    if lookup_path.is_empty() {
        lookup_path.push_str("index.html");
    }
    let path = lookup_path.as_str();
    trace!("serve_static_using_embedded_from_path lookup_path: '{path}'");

    match WebAssets::get(path) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Ok(Response::builder()
                .header(http::header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .expect("Failed to build response body for static file serving"))
        }
        None => static_file_not_found(),
    }
}

#[cfg(debug_assertions)]
const WEB_CLIENT_BUILD_PATH_REL_TO_CARGO_MANIFEST_DIR: &str = "../../webclients/svelte/build";

#[cfg_attr(
    any(feature = "debug_static_embedded", not(debug_assertions)),
    derive(RustEmbed),
    folder = "../../webclients/svelte/build"
)]
#[cfg(any(feature = "debug_static_embedded", not(debug_assertions)))]
struct WebAssets;
