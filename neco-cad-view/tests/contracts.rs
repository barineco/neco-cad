use neco_cad_view::{
    CadViewFailure, Camera, CheckedRay3, PickTolerance, Point3, Vector3, Viewport, VisibleRange,
};

#[test]
fn validates_camera_viewport_range_and_pick_tolerance() {
    let eye = Point3::new(0.0, 0.0, 10.0).unwrap();
    let target = Point3::new(0.0, 0.0, 0.0).unwrap();
    let up = Vector3::new(0.0, 1.0, 0.0).unwrap();
    assert!(Camera::orthographic(eye, target, up, 4.0).is_ok());
    assert_eq!(Viewport::new(0, 1), Err(CadViewFailure::EmptyViewport));
    assert!(VisibleRange::new(0.0, 10.0).is_ok());
    assert!(VisibleRange::new(1.0, 1.0).is_err());
    assert!(PickTolerance::new(0.0).is_ok());
    assert!(PickTolerance::new(-1.0).is_err());
}

#[test]
fn zero_length_ray_direction_is_an_explicit_geometry_failure() {
    let error = CheckedRay3::try_new([0.0; 3], [0.0; 3], 0.0).unwrap_err();
    let error: neco_geometry::GeometryFailure = error.into();
    assert!(matches!(
        error,
        neco_geometry::GeometryFailure::FoundationFailure { .. }
    ));
}

#[test]
fn points_and_vectors_reject_non_finite_coordinates() {
    assert_eq!(
        Point3::new(f64::NAN, 0.0, 0.0),
        Err(CadViewFailure::NonFiniteCoordinate)
    );
    assert_eq!(
        Vector3::new(0.0, f64::INFINITY, 0.0),
        Err(CadViewFailure::NonFiniteCoordinate)
    );
}
