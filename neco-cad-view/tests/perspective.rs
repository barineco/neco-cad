use neco_brep::SourceRevision;
use neco_cad::{
    construct, derive_analytic_brep, derive_complete_boundary, tessellate_adaptive,
    AdaptivePrecision, ConstructionOperation, ConstructionOutput, ConstructionRequest,
    EntityIdentityBinding, ExactScalar, GeometryAccuracyPolicy, GeometryClosure,
    GeometryOutputCategory, GeometryOutputExpectation, IdentityLedger, OperationIdentity,
    OperationIdentityBinding, OperationOutputIdentity, OperationRequest, OutputSlot,
    PrimitiveDeclaration, Provenance, SourceIdentity, TessellationConsumer,
};
use neco_cad_view::{
    derive_cad_view_model, project_cpu_reference, ray_from_viewport_point, CadViewFailure,
    CadViewModelRequest, CadViewTessellation, Camera, CpuPrimitive, CpuProjectionFailure,
    DisplaySettings, Point2, Point3, Selection, Vector3, Viewport, Visibility, VisibleRange,
};

fn sphere_region() -> neco_geometry::AlgebraicRegion {
    let revision = SourceRevision::new(String::from("perspective-r1")).unwrap();
    let operation_identity = OperationIdentity::new("sphere").unwrap();
    let source_identity = SourceIdentity::new("sphere-source").unwrap();
    let output_identity = OperationOutputIdentity {
        operation: operation_identity.clone(),
        slot: OutputSlot::Primary,
    };
    let operation = ConstructionOperation {
        operation: operation_identity.clone(),
        source_object: source_identity.clone(),
        expected_output: GeometryOutputExpectation {
            category: GeometryOutputCategory::Solid,
            ambient_dimension: 3,
            manifold_dimension: 3,
            closure: GeometryClosure::Closed,
        },
        request: OperationRequest::Primitive(PrimitiveDeclaration::Sphere {
            radius: ExactScalar::new(1.0).unwrap(),
        }),
    };
    let ledger = IdentityLedger::new(
        vec![OperationIdentityBinding {
            source: source_identity.clone(),
            operation: operation_identity.clone(),
            output: output_identity.clone(),
        }],
        vec![EntityIdentityBinding {
            source: source_identity,
            operation: operation_identity,
            output: output_identity.clone(),
            produced: String::from("sphere:primary"),
        }],
    )
    .unwrap();
    construct(ConstructionRequest {
        source_revision: revision.clone(),
        precision: GeometryAccuracyPolicy::ExactOnly,
        identity_ledger: ledger,
        operations: vec![operation],
        roots: Vec::new(),
        provenance: Provenance::new(
            "schema",
            "ns",
            revision,
            "adapter",
            "exact",
            Vec::new(),
            Vec::new(),
        ),
    })
    .unwrap()
    .into_outputs()
    .into_iter()
    .find_map(|(identity, output)| {
        if identity != output_identity {
            return None;
        }
        match output {
            ConstructionOutput::Region(region) => Some(region),
            _ => None,
        }
    })
    .unwrap()
}

fn camera(fov: f64) -> Result<Camera, CadViewFailure> {
    Camera::perspective(
        Point3::new(0.5, 0.5, 2.0).unwrap(),
        Point3::new(0.5, 0.5, 0.0).unwrap(),
        Vector3::new(0.0, 1.0, 0.0).unwrap(),
        fov,
    )
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.17e} expected={expected:.17e} tolerance={tolerance:.3e}",
    );
}

