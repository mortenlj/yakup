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

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
    use k8s_openapi::api::core::v1::{Service, ServiceSpec};
    use pretty_assertions::assert_eq;
    use rstest::*;
    use assert_json_diff::assert_json_include;
    use serde_json::json;

    #[fixture]
    fn deployment() -> Deployment {
        Deployment {
            metadata: ObjectMeta {
                name: Some("test".to_string()),
                ..Default::default()
            },
            spec: Some(DeploymentSpec {
                replicas: Some(1),
                ..Default::default()
            }),
            status: Default::default(),
        }
    }

    #[fixture]
    fn service() -> Service {
        Service {
            metadata: ObjectMeta {
                name: Some("test".to_string()),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                external_name: Some("test".to_string()),
                ..Default::default()
            }),
            status: Default::default()
        }
    }

    #[rstest]
    #[case::deployment(deployment(), json!({"spec": {"replicas": 1}}))]
    #[case::service(service(), json!({"spec": {"externalName": "test"}}))]
    fn to_dynamic_object_success<K: Resource + ResourceExt + Serialize + Clone>(#[case] object: K, #[case] expected: serde_json::Value)
    where
        K::DynamicType: Default,
    {
        let dynamic_object = to_dynamic_object(object.clone()).unwrap();
        assert_eq!(&dynamic_object.metadata, object.meta(), "metadata mismatch");

        assert_json_include!(actual: dynamic_object.data, expected: expected);
    }
}