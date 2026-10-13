#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
extern crate alloc;
mod model;
mod picking;
mod projection;
pub(crate) use model::{cross, dot, normalize, sqrt, sub};
pub use model::{
    derive_cad_view_model, AchievedPrecision, CadElementIdentity, CadViewFailure, CadViewModel,
    CadViewModelRequest, CadViewModelVersion, CadViewTessellation, Camera, CertifiedPrecision,
    CorrespondenceClass, DisplaySettings, Point2, Point3, Projection, RequestedPrecision,
    Selection, Vector3, ViewModelIdentity, Viewport, Visibility, CAD_VIEW_MODEL_SCHEMA_VERSION,
};
pub use neco_geometry::CheckedRay3;
pub use picking::{pick_cpu, CpuPick, PickKind, PickTolerance};
#[cfg(feature = "test-fixtures")]
pub use projection::CpuProjectionFixture;
pub use projection::{
    project_cpu_reference, ray_from_viewport_point, CpuPrimitive, CpuProjection,
    CpuProjectionFailure, VisibleRange,
};
