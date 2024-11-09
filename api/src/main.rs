use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use kube::core::crd::merge_crds;
use kube::CustomResourceExt;

use api::v1;

pub fn main() -> Result<()> {
    let crd_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/crd/application.yaml");
    println!("Creating CRD manifest at {:?}", crd_file);
    fs::create_dir_all(crd_file.parent().unwrap())?;

    let crds = vec![
        v1::Application::crd(),
    ];
    let merged = merge_crds(crds, "v1").context("Failed to merge CRDs")?;

    let contents = ["---".to_string(),
        serde_yaml::to_string(&merged).unwrap()];
    let contents = contents.join("\n");
    fs::write(crd_file, contents).context("Failed to write file")
}
