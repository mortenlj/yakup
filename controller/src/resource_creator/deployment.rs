use std::collections::BTreeMap;
use std::sync::Arc;

use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::PodTemplateSpec;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use kube::ResourceExt;
use tracing::instrument;

use api::Application;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;
use crate::Result;

#[instrument()]
pub(crate) async fn process(app: Arc<Application>) -> Result<Vec<Operation>> {
    let app_name = app.name_any();
    let namespace = app.namespace().unwrap_or("default".to_string());
    let labels = BTreeMap::from([
        ("app.kubernetes.io/name".to_string(), app_name.clone()),
        ("app.kubernetes.io/managed-by".to_string(), "yakup".to_string()),
    ]);
    let deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(app_name.clone()),
            namespace: Some(namespace.clone()),
            ..Default::default()
        },
        spec: Some(k8s_openapi::api::apps::v1::DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels.clone()),
                    ..Default::default()
                }),
                spec: Some(k8s_openapi::api::core::v1::PodSpec {
                    containers: vec![k8s_openapi::api::core::v1::Container {
                        name: app_name.clone(),
                        image: Some(app.spec.image.clone()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    Ok(vec![
        Operation::CreateOrUpdate(Arc::new(to_dynamic_object(deployment)?))
    ])
}