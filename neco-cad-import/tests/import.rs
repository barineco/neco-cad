#[path = "import_support/fixtures.rs"]
mod fixtures;

use fixtures::{
    hole_shell_brep, insufficient_shell_brep, seam_shell_brep, tetrahedral_shell_brep,
    triangle_shell_brep, unresolved_seam_brep,
};
use neco_bigint::{BigInt, Dyadic};
use neco_brep::{evaluate_shell_crossing, ShellCrossingRequest};
use neco_cad_import::{
    region_from_mesh, region_from_shell, CertifiedMembershipClassification,
    CertifiedMembershipResult, CertifiedRegion, MeshMembershipPoint, RegionImportFailure,
    RegionImportLowerFailure, RegionImportRequest,
};
use neco_calculus::{
    AbsoluteBits, ConsumerId, ExactDecisionRequest, ExactExpressionRequirement,
    NecoImplementationSource, OwnerId,
};
use neco_expr::ExprId;
use neco_geometry::{
    CheckedRay3, CheckedRayCrossingResult, Enclosure, PositiveRayParameterRange, RayCrossingPolicy,
    RayCrossingUndeterminedReason, RootIsolationPolicy, RootRefinementMethod,
};
use neco_mesh::{validate_closed_oriented, validate_surface_mesh, MeshSource, SurfaceMesh};

fn root_policy() -> RootIsolationPolicy {
    RootIsolationPolicy::new(
        8,
        RootRefinementMethod::Bisection,
        Enclosure::new(
            Dyadic::new(BigInt::try_from(0).unwrap(), 0),
            Dyadic::new(BigInt::try_from(1).unwrap(), 8),
        )
        .unwrap(),
    )
    .unwrap()
}

fn policy(max_targets: usize, max_hits: usize) -> RayCrossingPolicy {
    RayCrossingPolicy::new(root_policy(), max_targets, max_hits).unwrap()
}

fn request() -> RegionImportRequest {
    RegionImportRequest::new(0.01, policy(64, 64)).unwrap()
}

fn lineage() -> (
    OwnerId,
    ExactExpressionRequirement,
    NecoImplementationSource,
) {
    (
        OwnerId::new(1),
        ExactExpressionRequirement::new(
            ConsumerId::new(11),
            &[ExprId::new(500)],
            ExactDecisionRequest::Zero(ExprId::new(500)),
            AbsoluteBits::new(16),
        )
        .unwrap(),
        NecoImplementationSource::current(),
    )
}

fn membership_at(
    region: &CertifiedRegion,
    query: [f64; 3],
) -> Result<CertifiedMembershipResult, RegionImportFailure> {
    let (owner, requirement, source) = lineage();
    region.membership(
        MeshMembershipPoint::new(query).unwrap(),
        owner,
        &requirement,
        source,
    )
}

fn import_shell(
    brep: &neco_brep::AnalyticBrep,
    shell_id: &str,
) -> Result<CertifiedRegion, RegionImportFailure> {
    let (owner, requirement, source) = lineage();
    region_from_shell(brep, shell_id, request(), owner, &requirement, source)
}

fn source(identity: &str) -> MeshSource {
    MeshSource::new(String::from(identity), String::from("revision-1")).unwrap()
}

fn closed(
    identity: &str,
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
) -> neco_mesh::ClosedOrientedSurfaceMesh {
    let mesh = SurfaceMesh::new(vertices, triangles, Vec::new()).unwrap();
    validate_closed_oriented(validate_surface_mesh(mesh).unwrap(), source(identity)).unwrap()
}

fn tetrahedron() -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    (
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
    )
}

fn octahedron() -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    (
        vec![
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ],
        vec![
            [0, 2, 4],
            [2, 1, 4],
            [1, 3, 4],
            [3, 0, 4],
            [0, 3, 5],
            [3, 1, 5],
            [1, 2, 5],
            [2, 0, 5],
        ],
    )
}

fn cube() -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    (
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ],
    )
}

