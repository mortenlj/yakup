use std::sync::Arc;
use tracing::instrument;

use api::Application;

use crate::models::Operation;
use crate::Result;

mod deployment;

#[instrument()]
pub async fn process(obj: Arc<Application>) -> Result<Vec<Operation>> {
    deployment::process(obj).await
}