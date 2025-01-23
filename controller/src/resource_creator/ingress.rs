use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use api::v1::{Application, IngressZone};
use api::Port;
use k8s_openapi::api::networking::v1::{HTTPIngressPath, HTTPIngressRuleValue, Ingress, IngressBackend, IngressRule, IngressServiceBackend, IngressSpec, ServiceBackendPort};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::ResourceExt;
use tracing::instrument;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;

#[instrument(skip(zones, app), fields(trace_id))]
pub(crate) fn process(
    app: &Arc<Application>,
    zones: &HashMap<String, Arc<IngressZone>>,
    object_meta: ObjectMeta,
    labels: BTreeMap<String, String>,
) -> Result<Vec<Operation>> {
    let ingresses: Vec<Ingress> = app
        .spec
        .ports
        .iter()
        .filter(|port| !port.ingress.is_empty())
        .map(|port| generate_ingresses(app.clone(), zones, object_meta.clone(), port))
        .flatten()
        .collect();
    let operations = ingresses
        .iter()
        .map(|ingress| -> Result<Operation> {
            let dynamic_object = to_dynamic_object(ingress.clone())?;
            Ok(Operation::CreateOrUpdate(Arc::new(dynamic_object)))
        })
        .filter_map(|operation| match operation {
            Ok(operation) => Some(operation),
            Err(e) => {
                tracing::error!(
                    error = e.to_string(),
                    "Failed to create operation for ingress"
                );
                None
            }
        })
        .collect();
    Ok(operations)
}

fn generate_ingresses(
    app: Arc<Application>,
    zones: &HashMap<String, Arc<IngressZone>>,
    object_meta: ObjectMeta,
    port: &Port,
) -> Vec<Ingress> {
    let ingresses = port
        .ingress
        .iter()
        .map(|ingress| generate_ingress(app.clone(), zones, object_meta.clone(), ingress))
        .filter_map(|ingress| match ingress {
            Ok(ingress) => Some(ingress),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to create ingress");
                None
            }
        })
        .collect();
    ingresses
}

fn generate_ingress(
    app: Arc<Application>,
    zones: &HashMap<String, Arc<IngressZone>>,
    mut object_meta: ObjectMeta,
    ingress: &api::Ingress,
) -> Result<Ingress> {
    let zone = zones
        .get(&ingress.zone)
        .ok_or_else(|| anyhow!("Ingress zone not found"))?;
    let host = zone.spec.host.replace("{appname}", app.name_any().as_str());
    let paths = ingress.paths.iter()
        .map(|path| {
            HTTPIngressPath {
                backend: IngressBackend {
                    resource: None,
                    service: Some(IngressServiceBackend {
                        name: app.name_any(),
                        port: Some(ServiceBackendPort {
                            name: Some("http".to_string()),
                            number: None,
                        }),
                    }),
                },
                path: Some(path.clone()),
                path_type: ingress.path_type.clone().unwrap_or_default().to_string(),
            }
        })
        .collect();
    object_meta.name = Some(format!("{}-{}", app.name_any(), zone.name_any()));
    let ingress = Ingress {
        metadata: object_meta,
        spec: Some(IngressSpec {
            ingress_class_name: zone.spec.ingress_class.clone(),
            rules: Some(vec![IngressRule {
                host: Some(host),
                http: Some(HTTPIngressRuleValue {
                    paths,
                })
            }]),
            // TODO: TLS
            ..Default::default()
        }),
        ..Default::default()
    };
    Ok(ingress)
}
