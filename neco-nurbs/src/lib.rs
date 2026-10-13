#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod definition;
mod evaluation;
mod fitting;

pub use definition::{
    define_nurbs, BezierPoint, CadFailure, NurbsDefinitionRequest, NurbsRepresentation, NurbsTypes,
    PolynomialNurbsCurve, PolynomialNurbsSurface, RationalNurbsCurve, RationalNurbsSurface,
};
pub use evaluation::{evaluate_curve_nurbs, evaluate_nurbs, NurbsEvaluationRequest};
pub use fitting::{fit_nurbs_curve, FittingRequest, FittingResult, NurbsFittingFailure};
pub use neco_bezier::EvaluationResult;
