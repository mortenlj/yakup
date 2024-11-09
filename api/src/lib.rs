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

        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub env: Vec<EnvValue>,

        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub env_from: Vec<EnvFrom>,

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
        //     resources:
        //       limits:
        //         cpu: 500m
        //         memory: 512Mi
        //       requests:
        //         cpu: 200m
        //         memory: 256Mi
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
        //  # Communication
        //   ingress:
        //     routes:
        //       - host: "myapp.nav.no"
        //         path: "/asd"
        //         port: 8080 # container port
        //         type: http # default
        //       - host: "grpc.nav.no"
        //         path: "/service"
        //         port: 8082
        //         type: grpc
        //       - host: "myapp-admin.nav.no"
        //         path: "/"
        //         port: 8081
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

#[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvFrom {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_map: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
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
