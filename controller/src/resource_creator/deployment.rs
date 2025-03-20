use std::collections::BTreeMap;
use std::sync::Arc;

use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    ConfigMapEnvSource, ConfigMapVolumeSource, Container, ContainerPort, EnvFromSource,
    HTTPGetAction, PodSpec, PodTemplateSpec, SecretEnvSource, SecretVolumeSource, TCPSocketAction,
    Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use kube::ResourceExt;
use tracing::instrument;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;
use anyhow::Result;
use api::application::v1::Application;
use api::application::{Probe, Probes};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

const DEFAULT_SECRET_MOUNT_PATH: &str = "/var/run/secrets/yakup.ibidem.no";
const DEFAULT_CONFIGMAP_MOUNT_PATH: &str = "/var/run/config/yakup.ibidem.no";

struct FromConfig {
    env_from: Option<Vec<EnvFromSource>>,
    volume_mounts: Option<Vec<VolumeMount>>,
    volumes: Option<Vec<Volume>>,
}

#[instrument(skip(app, object_meta), fields(trace_id))]
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
                    service_account_name: Some(app.name_any().clone()),
                    containers: vec![Container {
                        name: app.name_any().clone(),
                        image: Some(app.spec.image.clone()),
                        ports: generate_ports(app),
                        env: Some(app.spec.env.iter().map(|e| e.to_kube()).collect()),
                        env_from: from_config.env_from,
                        volume_mounts: from_config.volume_mounts,
                        liveness_probe: generate_probe(app, |probes: &Probes| {
                            probes.liveness.clone()
                        }),
                        readiness_probe: generate_probe(app, |probes: &Probes| {
                            probes.readiness.clone()
                        }),
                        startup_probe: generate_probe(app, |probes: &Probes| {
                            probes.startup.clone()
                        }),
                        resources: app.spec.resources.clone(),
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
        env_from.extend(generate_env_from(name));
        volume_mounts.extend(generate_volume_mounts(name));
        volumes.extend(generate_volumes(name));
    }

    for ef in app.spec.env_from.iter() {
        if let Some(name) = &ef.config_map {
            env_from.push(generate_env_from_configmap(name))
        }
        if let Some(name) = &ef.secret {
            env_from.push(generate_env_from_secret(name))
        }
    }

    let mut empty_dir_idx = 0;
    for ff in app.spec.files_from.iter() {
        if let Some(ffcm) = &ff.config_map {
            let name = ffcm.name.as_str();
            let mount_path: String = match &ffcm.mount_path {
                Some(mount_path) => mount_path.to_owned(),
                None => format!("{}/{}", DEFAULT_CONFIGMAP_MOUNT_PATH, name),
            };
            volume_mounts.push(generate_volume_mounts_from_configmap(
                name,
                mount_path.as_str(),
            ));
            volumes.push(generate_volume_for_configmap(name, None));
        }
        if let Some(ffs) = &ff.secret {
            let name = ffs.name.as_str();
            let mount_path: String = match &ffs.mount_path {
                Some(mount_path) => mount_path.to_owned(),
                None => format!("{}/{}", DEFAULT_SECRET_MOUNT_PATH, name),
            };
            volume_mounts.push(generate_volume_mounts_from_secret(
                name,
                mount_path.as_str(),
            ));
            volumes.push(generate_volume_for_secret(name, None));
        }
        if let Some(ffe) = &ff.empty_dir {
            let name = format!("emptydir-{}", empty_dir_idx);
            let mount_path = ffe.mount_path.as_str();
            volume_mounts.push(generate_volume_mounts_from(name.clone(), mount_path, None));
            volumes.push(generate_volume_for_empty_dir(name.as_str()));
            empty_dir_idx += 1;
        }
    }

    FromConfig {
        env_from: Some(env_from),
        volume_mounts: Some(volume_mounts),
        volumes: Some(volumes),
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
                tcp_socket: tcp_socket.to_owned(),
                initial_delay_seconds: http_delay.or(tcp_delay),
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

fn generate_volumes(app_name: &str) -> Vec<Volume> {
    vec![
        generate_volume_for_configmap(app_name, Some(true)),
        generate_volume_for_secret(app_name, Some(true)),
    ]
}

fn generate_volume_for_configmap(name: &str, optional: Option<bool>) -> Volume {
    Volume {
        name: format!("{}-configmap", name.to_owned()),
        config_map: Some(ConfigMapVolumeSource {
            name: name.to_owned(),
            optional,
            default_mode: Some(0o644),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn generate_volume_for_secret(name: &str, optional: Option<bool>) -> Volume {
    Volume {
        name: format!("{}-secret", name.to_owned()),
        secret: Some(SecretVolumeSource {
            secret_name: Some(name.to_owned()),
            optional,
            default_mode: Some(0o644),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn generate_volume_for_empty_dir(name: &str) -> Volume {
    Volume {
        name: name.to_owned(),
        empty_dir: Some(Default::default()),
        ..Default::default()
    }
}

fn generate_volume_mounts(app_name: &str) -> Vec<VolumeMount> {
    vec![
        generate_volume_mounts_from_configmap(
            app_name,
            &format!("{}/{}", DEFAULT_CONFIGMAP_MOUNT_PATH, app_name.to_owned()),
        ),
        generate_volume_mounts_from_secret(
            app_name,
            &format!("{}/{}", DEFAULT_SECRET_MOUNT_PATH, app_name.to_owned()),
        ),
    ]
}

fn generate_volume_mounts_from_configmap(name: &str, mount_path: &str) -> VolumeMount {
    generate_volume_mounts_from(
        format!("{}-{}", name.to_owned(), "configmap"),
        mount_path,
        Some(true),
    )
}

fn generate_volume_mounts_from_secret(name: &str, mount_path: &str) -> VolumeMount {
    generate_volume_mounts_from(
        format!("{}-{}", name.to_owned(), "secret"),
        mount_path,
        Some(true),
    )
}

fn generate_volume_mounts_from(
    name: String,
    mount_path: &str,
    read_only: Option<bool>,
) -> VolumeMount {
    VolumeMount {
        name,
        mount_path: mount_path.to_owned(),
        read_only,
        ..Default::default()
    }
}

fn generate_env_from_configmap(name: &str) -> EnvFromSource {
    EnvFromSource {
        config_map_ref: Some(ConfigMapEnvSource {
            name: name.to_owned(),
            optional: Some(true),
        }),
        ..Default::default()
    }
}

fn generate_env_from_secret(name: &str) -> EnvFromSource {
    EnvFromSource {
        secret_ref: Some(SecretEnvSource {
            name: name.to_owned(),
            optional: Some(true),
        }),
        ..Default::default()
    }
}

fn generate_env_from(app_name: &str) -> Vec<EnvFromSource> {
    vec![
        generate_env_from_configmap(app_name),
        generate_env_from_secret(app_name),
    ]
}

fn generate_ports(app: &Arc<Application>) -> Option<Vec<ContainerPort>> {
    let mut container_ports = Vec::new();
    if let Some(ports) = &app.spec.ports {
        if let Some(http_port) = &ports.http {
            container_ports.push(ContainerPort {
                name: Some("http".to_string()),
                container_port: http_port.port as i32,
                ..Default::default()
            });
        }

        if let Some(tcp_port) = &ports.tcp {
            container_ports.push(ContainerPort {
                name: Some("tcp".to_string()),
                container_port: tcp_port.port as i32,
                ..Default::default()
            });
        }
    }
    if container_ports.is_empty() {
        None
    } else {
        Some(container_ports)
    }
}
