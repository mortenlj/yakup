use anyhow::Result;
use controller::run;

#[tokio::main]
async fn main() -> Result<()> {
    run().await?;
    Ok(())
}
