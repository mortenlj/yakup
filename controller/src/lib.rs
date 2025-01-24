use futures::join;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures::StreamExt;
use k8s_openapi::api::apps::v1::Deployment;
use kube::runtime::controller::Action;
use kube::runtime::controller::Controller;
use kube::{Api, Client};
use opentelemetry::trace::TraceId;
use opentelemetry::KeyValue;
use opentelemetry_sdk::trace::Tracer;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource;
use tokio::sync::RwLock;
use tracing::level_filters::LevelFilter;
use tracing::{error, field, info, instrument, warn, Span};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry};

use api::v1::{Application, IngressZone};

pub mod models;
pub mod resource_creator;

#[derive(thiserror::Error, Debug)]
enum ReconcilerError {
    #[error("unable to resolve gvk for dynamic object")]
    GvkLookup,
    #[error("processing resource")]
    ResourceProcessing,
    #[error("applying operations")]
    ApplyOperations,
}

type ReconcileResult<T, E = ReconcilerError> = std::result::Result<T, E>;

pub struct Context {
    pub client: Client,
    pub ingress_zones: RwLock<HashMap<String, Arc<IngressZone>>>,
}

pub async fn run() -> Result<()> {
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
            return Err(e.context("initializing OpenTelemetry"));
        }
    }

    let client = Client::try_default()
        .await
        .map_err(|e| anyhow!(e).context("initializing Kubernetes client"))?;
    let apps = Api::<Application>::all(client.clone());
    let _deployments = Api::<Deployment>::all(client.clone());
    let ingress_zones = Api::<IngressZone>::all(client.clone());

    let ctx = Arc::new(Context {
        client,
        ingress_zones: RwLock::new(HashMap::new()),
    });

    let app_controller = Controller::new(apps.clone(), Default::default())
        //.owns(deployments.clone(), Default::default())
        .run(reconcile_apps, error_policy_apps, ctx.clone())
        .for_each(|_| futures::future::ready(()));
    let zone_controller = Controller::new(ingress_zones.clone(), Default::default())
        .run(reconcile_zones, error_policy_zones, ctx.clone())
        .for_each(|_| futures::future::ready(()));

    join!(app_controller, zone_controller);

    Ok(())
}

#[instrument(skip(ctx, obj), fields(trace_id))]
async fn reconcile_zones(obj: Arc<IngressZone>, ctx: Arc<Context>) -> ReconcileResult<Action> {
    let trace_id = get_trace_id();
    Span::current().record("trace_id", field::display(&trace_id));
    let mut zones = ctx.ingress_zones.write().await;
    let zone_name = obj.metadata.name.as_ref().unwrap().clone();
    info!("reconcile request received for zone {}", zone_name);
    zones.insert(zone_name, obj.clone());
    Ok(Action::requeue(Duration::from_secs(3600)))
}

#[instrument(skip(ctx, obj), fields(trace_id))]
async fn reconcile_apps(obj: Arc<Application>, ctx: Arc<Context>) -> ReconcileResult<Action> {
    let trace_id = get_trace_id();
    Span::current().record("trace_id", field::display(&trace_id));

    let zones = ctx.ingress_zones.read().await;

    info!("reconcile request received");
    match resource_creator::process(obj, &zones) {
        Err(e) => {
            error!("Error processing resource: {:?}", e);
            return Err(ReconcilerError::ResourceProcessing);
        }
        Ok(operations) => {
            for operation in operations.iter() {
                match operation.apply(ctx.client.clone()).await {
                    Ok(object) => {
                        let gvk = operation
                            .gvk(&object)
                            .await
                            .map_err(|_| ReconcilerError::GvkLookup)?;
                        info!(
                            "Operation {} for {} {} applied successfully",
                            operation,
                            gvk.kind,
                            object.metadata.name.as_ref().unwrap()
                        );
                    }
                    Err(e) => {
                        error!("Error applying operation: {:?}", e);
                        return Err(ReconcilerError::ApplyOperations);
                    }
                }
            }
        }
    };
    Ok(Action::requeue(Duration::from_secs(3600)))
}

fn error_policy_apps(
    _object: Arc<Application>,
    err: &ReconcilerError,
    _ctx: Arc<Context>,
) -> Action {
    warn!("Error occurred during reconciliation: {:?}", err);
    Action::requeue(Duration::from_secs(5))
}

fn error_policy_zones(
    _object: Arc<IngressZone>,
    err: &ReconcilerError,
    _ctx: Arc<Context>,
) -> Action {
    warn!("Error occurred during reconciliation: {:?}", err);
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
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .map_err(|e| anyhow!(e).context("installing opentelemetry tracker"))?;
    Ok(tracing_opentelemetry::layer().with_tracer(otel_tracer))
}

pub fn get_trace_id() -> TraceId {
    use opentelemetry::trace::TraceContextExt as _;
    use tracing_opentelemetry::OpenTelemetrySpanExt as _;

    Span::current().context().span().span_context().trace_id()
}
