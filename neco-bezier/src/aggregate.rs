use crate::{
    BezierPoint, BezierSplitResult, BezierTypes, CadFailure, ContainmentResult, ConvexHull,
    EvaluationResult,
};

pub struct BezierAggregationInput {
    definition: BezierTypes,
    evaluation: EvaluationResult,
    split: BezierSplitResult,
    containment: ContainmentResult,
}

impl BezierAggregationInput {
    pub fn new(
        definition: BezierTypes,
        evaluation: EvaluationResult,
        split: BezierSplitResult,
        containment: ContainmentResult,
    ) -> Self {
        Self {
            definition,
            evaluation,
            split,
            containment,
        }
    }

    pub fn definition(&self) -> &BezierTypes {
        &self.definition
    }

    pub fn evaluation(&self) -> &EvaluationResult {
        &self.evaluation
    }

    pub fn split(&self) -> &BezierSplitResult {
        &self.split
    }

    pub fn containment(&self) -> &ContainmentResult {
        &self.containment
    }
}

pub fn aggregate_bezier(input: BezierAggregationInput) -> Result<BezierTypes, CadFailure> {
    let BezierAggregationInput {
        definition,
        evaluation,
        split,
        containment,
    } = input;
    let definition_dimension = representation_dimension(&definition);
    let evaluated_point = evaluation.into_point();
    let split_dimensions = consume_split_dimensions(split);
    let containment_dimension = consume_containment_dimension(containment);

    ensure_point_dimension(&evaluated_point, definition_dimension)?;
    ensure_split_dimensions(split_dimensions, definition_dimension)?;
    ensure_containment_dimension(containment_dimension, definition_dimension)?;

    Ok(definition)
}

fn representation_dimension(representation: &BezierTypes) -> usize {
    match representation {
        BezierTypes::PolynomialCurve(curve) => curve.dimension(),
        BezierTypes::RationalCurve(curve) => curve.dimension(),
        BezierTypes::PolynomialSurface(surface) => surface.dimension(),
        BezierTypes::RationalSurface(surface) => surface.dimension(),
    }
}

fn consume_split_dimensions(split: BezierSplitResult) -> (usize, usize) {
    match split {
        BezierSplitResult::Curve { left, right } => (
            representation_dimension(&left),
            representation_dimension(&right),
        ),
        BezierSplitResult::Surface { lower, upper } => (
            representation_dimension(&lower),
            representation_dimension(&upper),
        ),
    }
}

fn consume_containment_dimension(containment: ContainmentResult) -> usize {
    match containment.into_hull() {
        ConvexHull::Point2(_) => 2,
        ConvexHull::Point3(_) => 3,
    }
}

fn ensure_point_dimension(
    point: &BezierPoint,
    definition_dimension: usize,
) -> Result<(), CadFailure> {
    if point.dimension() == definition_dimension {
        Ok(())
    } else {
        Err(CadFailure::ControlPointDimensionMismatch)
    }
}

fn ensure_split_dimensions(
    split_dimensions: (usize, usize),
    definition_dimension: usize,
) -> Result<(), CadFailure> {
    if split_dimensions.0 == definition_dimension && split_dimensions.1 == definition_dimension {
        Ok(())
    } else {
        Err(CadFailure::ControlPointDimensionMismatch)
    }
}

fn ensure_containment_dimension(
    containment_dimension: usize,
    definition_dimension: usize,
) -> Result<(), CadFailure> {
    if containment_dimension == definition_dimension {
        Ok(())
    } else {
        Err(CadFailure::ControlPointDimensionMismatch)
    }
}
