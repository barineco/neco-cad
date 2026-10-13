use neco_mesh::{
    validate_closed_oriented, validate_surface_mesh, AllowedSharedIndexIntersectionKind,
    MeshFailure, MeshPairIntersectionKind, MeshSource, SurfaceMesh, TriangleMesh2d,
};

fn source(identity: &str) -> MeshSource {
    MeshSource::new(String::from(identity), String::from("revision-1")).unwrap()
}

fn closed_mesh(
    identity: &str,
    vertices: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
) -> neco_mesh::ClosedOrientedSurfaceMesh {
    let mesh = SurfaceMesh::new(vertices, triangles, Vec::new()).unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    validate_closed_oriented(validated, source(identity)).unwrap()
}

fn tetrahedron_vertices() -> Vec<[f64; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ]
}

fn tetrahedron_triangles() -> Vec<[usize; 3]> {
    vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]]
}

fn octahedron_vertices() -> Vec<[f64; 3]> {
    vec![
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ]
}

fn octahedron_triangles() -> Vec<[usize; 3]> {
    vec![
        [0, 2, 4],
        [2, 1, 4],
        [1, 3, 4],
        [3, 0, 4],
        [0, 3, 5],
        [3, 1, 5],
        [1, 2, 5],
        [2, 0, 5],
    ]
}

fn cube_vertices() -> Vec<[f64; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ]
}

fn cube_triangles() -> Vec<[usize; 3]> {
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
    ]
}

fn concave_vertices() -> Vec<[f64; 3]> {
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
    ]
}

fn concave_triangles() -> Vec<[usize; 3]> {
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
    ]
}

fn shared_index_count(first: [usize; 3], second: [usize; 3]) -> usize {
    first.iter().filter(|index| second.contains(index)).count()
}

fn allowed_kind(shared: usize) -> AllowedSharedIndexIntersectionKind {
    match shared {
        0 => AllowedSharedIndexIntersectionKind::Empty,
        1 => AllowedSharedIndexIntersectionKind::SharedVertex,
        2 => AllowedSharedIndexIntersectionKind::SharedEdgeSegment,
        _ => panic!("valid observer pairs share at most two indices"),
    }
}

fn observe_pairs(identity: &str, vertices: Vec<[f64; 3]>, triangles: Vec<[usize; 3]>) {
    let expected_pairs: Vec<(usize, usize)> = (0..triangles.len())
        .flat_map(|first| ((first + 1)..triangles.len()).map(move |second| (first, second)))
        .collect();
    let closed = closed_mesh(identity, vertices, triangles.clone());
    assert_eq!(closed.source().identity(), identity);
    assert_eq!(closed.source().revision(), "revision-1");
    assert_eq!(closed.examined_pair_count(), expected_pairs.len());
    assert_eq!(closed.examined_pair_set(), expected_pairs);
    for pair in closed.examined_triangle_pairs() {
        let shared = shared_index_count(
            triangles[pair.first_triangle()],
            triangles[pair.second_triangle()],
        );
        assert_eq!(pair.shared_index_count(), shared);
        assert_eq!(pair.intersection_kind(), allowed_kind(shared));
    }
}

fn tetrahedron_mesh() -> SurfaceMesh {
    SurfaceMesh::new(tetrahedron_vertices(), tetrahedron_triangles(), Vec::new()).unwrap()
}

#[test]
fn validates_a_closed_oriented_tetrahedron() {
    let validated = validate_surface_mesh(tetrahedron_mesh()).unwrap();
    let closed = validate_closed_oriented(validated, source("tetrahedron")).unwrap();
    assert_eq!(closed.closure_witness().edge_count(), 6);
    assert_eq!(closed.orientation_witness().edge_count(), 6);
    assert_eq!(closed.manifold_witness().edge_count(), 6);
}

