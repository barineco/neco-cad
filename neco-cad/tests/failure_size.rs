use core::mem::size_of;

use neco_cad::CadFailure;

const MAXIMUM_BYTES: usize = 96;

#[test]
fn cad_failure_size_stays_within_bound() {
    let actual = size_of::<CadFailure>();
    assert!(
        actual <= MAXIMUM_BYTES,
        "CadFailure is {actual} bytes; the bound is {MAXIMUM_BYTES}"
    );
}
