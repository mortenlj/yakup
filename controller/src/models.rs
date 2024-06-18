use std::sync::Arc;

use k8s_openapi::api::apps::v1::Deployment;
use kube::Api;
use kube::api::PostParams;
use tracing::log::{debug, info};

const PP: PostParams = PostParams{ dry_run: false, field_manager: None };

#[derive(Debug)]
pub enum OperationType {
    CreateOrUpdate,
    DeleteIfExists,
}


#[derive(Debug)]
pub struct Operation {
    pub operation_type: OperationType,
    pub object: Arc<Deployment>,
}

impl Operation {
    pub async fn apply(self: &Self, client: kube::Client) -> Result<(), kube::Error> {
        let ns = self.object.metadata.namespace.clone().unwrap();
        let object_name = self.object.metadata.name.clone().unwrap();
        let api: Api<Deployment> = Api::namespaced(client, &ns);

        let existing = api.get(&self.object.metadata.name.as_ref().unwrap().as_str()).await;
        match existing {
            Ok(deployment) => {
                debug!("Deployment {:?} already exists", deployment);
                match self.operation_type {
                    OperationType::CreateOrUpdate => {
                        debug!("replacing existing deployment");
                        api.replace(&object_name, &PP, &self.object).await?;
                    }
                    OperationType::DeleteIfExists => {}
                }
            }
            Err(e) => {
                // TODO: Check for not found error before assuming
                // Error: Api(ErrorResponse { status: "Failure", message: "deployments.apps \"test-deployment\" not found", reason: "NotFound", code: 404 })
                info!("error getting deployment {}: {:?}", object_name, e);
                match self.operation_type {
                    OperationType::CreateOrUpdate => {
                        debug!("attempting create");
                        api.create(&PP, &self.object).await?;
                    }
                    OperationType::DeleteIfExists => {
                        debug!("assuming not found, skipping delete");
                    }
                }
            }
        }
        Ok(())
    }
}