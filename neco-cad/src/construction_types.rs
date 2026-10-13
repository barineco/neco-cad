use crate::provenance::{
    BodyComposition, ConstructionGraph, IdentityLedger, OperationIdentity, Provenance,
    SourceIdentity,
};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigUint, Dyadic, RawRational};
use neco_brep::{CurveRepresentation, SourceRevision, SurfaceRepresentation};
use neco_cad_import::{CertifiedRegion, RegionImportFailure};
use neco_geometry::{AlgebraicRegion, GeometryFailure};
use neco_mesh::MeshFailure;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GeometryAccuracyPolicy {
    #[default]
    ExactOnly,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryOutputCategory {
    Curve,
    Profile,
    Sheet,
    Solid,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryClosure {
    Open,
    Closed,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryOutputExpectation {
    pub category: GeometryOutputCategory,
    pub ambient_dimension: u8,
    pub manifold_dimension: u8,
    pub closure: GeometryClosure,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OutputSlot {
    Primary,
    Named(String),
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct OperationOutputIdentity {
    pub operation: OperationIdentity,
    pub slot: OutputSlot,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationOutputRef {
    pub output: OperationOutputIdentity,
}
impl OperationOutputRef {
    pub fn primary(operation: OperationIdentity) -> Self {
        Self {
            output: OperationOutputIdentity {
                operation,
                slot: OutputSlot::Primary,
            },
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputRole {
    Source,
    Profile,
    Spine,
    Left,
    Right,
    Section,
    InstanceGeometry,
    BodyGeometry,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactConversionFailureKind {
    NonFinite,
    ExactConstruction,
    ZeroDirection,
    NonUnitQuaternion,
    InvalidWeight,
    InvalidOrder,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactConversionFailure {
    pub kind: ExactConversionFailureKind,
    pub field_path: String,
    pub source: Option<SourceIdentity>,
    pub raw_bits: Option<u64>,
}
fn exact_failure(
    kind: ExactConversionFailureKind,
    path: impl Into<String>,
    bits: Option<u64>,
) -> CadConstructionFailure {
    CadConstructionFailure::ExactConversion(ExactConversionFailure {
        kind,
        field_path: path.into(),
        source: None,
        raw_bits: bits,
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactScalar(Arc<RealAlgebraic>);
impl ExactScalar {
    pub fn new(v: f64) -> Result<Self, CadConstructionFailure> {
        Self::from_f64_exact_binary(v, "scalar")
    }
    pub fn from_f64_exact_binary(
        v: f64,
        path: impl Into<String>,
    ) -> Result<Self, CadConstructionFailure> {
        let path = path.into();
        if !v.is_finite() {
            return Err(exact_failure(
                ExactConversionFailureKind::NonFinite,
                path,
                Some(v.to_bits()),
            ));
        }
        let d = Dyadic::from_f64_exact(v).map_err(|_| {
            exact_failure(
                ExactConversionFailureKind::ExactConstruction,
                path.clone(),
                Some(v.to_bits()),
            )
        })?;
        let denom = BigUint::one()
            .and_then(|x| x.shl_bits(d.exponent() as usize))
            .map_err(|_| {
                exact_failure(
                    ExactConversionFailureKind::ExactConstruction,
                    path.clone(),
                    Some(v.to_bits()),
                )
            })?;
        let rational = RawRational::new(
            d.integer().try_clone().map_err(|_| {
                exact_failure(
                    ExactConversionFailureKind::ExactConstruction,
                    path.clone(),
                    Some(v.to_bits()),
                )
            })?,
            denom,
        )
        .reduce()
        .map_err(|_| {
            exact_failure(
                ExactConversionFailureKind::ExactConstruction,
                path.clone(),
                Some(v.to_bits()),
            )
        })?
        .into_reduced();
        RealAlgebraic::from_reduced_rational(&rational)
            .map(|value| Self(Arc::new(value)))
            .map_err(|_| {
                exact_failure(
                    ExactConversionFailureKind::ExactConstruction,
                    path,
                    Some(v.to_bits()),
                )
            })
    }
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
    pub fn compare_exact(&self, r: &Self) -> Result<Ordering, CadConstructionFailure> {
        self.0.compare(&r.0).map_err(|_| {
            exact_failure(
                ExactConversionFailureKind::ExactConstruction,
                "comparison",
                None,
            )
        })
    }
    pub fn to_real_algebraic(&self) -> Result<RealAlgebraic, CadConstructionFailure> {
        self.0.try_clone().map_err(|_| {
            exact_failure(
                ExactConversionFailureKind::ExactConstruction,
                "to_real_algebraic",
                None,
            )
        })
    }
    pub(crate) fn mul(&self, r: &Self) -> Result<Self, CadConstructionFailure> {
        self.0
            .mul(&r.0)
            .map(|value| Self(Arc::new(value)))
            .map_err(|_| {
                exact_failure(
                    ExactConversionFailureKind::ExactConstruction,
                    "product",
                    None,
                )
            })
    }
    pub(crate) fn add(&self, r: &Self) -> Result<Self, CadConstructionFailure> {
        self.0
            .add(&r.0)
            .map(|value| Self(Arc::new(value)))
            .map_err(|_| exact_failure(ExactConversionFailureKind::ExactConstruction, "sum", None))
    }
    pub(crate) fn sub(&self, r: &Self) -> Result<Self, CadConstructionFailure> {
        self.0
            .sub(&r.0)
            .map(|value| Self(Arc::new(value)))
            .map_err(|_| {
                exact_failure(
                    ExactConversionFailureKind::ExactConstruction,
                    "difference",
                    None,
                )
            })
    }
    pub(crate) fn div(&self, r: &Self) -> Result<Self, CadConstructionFailure> {
        self.0
            .div(&r.0)
            .map(|value| Self(Arc::new(value)))
            .map_err(|_| {
                exact_failure(
                    ExactConversionFailureKind::ExactConstruction,
                    "quotient",
                    None,
                )
            })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactVector3(pub [ExactScalar; 3]);
impl ExactVector3 {
    pub fn new(v: [f64; 3]) -> Result<Self, CadConstructionFailure> {
        let x = [
            ExactScalar::from_f64_exact_binary(v[0], "vector[0]")?,
            ExactScalar::from_f64_exact_binary(v[1], "vector[1]")?,
            ExactScalar::from_f64_exact_binary(v[2], "vector[2]")?,
        ];
        if x.iter().all(ExactScalar::is_zero) {
            return Err(exact_failure(
                ExactConversionFailureKind::ZeroDirection,
                "vector",
                None,
            ));
        }
        Ok(Self(x))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactPoint3(pub [ExactScalar; 3]);
impl ExactPoint3 {
    pub fn new(v: [f64; 3]) -> Result<Self, CadConstructionFailure> {
        Ok(Self([
            ExactScalar::from_f64_exact_binary(v[0], "point[0]")?,
            ExactScalar::from_f64_exact_binary(v[1], "point[1]")?,
            ExactScalar::from_f64_exact_binary(v[2], "point[2]")?,
        ]))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRigidPlacement {
    pub origin: ExactPoint3,
    pub quaternion: [ExactScalar; 4],
}
impl ExactRigidPlacement {
    pub fn new(origin: ExactPoint3, q: [f64; 4]) -> Result<Self, CadConstructionFailure> {
        let q = [
            ExactScalar::from_f64_exact_binary(q[0], "q.w")?,
            ExactScalar::from_f64_exact_binary(q[1], "q.x")?,
            ExactScalar::from_f64_exact_binary(q[2], "q.y")?,
            ExactScalar::from_f64_exact_binary(q[3], "q.z")?,
        ];
        let n = q[0]
            .mul(&q[0])?
            .add(&q[1].mul(&q[1])?)?
            .add(&q[2].mul(&q[2])?)?
            .add(&q[3].mul(&q[3])?)?;
        if n.compare_exact(&ExactScalar::new(1.)?)? != Ordering::Equal {
            return Err(exact_failure(
                ExactConversionFailureKind::NonUnitQuaternion,
                "placement.rotation",
                None,
            ));
        }
        Ok(Self {
            origin,
            quaternion: q,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CurveDeclaration {
    Line {
        origin: ExactPoint3,
        direction: ExactVector3,
        length: ExactScalar,
    },
    Polyline {
        points: Vec<ExactPoint3>,
        closed: bool,
    },
    CircularArc {
        center: ExactPoint3,
        x_axis: ExactVector3,
        normal: ExactVector3,
        radius: ExactScalar,
        start_angle: ExactScalar,
        end_angle: ExactScalar,
    },
    Nurbs {
        degree: u32,
        knots: Vec<ExactScalar>,
        control_points: Vec<ExactPoint3>,
        weights: Vec<ExactScalar>,
        parameter_domain: [ExactScalar; 2],
        closed: bool,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveDeclaration {
    Box {
        lengths: ExactVector3,
    },
    Cylinder {
        outer_radius: ExactScalar,
        inner_radius: Option<ExactScalar>,
        height: ExactScalar,
    },
    Cone {
        base_radius: ExactScalar,
        top_radius: ExactScalar,
        height: ExactScalar,
    },
    Sphere {
        radius: ExactScalar,
    },
    Ellipsoid {
        radii: ExactVector3,
    },
    Torus {
        major_radius: ExactScalar,
        minor_radius: ExactScalar,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileRequest {
    pub origin: ExactPoint3,
    pub x_axis: ExactVector3,
    pub normal: ExactVector3,
    pub corners: Vec<ProfileCorner>,
    pub segments: Vec<ProfileSegment>,
    pub corner_order: Vec<String>,
    pub segment_order: Vec<String>,
    pub closure: GeometryClosure,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileCorner {
    pub id: String,
    pub position: [ExactScalar; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileSegment {
    pub id: String,
    pub start_corner: String,
    pub end_corner: String,
    pub curve: ProfileSegmentCurve,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ProfileSegmentCurve {
    Line,
    CircularArc {
        center: [ExactScalar; 2],
        x_axis: [ExactScalar; 2],
        radius: ExactScalar,
        start_angle: ExactScalar,
        end_angle: ExactScalar,
    },
    Nurbs {
        degree: u32,
        knots: Vec<ExactScalar>,
        control_points: Vec<[ExactScalar; 2]>,
        weights: Vec<ExactScalar>,
        parameter_domain: [ExactScalar; 2],
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct SubentityRef {
    pub object: SourceIdentity,
    pub subentity: String,
    pub expected_kind: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterValueSample {
    pub parameter: ExactScalar,
    pub value: ExactScalar,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SweepFrameRule {
    ParallelTransport { initial_normal: ExactVector3 },
    Frenet,
    FixedUp { up_direction: ExactVector3 },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoftMode {
    Straight,
    Smooth,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LoftSection {
    pub slot: String,
    pub profile: OperationOutputRef,
    pub placement: ExactRigidPlacement,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanOperator {
    Union,
    Intersection,
    Difference,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanKind {
    Set,
    Solid,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BooleanRequest {
    pub kind: BooleanKind,
    pub operator: BooleanOperator,
}
#[derive(Clone, Debug, PartialEq)]
pub enum RevolveExtent {
    Full,
    FiniteAngle(ExactScalar),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoweringCondition {
    NonOrthonormalFrame,
    OpenProfile,
    DisconnectedProfile,
    OrderMismatch,
    SelfIntersection,
    ProfileHole,
    GeneralNurbsSegment,
    ImplicitizationFailed,
    FiniteRevolveAngle,
    SweepUnsupported,
    LoftUnsupported,
    FilletUnsupported,
    ChamferUnsupported,
    ImportedBooleanChild,
    UnconvertibleCoefficient,
    ZeroDenominator,
    DuplicateExponent,
    DuplicateTerm,
    DimensionMismatch,
    UnknownAtom,
    NonPositiveScale,
    UnsupportedAnalyticOperation,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ExternalGeometryRequest {
    pub asset_id: String,
    pub source_relative_path: String,
    pub unit_scale: ExactScalar,
    pub adapter: String,
    pub asset_schema: GeometryOutputExpectation,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ExplicitSolidVertex {
    pub id: String,
    pub position: ExactPoint3,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplicitSolidFace {
    pub id: String,
    pub vertex_order: Vec<String>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NurbsTrimRole {
    Outer,
    Hole,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NurbsTrimDirection {
    Forward,
    Reverse,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NurbsSurfaceOrientation {
    pub u_reversed: bool,
    pub v_reversed: bool,
    pub normal_reversed: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsSurfaceTrim {
    pub trim_id: String,
    pub role: NurbsTrimRole,
    pub direction: NurbsTrimDirection,
    pub degree: u32,
    pub knots: Vec<ExactScalar>,
    pub control_points: Vec<[ExactScalar; 2]>,
    pub weights: Vec<ExactScalar>,
    pub parameter_domain: [ExactScalar; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsSurfacePatchRequest {
    pub u_degree: u32,
    pub v_degree: u32,
    pub u_knots: Vec<ExactScalar>,
    pub v_knots: Vec<ExactScalar>,
    pub control_net: Vec<Vec<ExactPoint3>>,
    pub weight_net: Vec<Vec<ExactScalar>>,
    pub parameter_domain: [[ExactScalar; 2]; 2],
    pub orientation: NurbsSurfaceOrientation,
    pub trims: Vec<NurbsSurfaceTrim>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OperationRequest {
    Curve(CurveDeclaration),
    Primitive(PrimitiveDeclaration),
    Profile(ProfileRequest),
    Fillet {
        source: OperationOutputRef,
        targets: Vec<SubentityRef>,
        radius: ExactScalar,
    },
    Chamfer {
        source: OperationOutputRef,
        targets: Vec<SubentityRef>,
        distance: ExactScalar,
    },
    Transform {
        source: OperationOutputRef,
        placement: ExactRigidPlacement,
        scale: ExactScalar,
    },
    Extrude {
        profile: OperationOutputRef,
        direction: ExactVector3,
        distance: ExactScalar,
        symmetric: bool,
    },
    Revolve {
        profile: OperationOutputRef,
        axis_origin: ExactPoint3,
        axis_direction: ExactVector3,
        extent: RevolveExtent,
    },
    Sweep {
        profile: OperationOutputRef,
        spine: OperationOutputRef,
        frame_rule: SweepFrameRule,
        scale_samples: Vec<ParameterValueSample>,
        twist_samples: Vec<ParameterValueSample>,
    },
    Loft {
        sections: Vec<LoftSection>,
        section_order: Vec<String>,
        mode: LoftMode,
    },
    Boolean {
        request: BooleanRequest,
        left: OperationOutputRef,
        right: OperationOutputRef,
    },
    ExternalGeometry(ExternalGeometryRequest),
    ExplicitSolid {
        vertices: Vec<ExplicitSolidVertex>,
        faces: Vec<ExplicitSolidFace>,
    },
    NurbsSurfacePatch(NurbsSurfacePatchRequest),
}
impl OperationRequest {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Curve(_) => "Curve",
            Self::Primitive(_) => "Primitive",
            Self::Profile(_) => "Profile",
            Self::Fillet { .. } => "Fillet",
            Self::Chamfer { .. } => "Chamfer",
            Self::Transform { .. } => "Transform",
            Self::Extrude { .. } => "Extrude",
            Self::Revolve { .. } => "Revolve",
            Self::Sweep { .. } => "Sweep",
            Self::Loft { .. } => "Loft",
            Self::Boolean { .. } => "Boolean",
            Self::ExternalGeometry(_) => "ExternalGeometry",
            Self::ExplicitSolid { .. } => "ExplicitSolid",
            Self::NurbsSurfacePatch(_) => "NurbsSurfacePatch",
        }
    }
    pub fn dependencies(&self) -> Vec<(InputRole, OperationOutputRef, Option<u32>)> {
        match self {
            Self::Fillet { source, .. }
            | Self::Chamfer { source, .. }
            | Self::Transform { source, .. } => vec![(InputRole::Source, source.clone(), None)],
            Self::Extrude { profile, .. } | Self::Revolve { profile, .. } => {
                vec![(InputRole::Profile, profile.clone(), None)]
            }
            Self::Sweep { profile, spine, .. } => vec![
                (InputRole::Profile, profile.clone(), None),
                (InputRole::Spine, spine.clone(), None),
            ],
            Self::Loft { sections, .. } => sections
                .iter()
                .enumerate()
                .map(|(i, s)| (InputRole::Section, s.profile.clone(), Some(i as u32)))
                .collect(),
            Self::Boolean { left, right, .. } => vec![
                (InputRole::Left, left.clone(), None),
                (InputRole::Right, right.clone(), None),
            ],
            _ => vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ConstructionOperation {
    pub operation: OperationIdentity,
    pub source_object: SourceIdentity,
    pub expected_output: GeometryOutputExpectation,
    pub request: OperationRequest,
}
impl ConstructionOperation {
    pub fn output_identity(&self) -> OperationOutputIdentity {
        OperationOutputIdentity {
            operation: self.operation.clone(),
            slot: OutputSlot::Primary,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ConstructionRoot {
    GeometryInstance {
        instance: SourceIdentity,
        output: OperationOutputRef,
        placement: ExactRigidPlacement,
    },
    PhysicalBody {
        body: SourceIdentity,
        output: OperationOutputRef,
        ordinal: u32,
    },
    PhysicalDomain {
        domain: SourceIdentity,
        bodies: Vec<SourceIdentity>,
        composition: BodyComposition,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct ConstructionRequest {
    pub source_revision: SourceRevision,
    pub precision: GeometryAccuracyPolicy,
    pub operations: Vec<ConstructionOperation>,
    pub roots: Vec<ConstructionRoot>,
    pub provenance: Provenance,
    pub identity_ledger: IdentityLedger,
}
#[derive(Debug, PartialEq)]
pub enum CurveOrSurfaceRepresentation {
    Curve(CurveRepresentation),
    Surface(SurfaceRepresentation),
}
#[derive(Debug, PartialEq)]
pub enum ConstructionOutput {
    Region(AlgebraicRegion),
    AnalyticRepresentation(CurveOrSurfaceRepresentation),
    ImportedRegion(Box<CertifiedRegion>),
}
#[derive(Debug, PartialEq)]
pub struct ConstructionResult {
    graph: ConstructionGraph,
    outputs: Vec<(OperationOutputIdentity, ConstructionOutput)>,
    roots: Vec<ConstructionRoot>,
    composition: Vec<BodyComposition>,
    provenance: Provenance,
    identity_ledger: IdentityLedger,
    source_revision: SourceRevision,
}
impl ConstructionResult {
    pub(crate) fn new(
        graph: ConstructionGraph,
        outputs: Vec<(OperationOutputIdentity, ConstructionOutput)>,
        roots: Vec<ConstructionRoot>,
        composition: Vec<BodyComposition>,
        provenance: Provenance,
        identity_ledger: IdentityLedger,
        source_revision: SourceRevision,
    ) -> Self {
        Self {
            graph,
            outputs,
            roots,
            composition,
            provenance,
            identity_ledger,
            source_revision,
        }
    }
    pub fn construction_graph(&self) -> &ConstructionGraph {
        &self.graph
    }
    pub fn outputs(&self) -> &[(OperationOutputIdentity, ConstructionOutput)] {
        &self.outputs
    }
    pub fn into_outputs(self) -> Vec<(OperationOutputIdentity, ConstructionOutput)> {
        self.outputs
    }
    pub fn output(&self, identity: &OperationOutputIdentity) -> Option<&ConstructionOutput> {
        self.outputs
            .iter()
            .find(|(key, _)| key == identity)
            .map(|(_, output)| output)
    }
    pub fn region(&self, identity: &OperationOutputIdentity) -> Option<&AlgebraicRegion> {
        match self.output(identity)? {
            ConstructionOutput::Region(region) => Some(region),
            _ => None,
        }
    }
    pub fn analytic(
        &self,
        identity: &OperationOutputIdentity,
    ) -> Option<&CurveOrSurfaceRepresentation> {
        match self.output(identity)? {
            ConstructionOutput::AnalyticRepresentation(representation) => Some(representation),
            _ => None,
        }
    }
    pub fn imported(&self, identity: &OperationOutputIdentity) -> Option<&CertifiedRegion> {
        match self.output(identity)? {
            ConstructionOutput::ImportedRegion(region) => Some(region.as_ref()),
            _ => None,
        }
    }
    pub fn roots(&self) -> &[ConstructionRoot] {
        &self.roots
    }
    pub fn body_composition(&self) -> &[BodyComposition] {
        &self.composition
    }
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
    pub fn identity_ledger(&self) -> &IdentityLedger {
        &self.identity_ledger
    }
    pub fn source_revision(&self) -> &SourceRevision {
        &self.source_revision
    }
}
#[derive(Debug, PartialEq)]
pub enum CadConstructionLowerFailure {
    Geometry(GeometryFailure),
    Import(Box<RegionImportFailure>),
    Mesh(MeshFailure),
    Nurbs(String),
}
#[derive(Debug, PartialEq)]
pub enum CadConstructionFailure {
    ExactConversion(ExactConversionFailure),
    Graph(crate::ConstructionGraphFailure),
    Identity(crate::IdentityFailure),
    SourceRevisionMismatch {
        request: String,
        artifact: String,
    },
    OutputSchemaMismatch {
        operation: OperationIdentity,
        expected: GeometryOutputExpectation,
        actual: GeometryOutputExpectation,
    },
    PayloadInvalid {
        operation: OperationIdentity,
        field: &'static str,
    },
    Lowering {
        operation: OperationIdentity,
        condition: LoweringCondition,
        field: Option<&'static str>,
        operand_provenance: Option<OperationIdentity>,
        lower: Option<Box<CadConstructionLowerFailure>>,
    },
    ExternalAdapter {
        operation: OperationIdentity,
        asset_id: String,
        source_relative_path: String,
        adapter: String,
        asset_schema: GeometryOutputExpectation,
        condition: String,
    },
}
impl From<crate::ConstructionGraphFailure> for CadConstructionFailure {
    fn from(v: crate::ConstructionGraphFailure) -> Self {
        Self::Graph(v)
    }
}
impl From<crate::IdentityFailure> for CadConstructionFailure {
    fn from(v: crate::IdentityFailure) -> Self {
        Self::Identity(v)
    }
}

pub(crate) fn expected(
    category: GeometryOutputCategory,
    closure: GeometryClosure,
) -> GeometryOutputExpectation {
    GeometryOutputExpectation {
        category,
        ambient_dimension: 3,
        manifold_dimension: match category {
            GeometryOutputCategory::Curve => 1,
            GeometryOutputCategory::Profile | GeometryOutputCategory::Sheet => 2,
            GeometryOutputCategory::Solid => 3,
        },
        closure,
    }
}
pub(crate) fn valid_nurbs(
    knots: &[ExactScalar],
    degree: u32,
    controls: usize,
    weights: &[ExactScalar],
    domain: &[ExactScalar; 2],
) -> Result<(), &'static str> {
    if degree == 0
        || controls <= degree as usize
        || knots.len() != controls + degree as usize + 1
        || weights.len() != controls
    {
        return Err("nurbs.cardinality");
    }
    for p in knots.windows(2) {
        if p[0].compare_exact(&p[1]).map_err(|_| "nurbs.compare")? == Ordering::Greater {
            return Err("nurbs.knots");
        }
    }
    let zero = ExactScalar::new(0.).map_err(|_| "nurbs.zero")?;
    if weights
        .iter()
        .any(|w| w.compare_exact(&zero) != Ok(Ordering::Greater))
    {
        return Err("nurbs.weights");
    }
    if domain[0]
        .compare_exact(&domain[1])
        .map_err(|_| "nurbs.domain")?
        != Ordering::Less
        || domain[0]
            .compare_exact(&knots[degree as usize])
            .map_err(|_| "nurbs.domain")?
            != Ordering::Equal
        || domain[1]
            .compare_exact(&knots[knots.len() - degree as usize - 1])
            .map_err(|_| "nurbs.domain")?
            != Ordering::Equal
    {
        return Err("nurbs.domain");
    }
    Ok(())
}
