use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use k8s_openapi::api::core::v1::ServiceAccount;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use tracing::instrument;

use api::v1::Application;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;

#[instrument(skip(_app, object_meta), fields(trace_id))]
pub(crate) fn process(
    _app: &Arc<Application>,
    object_meta: ObjectMeta,
    _labels: BTreeMap<String, String>,
) -> Result<Vec<Operation>> {
    let sa = ServiceAccount {
        metadata: object_meta,
        automount_service_account_token: Some(true),
        ..Default::default()
    };

    Ok(vec![Operation::CreateOrUpdate(Arc::new(
        to_dynamic_object(sa)?,
    ))])
}
