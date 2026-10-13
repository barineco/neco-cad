use neco_brep::AnalyticBrep;
use neco_cad_import::{
    region_from_mesh, region_from_shell, CertifiedMembershipResult, CertifiedRegion,
    MeshMembershipPoint, RegionImportFailure, RegionImportRequest,
};
use neco_geometry::{
    ExactExpressionRequirement, NecoImplementationSource, OwnerId, RayCrossingPolicy,
};
use neco_mesh::ClosedOrientedSurfaceMesh;

type RegionFromShellFn = fn(
    &AnalyticBrep,
    &str,
    RegionImportRequest,
    OwnerId,
    &ExactExpressionRequirement,
    NecoImplementationSource,
) -> Result<CertifiedRegion, RegionImportFailure>;

#[test]
fn exposes_requested_precision_without_achieved_error() {
    fn observe(request: &RegionImportRequest, region: &CertifiedRegion) {
        let _ = (
            request.requested_maximum_error(),
            region.requested_maximum_error(),
        );
    }
    let _ = observe;
}

#[test]
fn uses_region_from_shell_signature_and_region_from_mesh_signature() {
    let from_mesh: fn(
        ClosedOrientedSurfaceMesh,
        RegionImportRequest,
    ) -> Result<CertifiedRegion, RegionImportFailure> = region_from_mesh;
    let from_shell: RegionFromShellFn = region_from_shell;
    let membership: fn(
        &CertifiedRegion,
        MeshMembershipPoint,
        OwnerId,
        &ExactExpressionRequirement,
        NecoImplementationSource,
    ) -> Result<CertifiedMembershipResult, RegionImportFailure> = CertifiedRegion::membership;
    let _ = (from_mesh, from_shell, membership);
    fn observe_policy(_: &RayCrossingPolicy) {}
    let _ = observe_policy;
}