fn concave() -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    (
        vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 1.0],
            [2.0, 0.0, 1.0],
            [2.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 2.0, 1.0],
            [0.0, 2.0, 1.0],
        ],
        vec![
            [0, 2, 1],
            [0, 3, 2],
            [0, 5, 3],
            [5, 4, 3],
            [6, 7, 8],
            [6, 8, 9],
            [6, 9, 11],
            [11, 9, 10],
            [0, 1, 7],
            [0, 7, 6],
            [1, 2, 8],
            [1, 8, 7],
            [2, 3, 9],
            [2, 9, 8],
            [3, 4, 10],
            [3, 10, 9],
            [4, 5, 11],
            [4, 11, 10],
            [5, 0, 6],
            [5, 6, 11],
        ],
    )
}

fn translate(vertices: &[[f64; 3]], delta: [f64; 3]) -> Vec<[f64; 3]> {
    vertices
        .iter()
        .map(|vertex| {
            [
                vertex[0] + delta[0],
                vertex[1] + delta[1],
                vertex[2] + delta[2],
            ]
        })
        .collect()
}

fn scale(vertices: &[[f64; 3]], factor: f64) -> Vec<[f64; 3]> {
    vertices
        .iter()
        .map(|vertex| [vertex[0] * factor, vertex[1] * factor, vertex[2] * factor])
        .collect()
}

fn classify(
    identity: &str,
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
    query: [f64; 3],
) -> CertifiedMembershipClassification {
    let region = region_from_mesh(closed(identity, vertices, triangles), request()).unwrap();
    membership_at(&region, query).unwrap().classification()
}

#[test]
fn imports_all_declared_mesh_shapes_with_provenance_membership_permutation_and_common_transform() {
    for (identity, vertices, triangles, interior) in [
        (
            "tetrahedron",
            tetrahedron().0,
            tetrahedron().1,
            [0.1, 0.1, 0.1],
        ),
        (
            "octahedron",
            octahedron().0,
            octahedron().1,
            [0.1, 0.1, 0.1],
        ),
        ("cube", cube().0, cube().1, [0.5, 0.2, 0.4]),
        ("concave", concave().0, concave().1, [0.4, 0.2, 0.4]),
    ] {
        let region = region_from_mesh(
            closed(identity, vertices.clone(), triangles.clone()),
            request(),
        )
        .unwrap();
        assert_eq!(region.input_identity(), identity);
        assert_eq!(region.source_revision(), "revision-1");
        assert_eq!(region.requested_maximum_error(), 0.01);
        assert_eq!(
            membership_at(&region, interior).unwrap().classification(),
            CertifiedMembershipClassification::Inside,
            "{identity}"
        );
        let mut rotated = triangles.clone();
        rotated.rotate_left(1);
        let permuted = region_from_mesh(closed(identity, vertices, rotated), request()).unwrap();
        assert_eq!(
            membership_at(&permuted, interior).unwrap().classification(),
            CertifiedMembershipClassification::Inside
        );
    }
}

#[test]
fn preserves_witness_membership_and_crossing_count_under_triangle_permutation() {
    for (identity, vertices, triangles, interior) in [
        (
            "tetrahedron",
            tetrahedron().0,
            tetrahedron().1,
            [0.1, 0.1, 0.1],
        ),
        (
            "octahedron",
            octahedron().0,
            octahedron().1,
            [0.1, 0.1, 0.1],
        ),
        ("cube", cube().0, cube().1, [0.5, 0.2, 0.4]),
    ] {
        let original = region_from_mesh(
            closed(identity, vertices.clone(), triangles.clone()),
            request(),
        )
        .unwrap();
        let mut rotated = triangles;
        rotated.rotate_left(1);
        let permuted = region_from_mesh(closed(identity, vertices, rotated), request()).unwrap();
        let first = membership_at(&original, interior).unwrap();
        let second = membership_at(&permuted, interior).unwrap();
        assert_eq!(first.classification(), second.classification());
        assert_eq!(first.witness().hit_count(), second.witness().hit_count());
        assert_eq!(
            original.closure_witness().edge_count(),
            permuted.closure_witness().edge_count()
        );
    }
}

