use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq)]
pub struct TriangleMesh2d {
    vertices: Vec<[f64; 2]>,
    triangles: Vec<[usize; 3]>,
}

impl TriangleMesh2d {
    pub fn new(vertices: Vec<[f64; 2]>, triangles: Vec<[usize; 3]>) -> Result<Self, MeshFailure> {
        validate_2d_mesh(&vertices, &triangles)?;
        Ok(Self {
            vertices,
            triangles,
        })
    }

    pub fn vertices(&self) -> &[[f64; 2]] {
        &self.vertices
    }

    pub fn triangles(&self) -> &[[usize; 3]] {
        &self.triangles
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceMesh {
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
    attributes: Vec<Vec<u8>>,
}

impl SurfaceMesh {
    pub fn new(
        vertices: Vec<[f64; 3]>,
        triangles: Vec<[usize; 3]>,
        attributes: Vec<Vec<u8>>,
    ) -> Result<Self, MeshFailure> {
        if !attributes.is_empty() && attributes.len() != triangles.len() {
            return Err(MeshFailure::AttributeCount {
                attribute_count: attributes.len(),
                triangle_count: triangles.len(),
            });
        }
        Ok(Self {
            vertices,
            triangles,
            attributes,
        })
    }

    pub fn vertices(&self) -> &[[f64; 3]] {
        &self.vertices
    }

    pub fn triangles(&self) -> &[[usize; 3]] {
        &self.triangles
    }

    pub fn attributes(&self) -> &[Vec<u8>] {
        &self.attributes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceMeshValidationWitness {
    vertex_count: usize,
    triangle_count: usize,
}

impl SurfaceMeshValidationWitness {
    fn new(vertex_count: usize, triangle_count: usize) -> Self {
        Self {
            vertex_count,
            triangle_count,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub fn triangle_count(&self) -> usize {
        self.triangle_count
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedSurfaceMesh {
    mesh: SurfaceMesh,
    validation_witness: SurfaceMeshValidationWitness,
}

impl ValidatedSurfaceMesh {
    pub fn mesh(&self) -> &SurfaceMesh {
        &self.mesh
    }

    pub fn validation_witness(&self) -> SurfaceMeshValidationWitness {
        self.validation_witness
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClosureWitness {
    edge_count: usize,
}

impl ClosureWitness {
    fn new(edge_count: usize) -> Self {
        Self { edge_count }
    }

    pub fn from_edge_count(edge_count: usize) -> Self {
        Self::new(edge_count)
    }

    pub fn edge_count(&self) -> usize {
        self.edge_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrientationWitness {
    edge_count: usize,
}

impl OrientationWitness {
    fn new(edge_count: usize) -> Self {
        Self { edge_count }
    }

    pub fn from_edge_count(edge_count: usize) -> Self {
        Self::new(edge_count)
    }

    pub fn edge_count(&self) -> usize {
        self.edge_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifoldWitness {
    edge_count: usize,
}

impl ManifoldWitness {
    fn new(edge_count: usize) -> Self {
        Self { edge_count }
    }

    pub fn from_edge_count(edge_count: usize) -> Self {
        Self::new(edge_count)
    }

    pub fn edge_count(&self) -> usize {
        self.edge_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeshSource {
    identity: String,
    revision: String,
}

impl MeshSource {
    pub fn new(identity: String, revision: String) -> Result<Self, MeshFailure> {
        if identity.is_empty() {
            return Err(MeshFailure::EmptySourceIdentity);
        }
        if revision.is_empty() {
            return Err(MeshFailure::EmptySourceRevision);
        }
        Ok(Self { identity, revision })
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AllowedSharedIndexIntersectionKind {
    Empty,
    SharedVertex,
    SharedEdgeSegment,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MeshPairIntersectionKind {
    Empty,
    SharedVertex,
    SharedEdgeSegment,
    NonPermittedCrossing,
    CoplanarOverlap,
    DistinctIndexCoincidentContact,
    DuplicateFace,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeshTrianglePairWitness {
    first_triangle: usize,
    second_triangle: usize,
    shared_index_count: usize,
    intersection_kind: AllowedSharedIndexIntersectionKind,
}

impl MeshTrianglePairWitness {
    pub fn first_triangle(&self) -> usize {
        self.first_triangle
    }

    pub fn second_triangle(&self) -> usize {
        self.second_triangle
    }

    pub fn shared_index_count(&self) -> usize {
        self.shared_index_count
    }

    pub fn intersection_kind(&self) -> AllowedSharedIndexIntersectionKind {
        self.intersection_kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeshSelfIntersectionWitness {
    first_triangle: usize,
    second_triangle: usize,
    kind: MeshPairIntersectionKind,
}

impl MeshSelfIntersectionWitness {
    pub fn first_triangle(&self) -> usize {
        self.first_triangle
    }

    pub fn second_triangle(&self) -> usize {
        self.second_triangle
    }

    pub fn kind(&self) -> MeshPairIntersectionKind {
        self.kind
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClosedOrientedSurfaceMesh {
    mesh: ValidatedSurfaceMesh,
    source: MeshSource,
    closure_witness: ClosureWitness,
    orientation_witness: OrientationWitness,
    manifold_witness: ManifoldWitness,
    triangle_pairs: Vec<MeshTrianglePairWitness>,
}

impl ClosedOrientedSurfaceMesh {
    pub fn mesh(&self) -> &ValidatedSurfaceMesh {
        &self.mesh
    }

    pub fn source(&self) -> &MeshSource {
        &self.source
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

    pub fn examined_triangle_pairs(&self) -> &[MeshTrianglePairWitness] {
        &self.triangle_pairs
    }

    pub fn examined_pair_count(&self) -> usize {
        self.triangle_pairs.len()
    }

    pub fn examined_pair_set(&self) -> Vec<(usize, usize)> {
        self.triangle_pairs
            .iter()
            .map(|pair| (pair.first_triangle, pair.second_triangle))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeshFailure {
    AttributeCount {
        attribute_count: usize,
        triangle_count: usize,
    },
    NonFiniteVertex {
        vertex_index: usize,
    },
    TriangleIndexOutOfBounds {
        triangle_index: usize,
        vertex_index: usize,
    },
    DegenerateTriangle {
        triangle_index: usize,
    },
    OpenEdge {
        first_vertex: usize,
        second_vertex: usize,
    },
    NonManifoldEdge {
        first_vertex: usize,
        second_vertex: usize,
        incidence_count: usize,
    },
    OrientationMismatch {
        first_vertex: usize,
        second_vertex: usize,
    },
    EmptySourceIdentity,
    EmptySourceRevision,
    SelfIntersection(MeshSelfIntersectionWitness),
}

pub fn validate_surface_mesh(mesh: SurfaceMesh) -> Result<ValidatedSurfaceMesh, MeshFailure> {
    for (vertex_index, vertex) in mesh.vertices.iter().enumerate() {
        if !vertex.iter().all(|coordinate| coordinate.is_finite()) {
            return Err(MeshFailure::NonFiniteVertex { vertex_index });
        }
    }
    for (triangle_index, triangle) in mesh.triangles.iter().enumerate() {
        validate_triangle_indices(mesh.vertices.len(), triangle_index, triangle)?;
        let [first, second, third] = *triangle;
        if squared_double_area(
            mesh.vertices[first],
            mesh.vertices[second],
            mesh.vertices[third],
        ) == 0.0
        {
            return Err(MeshFailure::DegenerateTriangle { triangle_index });
        }
    }
    Ok(ValidatedSurfaceMesh {
        validation_witness: SurfaceMeshValidationWitness::new(
            mesh.vertices.len(),
            mesh.triangles.len(),
        ),
        mesh,
    })
}

pub fn validate_closed_oriented(
    mesh: ValidatedSurfaceMesh,
    source: MeshSource,
) -> Result<ClosedOrientedSurfaceMesh, MeshFailure> {
    let mut edges: BTreeMap<(usize, usize), Vec<(usize, usize)>> = BTreeMap::new();
    for triangle in mesh.mesh.triangles() {
        for (from, to) in triangle_edges(*triangle) {
            let key = canonical_edge(from, to);
            edges.entry(key).or_default().push((from, to));
        }
    }
    for ((first_vertex, second_vertex), incidences) in &edges {
        match incidences.as_slice() {
            [_] => {
                return Err(MeshFailure::OpenEdge {
                    first_vertex: *first_vertex,
                    second_vertex: *second_vertex,
                });
            }
            [(first_from, first_to), (second_from, second_to)] => {
                if first_from == second_from || first_to == second_to {
                    return Err(MeshFailure::OrientationMismatch {
                        first_vertex: *first_vertex,
                        second_vertex: *second_vertex,
                    });
                }
            }
            values => {
                return Err(MeshFailure::NonManifoldEdge {
                    first_vertex: *first_vertex,
                    second_vertex: *second_vertex,
                    incidence_count: values.len(),
                });
            }
        }
    }
    let triangle_pairs = examine_triangle_pairs(mesh.mesh.vertices(), mesh.mesh.triangles())?;
    let edge_count = edges.len();
    Ok(ClosedOrientedSurfaceMesh {
        mesh,
        source,
        closure_witness: ClosureWitness::new(edge_count),
        orientation_witness: OrientationWitness::new(edge_count),
        manifold_witness: ManifoldWitness::new(edge_count),
        triangle_pairs,
    })
}

fn examine_triangle_pairs(
    vertices: &[[f64; 3]],
    triangles: &[[usize; 3]],
) -> Result<Vec<MeshTrianglePairWitness>, MeshFailure> {
    let mut pairs = Vec::new();
    for first in 0..triangles.len() {
        for second in (first + 1)..triangles.len() {
            pairs.push(classify_triangle_pair(
                vertices,
                first,
                second,
                triangles[first],
                triangles[second],
            )?);
        }
    }
    Ok(pairs)
}

fn classify_triangle_pair(
    vertices: &[[f64; 3]],
    first: usize,
    second: usize,
    first_triangle: [usize; 3],
    second_triangle: [usize; 3],
) -> Result<MeshTrianglePairWitness, MeshFailure> {
    let shared = shared_indices(first_triangle, second_triangle);
    if shared.len() == 3 {
        return Err(MeshFailure::SelfIntersection(MeshSelfIntersectionWitness {
            first_triangle: first,
            second_triangle: second,
            kind: MeshPairIntersectionKind::DuplicateFace,
        }));
    }
    if has_distinct_index_coincident_contact(vertices, first_triangle, second_triangle) {
        return Err(MeshFailure::SelfIntersection(MeshSelfIntersectionWitness {
            first_triangle: first,
            second_triangle: second,
            kind: MeshPairIntersectionKind::DistinctIndexCoincidentContact,
        }));
    }
    let first_points = triangle_points(vertices, first_triangle);
    let second_points = triangle_points(vertices, second_triangle);
    let geometric_intersection = if shared.len() == 2 {
        let first_normal = triangle_normal(first_points);
        let second_normal = triangle_normal(second_points);
        let crossed = cross(first_normal, second_normal);
        let crossed_length = dot(crossed, crossed);
        if crossed_length > 0.0 && !is_uncertain_parallel(first_normal, second_normal) {
            PairGeometry::Segment(vertices[shared[0]], vertices[shared[1]])
        } else {
            let second_offsets = [
                plane_offset(first_normal, first_points[0], second_points[0]),
                plane_offset(first_normal, first_points[0], second_points[1]),
                plane_offset(first_normal, first_points[0], second_points[2]),
            ];
            let tolerance = coplanar_offset_tolerance(first_points, second_points, first_normal);
            if second_offsets.iter().all(|value| value.abs() <= tolerance) {
                coplanar_intersection(first_points, second_points)
            } else {
                PairGeometry::Segment(vertices[shared[0]], vertices[shared[1]])
            }
        }
    } else {
        triangle_intersection(first_points, second_points, !shared.is_empty())
    };
    let intersection = match geometric_intersection {
        PairGeometry::Indeterminate => {
            return Err(MeshFailure::SelfIntersection(MeshSelfIntersectionWitness {
                first_triangle: first,
                second_triangle: second,
                kind: MeshPairIntersectionKind::Indeterminate,
            }));
        }
        PairGeometry::CoplanarOverlap => {
            return Err(MeshFailure::SelfIntersection(MeshSelfIntersectionWitness {
                first_triangle: first,
                second_triangle: second,
                kind: MeshPairIntersectionKind::CoplanarOverlap,
            }));
        }
        PairGeometry::Crossing => {
            return Err(MeshFailure::SelfIntersection(MeshSelfIntersectionWitness {
                first_triangle: first,
                second_triangle: second,
                kind: MeshPairIntersectionKind::NonPermittedCrossing,
            }));
        }
        PairGeometry::Empty => PairGeometry::Empty,
        PairGeometry::Point(point) => PairGeometry::Point(point),
        PairGeometry::Segment(start, end) => PairGeometry::Segment(start, end),
    };
    let allowed = match shared.len() {
        0 => match intersection {
            PairGeometry::Empty => AllowedSharedIndexIntersectionKind::Empty,
            _ => {
                return Err(self_intersection(
                    first,
                    second,
                    MeshPairIntersectionKind::NonPermittedCrossing,
                ));
            }
        },
        1 => {
            if matches_shared_vertex(&intersection, vertices[shared[0]]) {
                AllowedSharedIndexIntersectionKind::SharedVertex
            } else {
                return Err(self_intersection(
                    first,
                    second,
                    MeshPairIntersectionKind::NonPermittedCrossing,
                ));
            }
        }
        2 => {
            if segment_matches_shared_edge(&intersection, vertices[shared[0]], vertices[shared[1]])
            {
                AllowedSharedIndexIntersectionKind::SharedEdgeSegment
            } else {
                return Err(self_intersection(
                    first,
                    second,
                    MeshPairIntersectionKind::NonPermittedCrossing,
                ));
            }
        }
        _ => {
            return Err(self_intersection(
                first,
                second,
                MeshPairIntersectionKind::DuplicateFace,
            ));
        }
    };
    Ok(MeshTrianglePairWitness {
        first_triangle: first,
        second_triangle: second,
        shared_index_count: shared.len(),
        intersection_kind: allowed,
    })
}

fn self_intersection(first: usize, second: usize, kind: MeshPairIntersectionKind) -> MeshFailure {
    MeshFailure::SelfIntersection(MeshSelfIntersectionWitness {
        first_triangle: first,
        second_triangle: second,
        kind,
    })
}

fn matches_shared_vertex(intersection: &PairGeometry, vertex: [f64; 3]) -> bool {
    match intersection {
        PairGeometry::Empty => true,
        PairGeometry::Point(point) => point_eq(*point, vertex),
        _ => false,
    }
}

fn segment_matches_shared_edge(
    intersection: &PairGeometry,
    start: [f64; 3],
    end: [f64; 3],
) -> bool {
    match intersection {
        PairGeometry::Empty => true,
        PairGeometry::Point(point) => point_eq(*point, start) || point_eq(*point, end),
        PairGeometry::Segment(left, right) => {
            (point_eq(*left, start) && point_eq(*right, end))
                || (point_eq(*left, end) && point_eq(*right, start))
                || point_on_segment(*left, start, end) && point_on_segment(*right, start, end)
        }
        PairGeometry::Crossing | PairGeometry::CoplanarOverlap | PairGeometry::Indeterminate => {
            false
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PairGeometry {
    Empty,
    Point([f64; 3]),
    Segment([f64; 3], [f64; 3]),
    Crossing,
    CoplanarOverlap,
    Indeterminate,
}

fn triangle_intersection(
    first: [[f64; 3]; 3],
    second: [[f64; 3]; 3],
    connected: bool,
) -> PairGeometry {
    let first_normal = triangle_normal(first);
    let second_normal = triangle_normal(second);
    if !is_finite_vec(first_normal) || !is_finite_vec(second_normal) {
        return PairGeometry::Indeterminate;
    }
    let first_dots = [
        plane_offset(second_normal, second[0], first[0]),
        plane_offset(second_normal, second[0], first[1]),
        plane_offset(second_normal, second[0], first[2]),
    ];
    let second_dots = [
        plane_offset(first_normal, first[0], second[0]),
        plane_offset(first_normal, first[0], second[1]),
        plane_offset(first_normal, first[0], second[2]),
    ];
    if first_dots.iter().any(|value| !value.is_finite())
        || second_dots.iter().any(|value| !value.is_finite())
    {
        return PairGeometry::Indeterminate;
    }
    let first_tolerance = if connected {
        coplanar_offset_tolerance(first, second, second_normal)
    } else {
        0.0
    };
    let second_tolerance = if connected {
        coplanar_offset_tolerance(first, second, first_normal)
    } else {
        0.0
    };
    if first_dots
        .iter()
        .all(|value| value.abs() <= first_tolerance)
        || second_dots
            .iter()
            .all(|value| value.abs() <= second_tolerance)
    {
        return coplanar_intersection(first, second);
    }
    if same_strict_sign(first_dots) || same_strict_sign(second_dots) {
        return PairGeometry::Empty;
    }
    if is_uncertain_parallel(first_normal, second_normal) {
        return PairGeometry::Indeterminate;
    }
    let mut hits = Vec::new();
    collect_segment_triangle_hits(&mut hits, first[0], first[1], second);
    collect_segment_triangle_hits(&mut hits, first[1], first[2], second);
    collect_segment_triangle_hits(&mut hits, first[2], first[0], second);
    collect_segment_triangle_hits(&mut hits, second[0], second[1], first);
    collect_segment_triangle_hits(&mut hits, second[1], second[2], first);
    collect_segment_triangle_hits(&mut hits, second[2], second[0], first);
    if hits
        .iter()
        .any(|hit| matches!(hit, PairGeometry::Indeterminate))
    {
        return PairGeometry::Indeterminate;
    }
    if hits
        .iter()
        .any(|hit| matches!(hit, PairGeometry::CoplanarOverlap))
    {
        return PairGeometry::CoplanarOverlap;
    }
    let mut points = Vec::new();
    for hit in hits {
        match hit {
            PairGeometry::Point(point) => push_unique_point(&mut points, point),
            PairGeometry::Segment(start, end) => {
                push_unique_point(&mut points, start);
                push_unique_point(&mut points, end);
            }
            PairGeometry::Crossing => return PairGeometry::Crossing,
            PairGeometry::Empty => {}
            PairGeometry::CoplanarOverlap => return PairGeometry::CoplanarOverlap,
            PairGeometry::Indeterminate => return PairGeometry::Indeterminate,
        }
    }
    match points.as_slice() {
        [] => PairGeometry::Empty,
        [point] => PairGeometry::Point(*point),
        [start, end] => PairGeometry::Segment(*start, *end),
        _ => PairGeometry::Crossing,
    }
}

fn collect_segment_triangle_hits(
    hits: &mut Vec<PairGeometry>,
    start: [f64; 3],
    end: [f64; 3],
    triangle: [[f64; 3]; 3],
) {
    hits.push(segment_triangle_intersection(start, end, triangle));
}

fn segment_triangle_intersection(
    start: [f64; 3],
    end: [f64; 3],
    triangle: [[f64; 3]; 3],
) -> PairGeometry {
    let normal = triangle_normal(triangle);
    let start_offset = plane_offset(normal, triangle[0], start);
    let end_offset = plane_offset(normal, triangle[0], end);
    if !start_offset.is_finite() || !end_offset.is_finite() {
        return PairGeometry::Indeterminate;
    }
    let tolerance = segment_plane_offset_tolerance(normal, start, end, triangle);
    let start_on_plane = start_offset.abs() <= tolerance;
    let end_on_plane = end_offset.abs() <= tolerance;
    if start_on_plane && end_on_plane {
        return coplanar_segment_in_triangle(start, end, triangle);
    }
    if start_on_plane {
        return point_plane_contact(start, triangle);
    }
    if end_on_plane {
        return point_plane_contact(end, triangle);
    }
    if start_offset * end_offset > 0.0 {
        return PairGeometry::Empty;
    }
    let denominator = start_offset - end_offset;
    if denominator == 0.0 {
        return PairGeometry::Indeterminate;
    }
    let parameter = start_offset / denominator;
    if !parameter.is_finite() {
        return PairGeometry::Indeterminate;
    }
    let point = lerp(start, end, parameter);
    point_plane_contact(point, triangle)
}

fn point_plane_contact(point: [f64; 3], triangle: [[f64; 3]; 3]) -> PairGeometry {
    match point_in_triangle(point, triangle) {
        PointInTriangle::Outside => PairGeometry::Empty,
        PointInTriangle::Vertex | PointInTriangle::Edge | PointInTriangle::Interior => {
            PairGeometry::Point(point)
        }
        PointInTriangle::Indeterminate => PairGeometry::Indeterminate,
    }
}

fn segment_plane_offset_tolerance(
    normal: [f64; 3],
    start: [f64; 3],
    end: [f64; 3],
    triangle: [[f64; 3]; 3],
) -> f64 {
    let segment = sub(end, start);
    let mut edge_scale = dot(segment, segment);
    for (a, b) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
        let edge = sub(triangle[a], triangle[b]);
        edge_scale = edge_scale.max(dot(edge, edge));
    }
    sqrt(dot(normal, normal)) * sqrt(edge_scale) * 1e-9
}

fn coplanar_segment_in_triangle(
    start: [f64; 3],
    end: [f64; 3],
    triangle: [[f64; 3]; 3],
) -> PairGeometry {
    let start_class = point_in_triangle(start, triangle);
    let end_class = point_in_triangle(end, triangle);
    if matches!(start_class, PointInTriangle::Indeterminate)
        || matches!(end_class, PointInTriangle::Indeterminate)
    {
        return PairGeometry::Indeterminate;
    }
    let mut points = Vec::new();
    if !matches!(start_class, PointInTriangle::Outside) {
        push_unique_point(&mut points, start);
    }
    if !matches!(end_class, PointInTriangle::Outside) {
        push_unique_point(&mut points, end);
    }
    for edge in triangle_edges_points(triangle) {
        match segment_segment_intersection(start, end, edge[0], edge[1]) {
            SegmentOverlap::Indeterminate => return PairGeometry::Indeterminate,
            SegmentOverlap::Point(point) => push_unique_point(&mut points, point),
            SegmentOverlap::Segment(left, right) => {
                push_unique_point(&mut points, left);
                push_unique_point(&mut points, right);
            }
            SegmentOverlap::Empty => {}
        }
    }
    if matches!(start_class, PointInTriangle::Interior)
        || matches!(end_class, PointInTriangle::Interior)
    {
        return PairGeometry::CoplanarOverlap;
    }
    if points.len() >= 2 {
        let mid = lerp(points[0], points[1], 0.5);
        if matches!(point_in_triangle(mid, triangle), PointInTriangle::Interior) {
            return PairGeometry::CoplanarOverlap;
        }
    }
    match points.as_slice() {
        [] => PairGeometry::Empty,
        [point] => PairGeometry::Point(*point),
        [left, right] => PairGeometry::Segment(*left, *right),
        _ => PairGeometry::CoplanarOverlap,
    }
}

enum SegmentOverlap {
    Empty,
    Point([f64; 3]),
    Segment([f64; 3], [f64; 3]),
    Indeterminate,
}

fn triangle_edges_points(triangle: [[f64; 3]; 3]) -> [[[f64; 3]; 2]; 3] {
    [
        [triangle[0], triangle[1]],
        [triangle[1], triangle[2]],
        [triangle[2], triangle[0]],
    ]
}

fn segment_segment_intersection(
    a0: [f64; 3],
    a1: [f64; 3],
    b0: [f64; 3],
    b1: [f64; 3],
) -> SegmentOverlap {
    let da = sub(a1, a0);
    let db = sub(b1, b0);
    let dc = sub(b0, a0);
    let normal = cross(da, db);
    let normal_length = dot(normal, normal);
    if !normal_length.is_finite() {
        return SegmentOverlap::Indeterminate;
    }
    if normal_length == 0.0 {
        return colinear_segment_overlap(a0, a1, b0, b1);
    }
    if dot(normal, dc) != 0.0 {
        return SegmentOverlap::Empty;
    }
    let s = dot(cross(dc, db), normal) / normal_length;
    let t = dot(cross(dc, da), normal) / normal_length;
    if !s.is_finite() || !t.is_finite() {
        return SegmentOverlap::Indeterminate;
    }
    if (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t) {
        if s <= 1e-12 {
            return SegmentOverlap::Point(a0);
        }
        if s >= 1.0 - 1e-12 {
            return SegmentOverlap::Point(a1);
        }
        if t <= 1e-12 {
            return SegmentOverlap::Point(b0);
        }
        if t >= 1.0 - 1e-12 {
            return SegmentOverlap::Point(b1);
        }
        SegmentOverlap::Point(lerp(a0, a1, s))
    } else {
        SegmentOverlap::Empty
    }
}

fn colinear_segment_overlap(
    a0: [f64; 3],
    a1: [f64; 3],
    b0: [f64; 3],
    b1: [f64; 3],
) -> SegmentOverlap {
    if !point_on_line(b0, a0, a1) || !point_on_line(b1, a0, a1) {
        return SegmentOverlap::Empty;
    }
    let mut points = Vec::new();
    if point_on_segment(b0, a0, a1) {
        push_unique_point(&mut points, b0);
    }
    if point_on_segment(b1, a0, a1) {
        push_unique_point(&mut points, b1);
    }
    if point_on_segment(a0, b0, b1) {
        push_unique_point(&mut points, a0);
    }
    if point_on_segment(a1, b0, b1) {
        push_unique_point(&mut points, a1);
    }
    match points.as_slice() {
        [] => SegmentOverlap::Empty,
        [point] => SegmentOverlap::Point(*point),
        [left, right] => SegmentOverlap::Segment(*left, *right),
        _ => SegmentOverlap::Indeterminate,
    }
}

fn point_on_line(point: [f64; 3], start: [f64; 3], end: [f64; 3]) -> bool {
    let direction = sub(end, start);
    let offset = sub(point, start);
    let crossed = cross(direction, offset);
    dot(crossed, crossed) == 0.0
}

fn coplanar_intersection(first: [[f64; 3]; 3], second: [[f64; 3]; 3]) -> PairGeometry {
    let mut interior_overlap = false;
    let mut points = Vec::new();
    for point in first {
        match point_in_triangle(point, second) {
            PointInTriangle::Interior => interior_overlap = true,
            PointInTriangle::Vertex | PointInTriangle::Edge => {
                push_unique_point(&mut points, point)
            }
            PointInTriangle::Outside => {}
            PointInTriangle::Indeterminate => return PairGeometry::Indeterminate,
        }
    }
    for point in second {
        match point_in_triangle(point, first) {
            PointInTriangle::Interior => interior_overlap = true,
            PointInTriangle::Vertex | PointInTriangle::Edge => {
                push_unique_point(&mut points, point)
            }
            PointInTriangle::Outside => {}
            PointInTriangle::Indeterminate => return PairGeometry::Indeterminate,
        }
    }
    if interior_overlap {
        return PairGeometry::CoplanarOverlap;
    }
    match points.as_slice() {
        [] => PairGeometry::Empty,
        [point] => PairGeometry::Point(*point),
        [start, end] => PairGeometry::Segment(*start, *end),
        _ => PairGeometry::CoplanarOverlap,
    }
}

enum PointInTriangle {
    Outside,
    Vertex,
    Edge,
    Interior,
    Indeterminate,
}

fn point_in_triangle(point: [f64; 3], triangle: [[f64; 3]; 3]) -> PointInTriangle {
    let [a, b, c] = triangle;
    let v0 = sub(c, a);
    let v1 = sub(b, a);
    let v2 = sub(point, a);
    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);
    let denominator = dot00 * dot11 - dot01 * dot01;
    if denominator == 0.0 || !denominator.is_finite() {
        return PointInTriangle::Indeterminate;
    }
    let u = (dot11 * dot02 - dot01 * dot12) / denominator;
    let v = (dot00 * dot12 - dot01 * dot02) / denominator;
    if !u.is_finite() || !v.is_finite() {
        return PointInTriangle::Indeterminate;
    }
    let w = 1.0 - u - v;
    let weights = [w, v, u];
    if weights.iter().any(|weight| *weight < 0.0) {
        return PointInTriangle::Outside;
    }
    let zero_count = weights.iter().filter(|weight| **weight == 0.0).count();
    match zero_count {
        2 => PointInTriangle::Vertex,
        1 => PointInTriangle::Edge,
        0 => PointInTriangle::Interior,
        _ => PointInTriangle::Vertex,
    }
}

fn shared_indices(first: [usize; 3], second: [usize; 3]) -> Vec<usize> {
    first
        .iter()
        .copied()
        .filter(|index| second.contains(index))
        .collect()
}

fn has_distinct_index_coincident_contact(
    vertices: &[[f64; 3]],
    first: [usize; 3],
    second: [usize; 3],
) -> bool {
    for left in first {
        for right in second {
            if left != right && point_eq(vertices[left], vertices[right]) {
                return true;
            }
        }
    }
    false
}

fn triangle_points(vertices: &[[f64; 3]], triangle: [usize; 3]) -> [[f64; 3]; 3] {
    [
        vertices[triangle[0]],
        vertices[triangle[1]],
        vertices[triangle[2]],
    ]
}

fn triangle_normal(triangle: [[f64; 3]; 3]) -> [f64; 3] {
    cross(sub(triangle[1], triangle[0]), sub(triangle[2], triangle[0]))
}

fn plane_offset(normal: [f64; 3], origin: [f64; 3], point: [f64; 3]) -> f64 {
    dot(normal, sub(point, origin))
}

fn same_strict_sign(values: [f64; 3]) -> bool {
    values.iter().all(|value| *value > 0.0) || values.iter().all(|value| *value < 0.0)
}

fn coplanar_offset_tolerance(first: [[f64; 3]; 3], second: [[f64; 3]; 3], normal: [f64; 3]) -> f64 {
    let mut edge_scale = 0.0_f64;
    for triangle in [first, second] {
        for (a, b) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
            let edge = sub(triangle[a], triangle[b]);
            edge_scale = edge_scale.max(dot(edge, edge));
        }
    }
    sqrt(dot(normal, normal)) * sqrt(edge_scale) * 1e-9
}

fn is_uncertain_parallel(first: [f64; 3], second: [f64; 3]) -> bool {
    let crossed = cross(first, second);
    let length = dot(crossed, crossed);
    let scale = dot(first, first) * dot(second, second);
    length > 0.0 && scale > 0.0 && length < scale * 1e-24
}

fn push_unique_point(points: &mut Vec<[f64; 3]>, point: [f64; 3]) {
    if !points.iter().any(|existing| point_eq(*existing, point)) {
        points.push(point);
    }
}

fn point_on_segment(point: [f64; 3], start: [f64; 3], end: [f64; 3]) -> bool {
    let direction = sub(end, start);
    let offset = sub(point, start);
    let length = dot(direction, direction);
    if length == 0.0 {
        return point_eq(point, start);
    }
    let parameter = dot(offset, direction) / length;
    if !(0.0..=1.0).contains(&parameter) {
        return false;
    }
    point_eq(point, lerp(start, end, parameter))
}

fn lerp(start: [f64; 3], end: [f64; 3], parameter: f64) -> [f64; 3] {
    if parameter == 0.0 {
        return start;
    }
    if parameter == 1.0 {
        return end;
    }
    add(start, scale(sub(end, start), parameter))
}

fn point_eq(left: [f64; 3], right: [f64; 3]) -> bool {
    left == right
}

fn is_finite_vec(value: [f64; 3]) -> bool {
    value.iter().all(|coordinate| coordinate.is_finite())
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn scale(value: [f64; 3], scalar: f64) -> [f64; 3] {
    [value[0] * scalar, value[1] * scalar, value[2] * scalar]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn sqrt(value: f64) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    let mut estimate = if value >= 1.0 { value } else { 1.0 };
    for _ in 0..24 {
        estimate = (estimate + value / estimate) / 2.0;
    }
    estimate
}

fn validate_2d_mesh(vertices: &[[f64; 2]], triangles: &[[usize; 3]]) -> Result<(), MeshFailure> {
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        if !vertex.iter().all(|coordinate| coordinate.is_finite()) {
            return Err(MeshFailure::NonFiniteVertex { vertex_index });
        }
    }
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        validate_triangle_indices(vertices.len(), triangle_index, triangle)?;
        let [first, second, third] = *triangle;
        let [ax, ay] = vertices[first];
        let [bx, by] = vertices[second];
        let [cx, cy] = vertices[third];
        if (bx - ax) * (cy - ay) - (by - ay) * (cx - ax) == 0.0 {
            return Err(MeshFailure::DegenerateTriangle { triangle_index });
        }
    }
    Ok(())
}

fn validate_triangle_indices(
    vertex_count: usize,
    triangle_index: usize,
    triangle: &[usize; 3],
) -> Result<(), MeshFailure> {
    for vertex_index in triangle {
        if *vertex_index >= vertex_count {
            return Err(MeshFailure::TriangleIndexOutOfBounds {
                triangle_index,
                vertex_index: *vertex_index,
            });
        }
    }
    Ok(())
}

fn squared_double_area(first: [f64; 3], second: [f64; 3], third: [f64; 3]) -> f64 {
    let area = triangle_normal([first, second, third]);
    dot(area, area)
}

fn triangle_edges(triangle: [usize; 3]) -> [(usize, usize); 3] {
    let [first, second, third] = triangle;
    [(first, second), (second, third), (third, first)]
}

fn canonical_edge(first: usize, second: usize) -> (usize, usize) {
    if first < second {
        (first, second)
    } else {
        (second, first)
    }
}
