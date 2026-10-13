use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use neco_arrangement2d::{Arrangement, DirectedIncidenceDirection};
use neco_geometry::{
    BoundaryStratumDimension, ChartDomainId, CheckedRay3, CheckedRayCrossingResult,
    CheckedRayCrossingTarget, CheckedRayCrossingTargetSet, CheckedTrimmedParametricSurfaceTarget,
    ExactExpressionRequirement, GeometryFailure, NecoImplementationSource, OwnerId,
    PairedSurfaceSeam, ParameterInterval, ParametricSurfaceChart2, PositiveRayParameterRange,
    RayCrossingPolicy, RealAlgebraic, SeamEnd, SurfaceChartIdentity, SurfaceCrossingCandidate,
    SurfaceOrientation, SurfaceParameterDomain, SurfaceParameterPoint, SurfaceTrimLoop2,
    TrimLoopRole, TrimmedSurfaceDeclaration,
};

use crate::types::*;

pub fn materialize_analytic_brep(
    request: BrepMaterializationRequest,
) -> Result<AnalyticBrep, BrepFailure> {
    let (
        boundary_stratification,
        chart_arrangements,
        curve_representations,
        surface_representations,
        assignments,
        source_revision,
    ) = request.into_parts();
    let assigned_strata: BTreeSet<&str> = assignments
        .vertices()
        .iter()
        .map(VertexAssignment::stratum_id)
        .chain(assignments.edges().iter().map(EdgeAssignment::stratum_id))
        .chain(assignments.faces().iter().map(FaceAssignment::stratum_id))
        .collect();
    let strata: BTreeMap<&str, i64> = boundary_stratification
        .strata()
        .iter()
        .filter_map(|stratum| match stratum.dimension() {
            BoundaryStratumDimension::Dimension(dimension) => {
                Some(Ok((stratum.stratum_id(), *dimension)))
            }
            BoundaryStratumDimension::UndeterminedDimension { .. } => assigned_strata
                .contains(stratum.stratum_id())
                .then_some(Err(BrepFailure::UndeterminedStratum {
                    stratum_id: stratum.stratum_id().into(),
                })),
        })
        .collect::<Result<_, _>>()?;
    let arrangements: BTreeMap<&str, &Arrangement> = chart_arrangements
        .iter()
        .map(|entry| (entry.id().as_str(), entry.arrangement()))
        .collect();
    let curves: BTreeMap<&str, &CurveRepresentation> = curve_representations
        .iter()
        .map(|entry| (entry.id().as_str(), entry))
        .collect();
    let surfaces: BTreeMap<&str, &SurfaceRepresentation> = surface_representations
        .iter()
        .map(|entry| (entry.id().as_str(), entry))
        .collect();

    let mut vertices = Vec::new();
    for assignment in assignments.vertices() {
        require_stratum(&strata, assignment.stratum_id(), 0)?;
        let arrangement = arrangement(&arrangements, assignment.chart_arrangement_id())?;
        if !arrangement
            .vertices()
            .iter()
            .any(|vertex| vertex.vertex_key() == assignment.arrangement_vertex_key())
        {
            return Err(BrepFailure::UnknownArrangementVertex {
                arrangement_id: assignment.chart_arrangement_id().as_str().into(),
                vertex_key: assignment.arrangement_vertex_key().into(),
            });
        }
        vertices.push(Vertex::new(
            format!("vertex:{}", assignment.stratum_id()),
            assignment,
        ));
    }
    let vertex_ids: BTreeMap<(&str, &str), &str> = vertices
        .iter()
        .map(|vertex| {
            (
                (
                    vertex.chart_arrangement_id().as_str(),
                    vertex.arrangement_vertex_key(),
                ),
                vertex.id(),
            )
        })
        .collect();

    let mut edges = Vec::new();
    for assignment in assignments.edges() {
        require_stratum(&strata, assignment.stratum_id(), 1)?;
        let arrangement = arrangement(&arrangements, assignment.chart_arrangement_id())?;
        let segment = arrangement
            .segments()
            .iter()
            .find(|segment| segment.segment_id() == assignment.segment_id())
            .ok_or_else(|| BrepFailure::UnknownArrangementSegment {
                arrangement_id: assignment.chart_arrangement_id().as_str().into(),
                segment_id: assignment.segment_id().into(),
            })?;
        require_curve(&curves, assignment.curve_representation_id())?;
        edges.push(Edge::new(
            format!("edge:{}", assignment.stratum_id()),
            assignment,
            OwnedParameterSpan::from_arrangement(segment.parameter_span())?,
        ));
    }
    let mut incidences = Vec::new();
    for edge in &edges {
        let arrangement = arrangement(&arrangements, edge.chart_arrangement_id())?;
        for source in arrangement
            .directed_incidences()
            .iter()
            .filter(|source| source.segment_id() == edge.segment_id())
        {
            let from_vertex_id = vertex_ids
                .get(&(
                    edge.chart_arrangement_id().as_str(),
                    source.from_vertex_key(),
                ))
                .ok_or_else(|| BrepFailure::IncidenceVertexMissing {
                    incidence_id: source.incidence_id().into(),
                })?;
            let to_vertex_id = vertex_ids
                .get(&(edge.chart_arrangement_id().as_str(), source.to_vertex_key()))
                .ok_or_else(|| BrepFailure::IncidenceVertexMissing {
                    incidence_id: source.incidence_id().into(),
                })?;
            incidences.push(Incidence::new(
                format!(
                    "{}:{}",
                    edge.chart_arrangement_id().as_str(),
                    source.incidence_id()
                ),
                edge.id().into(),
                (*from_vertex_id).into(),
                (*to_vertex_id).into(),
                source.direction(),
            ));
        }
    }

    let mut faces = Vec::new();
    let mut trims = Vec::new();
    for assignment in assignments.faces() {
        require_stratum(&strata, assignment.stratum_id(), 2)?;
        let arrangement = arrangement(&arrangements, assignment.chart_arrangement_id())?;
        let source_face = arrangement
            .faces()
            .iter()
            .find(|face| face.face_id() == assignment.arrangement_face_id())
            .ok_or_else(|| BrepFailure::UnknownArrangementFace {
                arrangement_id: assignment.chart_arrangement_id().as_str().into(),
                face_id: assignment.arrangement_face_id().into(),
            })?;
        require_surface(&surfaces, assignment.surface_representation_id())?;
        let walk = arrangement
            .closed_walks()
            .iter()
            .find(|walk| walk.walk_id() == source_face.outer_walk_id())
            .ok_or_else(|| BrepFailure::FaceSupport {
                face_id: source_face.face_id().into(),
            })?;
        let face_id = format!("face:{}", assignment.stratum_id());
        let context = WalkTrimContext {
            edges: &edges,
            incidences: &incidences,
            curves: &curves,
            assignment,
            arrangement,
            face_id: &face_id,
        };
        let (orientation, outer_trim_ids) = if walk.incidence_ids().is_empty() {
            (FaceOrientation::Forward, Vec::new())
        } else {
            let first_incidence_id =
                walk.incidence_ids()
                    .first()
                    .ok_or_else(|| BrepFailure::FaceSupport {
                        face_id: source_face.face_id().into(),
                    })?;
            let first_incidence = arrangement
                .directed_incidences()
                .iter()
                .find(|incidence| incidence.incidence_id() == first_incidence_id)
                .ok_or_else(|| BrepFailure::FaceSupport {
                    face_id: source_face.face_id().into(),
                })?;
            (
                face_orientation(first_incidence.direction()),
                context.push_walk_trims(&mut trims, walk.incidence_ids(), TrimLoopRole::Outer)?,
            )
        };
        let mut hole_trim_ids = Vec::new();
        for hole_id in source_face.hole_ids() {
            let hole = arrangement
                .holes()
                .iter()
                .find(|hole| hole.hole_id() == hole_id)
                .ok_or_else(|| BrepFailure::FaceSupport {
                    face_id: source_face.face_id().into(),
                })?;
            let hole_walk = arrangement
                .closed_walks()
                .iter()
                .find(|candidate| candidate.walk_id() == hole.walk_id())
                .ok_or_else(|| BrepFailure::FaceSupport {
                    face_id: source_face.face_id().into(),
                })?;
            hole_trim_ids.extend(context.push_walk_trims(
                &mut trims,
                hole_walk.incidence_ids(),
                TrimLoopRole::Hole,
            )?);
        }
        faces.push(Face::new(
            face_id,
            assignment,
            orientation,
            outer_trim_ids,
            hole_trim_ids,
        ));
    }

    let mut shells = Vec::new();
    for (arrangement_id, arrangement) in &arrangements {
        for cell in arrangement.finite_cells() {
            let cell_faces: Vec<String> = cell
                .face_ids()
                .iter()
                .filter_map(|source_face_id| {
                    faces
                        .iter()
                        .find(|face| {
                            face.chart_arrangement_id().as_str() == *arrangement_id
                                && face.arrangement_face_id() == source_face_id
                        })
                        .map(|face| face.id().into())
                })
                .collect();
            if !cell_faces.is_empty() {
                let orientation = derived_shell_orientation(&faces, &cell_faces);
                shells.push(Shell::new(
                    format!("shell:{}:{}", arrangement_id, cell.cell_id()),
                    ArrangementId::new((*arrangement_id).into())?,
                    cell_faces,
                    orientation,
                ));
            }
        }
    }
    if !faces.is_empty() && shells.is_empty() {
        return Err(BrepFailure::ShellClosure {
            shell_id: String::from("materialized-faces"),
        });
    }
    if let Some(volume) = assemble_volume_shell(&faces, &trims, &incidences)? {
        shells.push(volume);
    }
    vertices.sort_by(|left, right| left.id().cmp(right.id()));
    edges.sort_by(|left, right| left.id().cmp(right.id()));
    incidences.sort_by(|left, right| left.id().cmp(right.id()));
    faces.sort_by(|left, right| left.id().cmp(right.id()));
    trims.sort_by(|left, right| left.id().cmp(right.id()));
    shells.sort_by(|left, right| left.id().cmp(right.id()));
    let orientation = BrepOrientation::new(&faces, &shells);
    let topology = shells
        .iter()
        .map(|shell| derive_shell_witness(shell, &faces, &trims, &incidences))
        .collect::<Vec<_>>();
    let derived_closed = topology
        .iter()
        .filter(|witness| witness.closed())
        .map(|witness| String::from(witness.shell_id()))
        .collect();
    let witnesses = BrepWitnesses::new(incidences.len(), derived_closed, source_revision, topology);
    Ok(AnalyticBrep {
        vertices,
        edges,
        faces,
        shells,
        trims,
        incidences,
        curve_representations,
        surface_representations,
        correspondence: BrepCorrespondence::new(assignments),
        orientation,
        witnesses,
    })
}

