use neco_cad_observer::{protocol::decode, runtime::child};
use std::{path::Path, process::Command};
#[test]
fn failed_child_spawn_is_an_operation_failure_not_a_mathematical_verdict() {
    let missing = std::env::temp_dir().join(format!(
        "neco-cad-missing-executable-{}",
        std::process::id()
    ));
    let error = match child(
        &missing,
        &["generate"],
        Path::new(env!("CARGO_MANIFEST_DIR")),
        10,
        "producerGenerate",
    ) {
        Ok(_) => panic!("missing executable unexpectedly launched"),
        Err(e) => e,
    };
    assert_eq!(error.kind(), "processFailure");
    assert_eq!(error.0["failingOperation"], "producerGenerate");
    assert_eq!(error.0["witness"]["command"], missing.to_str().unwrap());
    assert!(["verified", "refuted", "undetermined", "invalid"]
        .iter()
        .all(|v| error.kind() != *v));
}
#[test]
fn actual_cli_reports_invocation_errors_without_counting_a_process_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_neco-cad-observer"))
        .args(["generate", "--unknown", "value"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let failure = decode(&output.stderr, "cliFailure").unwrap();
    assert_eq!(failure["classification"], "invalidProcessInvocation");
    assert_eq!(failure["processErrorCount"], 0);
    assert_eq!(failure["failure"]["failingOperation"], "parseCli");
}
#[test]
fn host_config_accepts_optional_positive_shard_parallelism_only() {
    use neco_cad_observer::runtime::host_parallelism;
    use serde_json::{json, Value};
    let base = || -> Value {
        json!({"workingDirectory":"/","temporaryDirectory":"/","reportDirectory":"/","domains":{},"controlProbePath":"p"})
    };
    for n in [1u64, 4] {
        let mut host = base();
        host["shardParallelism"] = json!(n);
        assert_eq!(host_parallelism(&host).unwrap(), n as usize);
    }
    assert!(host_parallelism(&base()).unwrap() >= 1);
    for (key, value) in [("shardParallelism", json!(0)), ("unknown", json!(1))] {
        let mut host = base();
        host[key] = value;
        assert_eq!(host_parallelism(&host).unwrap_err().kind(), "hostInvalid");
    }
}
