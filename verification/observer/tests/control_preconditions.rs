use neco_cad_observer::controls::mutations;
use serde_json::{json, Value};
fn term(coefficient: &str, u: u64) -> Value {
    json!({"coefficient":{"numerator":coefficient,"denominator":"1"},"exponentU":u,"exponentV":0})
}
fn point(x: &str, y: &str) -> Value {
    json!({"x":{"numerator":x,"denominator":"1"},"y":{"numerator":y,"denominator":"1"},"z":{"numerator":"0","denominator":"1"}})
}
fn curve(power: u64) -> Value {
    json!({"schema":"neco.cad.observation-batch.v1","proofSubjectRef":{"contentDigest":"a".repeat(64),"versionIdentity":"proof"},"testSetRef":{"contentDigest":"b".repeat(64),"versionIdentity":"test"},"cases":[{"caseId":"curve","input":{},"operationFamily":"certified-chord-bound","publicObservation":{"kind":"certifiedChordBound","value":{"patches":[{"chart":{"denominator":[term("1",0)],"x":[term("1",1)],"y":[term("1",power)],"z":[]}}],"edges":[{"patchIndex":0,"parameterization":{"kind":"line","tLower":{"numerator":"0","denominator":"1"},"tUpper":{"numerator":"1","denominator":"1"}},"establishedSquaredBound":{"integer":"1","exponent":3},"left":{"u":{"numerator":"0","denominator":"1"},"v":{"numerator":"0","denominator":"1"},"point":point("0","0")},"right":{"u":{"numerator":"1","denominator":"1"},"v":{"numerator":"0","denominator":"1"},"point":point("1","1")}}]}}}]})
}
#[test]
fn chord_control_proves_positive_exact_deviation_before_lowering_bound() {
    let (changed, _, witness) = mutations(&curve(2), "certified-chord-bound").unwrap();
    assert_eq!(
        witness["exactMidpointSquaredDeviation"],
        json!({"numerator":"1","denominator":"16"})
    );
    assert_eq!(
        changed["cases"][0]["publicObservation"]["value"]["edges"][0]["establishedSquaredBound"],
        json!({"integer":"0","exponent":0})
    );
    assert_eq!(
        mutations(&curve(1), "certified-chord-bound")
            .unwrap_err()
            .kind(),
        "coverageMissing"
    );
}
#[test]
fn finite_control_uses_actual_atom_residual_and_bound() {
    let atom = vec![
        json!({"coefficient":{"numerator":"-1","denominator":"1"},"exponentX":0,"exponentY":0,"exponentZ":0}),
        json!({"coefficient":{"numerator":"1","denominator":"1"},"exponentX":2,"exponentY":0,"exponentZ":0}),
    ];
    let original = json!({"cases":[{"caseId":"surface","input":{},"operationFamily":"finite-surface-representation","publicObservation":{"kind":"finiteSurfaceRepresentation","value":{"faceTriangles":[{"triangleIndices":[0],"atom":atom,"residualBound":{"integer":"1","exponent":3}}],"mesh":{"vertices":[{"x":{"integer":"1","exponent":0},"y":{"integer":"0","exponent":0},"z":{"integer":"0","exponent":0}}],"triangles":[[0,0,0]]}}}}]});
    let (_, _, witness) = mutations(&original, "finite-surface-representation").unwrap();
    assert_eq!(
        witness["exactAtomResidual"],
        json!({"numerator":"3","denominator":"1"})
    );
    assert_eq!(
        witness["declaredResidualBound"],
        json!({"numerator":"1","denominator":"8"})
    );
}