#[test]
fn observes_normal_shape_witnesses_and_open_invalid_witness() {
    let tetrahedron = closed_mesh(
        "tetrahedron",
        tetrahedron_vertices(),
        tetrahedron_triangles(),
    );
    assert_eq!(tetrahedron.closure_witness().edge_count(), 6);
    assert_eq!(tetrahedron.orientation_witness().edge_count(), 6);
    assert_eq!(tetrahedron.manifold_witness().edge_count(), 6);
    let octahedron = closed_mesh("octahedron", octahedron_vertices(), octahedron_triangles());
    assert_eq!(octahedron.closure_witness().edge_count(), 12);
    assert_eq!(octahedron.orientation_witness().edge_count(), 12);
    assert_eq!(octahedron.manifold_witness().edge_count(), 12);
    let rational_octahedron = closed_mesh(
        "rational-octahedron",
        vec![
            [0.0, 0.0, 1.0],
            [0.5999999999999996, 0.7999999999999989, 0.0],
            [-0.7999999999999989, 0.5999999999999996, 0.0],
            [-0.5999999999999996, -0.7999999999999989, 0.0],
            [0.7999999999999989, -0.5999999999999996, 0.0],
            [0.0, 0.0, -1.0],
        ],
        vec![
            [0, 1, 2],
            [0, 2, 3],
            [0, 3, 4],
            [0, 4, 1],
            [5, 2, 1],
            [5, 3, 2],
            [5, 4, 3],
            [5, 1, 4],
        ],
    );
    assert_eq!(rational_octahedron.closure_witness().edge_count(), 12);
    assert_eq!(rational_octahedron.orientation_witness().edge_count(), 12);
    assert_eq!(rational_octahedron.manifold_witness().edge_count(), 12);
    let cube = closed_mesh("cube", cube_vertices(), cube_triangles());
    assert_eq!(cube.closure_witness().edge_count(), 18);
    assert_eq!(cube.orientation_witness().edge_count(), 18);
    assert_eq!(cube.manifold_witness().edge_count(), 18);
    let concave = closed_mesh("concave", concave_vertices(), concave_triangles());
    assert_eq!(concave.closure_witness().edge_count(), 30);
    assert_eq!(concave.orientation_witness().edge_count(), 30);
    assert_eq!(concave.manifold_witness().edge_count(), 30);
    let open = SurfaceMesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        validate_closed_oriented(validate_surface_mesh(open).unwrap(), source("open")),
        Err(MeshFailure::OpenEdge {
            first_vertex: 0,
            second_vertex: 1,
        })
    );
}

