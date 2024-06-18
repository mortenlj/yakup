use std::sync::Arc;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::PodTemplateSpec;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use tracing::instrument;

use api::Application;

use crate::models::{Operation, OperationType};
use crate::Result;

#[instrument()]
pub(crate) async fn process(_obj: Arc<Application>) -> Result<Vec<Operation>> {
    Ok(vec![Operation {
        operation_type: OperationType::CreateOrUpdate,
        object: Arc::new(Deployment {
            metadata: ObjectMeta {
                name: Some("test-deployment".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: Some(k8s_openapi::api::apps::v1::DeploymentSpec {
                replicas: Some(1),
                selector: LabelSelector {
                    match_labels: Some(
                        [("app".to_string(), "test".to_string())]
                            .iter()
                            .cloned()
                            .collect(),
                    ),
                    ..Default::default()
                },
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(
                            [("app".to_string(), "test".to_string())]
                                .iter()
                                .cloned()
                                .collect(),
                        ),
                        ..Default::default()
                    }),
                    spec: Some(k8s_openapi::api::core::v1::PodSpec {
                        containers: vec![k8s_openapi::api::core::v1::Container {
                            name: "test".to_string(),
                            image: Some(_obj.spec.image.clone()),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        }),
    }])
}