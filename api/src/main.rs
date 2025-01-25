use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use kube::core::crd::merge_crds;
use kube::CustomResourceExt;

use api::ingress_zone::v1 as ingress_zone_v1;
use api::application::v1 as application_v1;

pub fn main() -> Result<()> {
    let crd_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/crd/manifests.yaml");
    println!("Creating CRD manifest at {:?}", crd_file);
    fs::create_dir_all(crd_file.parent().unwrap())?;

    let ingress_zone_versions = vec![
        ingress_zone_v1::IngressZone::crd(),
    ];
    let merged_inress_zone = merge_crds(ingress_zone_versions, "v1").context("Failed to merge CRDs")?;


    let application_versions = vec![
        application_v1::Application::crd(),
    ];
    let merged_application = merge_crds(application_versions, "v1").context("Failed to merge CRDs")?;

    let contents = [
        "---".to_string(),
        serde_yaml::to_string(&merged_inress_zone).unwrap(),
        "---".to_string(),
        serde_yaml::to_string(&merged_application).unwrap(),
    ];
    let contents = contents.join("\n");
    fs::write(crd_file, contents).context("Failed to write file")
}
