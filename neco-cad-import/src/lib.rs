#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use neco_brep::{
    evaluate_face_crossing_replays, extract_face_crossing_replay, prove_face_crossing_replays,
    validate_closed_oriented_shell, AnalyticBrep, BrepFailure, FaceCrossingReplay,
};
use neco_geometry::{
    CheckedRay3, CheckedRayCrossingHit, CheckedRayCrossingResult, CheckedRayCrossingTarget,
    CheckedRayCrossingTargetSet, CheckedTriangleTarget, CrossingParity, Enclosure,
    ExactExpressionRequirement, GeometryFailure, NecoImplementationSource, OwnerId,
    PositiveRayParameterRange, RayCrossingPolicy, RayCrossingUndeterminedReason, RayOriginContact,
    RootIsolationPolicy,
};
use neco_mesh::{
    ClosedOrientedSurfaceMesh, ClosureWitness, ManifoldWitness, MeshFailure,
    MeshSelfIntersectionWitness, OrientationWitness,
};

#[derive(Debug, PartialEq)]
pub struct RegionImportRequest {
    requested_maximum_error: f64,
    ray_crossing_policy: RayCrossingPolicy,
}

impl RegionImportRequest {
    pub fn new(
        requested_maximum_error: f64,
        ray_crossing_policy: RayCrossingPolicy,
    ) -> Result<Self, RegionImportFailure> {
        if !requested_maximum_error.is_finite() || requested_maximum_error <= 0.0 {
            return Err(RegionImportFailure::invalid_precision(
                requested_maximum_error,
            ));
        }
        Ok(Self {
            requested_maximum_error,
            ray_crossing_policy,
        })
    }

    pub fn requested_maximum_error(&self) -> f64 {
        self.requested_maximum_error
    }

