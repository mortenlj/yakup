use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use assert_json_diff::assert_json_include;
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use test_generator::test_resources;

use api::v1::{Application, ApplicationSpec};
use controller::models::Operation;
use controller::resource_creator::process;

#[derive(Debug, Serialize, Deserialize)]
struct TestCase {
    name: String,
    app_spec: ApplicationSpec,
    operations: Vec<Operation>,
}

#[test_resources("controller/tests/testdata/*.yaml")]
fn test_process(resource: PathBuf) {
    let cwd = env::current_dir().unwrap();
    let full_path = cwd.parent().expect("Could not find parent").join(resource);
    let f = File::open(full_path).expect("Could not open file.");
    let case: TestCase = serde_yaml::from_reader(f).expect("Could not read test case.");

    let app = Application::new("test-app", case.app_spec);
    let operations = process(Arc::new(app)).unwrap();

    assert_eq!(operations.len(), case.operations.len());
    for (operation, expected_operation) in operations.iter().zip(case.operations.iter()) {
        let actual = serde_json::to_value(operation).expect("Could not serialize operation.");
        let expected = serde_json::to_value(expected_operation)
            .expect("Could not serialize expected operation.");
        assert_json_include!(actual: actual, expected: expected);
    }
}
