use std::sync::Arc;

use anyhow::Result;
use k8s_openapi::api::core::v1::ServiceAccount;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use tracing::instrument;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;

#[instrument(skip(object_meta), fields(trace_id))]
pub(crate) fn process(object_meta: ObjectMeta) -> Result<Vec<Operation>> {
    let sa = ServiceAccount {
        metadata: object_meta,
        automount_service_account_token: Some(true),
        ..Default::default()
    };

    Ok(vec![Operation::CreateOrUpdate(Arc::new(
        to_dynamic_object(sa)?,
    ))])
}
