#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod aggregate;
mod containment;
mod definition;
mod evaluation;
mod failure;
mod split;

pub use aggregate::{aggregate_bezier, BezierAggregationInput};
pub use containment::{
    contain_bezier, ContainmentGeometricShape, ContainmentParameterDomain,
    ContainmentPointDimension, ContainmentRepresentationCategory, ContainmentResult,
    ContainmentWeightEvidence, ContainmentWitness, ConvexHull, CoordinateInterval,
    Point2ConvexHull, Point3ConvexHull,
};
pub use definition::{
    define_bezier, BezierDefinitionRequest, BezierPoint, BezierRepresentation, BezierTypes,
    CadFailure, PolynomialBezierCurve, PolynomialBezierSurface, RationalBezierCurve,
    RationalBezierSurface,
};
pub use evaluation::{evaluate_bezier, BezierEvaluationRequest, EvaluationResult};
pub use failure::{cad_failure_api, CadFailureApi, FailureApiRequest};
pub use split::{split_bezier, BezierSplitAxis, BezierSplitRequest, BezierSplitResult};
