use std::sync::Arc;
use k8s_openapi::serde::Serialize;
use kube::api::DynamicObject;
use kube::discovery::ApiResource;
use kube::api::Resource;
use kube::ResourceExt;
use tracing::instrument;

use api::Application;

use crate::models::Operation;
use crate::{Error, Result};

mod deployment;

#[instrument()]
pub async fn process(obj: Arc<Application>) -> Result<Vec<Operation>> {
    deployment::process(obj).await
}

fn to_dynamic_object<K: Resource + ResourceExt + Serialize>(resource: K) -> Result<DynamicObject>
where
    K::DynamicType: Default,
{
    let mut dynamic_object = DynamicObject::new(
        &resource.name_any().as_str(),
        &ApiResource::erase::<K>(&Default::default()),
    );

    dynamic_object.metadata = resource.meta().clone();
    dynamic_object.data = serde_json::to_value(resource).map_err(|_| Error::ConfigError)?;

    if let Some(data) = dynamic_object.data.as_object_mut() {
        data.remove("kind");
        data.remove("apiVersion");
        data.remove("metadata");
    }

    Ok(dynamic_object)
}
