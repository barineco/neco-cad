#![forbid(unsafe_code)]

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REQUEST_SCHEMA: &str = "neco.verification.generated-request-batch.v1";
pub const CAD_DOMAIN: &str = "neco.cad";
pub const ADAPTER_REVISION: &str = "cad-generated-request-v2";
pub const DRAW_SCHEDULE_REVISION: &str = "draw-schedule-v1";
pub const SELECTOR_NAMES: [&str; 9] = [
    "value", "familyParameter", "observationClass", "sign", "multiplicity",
    "intervalWidth", "formulaVariant", "mutation", "reserved",
];
pub const FAMILIES: [&str; 5] = [
    "exact-chart", "face-atom-covering", "finite-surface-representation",
    "certified-chord-bound", "mesh-judgment",
];

pub const CAD_OBSERVATION_SCHEMA: &str = "neco.cad.observation-batch.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CadObservationFailure {
    InvalidProcessInvocation,
    NonUtf8Batch,
    NonCanonicalBatch,
    MalformedRequest,
    UnsupportedCase,
    EngineFailure { case_id: String, family: String, condition: String },
    BatchResourceLimit { count: usize, maximum: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CadCheckerResult {
    BatchVerdict { batch_verdict: String, digest: String },
    Cases { cases: Vec<CadCase>, digest: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CadCase {
    pub case_id: String,
    pub verdict: String,
}

pub fn encode_checker_result(result: &CadCheckerResult) -> Result<Vec<u8>, CadObservationFailure> {
    let value = match result {
        CadCheckerResult::BatchVerdict { batch_verdict, digest } => json!({
            "batchVerdict": batch_verdict,
            "digest": digest
        }),
        CadCheckerResult::Cases { cases, digest } => {
            let encoded: Vec<Value> = cases
                .iter()
                .map(|case| json!({"caseId": case.case_id, "verdict": case.verdict}))
                .collect();
            json!({"cases": encoded, "digest": digest})
        }
    };
    canonical_json(&value)
}

pub fn canonical_json(value: &Value) -> Result<Vec<u8>, CadObservationFailure> {
    Ok(canonical_value(value).to_string().into_bytes())
}

fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut ordered = Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                ordered.insert(key.clone(), canonical_value(&map[key]));
            }
            Value::Object(ordered)
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_value).collect()),
        other => other.clone(),
    }
}

pub fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn decode_canonical(bytes: &[u8]) -> Result<Value, CadObservationFailure> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| CadObservationFailure::MalformedRequest)?;
    if canonical_json(&value)? != bytes {
        return Err(CadObservationFailure::NonCanonicalBatch);
    }
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedRequestCase {
    pub case_id: String,
    pub global_case_index: u64,
    pub domain_case_index: u64,
    pub family: String,
    pub requested_observation_kind: String,
    pub selectors: BTreeMap<String, u64>,
}

pub fn exact_fields(value: &Value, fields: &[&str]) -> Result<(), CadObservationFailure> {
    let object = value.as_object().ok_or(CadObservationFailure::MalformedRequest)?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(CadObservationFailure::MalformedRequest);
    }
    Ok(())
}

pub fn hex_u64(value: &Value) -> Result<u64, CadObservationFailure> {
    let text = value.as_str().ok_or(CadObservationFailure::MalformedRequest)?;
    if text.len() != 16 || !text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err(CadObservationFailure::MalformedRequest);
    }
    u64::from_str_radix(text, 16).map_err(|_| CadObservationFailure::MalformedRequest)
}

pub fn decode_generated_request(
    bytes: &[u8],
    maximum: usize,
) -> Result<Vec<GeneratedRequestCase>, CadObservationFailure> {
    let batch = decode_canonical(bytes)?;
    exact_fields(&batch, &["schema", "domain", "adapterRevision", "generator", "cases"])?;
    if batch["schema"] != REQUEST_SCHEMA || batch["domain"] != CAD_DOMAIN
        || batch["adapterRevision"] != ADAPTER_REVISION
    {
        return Err(CadObservationFailure::MalformedRequest);
    }
    let generator = &batch["generator"];
    exact_fields(generator, &["algorithm", "drawScheduleRevision", "seed"])?;
    if generator["algorithm"] != "splitmix64-v1"
        || generator["drawScheduleRevision"] != DRAW_SCHEDULE_REVISION
    {
        return Err(CadObservationFailure::MalformedRequest);
    }
    hex_u64(&generator["seed"])?;
    let values = batch["cases"].as_array().ok_or(CadObservationFailure::MalformedRequest)?;
    if values.is_empty() {
        return Err(CadObservationFailure::MalformedRequest);
    }
    if values.len() > maximum {
        return Err(CadObservationFailure::BatchResourceLimit { count: values.len(), maximum });
    }
    let mut ids = BTreeSet::new();
    let mut domain_indices = BTreeSet::new();
    values.iter().map(|value| {
        exact_fields(value, &["caseId", "globalCaseIndex", "domainCaseIndex", "family",
            "requestedObservationKind", "selectors"])?;
        let global_case_index = hex_u64(&value["globalCaseIndex"])?;
        let domain_case_index = hex_u64(&value["domainCaseIndex"])?;
        let case_id = value["caseId"].as_str().ok_or(CadObservationFailure::MalformedRequest)?;
        if case_id != format!("case-{global_case_index:016x}")
            || !ids.insert(global_case_index) || !domain_indices.insert(domain_case_index)
        {
            return Err(CadObservationFailure::MalformedRequest);
        }
        let family = value["family"].as_str().ok_or(CadObservationFailure::MalformedRequest)?;
        if !FAMILIES.contains(&family) {
            return Err(CadObservationFailure::UnsupportedCase);
        }
        let kind = value["requestedObservationKind"].as_str()
            .ok_or(CadObservationFailure::MalformedRequest)?;
        if !["success", "failure"].contains(&kind) {
            return Err(CadObservationFailure::MalformedRequest);
        }
        exact_fields(&value["selectors"], &SELECTOR_NAMES)?;
        let selectors = SELECTOR_NAMES.iter().map(|name| {
            Ok(((*name).to_owned(), hex_u64(&value["selectors"][*name])?))
        }).collect::<Result<BTreeMap<_, _>, CadObservationFailure>>()?;
        Ok(GeneratedRequestCase {
            case_id: case_id.to_owned(), global_case_index, domain_case_index,
            family: family.to_owned(), requested_observation_kind: kind.to_owned(), selectors,
        })
    }).collect()
}
