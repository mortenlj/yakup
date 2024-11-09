use std::collections::BTreeMap;
use std::sync::Arc;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{ConfigMapEnvSource, ConfigMapVolumeSource, Container, ContainerPort, EnvFromSource, GRPCAction, HTTPGetAction, PodSpec, PodTemplateSpec, SecretEnvSource, SecretVolumeSource, TCPSocketAction, Volume, VolumeMount};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use kube::ResourceExt;
use tracing::instrument;

use anyhow::Result;
use api::v1::Application;
use api::{Probe, Probes};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;

struct FromConfig {
    env_from: Option<Vec<EnvFromSource>>,
    volume_mounts: Option<Vec<VolumeMount>>,
    volumes: Option<Vec<Volume>>,
}

#[instrument()]
pub(crate) fn process(
    app: &Arc<Application>,
    object_meta: ObjectMeta,
    labels: BTreeMap<String, String>,
) -> Result<Vec<Operation>> {
    let from_config = generate_from_config(app);
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
                        ports: generate_ports(app),
                        env_from: from_config.env_from,
                        volume_mounts: from_config.volume_mounts,
                        liveness_probe: generate_probe(app, |probes: &Probes| probes.liveness.clone()),
                        readiness_probe: generate_probe(app, |probes: &Probes| probes.readiness.clone()),
                        startup_probe: generate_probe(app, |probes: &Probes| probes.startup.clone()),
                        ..Default::default()
                    }],
                    volumes: from_config.volumes,
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

fn generate_from_config(app: &Arc<Application>) -> FromConfig {
    let mut env_from = vec![];
    let mut volume_mounts = vec![];
    let mut volumes = vec![];

    for name in [format!("{}-db", app.name_any()), app.name_any()].iter() {
        env_from.extend(generate_env_from(name.clone()));
        volume_mounts.extend(genereate_volume_mounts(name.clone()));
        volumes.extend(generate_volumes(name.clone()));
    }

    FromConfig {
        env_from: Some(env_from),
        volume_mounts: Some(volume_mounts),
        volumes: Some(volumes)
    }
}

fn generate_probe(
    app: &Arc<Application>,
    probe_getter: fn(&Probes) -> Option<Probe>,
) -> Option<k8s_openapi::api::core::v1::Probe> {
    match &app.spec.probes {
        Some(probes) => probe_getter(probes).map(|probe| {
            let mut http_delay = None;
            let http_get = &probe.http.map(|http| {
                http_delay = Some(http.config.initial_delay_seconds as i32);
                HTTPGetAction {
                    host: None,
                    http_headers: None,
                    path: http.path,
                    port: IntOrString::String(http.config.port_name.clone()),
                    scheme: None,
                }
            });
            let mut grpc_delay = None;
            let grpc = &probe.grpc.map(|grpc| {
                grpc_delay = Some(grpc.config.initial_delay_seconds as i32);
                let grpc_port = &app
                    .spec
                    .ports
                    .iter()
                    .find(|port| port.name() == grpc.config.port_name)
                    .map_or(1234, |port| port.port);
                GRPCAction {
                    port: grpc_port.to_owned() as i32,
                    service: grpc.service,
                }
            });
            let mut tcp_delay = None;
            let tcp_socket = &probe.tcp.map(|tcp| {
                tcp_delay = Some(tcp.config.initial_delay_seconds as i32);
                TCPSocketAction {
                    host: None,
                    port: IntOrString::String(tcp.config.port_name.clone()),
                }
            });
            k8s_openapi::api::core::v1::Probe {
                http_get: http_get.to_owned(),
                grpc: grpc.to_owned(),
                tcp_socket: tcp_socket.to_owned(),
                initial_delay_seconds: http_delay.or(grpc_delay).or(tcp_delay),
                period_seconds: Some(10),
                timeout_seconds: Some(1),
                success_threshold: Some(1),
                failure_threshold: Some(3),
                ..Default::default()
            }
        }),
        None => None,
    }
}

fn generate_volumes(app_name: String) -> Vec<Volume> {
    vec![
        Volume {
            name: format!("{}-configmap", app_name.clone()),
            config_map: Some(ConfigMapVolumeSource {
                name: app_name.clone(),
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
        },
    ]
}

fn genereate_volume_mounts(app_name: String) -> Vec<VolumeMount> {
    vec![
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
        },
    ]
}

fn generate_env_from(app_name: String) -> Vec<EnvFromSource> {
    vec![
        EnvFromSource {
            config_map_ref: Some(ConfigMapEnvSource {
                name: app_name.clone(),
                optional: Some(true),
            }),
            ..Default::default()
        },
        EnvFromSource {
            secret_ref: Some(SecretEnvSource {
                name: app_name.clone(),
                optional: Some(true),
            }),
            ..Default::default()
        },
    ]
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
