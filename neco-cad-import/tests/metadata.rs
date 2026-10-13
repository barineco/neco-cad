use std::path::Path;
use std::process::Command;

#[test]
fn verifies_workspace_member_package_id_and_dependency_sources() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace");
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .current_dir(workspace)
        .output()
        .expect("cargo metadata");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"name\":\"neco-cad-import\""));
    assert!(text.contains(&format!("\"version\":\"{}\"", env!("CARGO_PKG_VERSION"))));
    assert!(text.contains("neco-brep"));
    assert!(text.contains("neco-mesh"));
    assert!(text.contains("neco-geometry"));
}
