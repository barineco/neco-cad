use neco_tessellation::TessellationRequest;

#[test]
fn preserves_the_declared_precision_requirement() {
    let request = TessellationRequest::new(0.01).unwrap();
    assert_eq!(request.maximum_chord_error(), 0.01);
}

#[test]
fn rejects_an_invalid_precision_request() {
    assert!(TessellationRequest::new(0.0).is_err());
}

#[test]
fn rejects_non_finite_precision_requests() {
    assert!(TessellationRequest::new(f64::NAN).is_err());
    assert!(TessellationRequest::new(f64::INFINITY).is_err());
}
