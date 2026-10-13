use neco_cdt::{triangulate, CdtFailure, CdtRequest};

#[test]
fn triangulates_a_convex_constraint_loop() {
    let mesh =
        triangulate(CdtRequest::new(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]).unwrap())
            .unwrap();
    assert_eq!(mesh.vertices().len(), 4);
    assert_eq!(mesh.triangles().len(), 2);
}

#[test]
fn preserves_reversed_constraint_orientation() {
    let mesh =
        triangulate(CdtRequest::new(vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]).unwrap())
            .unwrap();
    assert_eq!(mesh.triangles().len(), 2);
}

#[test]
fn rejects_a_degenerate_constraint_loop() {
    assert_eq!(
        CdtRequest::new(vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]),
        Err(CdtFailure::DegenerateConstraint)
    );
}