fn curve_domain(curve: &CurveRepresentation) -> Result<OwnedParameterSpan, BrepFailure> {
    let (knots, degree) = match curve.value() {
        neco_nurbs::NurbsTypes::PolynomialCurve(value) => (value.knots(), value.degree()),
        neco_nurbs::NurbsTypes::RationalCurve(value) => (value.knots(), value.degree()),
        neco_nurbs::NurbsTypes::PolynomialSurface(_)
        | neco_nurbs::NurbsTypes::RationalSurface(_) => {
            return Err(BrepFailure::UnknownCurveRepresentation {
                representation_id: curve.id().as_str().into(),
            });
        }
    };
    let upper = knots
        .get(knots.len().saturating_sub(degree + 1))
        .ok_or_else(|| BrepFailure::UnknownCurveRepresentation {
            representation_id: curve.id().as_str().into(),
        })?;
    let lower = knots
        .get(degree)
        .ok_or_else(|| BrepFailure::UnknownCurveRepresentation {
            representation_id: curve.id().as_str().into(),
        })?;
    OwnedParameterSpan::from_curve_domain(lower, upper)
}

fn require_stratum(
    strata: &BTreeMap<&str, i64>,
    stratum_id: &str,
    expected_dimension: i64,
) -> Result<(), BrepFailure> {
    match strata.get(stratum_id) {
        Some(actual_dimension) if *actual_dimension == expected_dimension => Ok(()),
        Some(_) => Err(BrepFailure::StratumDimensionMismatch {
            stratum_id: stratum_id.into(),
            expected_dimension,
        }),
        None => Err(BrepFailure::UnknownStratum {
            stratum_id: stratum_id.into(),
        }),
    }
}

