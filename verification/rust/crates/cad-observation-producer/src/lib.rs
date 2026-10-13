#![forbid(unsafe_code)]

mod algebra;
pub mod cli;
mod descriptor;
mod encode;
mod engine;
mod families;
mod generated_request;

use cad_observation_types::{canonical_json, decode_canonical, digest_hex, exact_fields,
    CadObservationFailure, CAD_OBSERVATION_SCHEMA};
use generated_request::ProjectedRequest;
use serde_json::{json, Value};

pub fn describe() -> Result<Vec<u8>, CadObservationFailure> {
    descriptor::describe()
}

fn family_observation(family: &str, input: &Value) -> Result<Value, families::FamilyError> {
    match family {
        "exact-chart" => families::exact_chart(input),
        "face-atom-covering" => families::face_atom_covering(input),
        "finite-surface-representation" => families::finite_surface(input),
        "certified-chord-bound" => families::chord_bound(input),
        "mesh-judgment" => families::mesh_judgment(input),
        _ => Err(families::FamilyError::Unsupported("operation family")),
    }
}

fn family_kind(family: &str) -> Result<&'static str, CadObservationFailure> {
    match family {
        "exact-chart" => Ok("exactChart"),
        "face-atom-covering" => Ok("faceAtomCovering"),
        "finite-surface-representation" => Ok("finiteSurfaceRepresentation"),
        "certified-chord-bound" => Ok("certifiedChordBound"),
        "mesh-judgment" => Ok("meshJudgment"),
        _ => Err(CadObservationFailure::UnsupportedCase),
    }
}

fn observe(request: &ProjectedRequest) -> Result<Value, CadObservationFailure> {
    let observation = match family_observation(&request.family, &request.input) {
        Ok(value) => {
            let rejected_mesh = request.family == "mesh-judgment" && value["outcome"]["kind"] == "rejected";
            if rejected_mesh != (request.requested_kind == "failure") {
                return Err(CadObservationFailure::EngineFailure {
                    case_id: request.case_id.clone(),
                    family: request.family.clone(),
                    condition: format!("requested {} for input {}; actual outcome: {}",
                        request.requested_kind, request.input, value),
                });
            }
            json!({"kind": family_kind(&request.family)?, "value": value})
        }
        Err(families::FamilyError::Engine(engine::EngineError::Construction(error)))
            if request.family == "exact-chart" && request.requested_kind == "failure"
                && request.input["scale"] == 0.0
                && matches!(&*error, neco_cad::CadConstructionFailure::PayloadInvalid {
                    field: "transform.scale", ..
                }) =>
        {
            json!({"kind": "typedFailure", "value": {
                "failureKind": "engineRejection", "reason": format!("{error:?}"),
                "provenance": {"failingOperation": "construct", "caseId": request.case_id,
                    "input": request.input, "violatedCondition": "transform.scale must be positive"},
            }})
        }
        Err(families::FamilyError::Engine(engine::EngineError::Cad(error)))
            if request.family == "exact-chart" && request.requested_kind == "failure"
                && request.input["scale"] == 1.25
                && request.input["primitiveExtent"] == 1.0
                && request.input["surfaceFamily"] == "sphere-south"
                && matches!(&*error, neco_cad::CadFailure::Boundary {
                    condition: neco_cad::BoundaryDerivationCondition::IncompleteCoverage, ..
                }) =>
        {
            json!({"kind": "typedFailure", "value": {
                "failureKind": "boundaryRejection", "reason": format!("{error:?}"),
                "provenance": {"failingOperation": "derive_complete_boundary", "caseId": request.case_id,
                    "input": request.input,
                    "violatedCondition": "no exact boundary seed in integer lattice [-3,3]^3"},
            }})
        }
        Err(error) => return Err(CadObservationFailure::EngineFailure {
            case_id: request.case_id.clone(),
            family: request.family.clone(),
            condition: format!("public API failed for input {}: {error}", request.input),
        }),
    };
    Ok(json!({"caseId": request.case_id, "input": request.input,
        "operationFamily": request.family, "publicObservation": observation}))
}

pub fn generate(request: &[u8]) -> Result<Vec<u8>, CadObservationFailure> {
    let projected = generated_request::project_batch(request)?;
    let cases = projected.iter().map(observe).collect::<Result<Vec<_>, _>>()?;
    canonical_json(&json!({"schema": CAD_OBSERVATION_SCHEMA,
        "proofSubjectRef": descriptor::proof_ref()?, "testSetRef": descriptor::test_set_ref()?,
        "cases": cases}))
}

pub fn self_check(request: &[u8], batch: &[u8]) -> Result<Vec<u8>, CadObservationFailure> {
    let projected = generated_request::project_batch(request)?;
    let supplied = decode_canonical(batch)?;
    exact_fields(&supplied, &["schema", "proofSubjectRef", "testSetRef", "cases"])?;
    if supplied["schema"] != CAD_OBSERVATION_SCHEMA
        || supplied["proofSubjectRef"] != descriptor::proof_ref()?
        || supplied["testSetRef"] != descriptor::test_set_ref()?
    {
        return Err(CadObservationFailure::MalformedRequest);
    }
    let supplied_cases = supplied["cases"].as_array().ok_or(CadObservationFailure::MalformedRequest)?;
    if supplied_cases.len() != projected.len() {
        return Err(CadObservationFailure::MalformedRequest);
    }
    let mut reports = Vec::with_capacity(projected.len());
    for (request, supplied_case) in projected.iter().zip(supplied_cases) {
        exact_fields(supplied_case, &["caseId", "input", "operationFamily", "publicObservation"])?;
        let replayed = observe(request)?;
        let verified = &replayed == supplied_case;
        reports.push(json!({"caseId": request.case_id, "verified": verified,
            "replayedObservationDigest": digest_hex(&canonical_json(&replayed)?),
            "suppliedObservationDigest": digest_hex(&canonical_json(supplied_case)?)}));
    }
    let verified = reports.iter().all(|case| case["verified"] == true);
    canonical_json(&json!({"schema": "neco.verification.public-api-parity-result.v1", "verified": verified,
        "requestDigest": digest_hex(request), "observationDigest": digest_hex(batch), "cases": reports}))
}
