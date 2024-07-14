use std::collections::BTreeMap;
use std::sync::Arc;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use kube::ResourceExt;
use tracing::instrument;

use anyhow::Result;
use api::Application;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;

#[instrument()]
pub(crate) fn process(
    app: &Arc<Application>,
    object_meta: ObjectMeta,
    labels: BTreeMap<String, String>,
) -> Result<Vec<Operation>> {
    let deployment = Deployment {
        metadata: object_meta,
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
                        name: app.name_any().clone(),
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

    Ok(vec![Operation::CreateOrUpdate(Arc::new(
        to_dynamic_object(deployment)?,
    ))])
}

fn generate_ports(app: &Arc<Application>) -> Option<Vec<ContainerPort>> {
    let container_ports = app
        .spec
        .ports
        .iter()
        .map(|port| ContainerPort {
            name: Some(port.name()),
            container_port: port.port as i32,
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if container_ports.is_empty() {
        None
    } else {
        Some(container_ports)
    }
}
