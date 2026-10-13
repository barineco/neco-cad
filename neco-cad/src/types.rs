use neco_brep::BrepFailure;
use neco_geometry::{AlgebraicRegion, GeometryFailure, NormalizedRegionFormula};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryDerivationCondition {
    EmptyRegion,
    IncompleteCoverage,
    SingularityUnsupported,
    SeamUnsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrepDerivationCondition {
    IncompleteCoverage,
    OpenBoundary,
    NonManifold,
    UndeterminedDimension,
    OrientationInconsistent,
    AmbiguousSupportAtom,
    UnsupportedSurfaceCovering,
    SurfaceRepresentation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericProjectionCondition {
    AlgebraicEnclosure,
    NonFiniteCoordinate,
    SurfaceEvaluation,
    ResidualUnboundable,
    EmptyIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdaptiveTessellationCondition {
    EmptyIdentity,
    InvalidPrecision,
    EmptyFaces,
    MissingCovering,
    UnknownFace,
    SurfaceEvaluation,
    Projection,
    ChordUnboundable,
    SurfaceErrorExceeded,
    TrimClassification,
    SeamUnpaired,
    SubdivisionLimit,
    InvalidMesh,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CadFailure {
    Geometry(GeometryFailure),
    Brep(BrepFailure),
    Boundary {
        condition: BoundaryDerivationCondition,
        atom_id: Option<alloc::string::String>,
    },
    BrepDerivation {
        condition: BrepDerivationCondition,
        stratum_id: Option<alloc::string::String>,
    },
    NumericProjection {
        condition: NumericProjectionCondition,
        identity: Option<alloc::string::String>,
    },
    AdaptiveTessellation {
        condition: AdaptiveTessellationCondition,
        identity: Option<alloc::string::String>,
    },
    Tessellation(alloc::boxed::Box<neco_tessellation::TessellationFailure>),
}

impl From<GeometryFailure> for CadFailure {
    fn from(error: GeometryFailure) -> Self {
        Self::Geometry(error)
    }
}

impl From<BrepFailure> for CadFailure {
    fn from(error: BrepFailure) -> Self {
        Self::Brep(error)
    }
}

impl From<neco_tessellation::TessellationFailure> for CadFailure {
    fn from(error: neco_tessellation::TessellationFailure) -> Self {
        match error {
            neco_tessellation::TessellationFailure::InvalidPrecision => {
                Self::AdaptiveTessellation {
                    condition: AdaptiveTessellationCondition::InvalidPrecision,
                    identity: None,
                }
            }
            error => Self::Tessellation(alloc::boxed::Box::new(error)),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SetBooleanRequest {
    pub left: AlgebraicRegion,
    pub right: AlgebraicRegion,
    pub formula: NormalizedRegionFormula,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SolidBooleanRequest {
    pub left: AlgebraicRegion,
    pub right: AlgebraicRegion,
    pub formula: NormalizedRegionFormula,
}
