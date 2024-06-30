use std::fmt::{Display, Formatter};
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
use serde::{Deserialize, Serialize};
use tracing::log::{debug, info};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "operation", content = "object")]
pub enum Operation {
    CreateOrUpdate(Arc<DynamicObject>),
    DeleteIfExists(Arc<DynamicObject>),
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::CreateOrUpdate(_obj) => write!(f, "CreateOrUpdate"),
            Operation::DeleteIfExists(_obj) => write!(f, "DeleteIfExists"),
        }
    }
}

impl Operation {
    pub async fn apply(self: &Self, client: Client) -> Result<Arc<DynamicObject>, Error> {
        match self {
            Operation::CreateOrUpdate(object) => {
                self.apply_create_or_update(client, object).await?;
                Ok(object.clone())
            }
            Operation::DeleteIfExists(object) => {
                self.apply_delete_if_exists(client, object).await?;
                Ok(object.clone())
            }
        }
    }

    pub async fn gvk(self: &Self, object: &Arc<DynamicObject>) -> Result<GroupVersionKind, Error> {
        let gvk = if let Some(tm) = &object.types {
            GroupVersionKind::try_from(tm).map_err(|_| Error::ConfigError)?
        } else {
            return Err(Error::ConfigError);
        };
        Ok(gvk)
    }

    async fn apply_create_or_update(&self, client: Client, object: &Arc<DynamicObject>) -> Result<(), Error> {
        let discovery = Discovery::new(client.clone()).run().await.map_err(|_| Error::ConfigError)?;
        let namespace = object.metadata.namespace.as_deref();
        let gvk = self.gvk(object).await?;
        let api = if let Some((ar, caps)) = discovery.resolve_gvk(&gvk) {
            dynamic_api(ar, caps, client.clone(), namespace, false)
        } else {
            return Err(Error::ConfigError);
        };

        let object_name = object.metadata.name.clone().unwrap();
        let existing = api.get(&object_name).await;
        match existing {
            Ok(existing_obj) => {
                debug!("{} {:?} already exists", gvk.kind, object_name);
                let mut obj = object.deref().clone();
                obj.metadata.resource_version = existing_obj.metadata.resource_version.clone();
                api.replace(&object_name, &PostParams::default(), &obj).await.map_err(|_| Error::ConfigError)?;
            }
            Err(e) => {
                if let KubeError::Api(api_error) = e {
                    if vec![404, 410].contains(&api_error.code) {
                        debug!("{} {:?} not found, creating", gvk.kind, object_name);
                        api.create(&PostParams::default(), &object).await.map_err(|_| Error::ConfigError)?;
                    } else {
                        return Err(Error::ConfigError);
                    }
                }
            }
        }
        Ok(())
    }

    async fn apply_delete_if_exists(&self, client: Client, object: &Arc<DynamicObject>) -> Result<(), Error> {
        let discovery = Discovery::new(client.clone()).run().await.map_err(|_| Error::ConfigError)?;
        let namespace = object.metadata.namespace.as_deref();
        let gvk = self.gvk(object).await?;
        let api = if let Some((ar, caps)) = discovery.resolve_gvk(&gvk) {
            dynamic_api(ar, caps, client.clone(), namespace, false)
        } else {
            return Err(Error::ConfigError);
        };

        let object_name = object.metadata.name.clone().unwrap();
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
