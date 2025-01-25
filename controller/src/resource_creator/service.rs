use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use tracing::instrument;

use api::application::v1::Application;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;

#[instrument(skip(app, object_meta), fields(trace_id))]
pub(crate) fn process(
    app: &Arc<Application>,
    object_meta: ObjectMeta,
    labels: BTreeMap<String, String>,
) -> Result<Vec<Operation>> {
    let ports = generate_ports(app.clone());
    if ports.is_none() || ports.as_ref().unwrap().is_empty() {
        return Ok(vec![Operation::DeleteIfExists(Arc::new(
            to_dynamic_object(Service {
                metadata: object_meta,
                ..Default::default()
            })?,
        ))]);
    }
    let svc = Service {
        metadata: object_meta,
        spec: Some(ServiceSpec {
            selector: Some(labels.clone()),
            ports,
            ..Default::default()
        }),
        ..Default::default()
    };

    Ok(vec![Operation::CreateOrUpdate(Arc::new(
        to_dynamic_object(svc)?,
    ))])
}

fn generate_ports(app: Arc<Application>) -> Option<Vec<ServicePort>> {
    let mut service_ports = Vec::new();
    if let Some(ports) = &app.spec.ports {
        if let Some(_http_port) = &ports.http {
            service_ports.push(ServicePort {
                name: Some("http".to_string()),
                port: 80,
                target_port: Some(IntOrString::String("http".to_string())),
                ..Default::default()
            });
        }

        if let Some(tcp_port) = &ports.tcp {
            service_ports.push(ServicePort {
                name: Some("tcp".to_string()),
                port: tcp_port.port as i32,
                target_port: Some(IntOrString::String("tcp".to_string())),
                ..Default::default()
            });
        }
    }
    if service_ports.is_empty() {
        None
    } else {
        Some(service_ports)
    }
}
