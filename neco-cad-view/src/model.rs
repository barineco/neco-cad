use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use neco_bigint::{BigintError, Dyadic};
use neco_brep::{AnalyticBrep, SourceRevision};
use neco_cad::{AdaptiveTessellation, SourceIdentity};
use neco_mesh::ValidatedSurfaceMesh;
use neco_nurbs::NurbsTypes;
use neco_tessellation::{
    AdaptivePrecision, AdaptivePrecisionWitness, TessellationConsumer, TessellationRequest,
    TessellationResult,
};
pub const CAD_VIEW_MODEL_SCHEMA_VERSION: u32 = 2;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CadViewModelVersion {
    schema_version: u32,
}
impl CadViewModelVersion {
    pub const V2: Self = Self {
        schema_version: CAD_VIEW_MODEL_SCHEMA_VERSION,
    };
    pub fn schema_version(self) -> u32 {
        self.schema_version
    }
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CadElementIdentity {
    Vertex(String),
    Edge(String),
    Face(String),
    Trim(String),
    Shell(String),
    Incidence(String),
    ControlPoint {
        surface_representation_id: String,
        row: usize,
        column: usize,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewModelIdentity {
    source_revision: SourceRevision,
    mesh_identity: Option<SourceIdentity>,
    model_version: CadViewModelVersion,
}
impl ViewModelIdentity {
    #[cfg(test)]
    pub(crate) fn test() -> Self {
        Self {
            source_revision: SourceRevision::new(String::from("source")).unwrap(),
            mesh_identity: None,
            model_version: CadViewModelVersion::V2,
        }
    }
    pub fn source_revision(&self) -> &SourceRevision {
        &self.source_revision
    }
    pub fn mesh_identity(&self) -> Option<&SourceIdentity> {
        self.mesh_identity.as_ref()
    }
    pub fn model_version(&self) -> CadViewModelVersion {
        self.model_version
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    x: f64,
    y: f64,
}
impl Point2 {
    pub fn new(x: f64, y: f64) -> Result<Self, CadViewFailure> {
        if x.is_finite() && y.is_finite() {
            Ok(Self { x, y })
        } else {
            Err(CadViewFailure::NonFiniteCoordinate)
        }
    }
    pub fn x(self) -> f64 {
        self.x
    }
    pub fn y(self) -> f64 {
        self.y
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point3 {
    x: f64,
    y: f64,
    z: f64,
}
impl Point3 {
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, CadViewFailure> {
        if x.is_finite() && y.is_finite() && z.is_finite() {
            Ok(Self { x, y, z })
        } else {
            Err(CadViewFailure::NonFiniteCoordinate)
        }
    }
    pub fn x(self) -> f64 {
        self.x
    }
    pub fn y(self) -> f64 {
        self.y
    }
    pub fn z(self) -> f64 {
        self.z
    }
    pub(crate) fn array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}
impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, CadViewFailure> {
        if x.is_finite() && y.is_finite() && z.is_finite() {
            Ok(Self { x, y, z })
        } else {
            Err(CadViewFailure::NonFiniteCoordinate)
        }
    }
    pub fn x(self) -> f64 {
        self.x
    }
    pub fn y(self) -> f64 {
        self.y
    }
    pub fn z(self) -> f64 {
        self.z
    }
    pub(crate) fn array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    Orthographic,
    Perspective { vertical_field_of_view_radians: f64 },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    eye: Point3,
    target: Point3,
    up: Vector3,
    projection: Projection,
    vertical_span: f64,
    perspective_scale: f64,
}
impl Camera {
    pub fn orthographic(
        eye: Point3,
        target: Point3,
        up: Vector3,
        vertical_span: f64,
    ) -> Result<Self, CadViewFailure> {
        Self::new(
            eye,
            target,
            up,
            Projection::Orthographic,
            vertical_span,
            0.0,
        )
    }
    pub fn perspective(
        eye: Point3,
        target: Point3,
        up: Vector3,
        vertical_field_of_view_radians: f64,
    ) -> Result<Self, CadViewFailure> {
        if !vertical_field_of_view_radians.is_finite()
            || vertical_field_of_view_radians <= 0.
            || vertical_field_of_view_radians >= core::f64::consts::PI
        {
            return Err(CadViewFailure::InvalidPerspectiveFieldOfView);
        }
        Self::new(
            eye,
            target,
            up,
            Projection::Perspective {
                vertical_field_of_view_radians,
            },
            0.,
            libm::tan(vertical_field_of_view_radians / 2.0),
        )
    }
    fn new(
        eye: Point3,
        target: Point3,
        up: Vector3,
        projection: Projection,
        vertical_span: f64,
        perspective_scale: f64,
    ) -> Result<Self, CadViewFailure> {
        if matches!(projection, Projection::Orthographic)
            && (!vertical_span.is_finite() || vertical_span <= 0.)
        {
            return Err(CadViewFailure::InvalidVerticalSpan);
        }
        let f = sub(target.array(), eye.array());
        if norm(f) == 0. || norm(up.array()) == 0. || norm(cross(f, up.array())) == 0. {
            return Err(CadViewFailure::DegenerateCamera);
        }
        Ok(Self {
            eye,
            target,
            up,
            projection,
            vertical_span,
            perspective_scale,
        })
    }
    pub fn eye(self) -> Point3 {
        self.eye
    }
    pub fn target(self) -> Point3 {
        self.target
    }
    pub fn up(self) -> Vector3 {
        self.up
    }
    pub fn projection(self) -> Projection {
        self.projection
    }
    pub fn vertical_span(self) -> Option<f64> {
        matches!(self.projection, Projection::Orthographic).then_some(self.vertical_span)
    }
    pub(crate) fn perspective_scale(self) -> f64 {
        self.perspective_scale
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    width_pixels: u32,
    height_pixels: u32,
}
impl Viewport {
    pub fn new(width_pixels: u32, height_pixels: u32) -> Result<Self, CadViewFailure> {
        if width_pixels == 0 || height_pixels == 0 {
            Err(CadViewFailure::EmptyViewport)
        } else {
            Ok(Self {
                width_pixels,
                height_pixels,
            })
        }
    }
    pub fn width_pixels(self) -> u32 {
        self.width_pixels
    }
    pub fn height_pixels(self) -> u32 {
        self.height_pixels
    }
    pub fn aspect_ratio(self) -> f64 {
        self.width_pixels as f64 / self.height_pixels as f64
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    Hidden,
    Visible,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplaySettings {
    faces: Visibility,
    edges: Visibility,
    trims: Visibility,
    vertices: Visibility,
    control_points: Visibility,
    backface_culling: bool,
}
impl DisplaySettings {
    pub fn new(
        faces: Visibility,
        edges: Visibility,
        trims: Visibility,
        vertices: Visibility,
        control_points: Visibility,
        backface_culling: bool,
    ) -> Self {
        Self {
            faces,
            edges,
            trims,
            vertices,
            control_points,
            backface_culling,
        }
    }
    pub fn faces(self) -> Visibility {
        self.faces
    }
    pub fn edges(self) -> Visibility {
        self.edges
    }
    pub fn trims(self) -> Visibility {
        self.trims
    }
    pub fn vertices(self) -> Visibility {
        self.vertices
    }
    pub fn control_points(self) -> Visibility {
        self.control_points
    }
    pub fn backface_culling(self) -> bool {
        self.backface_culling
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    elements: Vec<CadElementIdentity>,
}
impl Selection {
    pub fn new(elements: Vec<CadElementIdentity>) -> Result<Self, CadViewFailure> {
        let mut s = BTreeSet::new();
        for x in &elements {
            if !s.insert(x.clone()) {
                return Err(CadViewFailure::DuplicateSelectionIdentity {
                    identity: x.clone(),
                });
            }
        }
        Ok(Self { elements })
    }
    pub fn elements(&self) -> &[CadElementIdentity] {
        &self.elements
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RequestedPrecision {
    maximum_chord_error: f64,
    maximum_surface_error: Option<f64>,
}
impl RequestedPrecision {
    fn basic(request: TessellationRequest) -> Self {
        Self {
            maximum_chord_error: request.maximum_chord_error(),
            maximum_surface_error: None,
        }
    }
    fn adaptive(request: AdaptivePrecision) -> Self {
        Self {
            maximum_chord_error: request.maximum_chord_error(),
            maximum_surface_error: Some(request.maximum_surface_error()),
        }
    }
    pub fn maximum_chord_error(self) -> f64 {
        self.maximum_chord_error
    }
    pub fn maximum_surface_error(self) -> Option<f64> {
        self.maximum_surface_error
    }
}
#[derive(Debug, PartialEq)]
pub struct CertifiedPrecision {
    established_maximum_chord_error: f64,
    established_maximum_surface_error: Dyadic,
}
impl CertifiedPrecision {
    pub fn established_maximum_chord_error(&self) -> f64 {
        self.established_maximum_chord_error
    }
    pub fn established_maximum_surface_error(&self) -> &Dyadic {
        &self.established_maximum_surface_error
    }
}
#[derive(Debug, PartialEq)]
pub enum AchievedPrecision {
    Certified(CertifiedPrecision),
    NotEstablished { requested: RequestedPrecision },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrespondenceClass {
    Face,
    FaceTriangle,
    Edge,
    Trim,
    Vertex,
    ControlPoint,
}
#[derive(Debug, Eq, PartialEq)]
pub enum CadViewFailure {
    NonFiniteCoordinate,
    GeometryFailure {
        error: neco_geometry::GeometryFailure,
    },
    DegenerateCamera,
    InvalidVerticalSpan,
    InvalidPerspectiveFieldOfView,
    EmptyViewport,
    SourceRevisionMismatch {
        brep_revision: String,
        tessellation_revision: String,
    },
    SourceIdentityMismatch {
        expected: String,
        observed: String,
    },
    PrecisionConsumerMismatch {
        requested: TessellationConsumer,
        witnessed: TessellationConsumer,
    },
    RequestedPrecisionMismatch,
    PrecisionWitnessCopy {
        error: BigintError,
    },
    DuplicateSelectionIdentity {
        identity: CadElementIdentity,
    },
    UnknownSelectionIdentity {
        identity: CadElementIdentity,
    },
    UnknownCorrespondenceIdentity {
        class: CorrespondenceClass,
        identity: CadElementIdentity,
    },
    DuplicateCorrespondenceIdentity {
        class: CorrespondenceClass,
        identity: CadElementIdentity,
    },
    MissingCorrespondence {
        class: CorrespondenceClass,
        identity: Option<CadElementIdentity>,
    },
    InvalidCorrespondence {
        class: CorrespondenceClass,
        identity: CadElementIdentity,
    },
    UnsupportedCadViewModelVersion {
        schema_version: u32,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ViewGeometry {
    pub faces: Vec<(CadElementIdentity, [usize; 3])>,
    pub curves: Vec<(CadElementIdentity, Vec<[f64; 3]>)>,
    pub points: Vec<(CadElementIdentity, [f64; 3])>,
}
#[derive(Debug, PartialEq)]
pub struct CadViewModel {
    version: CadViewModelVersion,
    identity: ViewModelIdentity,
    camera: Camera,
    viewport: Viewport,
    display: DisplaySettings,
    selection: Selection,
    requested_precision: RequestedPrecision,
    achieved_precision: AchievedPrecision,
    mesh: ValidatedSurfaceMesh,
    elements: Vec<CadElementIdentity>,
    geometry: ViewGeometry,
}
impl CadViewModel {
    pub fn version(&self) -> CadViewModelVersion {
        self.version
    }
    pub fn identity(&self) -> &ViewModelIdentity {
        &self.identity
    }
    pub fn camera(&self) -> Camera {
        self.camera
    }
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
    pub fn display(&self) -> DisplaySettings {
        self.display
    }
    pub fn selection(&self) -> &Selection {
        &self.selection
    }
    pub fn requested_precision(&self) -> RequestedPrecision {
        self.requested_precision
    }
    pub fn achieved_precision(&self) -> &AchievedPrecision {
        &self.achieved_precision
    }
    pub fn mesh(&self) -> &ValidatedSurfaceMesh {
        &self.mesh
    }
    pub fn elements(&self) -> &[CadElementIdentity] {
        &self.elements
    }
    pub(crate) fn geometry(&self) -> &ViewGeometry {
        &self.geometry
    }
}
pub enum CadViewTessellation<'a> {
    Basic {
        result: &'a TessellationResult,
        request: TessellationRequest,
    },
    Adaptive {
        result: &'a AdaptiveTessellation,
        request: AdaptivePrecision,
        expected_source_identity: &'a SourceIdentity,
    },
}
pub struct CadViewModelRequest<'a> {
    pub brep: &'a AnalyticBrep,
    pub tessellation: CadViewTessellation<'a>,
    pub camera: Camera,
    pub viewport: Viewport,
    pub display: DisplaySettings,
    pub selection: Selection,
}
pub fn derive_cad_view_model(r: CadViewModelRequest<'_>) -> Result<CadViewModel, CadViewFailure> {
    let (
        validated_mesh,
        face_correspondence,
        edge_samples,
        trim_samples,
        vertex_positions,
        control_points,
        mesh_identity,
        requested,
        achieved_precision,
    ) = match r.tessellation {
        CadViewTessellation::Basic { result, request } => {
            if r.brep.witnesses().source_revision() != result.source_brep_revision() {
                return Err(CadViewFailure::SourceRevisionMismatch {
                    brep_revision: String::from(r.brep.witnesses().source_revision().as_str()),
                    tessellation_revision: String::from(result.source_brep_revision().as_str()),
                });
            }
            let witness = result.precision_witness();
            if request.maximum_chord_error() != witness.requested_maximum_chord_error() {
                return Err(CadViewFailure::RequestedPrecisionMismatch);
            }
            let requested = RequestedPrecision::basic(request);
            (
                result.validated_surface_mesh(),
                result.face_triangle_correspondence(),
                result.edge_samples(),
                result.trim_samples(),
                result.vertex_positions(),
                result.control_points(),
                None,
                requested,
                AchievedPrecision::NotEstablished { requested },
            )
        }
        CadViewTessellation::Adaptive {
            result,
            request,
            expected_source_identity,
        } => {
            if r.brep.witnesses().source_revision() != result.source_revision() {
                return Err(CadViewFailure::SourceRevisionMismatch {
                    brep_revision: String::from(r.brep.witnesses().source_revision().as_str()),
                    tessellation_revision: String::from(result.source_revision().as_str()),
                });
            }
            if expected_source_identity != result.identity() {
                return Err(CadViewFailure::SourceIdentityMismatch {
                    expected: String::from(expected_source_identity.as_str()),
                    observed: String::from(result.identity().as_str()),
                });
            }
            let witness: &AdaptivePrecisionWitness = result.precision_witness();
            if request.consumer() != witness.consumer() {
                return Err(CadViewFailure::PrecisionConsumerMismatch {
                    requested: request.consumer(),
                    witnessed: witness.consumer(),
                });
            }
            if request.maximum_chord_error() != witness.requested_maximum_chord_error()
                || request.maximum_surface_error() != witness.requested_maximum_surface_error()
            {
                return Err(CadViewFailure::RequestedPrecisionMismatch);
            }
            let established_surface = witness
                .established_maximum_surface_error()
                .try_clone()
                .map_err(|error| CadViewFailure::PrecisionWitnessCopy { error })?;
            let requested = RequestedPrecision::adaptive(request);
            (
                result.validated_surface_mesh(),
                result.face_triangle_correspondence(),
                result.edge_samples(),
                result.trim_samples(),
                result.vertex_positions(),
                result.control_points(),
                Some(result.identity().clone()),
                requested,
                AchievedPrecision::Certified(CertifiedPrecision {
                    established_maximum_chord_error: witness.established_maximum_chord_error(),
                    established_maximum_surface_error: established_surface,
                }),
            )
        }
    };
    let mut set = BTreeSet::new();
    for x in r.brep.vertices() {
        set.insert(CadElementIdentity::Vertex(String::from(x.id())));
    }
    for x in r.brep.edges() {
        set.insert(CadElementIdentity::Edge(String::from(x.id())));
    }
    for x in r.brep.faces() {
        set.insert(CadElementIdentity::Face(String::from(x.id())));
    }
    for x in r.brep.trims() {
        set.insert(CadElementIdentity::Trim(String::from(x.id())));
    }
    for x in r.brep.shells() {
        set.insert(CadElementIdentity::Shell(String::from(x.id())));
    }
    for x in r.brep.incidences() {
        set.insert(CadElementIdentity::Incidence(String::from(x.id())));
    }
    let expected_control_points = canonical_control_points(r.brep);
    for identity in &expected_control_points {
        set.insert(identity.clone());
    }
    for x in r.selection.elements() {
        if !set.contains(x) {
            return Err(CadViewFailure::UnknownSelectionIdentity {
                identity: x.clone(),
            });
        }
    }
    let mut geometry = ViewGeometry {
        faces: Vec::new(),
        curves: Vec::new(),
        points: Vec::new(),
    };
    let mesh = validated_mesh.mesh();
    let expected_faces: BTreeSet<_> = r
        .brep
        .faces()
        .iter()
        .map(|x| CadElementIdentity::Face(String::from(x.id())))
        .collect();
    let mut seen_faces = BTreeSet::new();
    let mut covered_triangles = BTreeSet::new();
    for x in face_correspondence {
        let identity = CadElementIdentity::Face(String::from(x.face_id()));
        if !expected_faces.contains(&identity) {
            return Err(CadViewFailure::UnknownCorrespondenceIdentity {
                class: CorrespondenceClass::Face,
                identity,
            });
        }
        if !seen_faces.insert(identity.clone()) {
            return Err(CadViewFailure::DuplicateCorrespondenceIdentity {
                class: CorrespondenceClass::Face,
                identity,
            });
        }
        if x.triangle_indices().is_empty() {
            return Err(CadViewFailure::InvalidCorrespondence {
                class: CorrespondenceClass::Face,
                identity,
            });
        }
        for i in x.triangle_indices() {
            let t =
                *mesh
                    .triangles()
                    .get(*i)
                    .ok_or_else(|| CadViewFailure::InvalidCorrespondence {
                        class: CorrespondenceClass::FaceTriangle,
                        identity: identity.clone(),
                    })?;
            if !covered_triangles.insert(*i) {
                return Err(CadViewFailure::InvalidCorrespondence {
                    class: CorrespondenceClass::FaceTriangle,
                    identity,
                });
            }
            geometry.faces.push((identity.clone(), t));
        }
    }
    if let Some(identity) = expected_faces.difference(&seen_faces).next() {
        return Err(CadViewFailure::MissingCorrespondence {
            class: CorrespondenceClass::Face,
            identity: Some(identity.clone()),
        });
    }
    if covered_triangles.len() != mesh.triangles().len() {
        return Err(CadViewFailure::MissingCorrespondence {
            class: CorrespondenceClass::FaceTriangle,
            identity: None,
        });
    }
    append_curves(
        &mut geometry,
        edge_samples.iter().map(|x| {
            (
                CadElementIdentity::Edge(String::from(x.identity())),
                x.points(),
            )
        }),
        r.brep
            .edges()
            .iter()
            .map(|x| CadElementIdentity::Edge(String::from(x.id()))),
        CorrespondenceClass::Edge,
    )?;
    append_curves(
        &mut geometry,
        trim_samples.iter().map(|x| {
            (
                CadElementIdentity::Trim(String::from(x.identity())),
                x.points(),
            )
        }),
        r.brep
            .trims()
            .iter()
            .map(|x| CadElementIdentity::Trim(String::from(x.id()))),
        CorrespondenceClass::Trim,
    )?;
    append_points(
        &mut geometry,
        vertex_positions.iter().map(|x| {
            (
                CadElementIdentity::Vertex(String::from(x.vertex_id())),
                x.position(),
            )
        }),
        r.brep
            .vertices()
            .iter()
            .map(|x| CadElementIdentity::Vertex(String::from(x.id()))),
        CorrespondenceClass::Vertex,
    )?;
    append_points(
        &mut geometry,
        control_points.iter().map(|x| {
            (
                CadElementIdentity::ControlPoint {
                    surface_representation_id: String::from(x.surface_representation_id()),
                    row: x.row(),
                    column: x.column(),
                },
                x.position(),
            )
        }),
        expected_control_points.into_iter(),
        CorrespondenceClass::ControlPoint,
    )?;
    Ok(CadViewModel {
        version: CadViewModelVersion::V2,
        identity: ViewModelIdentity {
            source_revision: r.brep.witnesses().source_revision().clone(),
            mesh_identity,
            model_version: CadViewModelVersion::V2,
        },
        camera: r.camera,
        viewport: r.viewport,
        display: r.display,
        selection: r.selection,
        requested_precision: requested,
        achieved_precision,
        mesh: validated_mesh.clone(),
        elements: set.into_iter().collect(),
        geometry,
    })
}

fn canonical_control_points(brep: &AnalyticBrep) -> BTreeSet<CadElementIdentity> {
    let mut result = BTreeSet::new();
    for face in brep.faces() {
        let Some(surface) = brep
            .surface_representations()
            .iter()
            .find(|surface| surface.id() == face.surface_representation_id())
        else {
            continue;
        };
        let net = match surface.value() {
            NurbsTypes::PolynomialSurface(surface) => surface.control_net(),
            NurbsTypes::RationalSurface(surface) => surface.control_net(),
            NurbsTypes::PolynomialCurve(_) | NurbsTypes::RationalCurve(_) => continue,
        };
        for (row, values) in net.iter().enumerate() {
            for column in 0..values.len() {
                result.insert(CadElementIdentity::ControlPoint {
                    surface_representation_id: String::from(surface.id().as_str()),
                    row,
                    column,
                });
            }
        }
    }
    result
}

fn append_curves<'a>(
    geometry: &mut ViewGeometry,
    entries: impl Iterator<Item = (CadElementIdentity, &'a [[f64; 3]])>,
    expected: impl Iterator<Item = CadElementIdentity>,
    class: CorrespondenceClass,
) -> Result<(), CadViewFailure> {
    let expected: BTreeSet<_> = expected.collect();
    let mut seen = BTreeSet::new();
    for (identity, points) in entries {
        if !expected.contains(&identity) {
            return Err(CadViewFailure::UnknownCorrespondenceIdentity { class, identity });
        }
        if !seen.insert(identity.clone()) {
            return Err(CadViewFailure::DuplicateCorrespondenceIdentity { class, identity });
        }
        if points.len() < 2 || points.iter().flatten().any(|value| !value.is_finite()) {
            return Err(CadViewFailure::InvalidCorrespondence { class, identity });
        }
        geometry.curves.push((identity, points.to_vec()));
    }
    if let Some(identity) = expected.difference(&seen).next() {
        return Err(CadViewFailure::MissingCorrespondence {
            class,
            identity: Some(identity.clone()),
        });
    }
    Ok(())
}

fn append_points(
    geometry: &mut ViewGeometry,
    entries: impl Iterator<Item = (CadElementIdentity, [f64; 3])>,
    expected: impl Iterator<Item = CadElementIdentity>,
    class: CorrespondenceClass,
) -> Result<(), CadViewFailure> {
    let expected: BTreeSet<_> = expected.collect();
    let mut seen = BTreeSet::new();
    for (identity, point) in entries {
        if !expected.contains(&identity) {
            return Err(CadViewFailure::UnknownCorrespondenceIdentity { class, identity });
        }
        if !seen.insert(identity.clone()) {
            return Err(CadViewFailure::DuplicateCorrespondenceIdentity { class, identity });
        }
        if point.iter().any(|value| !value.is_finite()) {
            return Err(CadViewFailure::InvalidCorrespondence { class, identity });
        }
        geometry.points.push((identity, point));
    }
    if let Some(identity) = expected.difference(&seen).next() {
        return Err(CadViewFailure::MissingCorrespondence {
            class,
            identity: Some(identity.clone()),
        });
    }
    Ok(())
}

pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn sqrt(value: f64) -> f64 {
    libm::sqrt(value)
}
pub(crate) fn norm(a: [f64; 3]) -> f64 {
    sqrt(dot(a, a))
}
pub(crate) fn normalize(a: [f64; 3]) -> [f64; 3] {
    let n = norm(a);
    [a[0] / n, a[1] / n, a[2] / n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_reports_actual_missing_edge_identity() {
        let mut geometry = ViewGeometry {
            faces: Vec::new(),
            curves: Vec::new(),
            points: Vec::new(),
        };
        let edge = CadElementIdentity::Edge(String::from("edge-a"));
        assert_eq!(
            append_curves(
                &mut geometry,
                core::iter::empty(),
                core::iter::once(edge.clone()),
                CorrespondenceClass::Edge,
            ),
            Err(CadViewFailure::MissingCorrespondence {
                class: CorrespondenceClass::Edge,
                identity: Some(edge),
            })
        );
    }

    #[test]
    fn fixture_separates_unknown_from_duplicate_correspondence() {
        let mut geometry = ViewGeometry {
            faces: Vec::new(),
            curves: Vec::new(),
            points: Vec::new(),
        };
        let known = CadElementIdentity::Vertex(String::from("vertex-a"));
        let unknown = CadElementIdentity::Vertex(String::from("vertex-b"));
        assert_eq!(
            append_points(
                &mut geometry,
                core::iter::once((unknown.clone(), [0.0, 0.0, 0.0])),
                core::iter::once(known),
                CorrespondenceClass::Vertex,
            ),
            Err(CadViewFailure::UnknownCorrespondenceIdentity {
                class: CorrespondenceClass::Vertex,
                identity: unknown,
            })
        );
    }
}
