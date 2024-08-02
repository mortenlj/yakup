use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use k8s_openapi::serde::{Deserialize, Serialize};
use kube::CustomResource;
use schemars::JsonSchema;
use std::fmt::Display;

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
    pub image: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probes: Option<Probes>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationStatus {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    pub kind: PortKind,
    pub port: u16,
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
    pub fn name(self: &Self) -> String {
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
