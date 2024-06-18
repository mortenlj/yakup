use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{Api, Client, ResourceExt};
use kube::runtime::controller::Action;
use kube::runtime::controller::Controller;
use opentelemetry_sdk::trace::Tracer;
use tokio;
use tracing::{error, field, info, instrument, Span};
use tracing_opentelemetry::OpenTelemetryLayer;
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
    let telemetry = init_tracer().await;
    let collector = Registry::default().with(telemetry).with(logger).with(env_filter);
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

#[instrument(skip(ctx), fields(trace_id))]
async fn reconcile(obj: Arc<Application>, ctx: Arc<Context>) -> Result<Action> {
    let trace_id = get_trace_id();
    Span::current().record("trace_id", &field::display(&trace_id));

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

async fn init_tracer() -> Option<OpenTelemetryLayer<Registry, Tracer>> {
    if let Ok(otlp_endpoint) = std::env::var("OPENTELEMETRY_ENDPOINT_URL") {
        let channel = tonic::transport::Channel::from_shared(otlp_endpoint)
            .unwrap()
            .connect()
            .await
            .unwrap();

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_channel(channel))
            .with_trace_config(opentelemetry_sdk::trace::config().with_resource(
                opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                    "service.name",
                    "yakup",
                )]),
            ))
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .unwrap();
        return Some(tracing_opentelemetry::layer().with_tracer(tracer))
    }
    None
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