#[test]
fn classifies_inside_boundary_and_outside_for_declared_meshes() {
    let cases = [
        (
            "tetrahedron",
            tetrahedron(),
            [0.1, 0.1, 0.1],
            [0.2, 0.2, 0.6],
            [-0.2, -0.2, -0.2],
        ),
        (
            "octahedron",
            octahedron(),
            [0.1, 0.1, 0.1],
            [0.2, 0.2, 0.6],
            [2.0, 0.0, 0.0],
        ),
        (
            "cube",
            cube(),
            [0.5, 0.2, 0.4],
            [0.0, 0.2, 0.4],
            [-0.5, 0.2, 0.4],
        ),
        (
            "concave",
            concave(),
            [0.4, 0.2, 0.4],
            [0.0, 0.2, 0.4],
            [-0.5, 0.2, 0.4],
        ),
    ];
    for (identity, (vertices, triangles), interior, boundary, outside) in cases {
        assert_eq!(
            classify(identity, vertices.clone(), triangles.clone(), interior),
            CertifiedMembershipClassification::Inside,
            "{identity} interior"
        );
        assert_eq!(
            classify(identity, vertices.clone(), triangles.clone(), boundary),
            CertifiedMembershipClassification::Boundary,
            "{identity} boundary"
        );
        assert_eq!(
            classify(identity, vertices, triangles, outside),
            CertifiedMembershipClassification::Outside,
            "{identity} outside"
        );
    }
}

#[test]
fn returns_typed_undetermined_witness_for_ray_degeneracies() {
    let (vertices, triangles) = tetrahedron();
    let region = region_from_mesh(closed("tetrahedron", vertices, triangles), request()).unwrap();
    let edge = membership_at(&region, [0.0, 0.4, 0.6]).unwrap();
    assert_eq!(
        edge.classification(),
        CertifiedMembershipClassification::Undetermined
    );
    let reason = edge
        .witness()
        .undetermined()
        .map(|witness| witness.reason());
    assert!(
        matches!(
            reason,
            Some(RayCrossingUndeterminedReason::Edge | RayCrossingUndeterminedReason::Vertex)
        ),
        "{reason:?}"
    );
}

#[test]
fn rejects_invalid_and_accepts_finite_positive_precision() {
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(RegionImportRequest::new(value, policy(8, 8)).is_err());
    }
    assert!(RegionImportRequest::new(0.25, policy(8, 8)).is_ok());
}

#[test]
fn rejects_invalid_crossing_policy() {
    assert!(RayCrossingPolicy::new(root_policy(), 0, 8).is_err());
    assert!(RayCrossingPolicy::new(root_policy(), 8, 0).is_err());
    assert!(
        RootIsolationPolicy::new(0, RootRefinementMethod::Bisection, standard_width()).is_err()
    );
}

fn standard_width() -> Enclosure {
    Enclosure::new(
        Dyadic::new(BigInt::try_from(0).unwrap(), 0),
        Dyadic::new(BigInt::try_from(1).unwrap(), 8),
    )
    .unwrap()
}

#[test]
fn propagates_target_limit_failure_with_partial_evidence() {
    let (vertices, triangles) = tetrahedron();
    let region = region_from_mesh(
        closed("tetrahedron", vertices, triangles),
        RegionImportRequest::new(0.01, policy(1, 64)).unwrap(),
    )
    .unwrap();
    let failure = membership_at(&region, [0.1, 0.1, 0.1]).unwrap_err();
    assert_evidence(&failure);
    assert!(matches!(
        failure.lower(),
        Some(RegionImportLowerFailure::Geometry(_))
    ));
}

#[test]
fn propagates_hit_limit_failure_with_partial_evidence() {
    let (vertices, triangles) = tetrahedron();
    let region = region_from_mesh(
        closed("tetrahedron", vertices, triangles),
        RegionImportRequest::new(0.01, policy(64, 1)).unwrap(),
    )
    .unwrap();
    let failure = membership_at(&region, [-0.2, 0.1, 0.1]).unwrap_err();
    assert_evidence(&failure);
    assert!(matches!(
        failure.lower(),
        Some(RegionImportLowerFailure::Geometry(_))
    ));
}

