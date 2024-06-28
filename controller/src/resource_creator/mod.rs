use std::collections::BTreeMap;
use std::sync::Arc;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
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
mod service;

#[instrument()]
pub async fn process(app: Arc<Application>) -> Result<Vec<Operation>> {
    let app_name = app.name_any();
    let namespace = app.namespace().unwrap_or("default".to_string());
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".to_string(), app_name.clone()),
        ("app.kubernetes.io/managed-by".to_string(), "yakup".to_string()),
    ]);
    let object_meta = ObjectMeta {
        name: Some(app_name.clone()),
        namespace: Some(namespace.clone()),
        labels: Some(labels.clone()),
        ..Default::default()
    };

    let mut operations = Vec::new();
    operations.extend(deployment::process(&app, object_meta.clone(), labels.clone()).await?);
    operations.extend(service::process(&app, object_meta.clone(), labels.clone()).await?);
    Ok(operations)
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
