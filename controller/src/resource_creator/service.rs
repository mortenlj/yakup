use std::collections::BTreeMap;
use std::sync::Arc;
use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use tracing::instrument;

use api::Application;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;
use crate::Result;

#[instrument()]
pub(crate) async fn process(app: &Arc<Application>, object_meta: ObjectMeta, labels: BTreeMap<String, String>) -> Result<Vec<Operation>> {
    let ports = generate_ports(app);
    if ports.is_none() || ports.as_ref().unwrap().is_empty() {
        return Ok(vec![Operation::DeleteIfExists(Arc::new(to_dynamic_object(Service {
            metadata: object_meta,
            ..Default::default()
        })?))]);
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

    Ok(vec![
        Operation::CreateOrUpdate(Arc::new(to_dynamic_object(svc)?))
    ])
}

fn generate_ports(app: &Arc<Application>) -> Option<Vec<ServicePort>> {
    app.spec.ports.as_ref().map(|ports| {
        ports.iter().map(|port| {
            let port_num : i32 = match port.kind {
                api::PortKind::HTTP => 80,
                api::PortKind::Metrics => 9090,
                api::PortKind::TCP => port.port as i32,
            };
            ServicePort {
                name: Some(port.name()),
                port: port_num,
                target_port: Some(IntOrString::String(port.name())),
                ..Default::default()
            }
        }).collect()
    })
}