#[test]
fn propagates_lower_crossing_failure_with_partial_evidence() {
    let region = region_from_mesh(
        closed("tetrahedron", tetrahedron().0, tetrahedron().1),
        RegionImportRequest::new(0.01, policy(1, 1)).unwrap(),
    )
    .unwrap();
    let failure = membership_at(&region, [0.1, 0.1, 0.1]).unwrap_err();
    assert_evidence(&failure);
    assert!(matches!(
        failure.lower(),
        Some(RegionImportLowerFailure::Geometry(_))
    ));
}

fn assert_evidence(failure: &RegionImportFailure) {
    assert!(!failure.operation().is_empty());
    assert!(!failure.input_provenance().is_empty());
    assert!(!failure.condition().is_empty());
    assert!(!failure.prerequisite_list().is_empty());
}

#[test]
fn membership_is_preserved_by_finite_common_translation_or_positive_uniform_scale() {
    let (vertices, triangles) = tetrahedron();
    assert_eq!(
        classify(
            "tetrahedron",
            translate(&vertices, [3.0, 0.0, 0.0]),
            triangles.clone(),
            [3.1, 0.1, 0.1],
        ),
        CertifiedMembershipClassification::Inside
    );
    assert_eq!(
        classify(
            "tetrahedron",
            scale(&vertices, 2.0),
            triangles,
            [0.2, 0.2, 0.2],
        ),
        CertifiedMembershipClassification::Inside
    );
}

#[test]
fn membership_changes_when_only_mesh_is_transformed() {
    let (vertices, triangles) = tetrahedron();
    assert_eq!(
        classify(
            "tetrahedron",
            vertices.clone(),
            triangles.clone(),
            [0.1, 0.1, 0.1]
        ),
        CertifiedMembershipClassification::Inside
    );
    assert_eq!(
        classify(
            "tetrahedron",
            translate(&vertices, [3.0, 0.0, 0.0]),
            triangles,
            [0.1, 0.1, 0.1]
        ),
        CertifiedMembershipClassification::Outside
    );
}

fn shell_region(brep: &neco_brep::AnalyticBrep, shell_id: &str) -> CertifiedRegion {
    import_shell(brep, shell_id).unwrap()
}

fn shell_class(
    brep: &neco_brep::AnalyticBrep,
    shell_id: &str,
    query: [f64; 3],
) -> CertifiedMembershipClassification {
    membership_at(&shell_region(brep, shell_id), query)
        .unwrap()
        .classification()
}

#[test]
fn tetrahedral_analytic_shell_success() {
    let (brep, shell_id) = tetrahedral_shell_brep();
    let region = shell_region(&brep, &shell_id);
    assert_eq!(region.input_identity(), shell_id);
    assert_eq!(
        membership_at(&region, [1.0, 1.0, 1.0])
            .unwrap()
            .classification(),
        CertifiedMembershipClassification::Inside
    );
    assert!(region.closure_witness().edge_count() > 0);
    assert!(region.orientation_witness().edge_count() > 0);
    assert!(region.manifold_witness().edge_count() > 0);
}

#[test]
fn hole_trim_interior_outside_query_odd_inside() {
    let (brep, shell_id) = hole_shell_brep();
    assert_eq!(
        shell_class(&brep, &shell_id, [0.0, 3.0, 3.0]),
        CertifiedMembershipClassification::Inside
    );
}

#[test]
fn hole_shell_exterior_query_even_outside() {
    let (brep, shell_id) = hole_shell_brep();
    assert_eq!(
        shell_class(&brep, &shell_id, [0.0, 5.0, 5.0]),
        CertifiedMembershipClassification::Outside
    );
}

#[test]
fn hole_interior_query_trimmed_face_noncontributing() {
    let (brep, shell_id) = hole_shell_brep();
    assert_eq!(
        shell_class(&brep, &shell_id, [0.0, 1.5, 1.5]),
        CertifiedMembershipClassification::Outside
    );
}

#[test]
fn seam_trim_interior_query_odd_inside() {
    let (brep, shell_id) = seam_shell_brep();
    assert_eq!(
        shell_class(&brep, &shell_id, [0.0, 1.0, 2.0]),
        CertifiedMembershipClassification::Inside
    );
}

#[test]
fn seam_shell_exterior_query_even_outside() {
    let (brep, shell_id) = seam_shell_brep();
    assert_eq!(
        shell_class(&brep, &shell_id, [0.0, 5.0, 5.0]),
        CertifiedMembershipClassification::Outside
    );
}

