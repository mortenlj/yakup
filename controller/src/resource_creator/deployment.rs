use std::collections::BTreeMap;
use std::sync::Arc;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{ConfigMapEnvSource, ConfigMapVolumeSource, Container, ContainerPort, EnvFromSource, PodSpec, PodTemplateSpec, SecretEnvSource, SecretVolumeSource, Volume, VolumeMount};
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
                        env_from: generate_env_from(&app),
                        volume_mounts: genereate_volume_mounts(&app),
                        ..Default::default()
                    }],
                    volumes: generate_volumes(&app),
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

fn generate_volumes(app: &Arc<Application>) -> Option<Vec<Volume>> {
    let app_name = app.name_any();
    Some(vec![
        Volume {
            name: format!("{}-configmap", app_name.clone()),
            config_map: Some(ConfigMapVolumeSource {
                name: Some(app_name.clone()),
                optional: Some(true),
                default_mode: Some(0o644),
                ..Default::default()
            }),
            ..Default::default()
        },
        Volume {
            name: format!("{}-secret", app_name.clone()),
            secret: Some(SecretVolumeSource {
                secret_name: Some(app_name.clone()),
                optional: Some(true),
                default_mode: Some(0o644),
                ..Default::default()
            }),
            ..Default::default()
        }
    ])
}

fn genereate_volume_mounts(app: &Arc<Application>) -> Option<Vec<VolumeMount>> {
    let app_name = app.name_any();
    Some(vec![
        VolumeMount {
            name: format!("{}-configmap", app_name.clone()),
            mount_path: format!("/var/run/config/yakup.ibidem.no/{}", app_name.clone()),
            read_only: Some(true),
            ..Default::default()
        },
        VolumeMount {
            name: format!("{}-secret", app_name.clone()),
            mount_path: format!("/var/run/secrets/yakup.ibidem.no/{}", app_name.clone()),
            read_only: Some(true),
            ..Default::default()
        }
    ])
}

fn generate_env_from(app: &Arc<Application>) -> Option<Vec<EnvFromSource>> {
    let app_name = app.name_any();
    Some(vec![
        EnvFromSource {
            config_map_ref: Some(ConfigMapEnvSource {
                name: Some(app_name.clone()),
                optional: Some(true),
            }),
            ..Default::default()
        },
        EnvFromSource {
            secret_ref: Some(SecretEnvSource {
                name: Some(app_name.clone()),
                optional: Some(true),
            }),
            ..Default::default()
        }
    ])
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
