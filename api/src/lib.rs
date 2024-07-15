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
    pub ports: Vec<Port>,

    #[serde(default)]
    pub probes: Option<Probes>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub struct ApplicationStatus {
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub struct Port {
    pub kind: PortKind,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub enum PortKind {
    #[default]
    #[serde(alias = "http")]
    HTTP,
    #[serde(alias = "metrics")]
    Metrics,
    #[serde(alias = "tcp")]
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
pub struct Probes {
    pub readiness: Option<Probe>,
    pub liveness: Option<Probe>,
    pub startup: Option<Probe>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub struct Probe {
    pub initial_delay_seconds: u16,
    pub port_name: String,
    pub http_action: Option<HttpAction>,
    pub grpc_action: Option<GrpcAction>,
    pub tcp_action: Option<TcpAction>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct HttpAction {
    path: Option<String>
}

impl Default for HttpAction {
    fn default() -> Self {
        HttpAction { path: Some("/".to_string()) }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct GrpcAction {
    service: Option<String>
}

impl Default for GrpcAction {
    fn default() -> Self {
        GrpcAction { service: None }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub struct TcpAction {
}

