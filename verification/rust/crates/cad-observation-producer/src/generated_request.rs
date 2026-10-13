use cad_observation_types::{decode_generated_request, CadObservationFailure, GeneratedRequestCase};
use serde_json::{json, Value};

pub(crate) struct ProjectedRequest {
    pub case_id: String,
    pub family: String,
    pub requested_kind: String,
    pub input: Value,
}

pub(crate) fn project_batch(bytes: &[u8]) -> Result<Vec<ProjectedRequest>, CadObservationFailure> {
    decode_generated_request(bytes, crate::descriptor::max_batch_cases()?)?
        .iter().map(project).collect()
}

fn project(case: &GeneratedRequestCase) -> Result<ProjectedRequest, CadObservationFailure> {
    let s = &case.selectors;
    let revision = format!("cad-{:016x}-{:016x}", case.domain_case_index, s["value"]);
    let failure = case.requested_observation_kind == "failure";
    if failure && !["exact-chart", "mesh-judgment"].contains(&case.family.as_str()) {
        return Err(CadObservationFailure::UnsupportedCase);
    }
    let input = match case.family.as_str() {
        "exact-chart" => {
            let surface = match s["familyParameter"] % 3 {
                0 => "plane", 1 => "sphere-south", _ => "sphere-north",
            };
            let (mut scale, mut primitive_extent) = [
                (1.0, 1.0), (2.0, 0.5), (4.0, 0.25), (8.0, 0.125),
            ][(s["multiplicity"] % 4) as usize];
            let direction = if s["sign"] % 2 == 0 { 1.0 } else { -1.0 };
            let mut translation = match s["value"] % 4 {
                0 => [0.0, 0.0, 0.0],
                1 => [direction, 0.0, 0.0],
                2 => [2.0 * direction, 0.0, 0.0],
                _ => [direction, direction, 0.0],
            };
            let mut surface = surface;
            if failure {
                primitive_extent = 1.0;
                if s["observationClass"] % 2 == 0 {
                    scale = 0.0;
                } else {
                    surface = "sphere-south";
                    scale = 1.25;
                    translation = [-0.5, 0.0, 0.0];
                }
            }
            json!({"kind": "surfaceChart", "surfaceFamily": surface,
                "primitiveExtent": primitive_extent, "scale": scale,
                "translation": translation, "sourceRevision": revision})
        }
        "face-atom-covering" => json!({"kind": "surfaceCovering", "surfaceFamily": "sphere",
            "sourceRevision": revision, "firstPatch": s["formulaVariant"] % 2}),
        "finite-surface-representation" | "certified-chord-bound" => {
            let mut input = json!({
                "kind": "adaptiveTessellation", "surfaceFamily": "sphere", "sourceRevision": revision,
                "maximumChordError": 0.5 + (s["intervalWidth"] % 2) as f64 / 4.0,
                "maximumSurfaceError": 1.0 + (s["multiplicity"] % 2) as f64 / 2.0,
            });
            if case.family == "certified-chord-bound" {
                input["edgeOrder"] = json!(s["formulaVariant"] % 2);
            }
            input
        }
        "mesh-judgment" => {
            let mesh = if failure {
                match s["observationClass"] % 3 {
                    0 => "open-tetrahedron", 1 => "reversed-face-tetrahedron", _ => "degenerate-tetrahedron",
                }
            } else { "tetrahedron" };
            let offset = (s["value"] % 4) as f64 / 2.0;
            let offset = if s["sign"] % 2 == 0 { offset } else { -offset };
            json!({"kind": "meshValidation", "mesh": mesh, "sourceRevision": revision,
                "scale": 1.0 + (s["multiplicity"] % 4) as f64 / 4.0,
                "translation": [offset, 0.0, 0.0]})
        }
        _ => return Err(CadObservationFailure::UnsupportedCase),
    };
    Ok(ProjectedRequest { case_id: case.case_id.clone(), family: case.family.clone(),
        requested_kind: case.requested_observation_kind.clone(), input })
}
