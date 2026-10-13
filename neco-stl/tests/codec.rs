use neco_mesh::{validate_surface_mesh, SurfaceMesh};
use neco_stl::{decode, encode, StlEncoding, StlFailure};

fn validated_triangle() -> neco_mesh::ValidatedSurfaceMesh {
    validate_surface_mesh(
        SurfaceMesh::new(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2]],
            Vec::new(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn binary_roundtrip_preserves_triangle_count() {
    let bytes = encode(&validated_triangle(), StlEncoding::Binary).unwrap();
    let decoded = decode(&bytes).unwrap();
    assert_eq!(decoded.triangles().len(), 1);
}

#[test]
fn ascii_roundtrip_preserves_triangle_count() {
    let bytes = encode(&validated_triangle(), StlEncoding::Ascii).unwrap();
    let decoded = decode(&bytes).unwrap();
    assert_eq!(decoded.triangles().len(), 1);
}

#[test]
fn rejects_truncated_binary_data() {
    assert_eq!(decode(&[0; 83]), Err(StlFailure::InvalidHeader),);
}

#[test]
fn rejects_a_body_shorter_than_the_declared_triangle_count() {
    let mut bytes = vec![0u8; 84];
    bytes[80] = 2;
    assert_eq!(
        decode(&bytes),
        Err(StlFailure::TruncatedBinary {
            expected_length: 184,
            actual_length: 84,
        })
    );
}
