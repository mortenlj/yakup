use std::sync::Arc;

use api::Application;

use crate::models::Operation;
use crate::Result;

mod deployment;

pub async fn process(obj: Arc<Application>) -> Result<Vec<Operation>> {
    deployment::process(obj).await
}