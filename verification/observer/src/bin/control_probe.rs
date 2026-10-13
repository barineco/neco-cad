use neco_cad_observer::protocol::*;
use neco_mesh::{
    validate_closed_oriented, validate_surface_mesh, AllowedSharedIndexIntersectionKind,
    MeshFailure, MeshPairIntersectionKind, MeshSource, SurfaceMesh,
};
use serde_json::{json, Value};
use std::{io::Write, path::Path, process::ExitCode};
fn pair_kind(kind: MeshPairIntersectionKind) -> &'static str {
    match kind {
        MeshPairIntersectionKind::Empty => "empty",
        MeshPairIntersectionKind::SharedVertex => "sharedVertex",
        MeshPairIntersectionKind::SharedEdgeSegment => "sharedEdgeSegment",
        MeshPairIntersectionKind::NonPermittedCrossing => "nonPermittedCrossing",
        MeshPairIntersectionKind::CoplanarOverlap => "coplanarOverlap",
        MeshPairIntersectionKind::DistinctIndexCoincidentContact => {
            "distinctIndexCoincidentContact"
        }
        MeshPairIntersectionKind::DuplicateFace => "duplicateFace",
        MeshPairIntersectionKind::Indeterminate => "indeterminate",
    }
}
fn failure(e: MeshFailure) -> Result<Value> {
    let (kind, detail) = match e {
        MeshFailure::DegenerateTriangle { triangle_index } => (
            "degenerateTriangle",
            json!({"triangleIndex":triangle_index}),
        ),
        MeshFailure::OpenEdge {
            first_vertex,
            second_vertex,
        } => (
            "openEdge",
            json!({"firstVertex":first_vertex,"secondVertex":second_vertex}),
        ),
        MeshFailure::OrientationMismatch {
            first_vertex,
            second_vertex,
        } => (
            "orientationMismatch",
            json!({"firstVertex":first_vertex,"secondVertex":second_vertex}),
        ),
        MeshFailure::NonManifoldEdge {
            first_vertex,
            second_vertex,
            incidence_count,
        } => (
            "nonManifoldEdge",
            json!({"firstVertex":first_vertex,"secondVertex":second_vertex,"incidenceCount":incidence_count}),
        ),
        MeshFailure::SelfIntersection(w) => (
            "selfIntersection",
            json!({"first":w.first_triangle(),"second":w.second_triangle(),"kind":pair_kind(w.kind())}),
        ),
        other => {
            return Err(Failure::new(
                "probeInputInvalid",
                "meshPublicApi",
                format!("unrepresentable public mesh failure: {other:?}"),
            ))
        }
    };
    Ok(json!({"kind":"rejected","failureKind":kind,"failure":detail}))
}
fn coordinate(v: &Value) -> Result<f64> {
    fields(
        v,
        &["integer", "exponent"],
        "probeInputInvalid",
        "decodeMesh",
    )?;
    let text = string(v, "integer", "decodeMesh")?;
    require(
        decimal(text, false),
        "probeInputInvalid",
        "decodeMesh",
        "canonical decimal dyadic numerator required",
    )?;
    let integer = text
        .parse::<i64>()
        .map_err(|e| Failure::new("probeInputInvalid", "decodeMesh", e))?;
    let exponent = v["exponent"]
        .as_u64()
        .filter(|n| *n <= 1022)
        .ok_or_else(|| {
            Failure::new(
                "probeInputInvalid",
                "decodeMesh",
                "dyadic exponent out of exact binary64 range",
            )
        })?;
    require(
        integer.unsigned_abs() <= (1u64 << 53),
        "probeInputInvalid",
        "decodeMesh",
        "dyadic numerator cannot be represented exactly as binary64",
    )?;
    let result = (integer as f64) * 2f64.powi(-(exponent as i32));
    require(
        result.is_finite() && (integer == 0 || result != 0.0),
        "probeInputInvalid",
        "decodeMesh",
        "coordinate is not finite/non-underflowing",
    )?;
    Ok(result)
}
fn run(path: &Path) -> Result<Value> {
    let input = load(path, "decodeMeshProbeInput")?;
    fields(
        &input,
        &["schema", "mesh", "source"],
        "probeInputInvalid",
        "decodeMeshProbeInput",
    )?;
    require(
        input["schema"] == "neco.cad.mesh-api-probe-request.v1",
        "probeInputInvalid",
        "decodeMeshProbeInput",
        "wrong probe request schema",
    )?;
    fields(
        &input["mesh"],
        &["vertices", "triangles"],
        "probeInputInvalid",
        "decodeMeshProbeInput",
    )?;
    fields(
        &input["source"],
        &["identity", "revision"],
        "probeInputInvalid",
        "decodeMeshProbeInput",
    )?;
    let vertices = array(&input["mesh"], "vertices", "decodeMesh")?
        .iter()
        .map(|v| {
            fields(v, &["x", "y", "z"], "probeInputInvalid", "decodeMesh")?;
            Ok([
                coordinate(&v["x"])?,
                coordinate(&v["y"])?,
                coordinate(&v["z"])?,
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    let triangles = array(&input["mesh"], "triangles", "decodeMesh")?
        .iter()
        .map(|v| {
            let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| {
                Failure::new(
                    "probeInputInvalid",
                    "decodeMesh",
                    "triangle must have three indices",
                )
            })?;
            let mut row = [0usize; 3];
            for i in 0..3 {
                row[i] = a[i]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| {
                        Failure::new(
                            "probeInputInvalid",
                            "decodeMesh",
                            "usize triangle index required",
                        )
                    })?;
            }
            Ok(row)
        })
        .collect::<Result<Vec<_>>>()?;
    let source = MeshSource::new(
        string(&input["source"], "identity", "decodeMesh")?.into(),
        string(&input["source"], "revision", "decodeMesh")?.into(),
    )
    .map_err(|e| Failure::new("probeInputInvalid", "meshPublicApi", format!("{e:?}")))?;
    let raw = SurfaceMesh::new(vertices, triangles, Vec::new())
        .map_err(|e| Failure::new("probeInputInvalid", "meshPublicApi", format!("{e:?}")))?;
    let outcome = match validate_surface_mesh(raw).and_then(|v| validate_closed_oriented(v, source))
    {
        Ok(mesh) => {
            json!({"kind":"validated","closure":{"edgeCount":mesh.closure_witness().edge_count()},"orientation":{"edgeCount":mesh.orientation_witness().edge_count()},"manifold":{"edgeCount":mesh.manifold_witness().edge_count()},"examinedPairs":mesh.examined_triangle_pairs().iter().map(|p|json!({"first":p.first_triangle(),"second":p.second_triangle(),"sharedIndexCount":p.shared_index_count(),"kind":match p.intersection_kind(){AllowedSharedIndexIntersectionKind::Empty=>"empty",AllowedSharedIndexIntersectionKind::SharedVertex=>"sharedVertex",AllowedSharedIndexIntersectionKind::SharedEdgeSegment=>"sharedEdgeSegment"}})).collect::<Vec<_>>()})
        }
        Err(e) => failure(e)?,
    };
    Ok(
        json!({"schema":"neco.cad.mesh-api-probe-result.v1","inputDigest":digest(&canonical(&input)),"mesh":input["mesh"],"outcome":outcome,"api":"neco_mesh::validate_surface_mesh/validate_closed_oriented"}),
    )
}
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command, flag, path] if command == "mesh" && flag == "--input" => run(Path::new(path)),
        _ => Err(Failure::new(
            "invalidProcessInvocation",
            "meshProbe",
            "mesh --input PATH required",
        )),
    };
    match result {
        Ok(v) => match std::io::stdout().write_all(&canonical(&v)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(1),
        },
        Err(e) => {
            let _ = std::io::stderr().write_all(&canonical(&e.0));
            ExitCode::from(2)
        }
    }
}
