use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference};
use k8s_openapi::serde::Serialize;
use kube::api::DynamicObject;
use kube::api::Resource;
use kube::discovery::ApiResource;
use kube::ResourceExt;
use tracing::instrument;

use crate::models::Operation;
use api::v1::{Application, IngressZone};

mod deployment;
mod ingress;
mod service;
mod service_account;

trait Owner {
    fn owner_reference(&self) -> OwnerReference;
}

impl Owner for Application {
    fn owner_reference(&self) -> OwnerReference {
        OwnerReference {
            api_version: Application::api_version(&()).to_string(),
            kind: Application::kind(&()).to_string(),
            name: self.name_any(),
            uid: self.uid().unwrap_or_default(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        }
    }
}

#[instrument(skip(zones, app), fields(trace_id))]
pub fn process(
    app: Arc<Application>,
    zones: &HashMap<String, Arc<IngressZone>>,
) -> Result<Vec<Operation>> {
    let app_name = app.name_any();
    let namespace = app.namespace().unwrap_or("default".to_string());
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".to_string(), app_name.clone()),
        (
            "app.kubernetes.io/managed-by".to_string(),
            "yakup".to_string(),
        ),
    ]);
    let object_meta = ObjectMeta {
        name: Some(app_name.clone()),
        namespace: Some(namespace.clone()),
        labels: Some(labels.clone()),
        owner_references: Some(vec![app.owner_reference()]),
        ..Default::default()
    };

    let mut operations = Vec::new();
    operations.extend(deployment::process(
        &app,
        object_meta.clone(),
        labels.clone(),
    )?);
    operations.extend(service::process(&app, object_meta.clone(), labels.clone())?);
    operations.extend(service_account::process(
        object_meta.clone(),
    )?);
    operations.extend(ingress::process(
        &app,
        zones,
        object_meta.clone(),
    )?);
    Ok(operations)
}

fn to_dynamic_object<K: Resource + ResourceExt + Serialize>(resource: K) -> Result<DynamicObject>
where
    K::DynamicType: Default,
{
    let mut dynamic_object = DynamicObject::new(
        resource.name_any().as_str(),
        &ApiResource::erase::<K>(&Default::default()),
    );

    dynamic_object.metadata = resource.meta().clone();
    dynamic_object.data = serde_json::to_value(&resource)
        .map_err(|e| anyhow!(e).context("serializing resource to JSON"))?;

    if let Some(data) = dynamic_object.data.as_object_mut() {
        data.remove("kind");
        data.remove("apiVersion");
        data.remove("metadata");
    }

    Ok(dynamic_object)
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_include;
    use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
    use k8s_openapi::api::core::v1::{Service, ServiceSpec};
    use pretty_assertions::assert_eq;
    use rstest::*;
    use serde_json::json;

    use super::*;

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
            status: Default::default(),
        }
    }

    #[rstest]
    #[case::deployment(deployment(), json!({"spec": {"replicas": 1}}))]
    #[case::service(service(), json!({"spec": {"externalName": "test"}}))]
    fn to_dynamic_object_success<K: Resource + ResourceExt + Serialize + Clone>(
        #[case] object: K,
        #[case] expected: serde_json::Value,
    ) where
        K::DynamicType: Default,
    {
        let dynamic_object = to_dynamic_object(object.clone()).unwrap();
        assert_eq!(&dynamic_object.metadata, object.meta(), "metadata mismatch");

        assert_json_include!(actual: dynamic_object.data, expected: expected);
    }
}
