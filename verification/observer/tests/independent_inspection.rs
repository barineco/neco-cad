use neco_cad_observer::{
    protocol::*,
    reconciliation::{reconcile, reconcile_bytes},
};
use serde_json::{json, Value};
fn batch() -> Value {
    json!({"schema":"neco.cad.observation-batch.v1","proofSubjectRef":{"contentDigest":"a".repeat(64),"versionIdentity":"proof"},"testSetRef":{"contentDigest":"b".repeat(64),"versionIdentity":"tests"},"cases":[{"caseId":"one","input":{},"operationFamily":"exact-chart","publicObservation":{"kind":"exactChart","value":{}}},{"caseId":"two","input":{},"operationFamily":"mesh-judgment","publicObservation":{"kind":"meshJudgment","value":{}}}]})
}
fn result(batch: &[u8]) -> Value {
    json!({"digest":digest(batch),"cases":[{"caseId":"one","verdict":"verified"},{"caseId":"two","verdict":"refuted"}]})
}
#[test]
fn result_correspondence_requires_digest_count_order_and_known_verdicts() {
    let b = canonical(&batch());
    let valid = result(&b);
    reconcile(&b, &valid).unwrap();
    let mut wrong = valid.clone();
    wrong["digest"] = json!("0".repeat(64));
    assert_eq!(
        reconcile(&b, &wrong).unwrap_err().kind(),
        "resultDigestMismatch"
    );
    let mut reordered = valid.clone();
    reordered["cases"].as_array_mut().unwrap().reverse();
    assert_eq!(
        reconcile(&b, &reordered).unwrap_err().kind(),
        "caseCorrespondenceMismatch"
    );
    let mut missing = valid.clone();
    missing["cases"].as_array_mut().unwrap().pop();
    assert_eq!(
        reconcile(&b, &missing).unwrap_err().kind(),
        "caseCorrespondenceMismatch"
    );
    let mut unknown = valid.clone();
    unknown["cases"][0]["verdict"] = json!("compiled");
    assert_eq!(reconcile(&b, &unknown).unwrap_err().kind(), "resultInvalid");
    let mut extra = valid.clone();
    extra["extra"] = json!(true);
    assert_eq!(reconcile(&b, &extra).unwrap_err().kind(), "resultInvalid");
}
#[test]
fn parser_preserves_duplicate_noncanonical_and_invalid_case_distinctions() {
    assert_eq!(
        decode(br#"{"a":1,"a":2}"#, "parse").unwrap_err().kind(),
        "duplicateKey"
    );
    assert_eq!(
        decode(br#"{"a":1} "#, "parse").unwrap_err().kind(),
        "noncanonicalBytes"
    );
    assert_eq!(decode(&[0xff], "parse").unwrap_err().kind(), "nonUtf8");
    let mut b = batch();
    b["cases"][0]["operationFamily"] = json!("unknown");
    let bytes = canonical(&b);
    let mut r = result(&bytes);
    assert_eq!(
        reconcile(&bytes, &r).unwrap_err().kind(),
        "resultContradiction"
    );
    r["cases"][0]["verdict"] = json!("invalid");
    assert_eq!(
        reconcile_bytes(&bytes, &canonical(&r)).unwrap()["classification"],
        "caseInvalid"
    );
}
#[test]
fn every_case_is_required_to_have_a_unique_nonempty_identity() {
    for value in [json!(""), json!("one"), json!("__digest__"), Value::Null] {
        let mut b = batch();
        b["cases"][1]["caseId"] = value;
        let bytes = canonical(&b);
        let r = result(&bytes);
        assert_eq!(reconcile(&bytes, &r).unwrap_err().kind(), "batchInvalid");
    }
    let mut b = batch();
    b["cases"][1].as_object_mut().unwrap().remove("caseId");
    let bytes = canonical(&b);
    assert!(reconcile(&bytes, &result(&bytes)).is_err());
    let empty = br#"{"cases":[]}"#;
    assert!(reconcile(empty, &json!({"digest":digest(empty),"cases":[]})).is_err());
}
#[test]
fn batch_invalid_requires_actual_non_enumerability_and_exact_digest() {
    let b = canonical(&batch());
    assert_eq!(
        reconcile(&b, &json!({"batchVerdict":"invalid","digest":digest(&b)}))
            .unwrap_err()
            .kind(),
        "resultContradiction"
    );
    let malformed = br#"{"cases":[]}"#;
    reconcile(
        malformed,
        &json!({"batchVerdict":"invalid","digest":digest(malformed)}),
    )
    .unwrap();
    assert_eq!(
        reconcile(
            malformed,
            &json!({"batchVerdict":"verified","digest":digest(malformed)})
        )
        .unwrap_err()
        .kind(),
        "resultInvalid"
    );
}
#[test]
fn result_decode_precedes_batch_or_digest_reconciliation() {
    let b = b"not-json";
    let duplicate = br#"{"digest":"x","digest":"y"}"#;
    assert_eq!(
        reconcile_bytes(b, duplicate).unwrap_err().kind(),
        "duplicateKey"
    );
    assert_eq!(
        reconcile(b, &json!({"cases":[],"digest":"0".repeat(64)}))
            .unwrap_err()
            .kind(),
        "resultInvalid"
    );
}
#[test]
fn typed_failure_never_becomes_normal_verified_observation() {
    let mut b = batch();
    b["cases"].as_array_mut().unwrap().truncate(1);
    b["cases"][0]["input"] = json!({"scale":0.0});
    b["cases"][0]["publicObservation"] = json!({"kind":"typedFailure","value":{"failureKind":"engineRejection","reason":"untrusted rejected transform claim","provenance":{"caseId":"one","input":{"scale":0.0},"failingOperation":"construct","violatedCondition":"transform.scale must be positive"}}});
    let bytes = canonical(&b);
    let mut r =
        json!({"digest":digest(&bytes),"cases":[{"caseId":"one","verdict":"undetermined"}]});
    reconcile(&bytes, &r).unwrap();
    for verdict in ["verified", "refuted", "invalid"] {
        r["cases"][0]["verdict"] = json!(verdict);
        assert_eq!(
            reconcile(&bytes, &r).unwrap_err().kind(),
            "resultContradiction"
        );
    }
    b["cases"][0]["publicObservation"]["value"]["failureKind"] = json!("boundaryRejection");
    b["cases"][0]["publicObservation"]["value"]["provenance"]["failingOperation"] =
        json!("derive_complete_boundary");
    b["cases"][0]["publicObservation"]["value"]["provenance"]["violatedCondition"] =
        json!("no exact boundary seed in integer lattice [-3,3]^3");
    b["cases"][0]["input"] = json!({"scale":1.25,"primitiveExtent":1.0,"surfaceFamily":"sphere-south","translation":[-0.5,0.0,0.0]});
    b["cases"][0]["publicObservation"]["value"]["provenance"]["input"] =
        b["cases"][0]["input"].clone();
    let boundary = canonical(&b);
    r["digest"] = json!(digest(&boundary));
    r["cases"][0]["verdict"] = json!("undetermined");
    reconcile(&boundary, &r).unwrap();
    for verdict in ["verified", "refuted", "invalid"] {
        r["cases"][0]["verdict"] = json!(verdict);
        assert_eq!(
            reconcile(&boundary, &r).unwrap_err().kind(),
            "resultContradiction"
        );
    }
    r["cases"][0]["verdict"] = json!("undetermined");
    b["cases"][0]["publicObservation"]["value"]["provenance"]["failingOperation"] =
        json!("construct");
    let mixed = canonical(&b);
    r["digest"] = json!(digest(&mixed));
    assert_eq!(
        reconcile(&mixed, &r).unwrap_err().kind(),
        "resultContradiction"
    );
    b["cases"][0]["publicObservation"]["value"]["provenance"]["failingOperation"] =
        json!("derive_complete_boundary");
    b["cases"][0]["publicObservation"]["value"]["provenance"]["caseId"] = json!("wrong");
    let bytes = canonical(&b);
    r["digest"] = json!(digest(&bytes));
    r["cases"][0]["verdict"] = json!("undetermined");
    assert_eq!(
        reconcile(&bytes, &r).unwrap_err().kind(),
        "resultContradiction"
    );
}

fn declared_failure_batch(boundary: bool) -> Value {
    let mut b = batch();
    b["cases"].as_array_mut().unwrap().truncate(1);
    let (input, kind, operation, condition) = if boundary {
        (
            json!({"scale":1.25,"primitiveExtent":1.0,"surfaceFamily":"sphere-south",
            "translation":[-0.5,0.0,0.0]}),
            "boundaryRejection",
            "derive_complete_boundary",
            "no exact boundary seed in integer lattice [-3,3]^3",
        )
    } else {
        (
            json!({"scale":0.0}),
            "engineRejection",
            "construct",
            "transform.scale must be positive",
        )
    };
    b["cases"][0]["input"] = input.clone();
    b["cases"][0]["publicObservation"] = json!({
        "kind":"typedFailure",
        "value":{
            "failureKind":kind,
            "reason":"untrusted failure claim with its original reason",
            "provenance":{
                "caseId":"one", "input":input,
                "failingOperation":operation, "violatedCondition":condition,
            },
        },
    });
    b
}

fn assert_failure_condition_rejected(b: &Value) {
    let bytes = canonical(b);
    let result =
        json!({"digest":digest(&bytes),"cases":[{"caseId":"one","verdict":"undetermined"}]});
    let failure = reconcile(&bytes, &result).unwrap_err();
    assert_eq!(failure.kind(), "resultContradiction");
    assert_eq!(failure.0["witness"]["kind"], "caseInvalid");
    assert_eq!(failure.0["witness"]["witness"], b["cases"][0]);
}

#[test]
fn a_construct_failure_claim_requires_an_actual_zero_scale_not_just_equal_inputs() {
    for scale in [
        json!(1),
        json!(0.5),
        json!(-1),
        json!("0"),
        Value::Null,
        json!(true),
    ] {
        let mut b = declared_failure_batch(false);
        b["cases"][0]["input"]["scale"] = scale;
        b["cases"][0]["publicObservation"]["value"]["provenance"]["input"] =
            b["cases"][0]["input"].clone();
        assert_failure_condition_rejected(&b);
    }
    let mut missing = declared_failure_batch(false);
    missing["cases"][0]["input"]
        .as_object_mut()
        .unwrap()
        .remove("scale");
    missing["cases"][0]["publicObservation"]["value"]["provenance"]["input"] =
        missing["cases"][0]["input"].clone();
    assert_failure_condition_rejected(&missing);
}

#[test]
fn boundary_failure_claim_requires_the_complete_declared_counterexample() {
    let mutations = [
        ("scale", json!(1)),
        ("scale", json!(2)),
        ("scale", json!("1.25")),
        ("primitiveExtent", json!(0.5)),
        ("primitiveExtent", json!(2)),
        ("primitiveExtent", json!("1")),
        ("surfaceFamily", json!("sphere-north")),
        ("surfaceFamily", json!("plane")),
        ("translation", json!([0.5, 0, 0])),
        ("translation", json!([-0.5, 1, 0])),
        ("translation", json!([-0.5, 0, 1])),
        ("translation", json!([-0.5, 0])),
        ("translation", json!([-0.5, 0, 0, 0])),
        ("translation", json!([-0.5, "0", 0])),
    ];
    for (field, value) in mutations {
        let mut b = declared_failure_batch(true);
        b["cases"][0]["input"][field] = value;
        b["cases"][0]["publicObservation"]["value"]["provenance"]["input"] =
            b["cases"][0]["input"].clone();
        assert_failure_condition_rejected(&b);
    }
    for field in ["scale", "primitiveExtent", "surfaceFamily", "translation"] {
        let mut b = declared_failure_batch(true);
        b["cases"][0]["input"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        b["cases"][0]["publicObservation"]["value"]["provenance"]["input"] =
            b["cases"][0]["input"].clone();
        assert_failure_condition_rejected(&b);
    }
    for boundary in [false, true] {
        let mut b = declared_failure_batch(boundary);
        b["cases"][0]["operationFamily"] = json!("mesh-judgment");
        assert_failure_condition_rejected(&b);
    }
}

#[test]
fn an_invalid_failure_condition_can_only_be_reconciled_as_case_invalid() {
    let mut b = declared_failure_batch(false);
    b["cases"][0]["input"]["scale"] = json!(1);
    b["cases"][0]["publicObservation"]["value"]["provenance"]["input"] =
        b["cases"][0]["input"].clone();
    let bytes = canonical(&b);
    let result = json!({"digest":digest(&bytes),"cases":[{"caseId":"one","verdict":"invalid"}]});
    let report = reconcile_bytes(&bytes, &canonical(&result)).unwrap();
    assert_eq!(report["classification"], "caseInvalid");
}

fn numeric_surface_term(numerator: &str, denominator: &str, u: u64, v: u64) -> Value {
    json!({"coefficient":{"numerator":numerator,"denominator":denominator},
        "exponentU":u,"exponentV":v})
}

fn numeric_chart_batch() -> Value {
    let mut b = batch();
    b["cases"].as_array_mut().unwrap().truncate(1);
    let interval = json!({"lower":{"numerator":"-1","denominator":"1"},
        "upper":{"numerator":"1","denominator":"1"},"lowerClosed":true,"upperClosed":true});
    b["cases"][0]["publicObservation"]["value"] = json!({
        "atom":[{"coefficient":{"numerator":"1","denominator":"1"},
            "exponentX":0,"exponentY":0,"exponentZ":1}],
        "atomId":"plane", "atomRelation":"zero", "faceId":"numeric-face", "patchIndex":0,
        "chart":{
            "denominator":[numeric_surface_term("1","1",0,0)],
            "x":[numeric_surface_term("-3","2",1,0)],
            "y":[numeric_surface_term("1","1",0,1)],
            "z":[numeric_surface_term("0","1",0,0)],
        },
        "chartDenominatorBound":{
            "constant":{"numerator":"1","denominator":"1"},"domainPowers":[],"squares":[],
        },
        "disk":false, "domain":{"u":interval.clone(),"v":interval},
    });
    b
}

fn numeric_case_result(b: &Value) -> Value {
    json!({"digest":digest(&canonical(b)),"cases":[{"caseId":"one","verdict":"verified"}]})
}

fn assert_numeric_rejection(b: &Value) {
    let bytes = canonical(b);
    let failure = reconcile(&bytes, &numeric_case_result(b)).unwrap_err();
    assert_eq!(failure.kind(), "resultContradiction");
    assert_eq!(failure.0["witness"]["kind"], "caseInvalid");
    assert_eq!(failure.0["witness"]["witness"], b["cases"][0]);
}

#[test]
fn a_compound_chart_denominator_is_a_polynomial_and_every_coefficient_is_checked() {
    let b = numeric_chart_batch();
    let report = reconcile_bytes(&canonical(&b), &canonical(&numeric_case_result(&b))).unwrap();
    assert_eq!(report["classification"], "caseVerdicts");
    assert_eq!(report["resultMatched"], true);
    for axis in ["denominator", "x", "y", "z"] {
        let mut changed = b.clone();
        changed["cases"][0]["publicObservation"]["value"]["chart"][axis][0]["coefficient"] =
            json!({"numerator":"1","denominator":"0"});
        assert_numeric_rejection(&changed);
        let mut missing = b.clone();
        missing["cases"][0]["publicObservation"]["value"]["chart"]
            .as_object_mut()
            .unwrap()
            .remove(axis);
        assert_numeric_rejection(&missing);
        let mut wrong_type = b.clone();
        wrong_type["cases"][0]["publicObservation"]["value"]["chart"][axis] =
            json!({"numerator":"1","denominator":"1"});
        assert_numeric_rejection(&wrong_type);
    }
    let mut unknown = b.clone();
    unknown["cases"][0]["publicObservation"]["value"]["chart"]["numerator"] = json!("1");
    assert_numeric_rejection(&unknown);
}

#[test]
fn rational_coefficients_require_exact_normalized_primitive_fields() {
    let invalid = [
        json!({}),
        json!({"numerator":"1"}),
        json!({"denominator":"1"}),
        json!({"numerator":1,"denominator":"1"}),
        json!({"numerator":"1","denominator":1}),
        json!({"numerator":"01","denominator":"2"}),
        json!({"numerator":"+1","denominator":"2"}),
        json!({"numerator":"1.0","denominator":"2"}),
        json!({"numerator":"-0","denominator":"1"}),
        json!({"numerator":"1","denominator":"0"}),
        json!({"numerator":"1","denominator":"-2"}),
        json!({"numerator":"1","denominator":"02"}),
        json!({"numerator":"0","denominator":"2"}),
        json!({"numerator":"2","denominator":"4"}),
        json!({"numerator":"-4","denominator":"6"}),
        json!({"numerator":"1","denominator":"1","unknown":0}),
    ];
    for coefficient in invalid {
        let mut b = numeric_chart_batch();
        b["cases"][0]["publicObservation"]["value"]["chart"]["x"][0]["coefficient"] = coefficient;
        assert_numeric_rejection(&b);
    }
}

#[test]
fn rational_coprimality_does_not_truncate_to_machine_integer_range() {
    let denominator = "10000000000000000000000000000000000000000";
    let mut b = numeric_chart_batch();
    b["cases"][0]["publicObservation"]["value"]["chart"]["x"][0]["coefficient"] =
        json!({"numerator":"10000000000000000000000000000000000000001","denominator":denominator});
    let report = reconcile_bytes(&canonical(&b), &canonical(&numeric_case_result(&b))).unwrap();
    assert_eq!(report["classification"], "caseVerdicts");
    b["cases"][0]["publicObservation"]["value"]["chart"]["x"][0]["coefficient"] =
        json!({"numerator":"20000000000000000000000000000000000000000","denominator":denominator});
    assert_numeric_rejection(&b);
}

#[test]
fn polynomial_field_sets_and_exponent_types_cannot_hide_missing_numeric_atoms() {
    for field in ["coefficient", "exponentU", "exponentV"] {
        let mut b = numeric_chart_batch();
        b["cases"][0]["publicObservation"]["value"]["chart"]["x"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert_numeric_rejection(&b);
    }
    for exponent in [json!(-1), json!(0.5), json!("1"), json!(4294967296_u64)] {
        let mut b = numeric_chart_batch();
        b["cases"][0]["publicObservation"]["value"]["chart"]["x"][0]["exponentU"] = exponent;
        assert_numeric_rejection(&b);
    }
    let mut extra = numeric_chart_batch();
    extra["cases"][0]["publicObservation"]["value"]["chart"]["x"][0]["exponentX"] = json!(0);
    assert_numeric_rejection(&extra);
    let mut atom = numeric_chart_batch();
    atom["cases"][0]["publicObservation"]["value"]["atom"][0]
        .as_object_mut()
        .unwrap()
        .remove("coefficient");
    assert_numeric_rejection(&atom);
}

fn numeric_mesh_batch() -> Value {
    let mut b = batch();
    b["cases"].as_array_mut().unwrap().truncate(1);
    b["cases"][0]["operationFamily"] = json!("mesh-judgment");
    let point = |x: &str, y: &str, z: &str| {
        json!({
            "x":{"integer":x,"exponent":0},"y":{"integer":y,"exponent":0},
            "z":{"integer":z,"exponent":0},
        })
    };
    let pairs: Vec<_> = (0..4)
        .flat_map(|first| {
            (first+1..4).map(move |second| {
        json!({"first":first,"second":second,"sharedIndexCount":2,"kind":"sharedEdgeSegment"})
    })
        })
        .collect();
    b["cases"][0]["publicObservation"] = json!({"kind":"meshJudgment","value":{
        "mesh":{"vertices":[point("0","0","0"),point("1","0","0"),point("0","1","0"),point("0","0","1")],
            "triangles":[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]},
        "outcome":{"kind":"validated","closure":{"edgeCount":6},"orientation":{"edgeCount":6},
            "manifold":{"edgeCount":6},"examinedPairs":pairs},
    }});
    b
}

#[test]
fn dyadic_coordinates_preserve_zero_odd_numerator_and_unsigned_exponent_normalization() {
    let b = numeric_mesh_batch();
    let report = reconcile_bytes(&canonical(&b), &canonical(&numeric_case_result(&b))).unwrap();
    assert_eq!(report["classification"], "caseVerdicts");
    let invalid = [
        json!({}),
        json!({"integer":"1"}),
        json!({"exponent":0}),
        json!({"integer":1,"exponent":0}),
        json!({"integer":"01","exponent":0}),
        json!({"integer":"+1","exponent":0}),
        json!({"integer":"-0","exponent":0}),
        json!({"integer":"0","exponent":1}),
        json!({"integer":"2","exponent":1}),
        json!({"integer":"-2","exponent":1}),
        json!({"integer":"1","exponent":-1}),
        json!({"integer":"1","exponent":0.5}),
        json!({"integer":"1","exponent":"0"}),
        json!({"integer":"1","exponent":4294967296_u64}),
        json!({"integer":"1","exponent":0,"unknown":0}),
    ];
    for coordinate in invalid {
        let mut changed = b.clone();
        changed["cases"][0]["publicObservation"]["value"]["mesh"]["vertices"][0]["x"] = coordinate;
        assert_numeric_rejection(&changed);
    }
    let mut point = b.clone();
    point["cases"][0]["publicObservation"]["value"]["mesh"]["vertices"][0]
        .as_object_mut()
        .unwrap()
        .remove("z");
    assert_numeric_rejection(&point);
}
