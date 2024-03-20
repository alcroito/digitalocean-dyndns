use aide::transform::TransformOperation;
use axum::{extract::State, Json};

use crate::db::crud::domain_record_ip_changes::{
    get_domain_record_ip_changes, DomainRecordIpChanges,
};
use crate::web::errors::WebApiResult;
use crate::web::server::WebServerState;

pub async fn list_domain_record_ip_changes(
    State(state): State<WebServerState>,
) -> WebApiResult<Json<DomainRecordIpChanges>> {
    let mut conn = state.conn.lock().expect("conn mutex poisoned");
    let ip_changes = get_domain_record_ip_changes(&mut conn)?;
    Ok(Json(ip_changes))
}

pub fn list_domain_record_ip_changes_docs(op: TransformOperation) -> TransformOperation {
    op.description("List all recent domain record ip changes")
}
