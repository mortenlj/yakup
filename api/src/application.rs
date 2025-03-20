use k8s_openapi::api::core::v1::ResourceRequirements;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub mod v1 {
    use super::*;

    #[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[kube(
        group = "yakup.ibidem.no",
        version = "v1",
        kind = "Application",
        namespaced,
        status = "ApplicationStatus",
        shortname = "app",
        doc = "Yet Another Application Kind",
        printcolumn = r#"{"name":"Image","type":"string","jsonPath":".spec.image"}"#
    )]
    #[serde(rename_all = "camelCase")]
    pub struct ApplicationSpec {
        /// The environment variables to set in the container.
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub env: Vec<EnvValue>,

        /// Inject environment variables from the listed sources.
        /// A source can be either a configmap or a secret, it is an error to use both in one list item.
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub env_from: Vec<EnvFrom>,

        /// Mount files from the listed sources.
        /// A source can be either a configmap, a secret, or an emptyDir.
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub files_from: Vec<FilesFrom>,

        /// The image to run.
        pub image: String,

        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ports: Option<Ports>,

        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub probes: Option<Probes>,

        /// Compute Resources required by this application.
        /// More info: https://kubernetes.io/docs/concepts/configuration/manage-resources-containers/
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub resources: Option<ResourceRequirements>,
        //   replicas:
        //     min: 1
        //     max: 5
        //     autoscaling:
        //       enabled: true
        //       cpu: 50%
        //       memory: 70%
        //       kafka:
        //         - topic: mytopic
        //           group: losGroupos
        //           maxLag: 123
        //  metrics:
        //     enabled: true
        //     path: /metrics
        //     port: 8080
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationStatus {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvValue {
    pub name: String,
    pub value: String,
}

impl EnvValue {
    pub fn to_kube(&self) -> k8s_openapi::api::core::v1::EnvVar {
        k8s_openapi::api::core::v1::EnvVar {
            name: self.name.clone(),
            value: Some(self.value.clone()),
            value_from: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvFrom {
    /// The name of a config map to get environment variables from.
    /// Keys that are not valid environment variable names will be skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map: Option<String>,

    /// The name of a secret to get environment variables from.
    /// Keys that are not valid environment variable names will be skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilesFrom {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map: Option<FilesFromConfigMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<FilesFromSecret>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_dir: Option<FilesFromEmptyDir>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilesFromConfigMap {
    /// The name of the configmap to mount.
    pub name: String,
    /// The path to mount the configmap to.
    /// The default value is /var/run/config/yakup.ibidem.no/<name>.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilesFromSecret {
    /// The name of the secret to mount.
    pub name: String,
    /// The path to mount the secret to.
    /// The default value is /var/run/secrets/yakup.ibidem.no/<name>.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilesFromEmptyDir {
    /// The path to mount the emptydir to.
    pub mount_path: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub enum PathType {
    #[default]
    Prefix,
    Exact,
}

impl Display for PathType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ingress {
    pub zone: String,

    #[serde(default = "default_path_type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_type: Option<PathType>,

    #[serde(default = "default_paths")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

fn default_paths() -> Vec<String> {
    vec!["/".to_string()]
}

fn default_path_type() -> Option<PathType> {
    Some(PathType::Prefix)
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Ports {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpPort>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp: Option<TcpPort>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpPort {
    /// Container port to expose.
    pub port: u16,

    /// If this port should be exposed as an ingress.
    /// `ingress` is only valid on ports of kind PortKind::HTTP.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ingress: Vec<Ingress>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TcpPort {
    /// Container port to expose.
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Probes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness: Option<Probe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liveness: Option<Probe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<Probe>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProbeConfig {
    #[serde(default = "default_initial_delay_seconds")]
    pub initial_delay_seconds: u16,

    pub port_name: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpAction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp: Option<TcpAction>,
}

fn default_initial_delay_seconds() -> u16 {
    15
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default = "default_http_path")]
    pub path: Option<String>,

    #[serde(flatten)]
    pub config: ProbeConfig,
}

fn default_http_path() -> Option<String> {
    Some("/".to_string())
}

impl Default for HttpAction {
    fn default() -> Self {
        HttpAction {
            path: default_http_path(),
            config: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TcpAction {
    #[serde(flatten)]
    pub config: ProbeConfig,
}