#[test]
fn seam_unresolved_query_undetermined_unresolved_seam() {
    let (brep, shell_id) = unresolved_seam_brep();
    let result = membership_at(&shell_region(&brep, &shell_id), [0.0, 1.0, 2.0]).unwrap();
    assert_eq!(
        result.classification(),
        CertifiedMembershipClassification::Undetermined
    );
    assert_eq!(
        result
            .witness()
            .undetermined()
            .map(|witness| witness.reason()),
        Some(RayCrossingUndeterminedReason::UnresolvedSeam)
    );
}

#[test]
fn seam_trim_boundary_query_undetermined_trim_boundary() {
    let (brep, shell_id) = hole_shell_brep();
    let result = membership_at(&shell_region(&brep, &shell_id), [0.0, 4.0, 2.0]).unwrap();
    assert_eq!(
        result.classification(),
        CertifiedMembershipClassification::Undetermined
    );
    assert_eq!(
        result
            .witness()
            .undetermined()
            .map(|witness| witness.reason()),
        Some(RayCrossingUndeterminedReason::TrimBoundary)
    );
}

#[test]
fn face_interior_origin_contact_boundary() {
    let (brep, shell_id) = hole_shell_brep();
    assert_eq!(
        shell_class(&brep, &shell_id, [1.0, 3.0, 3.0]),
        CertifiedMembershipClassification::Boundary
    );
}

#[test]
fn reports_shell_membership_unavailable_for_known_insufficient_evidence_shell() {
    let (brep, shell_id) = insufficient_shell_brep();
    let failure = import_shell(&brep, &shell_id).unwrap_err();
    assert_eq!(failure.condition(), "ShellMembershipUnavailable");
    assert!(!failure.prerequisite_list().is_empty());
}

#[test]
fn reports_unknown_shell_identity() {
    let (brep, _) = triangle_shell_brep();
    let failure = import_shell(&brep, "missing-shell").unwrap_err();
    assert_eq!(failure.condition(), "UnknownShellIdentity");
}

#[test]
fn propagates_region_import_failure_evidence() {
    let (brep, _) = triangle_shell_brep();
    let unknown = import_shell(&brep, "missing-shell").unwrap_err();
    assert_evidence(&unknown);
    assert!(matches!(
        unknown.lower(),
        Some(RegionImportLowerFailure::Brep(_))
    ));
    let (insufficient, shell_id) = insufficient_shell_brep();
    let unavailable = import_shell(&insufficient, &shell_id).unwrap_err();
    assert_evidence(&unavailable);
    assert!(matches!(
        unavailable.lower(),
        Some(RegionImportLowerFailure::Brep(_))
    ));
    let (vertices, triangles) = tetrahedron();
    let region = region_from_mesh(
        closed("tetrahedron", vertices, triangles),
        RegionImportRequest::new(0.01, policy(1, 1)).unwrap(),
    )
    .unwrap();
    let geometry = membership_at(&region, [0.1, 0.1, 0.1]).unwrap_err();
    assert_evidence(&geometry);
    assert!(matches!(
        geometry.lower(),
        Some(RegionImportLowerFailure::Geometry(_))
    ));
}

#[test]
fn shell_crossing_hits_keep_the_caller_lineage() {
    let (brep, shell_id) = tetrahedral_shell_brep();
    let (owner, requirement, source) = lineage();
    let ray = CheckedRay3::try_new([1.0, 1.0, 1.0], [1.0, 0.0, 0.0], 0.0).unwrap();
    let result = evaluate_shell_crossing(
        &brep,
        &shell_id,
        ShellCrossingRequest {
            ray,
            range: PositiveRayParameterRange::new(),
            policy: policy(64, 64),
            owner,
            requirement: &requirement,
            source,
        },
    )
    .unwrap();
    let CheckedRayCrossingResult::Proved { hits, .. } = result else {
        panic!("the shell crossing must be proved");
    };
    assert!(!hits.is_empty());
    for hit in &hits {
        assert_eq!(hit.owner(), owner);
        assert_eq!(hit.requirement(), &requirement);
        assert_eq!(hit.source(), source);
    }
}