#[test]
fn rejects_orientation_mismatch() {
    let mesh = SurfaceMesh::new(
        tetrahedron_vertices(),
        vec![[0, 1, 2], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    assert_eq!(
        validate_closed_oriented(validated, source("orientation")),
        Err(MeshFailure::OrientationMismatch {
            first_vertex: 0,
            second_vertex: 1,
        })
    );
}

#[test]
fn rejects_non_manifold_edge() {
    let mesh = SurfaceMesh::new(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ],
        vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    assert_eq!(
        validate_closed_oriented(validated, source("non-manifold")),
        Err(MeshFailure::NonManifoldEdge {
            first_vertex: 0,
            second_vertex: 1,
            incidence_count: 3,
        })
    );
}

#[test]
fn observes_tetrahedron_triangle_pairs() {
    observe_pairs(
        "tetrahedron",
        tetrahedron_vertices(),
        tetrahedron_triangles(),
    );
}

#[test]
fn observes_octahedron_triangle_pairs() {
    observe_pairs("octahedron", octahedron_vertices(), octahedron_triangles());
}

#[test]
fn observes_triangulated_cube_triangle_pairs() {
    observe_pairs("cube", cube_vertices(), cube_triangles());
}

#[test]
fn observes_triangulated_concave_polyhedron_triangle_pairs() {
    observe_pairs("concave", concave_vertices(), concave_triangles());
}

#[test]
fn rejects_non_permitted_triangle_pair_crossing() {
    let mesh = SurfaceMesh::new(
        vec![
            [1.0, 1.0, 1.0],
            [1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, -1.0, -1.0],
            [-1.0, 1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, -1.0],
        ],
        vec![
            [0, 2, 1],
            [0, 1, 3],
            [1, 2, 3],
            [2, 0, 3],
            [4, 6, 5],
            [4, 5, 7],
            [5, 6, 7],
            [6, 4, 7],
        ],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    match validate_closed_oriented(validated, source("crossing")) {
        Err(MeshFailure::SelfIntersection(witness)) => {
            assert_eq!(
                witness.kind(),
                MeshPairIntersectionKind::NonPermittedCrossing
            );
        }
        other => panic!("expected non-permitted crossing, got {other:?}"),
    }
}

#[test]
fn rejects_coplanar_overlap() {
    let mesh = SurfaceMesh::new(
        vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.5, 0.5, 1.0],
            [0.5, 0.5, 0.0],
            [2.5, 0.5, 0.0],
            [0.5, 2.5, 0.0],
            [1.0, 1.0, 1.0],
        ],
        vec![
            [0, 2, 1],
            [0, 1, 3],
            [1, 2, 3],
            [2, 0, 3],
            [4, 6, 5],
            [4, 5, 7],
            [5, 6, 7],
            [6, 4, 7],
        ],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    match validate_closed_oriented(validated, source("coplanar")) {
        Err(MeshFailure::SelfIntersection(witness)) => {
            assert_eq!(witness.kind(), MeshPairIntersectionKind::CoplanarOverlap);
        }
        other => panic!("expected coplanar overlap, got {other:?}"),
    }
}

#[test]
fn rejects_distinct_index_coincident_contact() {
    let mesh = SurfaceMesh::new(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![
            [0, 1, 3],
            [1, 2, 3],
            [2, 0, 3],
            [0, 2, 4],
            [2, 1, 4],
            [1, 0, 4],
        ],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    match validate_closed_oriented(validated, source("coincident")) {
        Err(MeshFailure::SelfIntersection(witness)) => {
            assert_eq!(
                witness.kind(),
                MeshPairIntersectionKind::DistinctIndexCoincidentContact
            );
        }
        other => panic!("expected distinct-index coincident contact, got {other:?}"),
    }
}

#[test]
fn rejects_duplicate_face() {
    let mesh = SurfaceMesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2], [0, 2, 1]],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    match validate_closed_oriented(validated, source("duplicate")) {
        Err(MeshFailure::SelfIntersection(witness)) => {
            assert_eq!(witness.kind(), MeshPairIntersectionKind::DuplicateFace);
        }
        other => panic!("expected duplicate face, got {other:?}"),
    }
}

#[test]
fn rejects_indeterminate_finite_pair() {
    let tilt = 1e-13;
    let mesh = SurfaceMesh::new(
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [10.0, 0.0, 0.0],
            [11.0, 0.0, 0.0],
            [10.0, 1.0, tilt],
            [10.0, 0.0, 1.0],
        ],
        vec![
            [0, 2, 1],
            [0, 1, 3],
            [1, 2, 3],
            [2, 0, 3],
            [4, 6, 5],
            [4, 5, 7],
            [5, 6, 7],
            [6, 4, 7],
        ],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    match validate_closed_oriented(validated, source("indeterminate")) {
        Err(MeshFailure::SelfIntersection(witness)) => {
            assert_eq!(witness.kind(), MeshPairIntersectionKind::Indeterminate);
        }
        other => panic!("expected indeterminate pair, got {other:?}"),
    }
}

#[test]
fn preserves_a_triangle_permutation() {
    let mesh = SurfaceMesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[2, 0, 1]],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    assert_eq!(validated.validation_witness().triangle_count(), 1);
}

#[test]
fn rejects_an_out_of_range_triangle_index() {
    let mesh = SurfaceMesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 3]],
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        validate_surface_mesh(mesh),
        Err(MeshFailure::TriangleIndexOutOfBounds {
            triangle_index: 0,
            vertex_index: 3,
        })
    );
}

#[test]
fn rejects_an_open_surface() {
    let mesh = SurfaceMesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
        Vec::new(),
    )
    .unwrap();
    let validated = validate_surface_mesh(mesh).unwrap();
    assert_eq!(
        validate_closed_oriented(validated, source("open")),
        Err(MeshFailure::OpenEdge {
            first_vertex: 0,
            second_vertex: 1,
        })
    );
}

#[test]
fn validates_a_two_dimensional_triangle_mesh() {
    let mesh =
        TriangleMesh2d::new(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], vec![[0, 1, 2]]).unwrap();
    assert_eq!(mesh.triangles(), &[[0, 1, 2]]);
}

#[test]
fn rejects_a_degenerate_triangle() {
    let collinear = SurfaceMesh::new(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
        vec![[0, 1, 2]],
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        validate_surface_mesh(collinear),
        Err(MeshFailure::DegenerateTriangle { triangle_index: 0 })
    );
}

#[test]
fn rejects_a_non_finite_vertex() {
    let non_finite = SurfaceMesh::new(
        vec![[f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        vec![[0, 1, 2]],
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        validate_surface_mesh(non_finite),
        Err(MeshFailure::NonFiniteVertex { vertex_index: 0 })
    );
}
