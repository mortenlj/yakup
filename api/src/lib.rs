use k8s_openapi::api::core::v1::ResourceRequirements;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use k8s_openapi::serde::{Deserialize, Serialize};
use kube::CustomResource;
use schemars::JsonSchema;
use std::fmt::Display;

pub mod v1 {
    use super::*;

    #[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[kube(
        group = "yakup.ibidem.no",
        version = "v1",
        kind = "IngressZone",
        shortname = "zone",
        doc = "Ingress Zone",
        printcolumn = r#"{"name":"Host","type":"string","jsonPath":".spec.host"}"#
    )]
    #[serde(rename_all = "camelCase")]
    pub struct IngressZoneSpec {
        /// The host to use for this zone.
        /// Can contain a variable in the form `{appname}` which will be replaced with the application name.
        pub host: String,

        /// IngressClass to use for this zone.
        /// If not set, the default class will be used.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ingress_class: Option<String>,
    }


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

        /// The image to run.
        pub image: String,

        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub ports: Vec<Port>,

        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub probes: Option<Probes>,

        /// Compute Resources required by this application.
        /// More info: https://kubernetes.io/docs/concepts/configuration/manage-resources-containers/
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub resources: Option<ResourceRequirements>,

        //     filesFrom:
        //       - secret:
        //           name: my-other-secret
        //           mountPath: /somewhere
        //       - configMap:
        //           name: my-third-cm
        //           mountPath: /config
        //       - emptyDir:
        //           medium: Memory
        //           mountPath: /tmp
        //       - emptyDir:
        //           medium: Disk
        //           mountPath: /mnt
        //       - persistentVolumeClaim:
        //           name: my-pvc
        //           mountPath: /tmp
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
pub struct Port {
    /// The kind of port to expose.
    pub kind: PortKind,

    /// Container port to expose.
    pub port: u16,

    /// If this port should be exposed as an ingress.
    /// `ingress` is only valid on ports of kind PortKind::HTTP.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ingress: Vec<Ingress>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PortKind {
    #[default]
    HTTP,
    Metrics,
    TCP,
}

impl Port {
    pub fn name(&self) -> String {
        self.kind.to_string().to_lowercase()
    }
}

impl Display for PortKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
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
    pub grpc: Option<GrpcAction>,

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

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrpcAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default = "default_grpc_service")]
    pub service: Option<String>,

    #[serde(flatten)]
    pub config: ProbeConfig,
}

fn default_grpc_service() -> Option<String> {
    None
}

impl Default for GrpcAction {
    fn default() -> Self {
        GrpcAction {
            service: default_grpc_service(),
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
