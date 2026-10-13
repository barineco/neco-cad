use neco_cad_observer::protocol::*;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
struct InputFile(PathBuf);
impl Drop for InputFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn probe(mesh: Value, label: &str) -> Value {
    let path = std::env::temp_dir().join(format!(
        "neco-cad-mesh-probe-{}-{label}.json",
        std::process::id()
    ));
    let file = InputFile(path);
    let input = json!({"schema":"neco.cad.mesh-api-probe-request.v1","mesh":mesh,"source":{"identity":"test-public-mesh","revision":"test-revision"}});
    fs::write(&file.0, canonical(&input)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_neco-cad-control-probe"))
        .args(["mesh", "--input"])
        .arg(&file.0)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = decode(&output.stdout, "probeResult").unwrap();
    assert_eq!(actual["inputDigest"], digest(&canonical(&input)));
    actual
}
fn tetrahedron() -> Value {
    let point = |x, y, z| json!({"x":{"integer":format!("{x}"),"exponent":0},"y":{"integer":format!("{y}"),"exponent":0},"z":{"integer":format!("{z}"),"exponent":0}});
    json!({"vertices":[point(0,0,0),point(1,0,0),point(0,1,0),point(0,0,1)],"triangles":[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]})
}
#[test]
fn public_api_preserves_cyclic_rotation_and_detects_orientation_open_and_degenerate_meshes() {
    let mesh = tetrahedron();
    let actual = probe(mesh.clone(), "normal");
    assert_eq!(actual["outcome"]["kind"], "validated");
    let mut equal = mesh.clone();
    for triangle in equal["triangles"].as_array_mut().unwrap() {
        triangle.as_array_mut().unwrap().rotate_left(1);
    }
    let replay = probe(equal, "cyclic");
    assert_eq!(replay["outcome"], actual["outcome"]);
    let mut orientation = mesh.clone();
    orientation["triangles"][0]
        .as_array_mut()
        .unwrap()
        .swap(1, 2);
    let observed = probe(orientation, "orientation");
    assert_eq!(observed["outcome"]["kind"], "rejected");
    assert_eq!(observed["outcome"]["failureKind"], "orientationMismatch");
    let mut open = mesh.clone();
    open["triangles"].as_array_mut().unwrap().pop();
    assert_eq!(probe(open, "open")["outcome"]["failureKind"], "openEdge");
    let mut degenerate = mesh;
    degenerate["vertices"][1] = degenerate["vertices"][0].clone();
    assert_eq!(
        probe(degenerate, "degenerate")["outcome"]["failureKind"],
        "degenerateTriangle"
    );
}
