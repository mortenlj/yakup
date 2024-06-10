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
    doc = "Ibidem Deploy Daemon Application",
    printcolumn = r#"{"name":"Image","type":"string","jsonPath":".spec.image"}"#,
)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationSpec {
    image: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
pub struct ApplicationStatus {
    conditions: Vec<Condition>,
}
