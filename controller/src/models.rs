use std::ops::Deref;
use std::sync::Arc;

use crate::Error;
use either::Either;
use kube::{
    Error as KubeError,
    api::{Api, DynamicObject, DeleteParams, PostParams},
    Client,
    core::GroupVersionKind,
    discovery::{ApiCapabilities, ApiResource, Discovery, Scope},
};
use tracing::log::{debug, info};

#[derive(Debug)]
pub enum OperationType {
    CreateOrUpdate,
    DeleteIfExists,
}

#[derive(Debug)]
pub struct Operation {
    pub operation_type: OperationType,
    pub object: Arc<DynamicObject>,
}

impl Operation {
    pub async fn apply(self: &Self, client: Client) -> Result<(), Error> {
        match self.operation_type {
            OperationType::CreateOrUpdate => {
                self.apply_create_or_update(client).await
            }
            OperationType::DeleteIfExists => {
                self.apply_delete_if_exists(client).await
            }
        }
    }

    pub async fn gvk(self: &Self) -> Result<GroupVersionKind, Error> {
        let gvk = if let Some(tm) = &self.object.types {
            GroupVersionKind::try_from(tm).map_err(|_| Error::ConfigError)?
        } else {
            return Err(Error::ConfigError);
        };
        Ok(gvk)
    }

    async fn apply_create_or_update(&self, client: Client) -> Result<(), Error> {
        let discovery = Discovery::new(client.clone()).run().await.map_err(|_| Error::ConfigError)?;
        let namespace = self.object.metadata.namespace.as_deref();
        let gvk = self.gvk().await?;
        let api = if let Some((ar, caps)) = discovery.resolve_gvk(&gvk) {
            dynamic_api(ar, caps, client.clone(), namespace, false)
        } else {
            return Err(Error::ConfigError);
        };

        let object_name = self.object.metadata.name.clone().unwrap();
        let existing = api.get(&object_name).await;
        match existing {
            Ok(existing_obj) => {
                debug!("{} {:?} already exists", gvk.kind, object_name);
                let mut obj = self.object.deref().clone();
                obj.metadata.resource_version = existing_obj.metadata.resource_version.clone();
                api.replace(&object_name, &PostParams::default(), &obj).await.map_err(|_| Error::ConfigError)?;
            }
            Err(e) => {
                if let KubeError::Api(api_error) = e {
                    if vec![404, 410].contains(&api_error.code) {
                        debug!("{} {:?} not found, creating", gvk.kind, object_name);
                        api.create(&PostParams::default(), &self.object).await.map_err(|_| Error::ConfigError)?;
                    } else {
                        return Err(Error::ConfigError);
                    }
                }
            }
        }
        Ok(())
    }

    async fn apply_delete_if_exists(&self, client: Client) -> Result<(), Error> {
        let discovery = Discovery::new(client.clone()).run().await.map_err(|_| Error::ConfigError)?;
        let namespace = self.object.metadata.namespace.as_deref();
        let gvk = self.gvk().await?;
        let api = if let Some((ar, caps)) = discovery.resolve_gvk(&gvk) {
            dynamic_api(ar, caps, client.clone(), namespace, false)
        } else {
            return Err(Error::ConfigError);
        };

        let object_name = self.object.metadata.name.clone().unwrap();
        match api.delete(object_name.as_str(), &DeleteParams::default()).await {
            Ok(res) => {
                match res {
                    Either::Left(_obj) => {
                        info!("Deleting {} {:?}", gvk.kind, object_name)
                    }
                    Either::Right(_status) => {
                        info!("{} {:?} deleted successfully", gvk.kind, object_name)
                    }
                }
            }
            Err(e) => {
                if let KubeError::Api(api_error) = e {
                    if vec![404, 410].contains(&api_error.code) {
                        info!("{} {:?} not found", gvk.kind, object_name)
                    } else {
                        return Err(Error::ConfigError);
                    }
                }
            }
        }
        Ok(())
    }
}


fn dynamic_api(
    ar: ApiResource,
    caps: ApiCapabilities,
    client: Client,
    ns: Option<&str>,
    all: bool,
) -> Api<DynamicObject> {
    if caps.scope == Scope::Cluster || all {
        Api::all_with(client, &ar)
    } else if let Some(namespace) = ns {
        Api::namespaced_with(client, namespace, &ar)
    } else {
        Api::default_namespaced_with(client, &ar)
    }
}