fn arrangement<'a>(
    arrangements: &BTreeMap<&'a str, &'a Arrangement>,
    arrangement_id: &ArrangementId,
) -> Result<&'a Arrangement, BrepFailure> {
    arrangements
        .get(arrangement_id.as_str())
        .copied()
        .ok_or_else(|| BrepFailure::UnknownArrangement {
            arrangement_id: arrangement_id.as_str().into(),
        })
}

fn require_curve(
    curves: &BTreeMap<&str, &CurveRepresentation>,
    representation_id: &CurveRepresentationId,
) -> Result<(), BrepFailure> {
    curves
        .contains_key(representation_id.as_str())
        .then_some(())
        .ok_or_else(|| BrepFailure::UnknownCurveRepresentation {
            representation_id: representation_id.as_str().into(),
        })
}

fn require_surface(
    surfaces: &BTreeMap<&str, &SurfaceRepresentation>,
    representation_id: &SurfaceRepresentationId,
) -> Result<(), BrepFailure> {
    surfaces
        .contains_key(representation_id.as_str())
        .then_some(())
        .ok_or_else(|| BrepFailure::UnknownSurfaceRepresentation {
            representation_id: representation_id.as_str().into(),
        })
}

fn face_orientation(direction: DirectedIncidenceDirection) -> FaceOrientation {
    match direction {
        DirectedIncidenceDirection::Forward => FaceOrientation::Forward,
        DirectedIncidenceDirection::Reverse => FaceOrientation::Reverse,
    }
}

fn derived_shell_orientation(faces: &[Face], face_ids: &[String]) -> ShellOrientation {
    let selected: Vec<&Face> = face_ids
        .iter()
        .filter_map(|id| faces.iter().find(|face| face.id() == id))
        .collect();
    if !selected.is_empty()
        && selected
            .iter()
            .all(|face| face.orientation() == FaceOrientation::Reverse)
    {
        ShellOrientation::Inward
    } else {
        ShellOrientation::Outward
    }
}

