use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{Api, Client, ResourceExt};
use kube::runtime::controller::Action;
use kube::runtime::controller::Controller;
use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::Tracer;
use opentelemetry_semantic_conventions::resource;
use tokio;
use tracing::{error, field, info, instrument, Span};
use tracing::level_filters::LevelFilter;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, Registry};
use tracing_subscriber::prelude::*;

use api::Application;

mod models;
mod resource_creator;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Config error")]
    ConfigError
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Context {
    pub client: Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    let logger = tracing_subscriber::fmt::layer().compact();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    match init_tracer().await {
        Ok(telemetry) => {
            let max_level_hint = env_filter.max_level_hint().unwrap_or(LevelFilter::OFF);
            let log_msg = format!("Starting controller with log level {:?}", max_level_hint);

            Registry::default()
                .with(telemetry)
                .with(logger)
                .with(env_filter)
                .init();

            info!(log_msg);
        }
        Err(e) => {
            error!("Error initializing OpenTelemetry: {:?}", e);
            return Err(Error::ConfigError);
        }
    }

    let client = Client::try_default().await.map_err(|_| Error::ConfigError)?;
    let apps = Api::<Application>::all(client.clone());
    let deployments = Api::<Deployment>::all(client.clone());

    Controller::new(apps.clone(), Default::default())
        .owns(deployments.clone(), Default::default())
        .run(reconcile, error_policy, Arc::new(Context { client }))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}

#[instrument(skip(ctx), fields(trace_id))]
async fn reconcile(obj: Arc<Application>, ctx: Arc<Context>) -> Result<Action> {
    let trace_id = get_trace_id();
    Span::current().record("trace_id", &field::display(&trace_id));

    info!("reconcile request: {}", obj.name_any());
    let operations = resource_creator::process(obj).await?;
    for operation in operations.iter() {
        match operation.apply(ctx.client.clone()).await {
            Ok(object) => {
                let gvk = operation.gvk(&object).await.map_err(|_| {Error::ConfigError})?;
                info!("Operation {} for {} {} applied successfully", operation, gvk.kind, object.metadata.name.as_ref().unwrap());
            }
            Err(e) => {
                error!("Error applying operation: {:?}", e);
                return Ok(Action::requeue(Duration::from_secs(5)));
            }
        }
    }
    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy(_object: Arc<Application>, err: &Error, _ctx: Arc<Context>) -> Action {
    error!("Error occurred during reconciliation: {:?}", err);
    Action::requeue(Duration::from_secs(5))
}

async fn init_tracer() -> Result<OpenTelemetryLayer<Registry, Tracer>> {
    let otel_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_trace_config(
            opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![
                KeyValue::new(resource::K8S_DEPLOYMENT_NAME, "yakup"),
                KeyValue::new(resource::SERVICE_NAME, "yakup"),
            ])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio).map_err(|_| Error::ConfigError)?;
    return Ok(tracing_opentelemetry::layer().with_tracer(otel_tracer));
}

pub fn get_trace_id() -> opentelemetry::trace::TraceId {
    use opentelemetry::trace::TraceContextExt as _;
    use tracing_opentelemetry::OpenTelemetrySpanExt as _;

    Span::current()
        .context()
        .span()
        .span_context()
        .trace_id()
}
