use cad_observation_producer::cli::dispatch;

#[test]
fn unknown_command_is_typed_process_failure() {
    assert!(dispatch(&["unknown".to_owned()]).is_err());
}

#[test]
fn construction_seedless_boundary_and_checkable_mesh_failures_remain_distinct() {
    let request = include_bytes!("../../../../vectors/requests/cad-failure-request-batch.json");
    let batch = cad_observation_producer::generate(request).unwrap();
    let observed: serde_json::Value = serde_json::from_slice(&batch).unwrap();
    let cases = observed["cases"].as_array().unwrap();
    assert_eq!(cases[0]["publicObservation"]["kind"], "typedFailure");
    assert_eq!(cases[0]["publicObservation"]["value"]["failureKind"], "engineRejection");
    assert_eq!(cases[0]["publicObservation"]["value"]["provenance"]["caseId"], cases[0]["caseId"]);
    assert_eq!(cases[0]["publicObservation"]["value"]["provenance"]["input"], cases[0]["input"]);
    assert_eq!(cases[0]["input"]["scale"], 0.0);
    assert_eq!(cases[1]["publicObservation"]["kind"], "typedFailure");
    let boundary = &cases[1]["publicObservation"]["value"];
    assert_eq!(boundary["failureKind"], "boundaryRejection");
    assert_eq!(boundary["provenance"]["failingOperation"], "derive_complete_boundary");
    assert_eq!(boundary["provenance"]["caseId"], cases[1]["caseId"]);
    assert_eq!(boundary["provenance"]["input"], cases[1]["input"]);
    assert_eq!(cases[1]["input"]["scale"], 1.25);
    assert_eq!(cases[1]["input"]["translation"], serde_json::json!([-0.5, 0.0, 0.0]));
    for (case, classification) in cases[2..].iter().zip([
        "openEdge", "orientationMismatch", "degenerateTriangle",
    ]) {
        assert_eq!(case["publicObservation"]["kind"], "meshJudgment");
        assert_eq!(case["publicObservation"]["value"]["outcome"]["kind"], "rejected");
        assert_eq!(case["publicObservation"]["value"]["outcome"]["failureKind"], classification);
    }
}
