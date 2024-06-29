use std::fmt::Display;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use k8s_openapi::serde::{Deserialize, Serialize};
use kube::CustomResource;
use schemars::JsonSchema;

#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(
    group = "yakup.ibidem.no",
    version = "v1",
    kind = "Application",
    namespaced,
    status = "ApplicationStatus",
    shortname = "app",
    doc = "Yet Another Application Kind",
    printcolumn = r#"{"name":"Image","type":"string","jsonPath":".spec.image"}"#,
)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationSpec {
    pub image: String,
    pub ports: Option<Vec<Port>>,
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