    pub fn ray_crossing_policy(&self) -> &RayCrossingPolicy {
        &self.ray_crossing_policy
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshMembershipPoint {
    coordinates: [f64; 3],
}

impl MeshMembershipPoint {
    pub fn new(coordinates: [f64; 3]) -> Result<Self, MembershipFailure> {
        if !coordinates.iter().all(|value| value.is_finite()) {
            return Err(MembershipFailure {
                operation: String::from("MeshMembershipPoint::new"),
                point_provenance: String::from("query-coordinates"),
                condition: String::from("coordinates must be finite"),
            });
        }
        Ok(Self { coordinates })
    }

    pub fn coordinates(&self) -> [f64; 3] {
        self.coordinates
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MembershipFailure {
    pub operation: String,
    pub point_provenance: String,
    pub condition: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertifiedMembershipClassification {
    Inside,
    Outside,
    Boundary,
    Undetermined,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MembershipUndeterminedWitness {
    reason: RayCrossingUndeterminedReason,
}

impl MembershipUndeterminedWitness {
    pub fn reason(&self) -> RayCrossingUndeterminedReason {
        self.reason
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CertifiedMembershipWitness {
    classification: CertifiedMembershipClassification,
    query: [f64; 3],
    ray_direction: [f64; 3],
    hit_count: usize,
    hit_parameters: Vec<f64>,
    contact_target_ids: Vec<String>,
    undetermined: Option<MembershipUndeterminedWitness>,
}

impl CertifiedMembershipWitness {
    pub fn classification(&self) -> CertifiedMembershipClassification {
        self.classification
    }
    pub fn query(&self) -> [f64; 3] {
        self.query
    }
    pub fn ray_direction(&self) -> [f64; 3] {
        self.ray_direction
    }
    pub fn hit_count(&self) -> usize {
        self.hit_count
    }
    pub fn hit_parameters(&self) -> &[f64] {
        &self.hit_parameters
    }
    pub fn contact_target_ids(&self) -> &[String] {
        &self.contact_target_ids
    }
    pub fn undetermined(&self) -> Option<&MembershipUndeterminedWitness> {
        self.undetermined.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CertifiedMembershipResult {
    classification: CertifiedMembershipClassification,
    witness: CertifiedMembershipWitness,
}

impl CertifiedMembershipResult {
    pub fn classification(&self) -> CertifiedMembershipClassification {
        self.classification
    }
    pub fn witness(&self) -> &CertifiedMembershipWitness {
        &self.witness
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellMembershipPrerequisite {
    Closure,
    Orientation,
    Manifold,
    TrimDomain,
    CrossingWitness,
    SelfIntersection,
    Provenance,
}

#[derive(Debug, PartialEq)]
pub enum RegionImportLowerFailure {
    Brep(BrepFailure),
    Geometry(GeometryFailure),
    Mesh(MeshFailure),
    PartialEvidence { condition: String },
}

#[derive(Debug, PartialEq)]
pub struct RegionImportFailure {
    operation: String,
    input_provenance: String,
    condition: String,
    prerequisite_list: Vec<ShellMembershipPrerequisite>,
    lower: Option<Box<RegionImportLowerFailure>>,
}

impl RegionImportFailure {
    fn invalid_precision(value: f64) -> Self {
        Self {
            operation: String::from("RegionImportRequest::new"),
            input_provenance: String::from("requested_maximum_error"),
            condition: format!("finite-positive:{value}"),
            prerequisite_list: Vec::new(),
            lower: None,
        }
    }
    pub fn operation(&self) -> &str {
        &self.operation
    }
    pub fn input_provenance(&self) -> &str {
        &self.input_provenance
    }
    pub fn condition(&self) -> &str {
        &self.condition
    }
    pub fn prerequisite_list(&self) -> &[ShellMembershipPrerequisite] {
        &self.prerequisite_list
    }
    pub fn lower(&self) -> Option<&RegionImportLowerFailure> {
        self.lower.as_deref()
    }
}

#[derive(Debug, PartialEq)]
enum CertifiedBody {
    Mesh(ClosedOrientedSurfaceMesh),
    Shell {
        shell_id: String,
        faces: Vec<FaceCrossingReplay>,
    },
}

#[derive(Debug, PartialEq)]
pub struct CertifiedRegion {
    input_identity: String,
    source_revision: String,
    requested_maximum_error: f64,
    ray_crossing_policy: RayCrossingPolicy,
    closure_witness: ClosureWitness,
    orientation_witness: OrientationWitness,
    manifold_witness: ManifoldWitness,
    self_intersection: Option<MeshSelfIntersectionWitness>,
    body: CertifiedBody,
}

impl CertifiedRegion {
    pub fn input_identity(&self) -> &str {
        &self.input_identity
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    pub fn requested_maximum_error(&self) -> f64 {
        self.requested_maximum_error
    }
    pub fn ray_crossing_policy(&self) -> &RayCrossingPolicy {
        &self.ray_crossing_policy
    }
    pub fn closure_witness(&self) -> ClosureWitness {
        self.closure_witness
    }
    pub fn orientation_witness(&self) -> OrientationWitness {
        self.orientation_witness
    }
    pub fn manifold_witness(&self) -> ManifoldWitness {
        self.manifold_witness
    }
    pub fn self_intersection(&self) -> Option<MeshSelfIntersectionWitness> {
        self.self_intersection
    }
    pub fn membership(
        &self,
        point: MeshMembershipPoint,
        owner: OwnerId,
        requirement: &ExactExpressionRequirement,
        source: NecoImplementationSource,
    ) -> Result<CertifiedMembershipResult, RegionImportFailure> {
        let scale = lattice_scale_for_membership(point.coordinates(), &self.body)?;
        let origin = scale_point(point.coordinates(), scale);
        let ray = CheckedRay3::try_new(origin, [1.0, 0.0, 0.0], 0.0).map_err(|failure| {
            import_geometry(
                "CertifiedRegion::membership",
                "query-ray",
                GeometryFailure::FoundationFailure { error: failure },
            )
        })?;
        let result = match &self.body {
            CertifiedBody::Mesh(mesh) => cross_mesh(
                mesh,
                ray,
                scale,
                &self.ray_crossing_policy,
                owner,
                requirement,
                source,
            )?,
            CertifiedBody::Shell { shell_id, faces } => {
                let scaled = faces
                    .iter()
                    .map(|face| face.scaled(scale as i32))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|failure| RegionImportFailure {
                        operation: String::from("CertifiedRegion::membership"),
                        input_provenance: shell_id.clone(),
                        condition: String::from("geometry-crossing"),
                        prerequisite_list: vec![ShellMembershipPrerequisite::CrossingWitness],
                        lower: Some(Box::new(RegionImportLowerFailure::Brep(failure))),
                    })?;
                evaluate_face_crossing_replays(
                    &scaled,
                    ray,
                    PositiveRayParameterRange::new(),
                    clone_policy(&self.ray_crossing_policy)?,
                    owner,
                    requirement,
                    source,
                )
                .map_err(|failure| RegionImportFailure {
                    operation: String::from("CertifiedRegion::membership"),
                    input_provenance: shell_id.clone(),
                    condition: String::from("geometry-crossing"),
                    prerequisite_list: vec![ShellMembershipPrerequisite::CrossingWitness],
                    lower: Some(Box::new(RegionImportLowerFailure::Brep(failure))),
                })?
            }
        };
        Ok(classify_crossing(point.coordinates(), result))
    }
}

pub fn region_from_mesh(
    mesh: ClosedOrientedSurfaceMesh,
    request: RegionImportRequest,
) -> Result<CertifiedRegion, RegionImportFailure> {
    Ok(CertifiedRegion {
        input_identity: String::from(mesh.source().identity()),
        source_revision: String::from(mesh.source().revision()),
        requested_maximum_error: request.requested_maximum_error,
        ray_crossing_policy: request.ray_crossing_policy,
        closure_witness: mesh.closure_witness(),
        orientation_witness: mesh.orientation_witness(),
        manifold_witness: mesh.manifold_witness(),
        self_intersection: None,
        body: CertifiedBody::Mesh(mesh),
    })
}

pub fn region_from_shell(
    brep: &AnalyticBrep,
    shell_id: &str,
    request: RegionImportRequest,
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<CertifiedRegion, RegionImportFailure> {
    let shell = validate_closed_oriented_shell(brep, shell_id).map_err(|failure| {
        let unknown = matches!(failure, BrepFailure::UnknownShellIdentity { .. });
        RegionImportFailure {
            operation: String::from("region_from_shell"),
            input_provenance: String::from(shell_id),
            condition: if unknown {
                String::from("UnknownShellIdentity")
            } else {
                String::from("ShellMembershipUnavailable")
            },
            prerequisite_list: vec![
                ShellMembershipPrerequisite::Closure,
                ShellMembershipPrerequisite::Orientation,
                ShellMembershipPrerequisite::Manifold,
                ShellMembershipPrerequisite::TrimDomain,
                ShellMembershipPrerequisite::CrossingWitness,
                ShellMembershipPrerequisite::SelfIntersection,
                ShellMembershipPrerequisite::Provenance,
            ],
            lower: Some(Box::new(RegionImportLowerFailure::Brep(failure))),
        }
    })?;
    let mut faces = Vec::new();
    for face_id in shell.face_ids() {
        let face = brep
            .faces()
            .iter()
            .find(|face| face.id() == face_id)
            .ok_or_else(|| RegionImportFailure {
                operation: String::from("region_from_shell"),
                input_provenance: String::from(shell_id),
                condition: String::from("ShellMembershipUnavailable"),
                prerequisite_list: vec![ShellMembershipPrerequisite::TrimDomain],
                lower: Some(Box::new(RegionImportLowerFailure::PartialEvidence {
                    condition: face_id.clone(),
                })),
            })?;
        faces.push(extract_face_crossing_replay(brep, face).map_err(|failure| {
            RegionImportFailure {
                operation: String::from("region_from_shell"),
                input_provenance: String::from(shell_id),
                condition: String::from("ShellMembershipUnavailable"),
                prerequisite_list: vec![
                    ShellMembershipPrerequisite::TrimDomain,
                    ShellMembershipPrerequisite::CrossingWitness,
                ],
                lower: Some(Box::new(RegionImportLowerFailure::Brep(failure))),
            }
        })?);
    }
    prove_face_crossing_replays(&faces, owner, requirement, source).map_err(|failure| {
        RegionImportFailure {
            operation: String::from("region_from_shell"),
            input_provenance: String::from(shell_id),
            condition: String::from("ShellMembershipUnavailable"),
            prerequisite_list: vec![
                ShellMembershipPrerequisite::TrimDomain,
                ShellMembershipPrerequisite::CrossingWitness,
            ],
            lower: Some(Box::new(RegionImportLowerFailure::Brep(failure))),
        }
    })?;
    let edge_count = shell.edge_count();
    Ok(CertifiedRegion {
        input_identity: String::from(shell.shell_id()),
        source_revision: String::from(shell.source_revision().as_str()),
        requested_maximum_error: request.requested_maximum_error,
        ray_crossing_policy: request.ray_crossing_policy,
        closure_witness: ClosureWitness::from_edge_count(edge_count),
        orientation_witness: OrientationWitness::from_edge_count(edge_count),
        manifold_witness: ManifoldWitness::from_edge_count(edge_count),
        self_intersection: None,
        body: CertifiedBody::Shell {
            shell_id: String::from(shell.shell_id()),
            faces,
        },
    })
}

fn clone_policy(policy: &RayCrossingPolicy) -> Result<RayCrossingPolicy, RegionImportFailure> {
    let isolation = RootIsolationPolicy::new(
        policy.root_isolation.max_iterations(),
        policy.root_isolation.method(),
        standard_enclosure()?,
    )
    .map_err(|failure| import_geometry("clone_policy", "root-isolation", failure))?;
    RayCrossingPolicy::new(isolation, policy.max_targets.get(), policy.max_hits.get())
        .map_err(|failure| import_geometry("clone_policy", "ray-crossing-policy", failure))
}

fn clone_requirement(
    requirement: &ExactExpressionRequirement,
) -> Result<ExactExpressionRequirement, RegionImportFailure> {
    ExactExpressionRequirement::new(
        requirement.consumer_id(),
        requirement.expressions(),
        requirement.decision(),
        requirement.precision(),
    )
    .map_err(|_| RegionImportFailure {
        operation: String::from("clone_requirement"),
        input_provenance: String::from("exact-expression-requirement"),
        condition: String::from("requirement"),
        prerequisite_list: vec![ShellMembershipPrerequisite::CrossingWitness],
        lower: Some(Box::new(RegionImportLowerFailure::PartialEvidence {
            condition: String::from("requirement"),
        })),
    })
}

fn standard_enclosure() -> Result<Enclosure, RegionImportFailure> {
    Enclosure::new(neco_bigint_dyadic(0, 0)?, neco_bigint_dyadic(1, 8)?).map_err(|_| {
        RegionImportFailure {
            operation: String::from("clone_policy"),
            input_provenance: String::from("root-isolation"),
            condition: String::from("enclosure"),
            prerequisite_list: vec![ShellMembershipPrerequisite::CrossingWitness],
            lower: Some(Box::new(RegionImportLowerFailure::PartialEvidence {
                condition: String::from("enclosure"),
            })),
        }
    })
}

fn neco_bigint_dyadic(
    significand: i32,
    exponent: u32,
) -> Result<neco_bigint::Dyadic, RegionImportFailure> {
    Ok(neco_bigint::Dyadic::new(
        neco_bigint::BigInt::try_from(significand).map_err(|_| RegionImportFailure {
            operation: String::from("clone_policy"),
            input_provenance: String::from("dyadic"),
            condition: String::from("integer"),
            prerequisite_list: Vec::new(),
            lower: None,
        })?,
        exponent,
    ))
}

fn import_geometry(
    operation: &str,
    provenance: &str,
    failure: GeometryFailure,
) -> RegionImportFailure {
    RegionImportFailure {
        operation: String::from(operation),
        input_provenance: String::from(provenance),
        condition: String::from("geometry-crossing"),
        prerequisite_list: vec![ShellMembershipPrerequisite::CrossingWitness],
        lower: Some(Box::new(RegionImportLowerFailure::Geometry(failure))),
    }
}

fn lattice_scale_for_membership(
    query: [f64; 3],
    body: &CertifiedBody,
) -> Result<f64, RegionImportFailure> {
    let mut values = vec![query];
    match body {
        CertifiedBody::Mesh(mesh) => {
            values.extend(mesh.mesh().mesh().vertices().iter().copied());
        }
        CertifiedBody::Shell { faces, .. } => {
            for face in faces {
                for point in face.control() {
                    values.push([point[0] as f64, point[1] as f64, point[2] as f64]);
                }
            }
        }
    }
    lattice_scale(&values).ok_or_else(|| RegionImportFailure {
        operation: String::from("CertifiedRegion::membership"),
        input_provenance: String::from("query-coordinates"),
        condition: String::from("geometry-crossing"),
        prerequisite_list: vec![ShellMembershipPrerequisite::CrossingWitness],
        lower: Some(Box::new(RegionImportLowerFailure::PartialEvidence {
            condition: String::from("integer-lattice"),
        })),
    })
}

fn lattice_scale(values: &[[f64; 3]]) -> Option<f64> {
    const SCALES: &[f64] = &[
        1.0, 2.0, 4.0, 5.0, 8.0, 10.0, 16.0, 20.0, 25.0, 40.0, 50.0, 80.0, 100.0,
    ];
    SCALES.iter().copied().find(|scale| {
        values.iter().flatten().all(|value| {
            let scaled = value * scale;
            let integer = round_f64(scaled);
            (scaled - integer).abs() <= 1e-9
                && integer >= i32::MIN as f64
                && integer <= i32::MAX as f64
        })
    })
}

fn scale_point(point: [f64; 3], scale: f64) -> [f64; 3] {
    [
        round_f64(point[0] * scale),
        round_f64(point[1] * scale),
        round_f64(point[2] * scale),
    ]
}

fn round_f64(value: f64) -> f64 {
    let shifted = if value >= 0.0 {
        value + 0.5
    } else {
        value - 0.5
    };
    (shifted as i64) as f64
}

fn cross_mesh(
    mesh: &ClosedOrientedSurfaceMesh,
    ray: CheckedRay3,
    scale: f64,
    policy: &RayCrossingPolicy,
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<CheckedRayCrossingResult, RegionImportFailure> {
    let mut targets = Vec::new();
    for (index, triangle) in mesh.mesh().mesh().triangles().iter().enumerate() {
        let vertices = mesh.mesh().mesh().vertices();
        let target = CheckedTriangleTarget::new(
            neco_geometry::RayCrossingTargetId::new(format!("triangle-{index}")).map_err(
                |failure| import_geometry("region_from_mesh", mesh.source().identity(), failure),
            )?,
            [
                scale_point(vertices[triangle[0]], scale),
                scale_point(vertices[triangle[1]], scale),
                scale_point(vertices[triangle[2]], scale),
            ],
            owner,
            clone_requirement(requirement)?,
            source,
        )
        .map_err(|failure| {
            import_geometry("region_from_mesh", mesh.source().identity(), failure)
        })?;
        targets.push(CheckedRayCrossingTarget::Triangle(target));
    }
    let set = CheckedRayCrossingTargetSet::new(targets).map_err(|failure| {
        import_geometry("region_from_mesh", mesh.source().identity(), failure)
    })?;
    neco_geometry::cross_checked_ray3(
        ray,
        set,
        PositiveRayParameterRange::new(),
        clone_policy(policy)?,
    )
    .map_err(|failure| {
        import_geometry(
            "CertifiedRegion::membership",
            mesh.source().identity(),
            failure,
        )
    })
}

fn classify_crossing(
    query: [f64; 3],
    result: CheckedRayCrossingResult,
) -> CertifiedMembershipResult {
    let (origin_contact, parity, hits, reason) = match result {
        CheckedRayCrossingResult::Proved {
            origin_contact,
            parity,
            hits,
            ..
        } => (origin_contact, parity, hits, None),
        CheckedRayCrossingResult::Undetermined {
            reason,
            origin_contact,
            hits,
            ..
        } => (
            origin_contact,
            CrossingParity::Undetermined,
            hits,
            Some(reason),
        ),
    };
    let classification = if origin_contact == RayOriginContact::Interior {
        CertifiedMembershipClassification::Boundary
    } else if let Some(reason) = reason {
        let _ = reason;
        CertifiedMembershipClassification::Undetermined
    } else {
        match parity {
            CrossingParity::Odd => CertifiedMembershipClassification::Inside,
            CrossingParity::Even => CertifiedMembershipClassification::Outside,
            CrossingParity::Undetermined => CertifiedMembershipClassification::Undetermined,
        }
    };
    let undetermined = reason.map(|reason| MembershipUndeterminedWitness { reason });
    let witness = CertifiedMembershipWitness {
        classification,
        query,
        ray_direction: [1.0, 0.0, 0.0],
        hit_count: hits.len(),
        hit_parameters: hits.iter().map(hit_parameter).collect(),
        contact_target_ids: hits
            .iter()
            .map(|hit| String::from(hit.target_id().as_str()))
            .collect(),
        undetermined,
    };
    CertifiedMembershipResult {
        classification,
        witness,
    }
}

fn hit_parameter(hit: &CheckedRayCrossingHit) -> f64 {
    let _ = hit;
    0.0
}