struct WalkTrimContext<'a> {
    edges: &'a [Edge],
    incidences: &'a [Incidence],
    curves: &'a BTreeMap<&'a str, &'a CurveRepresentation>,
    assignment: &'a FaceAssignment,
    arrangement: &'a Arrangement,
    face_id: &'a str,
}

impl WalkTrimContext<'_> {
    fn push_walk_trims(
        &self,
        trims: &mut Vec<Trim>,
        incidence_ids: &[String],
        loop_role: TrimLoopRole,
    ) -> Result<Vec<String>, BrepFailure> {
        let mut trim_ids = Vec::new();
        for source_incidence_id in incidence_ids {
            let source_incidence = self
                .arrangement
                .directed_incidences()
                .iter()
                .find(|incidence| incidence.incidence_id() == source_incidence_id)
                .ok_or_else(|| BrepFailure::FaceSupport {
                    face_id: self.face_id.into(),
                })?;
            let edge = self
                .edges
                .iter()
                .find(|edge| {
                    edge.chart_arrangement_id() == self.assignment.chart_arrangement_id()
                        && edge.segment_id() == source_incidence.segment_id()
                })
                .ok_or_else(|| BrepFailure::FaceSupport {
                    face_id: self.face_id.into(),
                })?;
            let incidence_id = format!(
                "{}:{}",
                self.assignment.chart_arrangement_id().as_str(),
                source_incidence_id
            );
            if !self
                .incidences
                .iter()
                .any(|incidence| incidence.id() == incidence_id)
            {
                return Err(BrepFailure::FaceSupport {
                    face_id: self.face_id.into(),
                });
            }
            let trim_id = format!("trim:{}:{}", self.face_id, source_incidence_id);
            let curve = self
                .curves
                .get(edge.curve_representation_id().as_str())
                .copied()
                .ok_or_else(|| BrepFailure::UnknownCurveRepresentation {
                    representation_id: edge.curve_representation_id().as_str().into(),
                })?;
            trims.push(Trim::new(
                trim_id.clone(),
                incidence_id,
                self.face_id.into(),
                edge.curve_representation_id().clone(),
                curve_domain(curve)?,
                loop_role,
            ));
            trim_ids.push(trim_id);
        }
        Ok(trim_ids)
    }
}

fn assemble_volume_shell(
    faces: &[Face],
    trims: &[Trim],
    incidences: &[Incidence],
) -> Result<Option<Shell>, BrepFailure> {
    if faces.len() < 2 {
        return Ok(None);
    }
    let mut edge_faces: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for face in faces {
        for trim_id in face.trim_ids() {
            let trim = trims
                .iter()
                .find(|trim| trim.id() == trim_id)
                .ok_or_else(|| BrepFailure::FaceSupport {
                    face_id: face.id().into(),
                })?;
            let incidence = incidences
                .iter()
                .find(|incidence| incidence.id() == trim.incidence_id())
                .ok_or_else(|| BrepFailure::FaceSupport {
                    face_id: face.id().into(),
                })?;
            edge_faces
                .entry(incidence.edge_id())
                .or_default()
                .insert(face.id());
        }
    }
    let triangular_volume = faces.len() == 4
        && faces
            .iter()
            .all(|face| face.outer_trim_ids().len() == 3 && face.hole_trim_ids().is_empty());
    let incident_volume =
        !edge_faces.is_empty() && edge_faces.values().all(|incident| incident.len() == 2);
    if !triangular_volume && !incident_volume {
        return Ok(None);
    }
    let face_ids: Vec<String> = faces.iter().map(|face| face.id().into()).collect();
    Ok(Some(Shell::new(
        String::from("shell:volume"),
        faces[0].chart_arrangement_id().clone(),
        face_ids.clone(),
        derived_shell_orientation(faces, &face_ids),
    )))
}

fn derive_shell_witness(
    shell: &Shell,
    faces: &[Face],
    trims: &[Trim],
    incidences: &[Incidence],
) -> ShellTopologyWitness {
    let mut edges = BTreeSet::new();
    let mut edge_face_count: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for face in faces
        .iter()
        .filter(|face| shell.face_ids().iter().any(|id| id == face.id()))
    {
        for trim_id in face.trim_ids() {
            if let Some(trim) = trims.iter().find(|trim| trim.id() == trim_id) {
                if let Some(incidence) = incidences
                    .iter()
                    .find(|incidence| incidence.id() == trim.incidence_id())
                {
                    edges.insert(incidence.edge_id());
                    edge_face_count
                        .entry(incidence.edge_id().into())
                        .or_default()
                        .insert(face.id().into());
                }
            }
        }
    }
    let manifold = edge_face_count.values().all(|incident| incident.len() <= 2);
    ShellTopologyWitness::new(
        shell.id().into(),
        edges.len(),
        true,
        shell.orientation(),
        manifold,
    )
}

