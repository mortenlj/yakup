use kube::CustomResource;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub ingress_class: Option<String>,

        /// TLS configuration for this zone.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tls: Option<IngressZoneTLS>,
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IngressZoneTLS {
    /// The cluster_issuer to use for this zone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_issuer: Option<String>,
}