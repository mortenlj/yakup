use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use api::application::v1::Application;
use api::application::Port;
use api::ingress_zone::v1::IngressZone;
use k8s_openapi::api::networking::v1::{
    HTTPIngressPath, HTTPIngressRuleValue, Ingress, IngressBackend, IngressRule,
    IngressServiceBackend, IngressSpec, IngressTLS, ServiceBackendPort,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::ResourceExt;
use md5::{Digest, Md5};
use tracing::instrument;

use crate::models::Operation;
use crate::resource_creator::to_dynamic_object;

#[instrument(skip(zones, app), fields(trace_id))]
pub(crate) fn process(
    app: &Arc<Application>,
    zones: &HashMap<String, Arc<IngressZone>>,
    object_meta: ObjectMeta,
) -> Result<Vec<Operation>> {
    let mut possible_ingresses: HashSet<String> =
        HashSet::from_iter(zones.keys().map(|k| format!("{}-{}", app.name_any(), k)));

    let ingresses: Vec<Ingress> = app
        .spec
        .ports
        .iter()
        .filter(|port| !port.ingress.is_empty())
        .flat_map(|port| generate_ingresses(app.clone(), zones, object_meta.clone(), port))
        .inspect(|ingress| {
            let ingress_labels = ingress.metadata.labels.clone().unwrap_or_default();
            if let Some(zone_name) = ingress_labels.get("yakup.ibidem.no/ingress_zone") {
                possible_ingresses.remove(&zone_name.clone());
            }
        })
        .collect();

    let mut operations: Vec<Operation> = ingresses
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

    for ingress_name in possible_ingresses {
        let delete_ingress = Ingress {
            metadata: ObjectMeta {
                name: Some(ingress_name),
                namespace: object_meta.namespace.clone(),
                ..Default::default()
            },
            ..Default::default()
        };
        let dynamic_object = to_dynamic_object(delete_ingress)?;
        operations.push(Operation::DeleteIfExists(Arc::new(dynamic_object)));
    }

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
    ingress: &api::application::Ingress,
) -> Result<Ingress> {
    let zone = zones
        .get(&ingress.zone)
        .ok_or_else(|| anyhow!("Ingress zone not found"))?;

    let host = zone.spec.host.replace("{appname}", app.name_any().as_str());

    let paths = ingress
        .paths
        .iter()
        .map(|path| HTTPIngressPath {
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
        })
        .collect();

    object_meta.name = Some(format!("{}-{}", app.name_any(), zone.name_any()));
    if let Some(labels) = &mut object_meta.labels {
        labels.insert(
            "yakup.ibidem.no/ingress_zone".to_string(),
            zone.name_any().clone(),
        );
    } else {
        object_meta.labels = Some(BTreeMap::from([(
            "yakup.ibidem.no/ingress_zone".to_string(),
            zone.name_any().clone(),
        )]));
    }

    let tls = match &zone.spec.tls {
        Some(zone_tls) => {
            object_meta.annotations = Some(BTreeMap::from([(
                "cert-manager.io/cluster-issuer".to_string(),
                zone_tls.cluster_issuer.clone().unwrap_or_default(),
            )]));
            let hosts_md5 = Md5::digest(host.clone().as_bytes());
            let hosts_id = fast32::base32::CROCKFORD_LOWER.encode(&hosts_md5);
            Some(vec![IngressTLS {
                hosts: Some(vec![host.clone()]),
                secret_name: Some(format!("cert-ingress-{}", hosts_id)),
            }])
        }
        None => None,
    };

    let ingress = Ingress {
        metadata: object_meta,
        spec: Some(IngressSpec {
            ingress_class_name: zone.spec.ingress_class.clone(),
            rules: Some(vec![IngressRule {
                host: Some(host.clone()),
                http: Some(HTTPIngressRuleValue { paths }),
            }]),
            tls,
            ..Default::default()
        }),
        ..Default::default()
    };
    Ok(ingress)
}
