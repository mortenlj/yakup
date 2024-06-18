use std::sync::Arc;
use either::Either;

use k8s_openapi::api::apps::v1::Deployment;
use kube::{Api, Client, Error};
use kube::api::{DeleteParams, PostParams};
use tracing::log::{debug, info};

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
        match self.operation_type {
            OperationType::CreateOrUpdate => {
                self.apply_create_or_update(client).await
            }
            OperationType::DeleteIfExists => {
                self.apply_delete_if_exists(client).await
            }
        }
    }

    async fn apply_create_or_update(&self, client: Client) -> Result<(), Error> {
        let ns = self.object.metadata.namespace.clone().unwrap();
        let object_name = self.object.metadata.name.clone().unwrap();
        let api: Api<Deployment> = Api::namespaced(client, &ns);

        let existing = api.get(&object_name).await;
        match existing {
            Ok(deployment) => {
                debug!("Deployment {:?} already exists", deployment);
                api.replace(&object_name, &PostParams::default(), &self.object).await?;
            }
            Err(e) => {
                if let Error::Api(api_error) = e {
                    if vec![404, 410].contains(&api_error.code) {
                        debug!("Deployment {:?} not found, creating", object_name);
                        api.create(&PostParams::default(), &self.object).await?;
                    } else {
                        return Err(Error::Api(api_error))
                    }
                }
            }
        }
        Ok(())
    }

    async fn apply_delete_if_exists(&self, client: Client) -> Result<(), Error> {
        let ns = self.object.metadata.namespace.clone().unwrap();
        let object_name = self.object.metadata.name.clone().unwrap();
        let api: Api<Deployment> = Api::namespaced(client, &ns);

        match api.delete(object_name.as_str(), &DeleteParams::default()).await {
            Ok(res) => {
                match res {
                    Either::Left(_deployment) => {
                        info!("Deleting deployment {:?}", object_name)
                    }
                    Either::Right(_status) => {
                        info!("Deployment {:?} deleted successfully", object_name)
                    }
                }
            }
            Err(e) => {
                if let Error::Api(api_error) = e {
                    if vec![404, 410].contains(&api_error.code) {
                        info!("Deployment {:?} not found", object_name)
                    } else {
                        return Err(Error::Api(api_error))
                    }
                }
            }
        }
        Ok(())
    }
}