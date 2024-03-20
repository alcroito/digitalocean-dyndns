use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

use aide::OperationIo;
use axum::body::Body;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use color_eyre::eyre::{eyre, Report, Result};
use http::StatusCode;
use schemars::gen::SchemaGenerator;
use schemars::schema::{Schema, SchemaObject};
use schemars::JsonSchema;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

pub type WebResult<T> = Result<T, WebError>;
pub type WebApiResult<T> = Result<T, WebApiError>;

#[derive(Debug)]
pub enum WebError {
    GenericError(Report),
    NotFound,
}

impl Display for WebError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GenericError(e) => write!(f, "generic error: {e}"),
            Self::NotFound => write!(f, "page not found"),
        }
    }
}
impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::GenericError(_) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{self}")),
            Self::NotFound => (StatusCode::NOT_FOUND, format!("{self}")),
        };

        let error_body = Body::from(error_message);
        (status, error_body).into_response()
    }
}

impl StdError for WebError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::GenericError(err) => Some(&**err),
            Self::NotFound => None,
        }
    }
}

impl From<Report> for WebError {
    fn from(err: Report) -> Self {
        Self::GenericError(err)
    }
}

#[serde_as]
#[derive(Debug, OperationIo, Serialize)]
pub enum WebApiError {
    GenericError(#[serde_as(as = "DisplayFromStr")] Report),
}

impl JsonSchema for WebApiError {
    fn schema_name() -> String {
        "WebApiError".to_owned()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::Object.into()),
            ..Default::default()
        };
        let obj = schema.object();
        obj.required.insert("GenericError".to_owned());
        obj.properties
            .insert("GenericError".to_owned(), gen.subschema_for::<String>());

        let mut final_schema = SchemaObject::default();
        final_schema.subschemas().one_of = Some(vec![schema.into()]);
        final_schema.into()
    }
}

impl WebApiError {
    pub fn generic() -> Self {
        Self::GenericError(eyre!("generic error"))
    }
}

impl Display for WebApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GenericError(e) => write!(f, "generic error: {e}"),
        }
    }
}
impl IntoResponse for WebApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::GenericError(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(self)),
        };
        (status, body).into_response()
    }
}

impl StdError for WebApiError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::GenericError(err) => Some(&**err),
        }
    }
}

impl From<Report> for WebApiError {
    fn from(err: Report) -> Self {
        Self::GenericError(err)
    }
}