fn difference(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn on_triangle(point: [f64; 3], vertices: [[f64; 3]; 3]) -> bool {
    let a = difference(vertices[1], vertices[0]);
    let b = difference(vertices[2], vertices[0]);
    let p = difference(point, vertices[0]);
    let aa = dot(a, a);
    let ab = dot(a, b);
    let bb = dot(b, b);
    let ap = dot(a, p);
    let bp = dot(b, p);
    let determinant = aa * bb - ab * ab;
    if determinant <= 0.0 {
        return false;
    }
    let u = (bb * ap - ab * bp) / determinant;
    let v = (aa * bp - ab * ap) / determinant;
    let residual = difference(
        p,
        [
            u * a[0] + v * b[0],
            u * a[1] + v * b[1],
            u * a[2] + v * b[2],
        ],
    );
    u >= -1e-9 && v >= -1e-9 && u + v <= 1.0 + 1e-9 && dot(residual, residual) <= 1e-18
}

#[test]
fn public_perspective_preserves_fov_in_rays_projection_and_clipping() {
    let revision = SourceRevision::new(String::from("perspective-r1")).unwrap();
    let (brep, _) = derive_analytic_brep(
        derive_complete_boundary(sphere_region()).unwrap(),
        revision.clone(),
    )
    .unwrap()
    .into_parts();
    let derived =
        derive_analytic_brep(derive_complete_boundary(sphere_region()).unwrap(), revision).unwrap();
    let source = SourceIdentity::new("mesh:perspective").unwrap();
    let precision = AdaptivePrecision::new(TessellationConsumer::Display, 0.5, 1.0).unwrap();
    let tessellation = tessellate_adaptive(
        derived
            .into_adaptive_tessellation_request(source.clone(), precision, 48)
            .unwrap(),
    )
    .unwrap();
    let make_model = |fov, viewport| {
        derive_cad_view_model(CadViewModelRequest {
            brep: &brep,
            tessellation: CadViewTessellation::Adaptive {
                result: &tessellation,
                request: precision,
                expected_source_identity: &source,
            },
            camera: camera(fov).unwrap(),
            viewport,
            display: DisplaySettings::new(
                Visibility::Visible,
                Visibility::Hidden,
                Visibility::Hidden,
                Visibility::Hidden,
                Visibility::Hidden,
                false,
            ),
            selection: Selection::new(Vec::new()).unwrap(),
        })
        .unwrap()
    };
    let square = Viewport::new(100, 100).unwrap();
    let range = VisibleRange::new(0.0, 4.0).unwrap();
    for (fov, slope) in [
        (core::f64::consts::FRAC_PI_2, 1.0),
        (core::f64::consts::PI / 3.0, 1.0 / 3.0_f64.sqrt()),
    ] {
        let model = make_model(fov, square);
        let ray = ray_from_viewport_point(&model, Point2::new(0.0, 49.5).unwrap()).unwrap();
        assert_eq!(ray.origin(), &[0.5, 0.5, 2.0]);
        close(ray.direction()[0] / ray.direction()[2], 0.99 * slope, 1e-14);
        close(ray.direction()[1], 0.0, 1e-14);
        close(dot(*ray.direction(), *ray.direction()), 1.0, 1e-14);

        let mesh = model.mesh().mesh();
        let projection = project_cpu_reference(&model, range).unwrap();
        let mut clipped_side = false;
        let mut interior_vertex = false;
        for primitive in projection.primitives() {
            let CpuPrimitive::Triangle {
                vertices, depth, ..
            } = primitive
            else {
                panic!("faces-only projection must return triangles");
            };
            for (position, &z) in vertices.iter().zip(depth) {
                assert!(position.x() >= -0.5 - 1e-10 && position.x() <= 99.5 + 1e-10);
                assert!(position.y() >= -0.5 - 1e-10 && position.y() <= 99.5 + 1e-10);
                assert!(z > 0.0 && z <= 4.0);
                let nx = (position.x() + 0.5) / 50.0 - 1.0;
                let ny = 1.0 - (position.y() + 0.5) / 50.0;
                let world = [0.5 + nx * z * slope, 0.5 + ny * z * slope, 2.0 - z];
                assert!(
                    mesh.triangles().iter().any(|triangle| {
                        on_triangle(
                            world,
                            [
                                mesh.vertices()[triangle[0]],
                                mesh.vertices()[triangle[1]],
                                mesh.vertices()[triangle[2]],
                            ],
                        )
                    }),
                    "projected vertex must reconstruct onto the original mesh: {world:?}"
                );
                clipped_side |=
                    (position.x() + 0.5).abs() < 1e-10 || (position.y() - 99.5).abs() < 1e-10;
                interior_vertex |= position.x() > 1.0
                    && position.x() < 98.0
                    && position.y() > 1.0
                    && position.y() < 98.0;
            }
        }
        if fov < core::f64::consts::FRAC_PI_2 {
            assert!(
                clipped_side,
                "sphere must intersect a side of the specified frustum"
            );
        }
        assert!(
            interior_vertex,
            "interior geometry must retain the same FOV mapping"
        );
    }

    let wide_viewport = Viewport::new(200, 100).unwrap();
    for fov in [
        170.0_f64.to_radians(),
        core::f64::consts::PI - 1e-8,
        f64::from_bits(core::f64::consts::PI.to_bits() - 1),
        f64::MIN_POSITIVE,
        f64::from_bits(1),
    ] {
        let model = make_model(fov, wide_viewport);
        let pixel = Point2::new(0.0, 0.0).unwrap();
        let ray = ray_from_viewport_point(&model, pixel).unwrap();
        assert!(ray.direction().iter().all(|value| value.is_finite()));
        close(dot(*ray.direction(), *ray.direction()), 1.0, 1e-14);
        let slope = (fov / 2.0).tan();
        close(
            ray.direction()[0] / ray.direction()[2],
            1.99 * slope,
            1e-14 * (1.99 * slope).abs().max(1.0),
        );
        close(
            ray.direction()[1] / -ray.direction()[2],
            0.99 * slope,
            1e-14 * slope.abs().max(1.0),
        );
        if slope == 0.0 {
            assert_eq!(
                project_cpu_reference(&model, range),
                Err(CpuProjectionFailure::CadView(
                    CadViewFailure::NonFiniteCoordinate
                )),
            );
        } else if fov > 3.0 {
            let projection = project_cpu_reference(&model, range).unwrap();
            for primitive in projection.primitives() {
                let CpuPrimitive::Triangle {
                    vertices, depth, ..
                } = primitive
                else {
                    panic!("faces-only projection must return triangles");
                };
                for (position, &z) in vertices.iter().zip(depth) {
                    assert!(position.x().is_finite() && position.y().is_finite());
                    assert!(z > 0.0);
                }
            }
        }
    }
}

#[test]
fn perspective_rejects_nonfinite_and_out_of_domain_fov() {
    for fov in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        -0.0,
        0.0,
        core::f64::consts::PI,
        core::f64::consts::PI + f64::EPSILON * 2.0,
    ] {
        assert_eq!(
            camera(fov),
            Err(CadViewFailure::InvalidPerspectiveFieldOfView)
        );
    }
}
