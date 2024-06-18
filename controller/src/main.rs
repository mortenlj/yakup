use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{Api, Client, ResourceExt};
use kube::runtime::controller::Action;
use kube::runtime::controller::Controller;
use tokio;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, Registry};
use tracing_subscriber::prelude::*;

use api::Application;

mod models;
mod resource_creator;

pub type Result<T, E = kube::Error> = std::result::Result<T, E>;

pub struct Context {
    pub client: Client,
}


#[tokio::main]
async fn main() -> Result<()> {
    let logger = tracing_subscriber::fmt::layer().json();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    let collector = Registry::default().with(logger).with(env_filter);
    tracing::subscriber::set_global_default(collector).unwrap();

    info!("Starting controller");

    let client = Client::try_default().await?;
    let apps = Api::<Application>::all(client.clone());
    let deployments = Api::<Deployment>::all(client.clone());

    Controller::new(apps.clone(), Default::default())
        .owns(deployments.clone(), Default::default())
        .run(reconcile, error_policy, Arc::new(Context { client }))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}

async fn reconcile(obj: Arc<Application>, ctx: Arc<Context>) -> Result<Action> {
    info!("reconcile request: {}", obj.name_any());
    let operations = resource_creator::process(obj).await?;
    for operation in operations.iter() {
        match operation.apply(ctx.client.clone()).await {
            Ok(_) => {
                info!("Operation {:?} for resource {:?} applied successfully", operation.operation_type, operation.object);
            },
            Err(e) => {
                error!("Error applying operation: {:?}", e);
                return Ok(Action::requeue(Duration::from_secs(5)));
            }
        }
    }
    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_object: Arc<Application>, err: &kube::Error, _ctx: Arc<Context>) -> Action {
    error!("Error occurred during reconciliation: {:?}", err);
    Action::requeue(Duration::from_secs(5))
}
