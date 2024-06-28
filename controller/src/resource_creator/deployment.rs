use std::collections::BTreeMap;
use std::sync::Arc;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
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
        spec: Some(DeploymentSpec {
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
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: app_name.clone(),
                        image: Some(app.spec.image.clone()),
                        ports: generate_ports(&app),
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

fn generate_ports(app: &Arc<Application>) -> Option<Vec<ContainerPort>> {
    app.spec.ports.as_ref().map(|ports| {
        ports.iter().map(|port| {
            ContainerPort {
                name: Some(port.kind.to_string().to_lowercase()),
                container_port: port.port as i32,
                ..Default::default()
            }
        }).collect()
    })
}