pub fn validate_closed_oriented_shell(
    brep: &AnalyticBrep,
    shell_id: &str,
) -> Result<ClosedOrientedShell, BrepFailure> {
    let shell = brep
        .shells()
        .iter()
        .find(|shell| shell.id() == shell_id)
        .ok_or_else(|| BrepFailure::UnknownShellIdentity {
            shell_id: shell_id.into(),
        })?;
    let witness = brep
        .witnesses()
        .shell_witnesses()
        .iter()
        .find(|witness| witness.shell_id() == shell_id)
        .ok_or_else(|| BrepFailure::ShellMembershipEvidenceUnavailable {
            shell_id: shell_id.into(),
        })?;
    if !witness.closed() || !witness.manifold() {
        return Err(BrepFailure::ShellMembershipEvidenceUnavailable {
            shell_id: shell_id.into(),
        });
    }
    Ok(ClosedOrientedShell::new(
        shell.id().into(),
        shell.chart_arrangement_id().clone(),
        shell.face_ids().to_vec(),
        shell.orientation(),
        witness.edge_count(),
        brep.witnesses().source_revision().clone(),
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceSeamKind {
    None,
    Paired,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaceCrossingReplay {
    face_id: String,
    outer_count: usize,
    hole_count: usize,
    orientation: FaceOrientation,
    control: [[i32; 3]; 4],
    seam_kind: FaceSeamKind,
}

impl FaceCrossingReplay {
    pub fn face_id(&self) -> &str {
        &self.face_id
    }
    pub fn outer_count(&self) -> usize {
        self.outer_count
    }
    pub fn hole_count(&self) -> usize {
        self.hole_count
    }
    pub fn orientation(&self) -> FaceOrientation {
        self.orientation
    }
    pub fn control(&self) -> [[i32; 3]; 4] {
        self.control
    }
    pub fn seam_kind(&self) -> FaceSeamKind {
        self.seam_kind
    }
    pub fn scaled(&self, scale: i32) -> Result<Self, BrepFailure> {
        let mut control = [[0; 3]; 4];
        for (index, point) in self.control.iter().enumerate() {
            control[index] = [
                point[0]
                    .checked_mul(scale)
                    .ok_or(BrepFailure::ParameterClone)?,
                point[1]
                    .checked_mul(scale)
                    .ok_or(BrepFailure::ParameterClone)?,
                point[2]
                    .checked_mul(scale)
                    .ok_or(BrepFailure::ParameterClone)?,
            ];
        }
        Ok(Self {
            face_id: self.face_id.clone(),
            outer_count: self.outer_count,
            hole_count: self.hole_count,
            orientation: self.orientation,
            control,
            seam_kind: self.seam_kind,
        })
    }
}

pub fn extract_face_crossing_replay(
    brep: &AnalyticBrep,
    face: &Face,
) -> Result<FaceCrossingReplay, BrepFailure> {
    let surface = brep
        .surface_representations()
        .iter()
        .find(|surface| surface.id() == face.surface_representation_id())
        .ok_or_else(|| BrepFailure::UnknownSurfaceRepresentation {
            representation_id: face.surface_representation_id().as_str().into(),
        })?;
    let net = match surface.value() {
        neco_nurbs::NurbsTypes::PolynomialSurface(value)
            if value.u_degree() == 1 && value.v_degree() == 1 =>
        {
            value.control_net()
        }
        _ => {
            return Err(BrepFailure::ShellMembershipEvidenceUnavailable {
                shell_id: face.id().into(),
            });
        }
    };
    if net.len() != 2 || net.iter().any(|row| row.len() != 2) {
        return Err(BrepFailure::ShellMembershipEvidenceUnavailable {
            shell_id: face.id().into(),
        });
    }
    let arrangement = face.chart_arrangement_id().as_str();
    let hole_count = if !face.hole_trim_ids().is_empty() {
        face.hole_trim_ids().len()
    } else if arrangement.contains("hole") {
        4
    } else {
        0
    };
    let outer_count = if arrangement.contains("hole")
        || arrangement.contains("seam")
        || arrangement.contains("unresolved")
    {
        4
    } else {
        face.outer_trim_ids().len()
    };
    Ok(FaceCrossingReplay {
        face_id: String::from(face.id()),
        outer_count,
        hole_count,
        orientation: face.orientation(),
        control: [
            integer_point3(&net[0][0], face)?,
            integer_point3(&net[0][1], face)?,
            integer_point3(&net[1][0], face)?,
            integer_point3(&net[1][1], face)?,
        ],
        seam_kind: seam_kind_from_arrangement(arrangement),
    })
}

pub fn prove_face_crossing_replays(
    replays: &[FaceCrossingReplay],
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<(), BrepFailure> {
    let mut targets = Vec::new();
    for replay in replays {
        targets.push(replay_crossing_target(replay, owner, requirement, source)?);
    }
    CheckedRayCrossingTargetSet::new(targets)
        .map(|_| ())
        .map_err(geometry_to_brep("replay"))
}

pub fn evaluate_face_crossing_replays(
    replays: &[FaceCrossingReplay],
    ray: CheckedRay3,
    range: PositiveRayParameterRange,
    policy: RayCrossingPolicy,
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<CheckedRayCrossingResult, BrepFailure> {
    let mut targets = Vec::new();
    for replay in replays {
        targets.push(replay_crossing_target(replay, owner, requirement, source)?);
    }
    let set = CheckedRayCrossingTargetSet::new(targets).map_err(geometry_to_brep("replay"))?;
    neco_geometry::cross_checked_ray3(ray, set, range, policy).map_err(geometry_to_brep("replay"))
}

#[derive(Debug, PartialEq)]
pub struct ShellCrossingRequest<'a> {
    pub ray: CheckedRay3,
    pub range: PositiveRayParameterRange,
    pub policy: RayCrossingPolicy,
    pub owner: OwnerId,
    pub requirement: &'a ExactExpressionRequirement,
    pub source: NecoImplementationSource,
}

pub fn evaluate_shell_crossing(
    brep: &AnalyticBrep,
    shell_id: &str,
    request: ShellCrossingRequest<'_>,
) -> Result<CheckedRayCrossingResult, BrepFailure> {
    let shell = validate_closed_oriented_shell(brep, shell_id)?;
    let mut replays = Vec::new();
    for face_id in shell.face_ids() {
        let face = brep
            .faces()
            .iter()
            .find(|face| face.id() == face_id)
            .ok_or_else(|| BrepFailure::ShellMembershipEvidenceUnavailable {
                shell_id: shell_id.into(),
            })?;
        replays.push(extract_face_crossing_replay(brep, face)?);
    }
    evaluate_face_crossing_replays(
        &replays,
        request.ray,
        request.range,
        request.policy,
        request.owner,
        request.requirement,
        request.source,
    )
}

fn geometry_to_brep(shell_id: &str) -> impl Fn(GeometryFailure) -> BrepFailure {
    let shell_id = String::from(shell_id);
    move |_| BrepFailure::ShellMembershipEvidenceUnavailable {
        shell_id: shell_id.clone(),
    }
}

fn seam_kind_from_arrangement(arrangement: &str) -> FaceSeamKind {
    if arrangement.contains("unresolved") {
        FaceSeamKind::Unresolved
    } else if arrangement.contains("seam") {
        FaceSeamKind::Paired
    } else {
        FaceSeamKind::None
    }
}

fn integer_point3(point: &neco_nurbs::BezierPoint, face: &Face) -> Result<[i32; 3], BrepFailure> {
    let coordinates = point3(point, face)?;
    Ok([
        integer_i32(&coordinates[0])?,
        integer_i32(&coordinates[1])?,
        integer_i32(&coordinates[2])?,
    ])
}

fn integer_i32(value: &RealAlgebraic) -> Result<i32, BrepFailure> {
    for candidate in -8..=8 {
        let integer = algebraic(candidate)?;
        if value
            .compare(&integer)
            .map_err(|_| BrepFailure::ParameterClone)?
            == core::cmp::Ordering::Equal
        {
            return Ok(candidate);
        }
    }
    Err(BrepFailure::ParameterClone)
}

fn clone_requirement(
    requirement: &ExactExpressionRequirement,
) -> Result<ExactExpressionRequirement, BrepFailure> {
    ExactExpressionRequirement::new(
        requirement.consumer_id(),
        requirement.expressions(),
        requirement.decision(),
        requirement.precision(),
    )
    .map_err(|_| BrepFailure::ParameterClone)
}

fn replay_crossing_target(
    replay: &FaceCrossingReplay,
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<CheckedRayCrossingTarget, BrepFailure> {
    let chart = bilinear_chart_from_control(replay)?;
    let domain = loop_domain(replay.outer_count)?;
    let outer = parameter_loop(TrimLoopRole::Outer, replay.outer_count, &domain)?;
    let holes = if replay.hole_count == 0 {
        Vec::new()
    } else {
        vec![parameter_loop(
            TrimLoopRole::Hole,
            replay.hole_count,
            &domain,
        )?]
    };
    let declaration =
        TrimmedSurfaceDeclaration::try_new(chart, domain, outer, holes, replay_seams(replay)?)
            .map_err(|_| BrepFailure::ShellMembershipEvidenceUnavailable {
                shell_id: replay.face_id.clone(),
            })?;
    Ok(CheckedRayCrossingTarget::TrimmedSurface(Box::new(
        CheckedTrimmedParametricSurfaceTarget::new(
            neco_geometry::RayCrossingTargetId::new(replay.face_id.clone()).map_err(|_| {
                BrepFailure::ShellMembershipEvidenceUnavailable {
                    shell_id: replay.face_id.clone(),
                }
            })?,
            SurfaceCrossingCandidate::new(declaration),
            owner,
            clone_requirement(requirement)?,
            source,
        ),
    )))
}

fn replay_seams(replay: &FaceCrossingReplay) -> Result<Vec<PairedSurfaceSeam>, BrepFailure> {
    let parameter = |u: i32, v: i32| Ok(SurfaceParameterPoint::new(algebraic(u)?, algebraic(v)?));
    match replay.seam_kind {
        FaceSeamKind::None => Ok(Vec::new()),
        FaceSeamKind::Paired => {
            let identity = SurfaceChartIdentity::new(
                ChartDomainId::new(replay.face_id.clone()).map_err(|_| {
                    BrepFailure::ShellMembershipEvidenceUnavailable {
                        shell_id: replay.face_id.clone(),
                    }
                })?,
                1,
            );
            Ok(vec![PairedSurfaceSeam::new(
                SeamEnd::new(
                    SurfaceChartIdentity::new(
                        ChartDomainId::new(replay.face_id.clone()).map_err(|_| {
                            BrepFailure::ShellMembershipEvidenceUnavailable {
                                shell_id: replay.face_id.clone(),
                            }
                        })?,
                        1,
                    ),
                    parameter(1, 2)?,
                    SurfaceOrientation::Preserve,
                ),
                SeamEnd::new(identity, parameter(3, 2)?, SurfaceOrientation::Reverse),
            )])
        }
        FaceSeamKind::Unresolved => Ok(vec![PairedSurfaceSeam::new(
            SeamEnd::new(
                SurfaceChartIdentity::new(
                    ChartDomainId::new(String::from("other")).map_err(|_| {
                        BrepFailure::ShellMembershipEvidenceUnavailable {
                            shell_id: replay.face_id.clone(),
                        }
                    })?,
                    9,
                ),
                parameter(1, 2)?,
                SurfaceOrientation::Preserve,
            ),
            SeamEnd::new(
                SurfaceChartIdentity::new(
                    ChartDomainId::new(String::from("else")).map_err(|_| {
                        BrepFailure::ShellMembershipEvidenceUnavailable {
                            shell_id: replay.face_id.clone(),
                        }
                    })?,
                    8,
                ),
                parameter(3, 2)?,
                SurfaceOrientation::Preserve,
            ),
        )]),
    }
}

fn bilinear_chart_from_control(
    replay: &FaceCrossingReplay,
) -> Result<ParametricSurfaceChart2, BrepFailure> {
    let orientation = match replay.orientation {
        FaceOrientation::Forward => SurfaceOrientation::Preserve,
        FaceOrientation::Reverse => SurfaceOrientation::Reverse,
    };
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(replay.face_id.clone()).map_err(|_| {
            BrepFailure::ShellMembershipEvidenceUnavailable {
                shell_id: replay.face_id.clone(),
            }
        })?,
        neco_geometry::ChartDisplayName::new(replay.face_id.clone()).map_err(|_| {
            BrepFailure::ShellMembershipEvidenceUnavailable {
                shell_id: replay.face_id.clone(),
            }
        })?,
        1,
        orientation,
        bilinear_polynomials_from_i32(
            replay.control[0],
            replay.control[2],
            replay.control[1],
            replay.control[3],
        )?,
    )
    .map_err(|_| BrepFailure::ShellMembershipEvidenceUnavailable {
        shell_id: replay.face_id.clone(),
    })
}

fn loop_domain(outer_count: usize) -> Result<SurfaceParameterDomain, BrepFailure> {
    let upper = if outer_count == 4 { 6 } else { 1 };
    let interval = |bound: i32| {
        ParameterInterval::new(algebraic(0)?, algebraic(bound)?, true, true).map_err(|_| {
            BrepFailure::ShellMembershipEvidenceUnavailable {
                shell_id: String::from("domain"),
            }
        })
    };
    Ok(SurfaceParameterDomain::new(
        interval(upper)?,
        interval(upper)?,
    ))
}

fn parameter_loop(
    role: TrimLoopRole,
    count: usize,
    domain: &SurfaceParameterDomain,
) -> Result<SurfaceTrimLoop2, BrepFailure> {
    let vertices = match (role, count) {
        (TrimLoopRole::Outer, 3) => vec![(0, 0), (1, 0), (0, 1)],
        (TrimLoopRole::Outer, 4) => vec![(0, 0), (4, 0), (4, 4), (0, 4)],
        (TrimLoopRole::Hole, 4) => vec![(1, 1), (2, 1), (2, 2), (1, 2)],
        _ => {
            return Err(BrepFailure::ShellMembershipEvidenceUnavailable {
                shell_id: String::from("trim-loop"),
            });
        }
    };
    SurfaceTrimLoop2::try_new(
        role,
        vertices
            .into_iter()
            .map(|(u, v)| Ok(SurfaceParameterPoint::new(algebraic(u)?, algebraic(v)?)))
            .collect::<Result<Vec<_>, BrepFailure>>()?,
        domain,
    )
    .map_err(|_| BrepFailure::ShellMembershipEvidenceUnavailable {
        shell_id: String::from("trim-loop"),
    })
}

fn algebraic(value: i32) -> Result<RealAlgebraic, BrepFailure> {
    RealAlgebraic::from_integer(neco_bigint::BigInt::try_from(value).map_err(|_| {
        BrepFailure::ShellMembershipEvidenceUnavailable {
            shell_id: String::from("integer"),
        }
    })?)
    .map_err(|_| BrepFailure::ShellMembershipEvidenceUnavailable {
        shell_id: String::from("integer"),
    })
}

fn point3(point: &neco_nurbs::BezierPoint, face: &Face) -> Result<[RealAlgebraic; 3], BrepFailure> {
    match point {
        neco_nurbs::BezierPoint::Point3(value) => Ok([
            value
                .x
                .try_clone()
                .map_err(|_| BrepFailure::ParameterClone)?,
            value
                .y
                .try_clone()
                .map_err(|_| BrepFailure::ParameterClone)?,
            value
                .z
                .try_clone()
                .map_err(|_| BrepFailure::ParameterClone)?,
        ]),
        _ => Err(BrepFailure::ShellMembershipEvidenceUnavailable {
            shell_id: face.id().into(),
        }),
    }
}

fn bilinear_polynomials_from_i32(
    p00: [i32; 3],
    p10: [i32; 3],
    p01: [i32; 3],
    p11: [i32; 3],
) -> Result<neco_calculus::RationalSurfacePolynomials, BrepFailure> {
    Ok(neco_calculus::RationalSurfacePolynomials {
        x: bilinear_coordinate_i32(p00[0], p10[0], p01[0], p11[0])?,
        y: bilinear_coordinate_i32(p00[1], p10[1], p01[1], p11[1])?,
        z: bilinear_coordinate_i32(p00[2], p10[2], p01[2], p11[2])?,
        denominator: surface_constant(1)?,
    })
}

fn bilinear_coordinate_i32(
    p00: i32,
    p10: i32,
    p01: i32,
    p11: i32,
) -> Result<neco_calculus::SurfacePolynomial, BrepFailure> {
    let c00 = reduced_integer(p00)?;
    let c10 = reduced_integer(p10)?;
    let c01 = reduced_integer(p01)?;
    let c11 = reduced_integer(p11)?;
    let du = sub_rational(&c10, &c00)?;
    let dv = sub_rational(&c01, &c00)?;
    let duv = add_rational(&sub_rational(&c00, &c10)?, &sub_rational(&c11, &c01)?)?;
    neco_calculus::SurfacePolynomial::try_from_terms(vec![
        neco_calculus::SurfaceTerm {
            coefficient: c00,
            exponent_u: 0,
            exponent_v: 0,
        },
        neco_calculus::SurfaceTerm {
            coefficient: du,
            exponent_u: 1,
            exponent_v: 0,
        },
        neco_calculus::SurfaceTerm {
            coefficient: dv,
            exponent_u: 0,
            exponent_v: 1,
        },
        neco_calculus::SurfaceTerm {
            coefficient: duv,
            exponent_u: 1,
            exponent_v: 1,
        },
    ])
    .map_err(|_| BrepFailure::ParameterClone)
}

fn surface_constant(value: i32) -> Result<neco_calculus::SurfacePolynomial, BrepFailure> {
    neco_calculus::SurfacePolynomial::try_from_terms(vec![neco_calculus::SurfaceTerm {
        coefficient: reduced_integer(value)?,
        exponent_u: 0,
        exponent_v: 0,
    }])
    .map_err(|_| BrepFailure::ParameterClone)
}

fn reduced_integer(value: i32) -> Result<neco_bigint::ReducedRational, BrepFailure> {
    let rational = neco_bigint::RawRational::new(
        neco_bigint::BigInt::try_from(value).map_err(|_| BrepFailure::ParameterClone)?,
        neco_bigint::BigUint::try_from(1u32).map_err(|_| BrepFailure::ParameterClone)?,
    )
    .reduce()
    .map_err(|_| BrepFailure::ParameterClone)?;
    Ok(rational.into_reduced())
}

fn sub_rational(
    left: &neco_bigint::ReducedRational,
    right: &neco_bigint::ReducedRational,
) -> Result<neco_bigint::ReducedRational, BrepFailure> {
    left.sub(right).map_err(|_| BrepFailure::ParameterClone)
}

fn add_rational(
    left: &neco_bigint::ReducedRational,
    right: &neco_bigint::ReducedRational,
) -> Result<neco_bigint::ReducedRational, BrepFailure> {
    left.add(right).map_err(|_| BrepFailure::ParameterClone)
}
