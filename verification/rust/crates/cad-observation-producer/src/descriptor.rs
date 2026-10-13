use cad_observation_types::{canonical_json, decode_canonical, digest_hex, CadObservationFailure,
    ADAPTER_REVISION, CAD_DOMAIN, FAMILIES};
use serde_json::{json, Value};

const SEMANTICS: &[u8] = include_bytes!("../../../../manifest/cad-semantics.json");
const PROJECTION: &[u8] = include_bytes!("../../../../manifest/cad-projection-declaration.json");
const RESOURCE: &[u8] = include_bytes!("../../../../manifest/cad-resource-declaration.json");
const PROOF: &[u8] = include_bytes!("../../../../manifest/cad-proof-subject.json");
const TEST_SET: &[u8] = include_bytes!("../../../../manifest/cad-finite-test-set.json");
const TRANSITIONS: &[u8] = include_bytes!("../../../../manifest/cad-transition-class-declaration.json");
const GENERATED: &[u8] = include_bytes!("../../../../manifest/cad-generated-request-declaration.json");
const RUSTC: &str = include_str!(concat!(env!("OUT_DIR"), "/rustc-identity.txt"));
const TARGET: &str = include_str!(concat!(env!("OUT_DIR"), "/target-triple.txt"));

pub(crate) fn max_batch_cases() -> Result<usize, CadObservationFailure> {
    let resource = decode_canonical(RESOURCE)?;
    resource["maxBatchCases"].as_u64().and_then(|n| usize::try_from(n).ok())
        .filter(|n| *n > 0).ok_or(CadObservationFailure::MalformedRequest)
}

pub(crate) fn reference(bytes: &[u8], field: &str) -> Result<Value, CadObservationFailure> {
    let declaration = decode_canonical(bytes)?;
    let identity = declaration[field].as_str().ok_or(CadObservationFailure::MalformedRequest)?;
    Ok(json!({"contentDigest": digest_hex(bytes), "versionIdentity": identity}))
}

pub(crate) fn proof_ref() -> Result<Value, CadObservationFailure> {
    reference(PROOF, "proofSubjectId")
}

pub(crate) fn test_set_ref() -> Result<Value, CadObservationFailure> {
    reference(TEST_SET, "testSetId")
}

pub(crate) fn describe() -> Result<Vec<u8>, CadObservationFailure> {
    let semantics = decode_canonical(SEMANTICS)?;
    if !semantics_declaration_valid(&semantics) {
        return Err(CadObservationFailure::MalformedRequest);
    }
    let projection = decode_canonical(PROJECTION)?;
    let resource = decode_canonical(RESOURCE)?;
    let proof = decode_canonical(PROOF)?;
    let test_set = decode_canonical(TEST_SET)?;
    if proof["semanticsRef"] != reference(SEMANTICS, "revision")?
        || test_set["proofSubjectRef"] != proof_ref()?
        || test_set["transitionClassRef"] != reference(TRANSITIONS, "transitionClassId")?
        || test_set["generatedRequestRef"] != reference(GENERATED, "adapterRevision")?
    {
        return Err(CadObservationFailure::MalformedRequest);
    }
    let families = projection["families"].as_array().ok_or(CadObservationFailure::MalformedRequest)?;
    if families.len() != FAMILIES.len()
        || families.iter().zip(FAMILIES).any(|(value, family)| value["family"] != family)
        || semantics["domain"] != CAD_DOMAIN || projection["domain"] != CAD_DOMAIN
        || resource["domain"] != CAD_DOMAIN
    {
        return Err(CadObservationFailure::MalformedRequest);
    }
    let projection_digest = digest_hex(&canonical_json(&projection["families"])?);
    canonical_json(&json!({
        "schema": "neco.verification.producer-description.v1",
        "domain": CAD_DOMAIN,
        "semanticsRef": {"declarationDigest": digest_hex(SEMANTICS), "revision": semantics["revision"]},
        "producerApiRevision": "cad-producer-api-v1",
        "adapterRevision": ADAPTER_REVISION,
        "rustcIdentity": RUSTC.trim_end(), "targetTriple": TARGET.trim_end(),
        "projectionClaim": {
            "projectionRevision": projection_digest,
            "projectionDeclarationDigest": projection_digest,
            "projectionDeclarations": projection["families"],
        },
        "resourceClaim": {
            "resourceRevision": resource["resourceRevision"],
            "resourceDeclarationDigest": digest_hex(RESOURCE),
            "maxBatchCases": max_batch_cases()?,
        },
    }))
}

fn semantics_declaration_valid(value: &Value) -> bool {
    fn fields(value: &Value, names: &[&str]) -> bool {
        value.as_object().is_some_and(|v| v.len() == names.len() && names.iter().all(|n| v.contains_key(*n)))
    }
    fn nonempty(value: &Value) -> bool {
        value.as_str().is_some_and(|s| !s.is_empty())
    }
    fn module_path(name: &str, path: &str) -> bool {
        path.strip_prefix("verification/lean/").and_then(|p| p.strip_suffix(".lean"))
            .is_some_and(|p| name.chars().map(|c| if c == '.' {'/'} else {c}).eq(p.chars()))
    }
    let owned = &value["ownedModules"];
    fields(value, &["schema", "domain", "revision", "cadCheckerApiRevision", "cadModuleRevision", "ownedModules", "upstreamSemantics"]) &&
        value["schema"] == "neco.verification.semantics-declaration.v2" && value["domain"] == CAD_DOMAIN &&
        ["revision", "cadCheckerApiRevision", "cadModuleRevision"].iter().all(|f| nonempty(&value[*f])) &&
        owned.as_array().is_some_and(|a| !a.is_empty() && a.iter().enumerate().all(|(i, m)|
            fields(m, &["module", "sourcePath"]) &&
            m["module"].as_str().zip(m["sourcePath"].as_str()).is_some_and(|(name, path)|
                name.starts_with("NecoCadVerify.") && module_path(name, path)) &&
            !a[..i].iter().any(|previous| previous["module"] == m["module"]))) &&
        value["upstreamSemantics"].as_array().is_some_and(|a| a.len() == 3 && a.iter().zip([
            ("neco.algebra", "verification/manifest/algebra-semantics.json"),
            ("neco.calculus", "verification/manifest/calculus-semantics.json"),
            ("neco.geometry", "verification/manifest/geometry-semantics.json")]).all(|(r, (d, p))|
            fields(r, &["domain", "revision", "sourcePath"]) && r["domain"] == d &&
            nonempty(&r["revision"]) && r["sourcePath"] == p))
}
