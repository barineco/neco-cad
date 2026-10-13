use core::cmp::Ordering;

use neco_algnum::RealAlgebraic;

use crate::{BezierPoint, BezierTypes, CadFailure};

pub struct CoordinateInterval {
    lower: RealAlgebraic,
    upper: RealAlgebraic,
}

impl CoordinateInterval {
    pub fn lower(&self) -> &RealAlgebraic {
        &self.lower
    }

    pub fn upper(&self) -> &RealAlgebraic {
        &self.upper
    }
}

pub struct Point2ConvexHull {
    x: CoordinateInterval,
    y: CoordinateInterval,
}

impl Point2ConvexHull {
    pub fn x(&self) -> &CoordinateInterval {
        &self.x
    }

    pub fn y(&self) -> &CoordinateInterval {
        &self.y
    }
}

pub struct Point3ConvexHull {
    x: CoordinateInterval,
    y: CoordinateInterval,
    z: CoordinateInterval,
}

impl Point3ConvexHull {
    pub fn x(&self) -> &CoordinateInterval {
        &self.x
    }

    pub fn y(&self) -> &CoordinateInterval {
        &self.y
    }

    pub fn z(&self) -> &CoordinateInterval {
        &self.z
    }
}

pub enum ConvexHull {
    Point2(Point2ConvexHull),
    Point3(Point3ConvexHull),
}

pub enum ContainmentRepresentationCategory {
    Polynomial,
    Rational,
}

pub enum ContainmentGeometricShape {
    Curve,
    Surface,
}

pub enum ContainmentPointDimension {
    Point2,
    Point3,
}

pub enum ContainmentWeightEvidence {
    PolynomialNoWeights,
    ValidatedPositiveWeights,
}

pub enum ContainmentParameterDomain {
    ClosedCurveUnitInterval,
    ClosedSurfaceUnitSquare,
}

pub struct ContainmentWitness {
    representation_category: ContainmentRepresentationCategory,
    geometric_shape: ContainmentGeometricShape,
    point_dimension: ContainmentPointDimension,
    row_first: bool,
    finite_control_values: bool,
    weight_evidence: ContainmentWeightEvidence,
    parameter_domain: ContainmentParameterDomain,
}

impl ContainmentWitness {
    pub fn representation_category(&self) -> &ContainmentRepresentationCategory {
        &self.representation_category
    }

    pub fn geometric_shape(&self) -> &ContainmentGeometricShape {
        &self.geometric_shape
    }

    pub fn point_dimension(&self) -> &ContainmentPointDimension {
        &self.point_dimension
    }

    pub fn row_first(&self) -> &bool {
        &self.row_first
    }

    pub fn finite_control_values(&self) -> &bool {
        &self.finite_control_values
    }

    pub fn weight_evidence(&self) -> &ContainmentWeightEvidence {
        &self.weight_evidence
    }

    pub fn parameter_domain(&self) -> &ContainmentParameterDomain {
        &self.parameter_domain
    }
}

pub struct ContainmentResult {
    hull: ConvexHull,
    witness: ContainmentWitness,
}

impl ContainmentResult {
    pub fn hull(&self) -> &ConvexHull {
        &self.hull
    }

    pub fn into_hull(self) -> ConvexHull {
        self.hull
    }

    pub fn witness(&self) -> &ContainmentWitness {
        &self.witness
    }
}

pub fn contain_bezier(representation: BezierTypes) -> Result<ContainmentResult, CadFailure> {
    let (hull, representation_category, geometric_shape, weight_evidence) = match representation {
        BezierTypes::PolynomialCurve(curve) => (
            scan_points(curve.control_points().iter())?,
            ContainmentRepresentationCategory::Polynomial,
            ContainmentGeometricShape::Curve,
            ContainmentWeightEvidence::PolynomialNoWeights,
        ),
        BezierTypes::RationalCurve(curve) => (
            scan_points(curve.control_points().iter())?,
            ContainmentRepresentationCategory::Rational,
            ContainmentGeometricShape::Curve,
            ContainmentWeightEvidence::ValidatedPositiveWeights,
        ),
        BezierTypes::PolynomialSurface(surface) => (
            scan_points(surface.control_net().iter().flatten())?,
            ContainmentRepresentationCategory::Polynomial,
            ContainmentGeometricShape::Surface,
            ContainmentWeightEvidence::PolynomialNoWeights,
        ),
        BezierTypes::RationalSurface(surface) => (
            scan_points(surface.control_net().iter().flatten())?,
            ContainmentRepresentationCategory::Rational,
            ContainmentGeometricShape::Surface,
            ContainmentWeightEvidence::ValidatedPositiveWeights,
        ),
    };

    let point_dimension = match &hull {
        ConvexHull::Point2(_) => ContainmentPointDimension::Point2,
        ConvexHull::Point3(_) => ContainmentPointDimension::Point3,
    };
    let parameter_domain = match geometric_shape {
        ContainmentGeometricShape::Curve => ContainmentParameterDomain::ClosedCurveUnitInterval,
        ContainmentGeometricShape::Surface => ContainmentParameterDomain::ClosedSurfaceUnitSquare,
    };

    Ok(ContainmentResult {
        hull,
        witness: ContainmentWitness {
            representation_category,
            geometric_shape,
            point_dimension,
            row_first: true,
            finite_control_values: true,
            weight_evidence,
            parameter_domain,
        },
    })
}

fn scan_points<'a>(
    points: impl Iterator<Item = &'a BezierPoint>,
) -> Result<ConvexHull, CadFailure> {
    let mut points = points;
    let first = points.next().ok_or(CadFailure::EmptyControlPoints)?;

    match first {
        BezierPoint::Point2(first) => {
            let mut x_lower = first.x.try_clone()?;
            let mut x_upper = first.x.try_clone()?;
            let mut y_lower = first.y.try_clone()?;
            let mut y_upper = first.y.try_clone()?;

            for point in points {
                let BezierPoint::Point2(point) = point else {
                    return Err(CadFailure::ControlPointDimensionMismatch);
                };
                update_extrema(&mut x_lower, &mut x_upper, &point.x)?;
                update_extrema(&mut y_lower, &mut y_upper, &point.y)?;
            }

            Ok(ConvexHull::Point2(Point2ConvexHull {
                x: CoordinateInterval {
                    lower: x_lower,
                    upper: x_upper,
                },
                y: CoordinateInterval {
                    lower: y_lower,
                    upper: y_upper,
                },
            }))
        }
        BezierPoint::Point3(first) => {
            let mut x_lower = first.x.try_clone()?;
            let mut x_upper = first.x.try_clone()?;
            let mut y_lower = first.y.try_clone()?;
            let mut y_upper = first.y.try_clone()?;
            let mut z_lower = first.z.try_clone()?;
            let mut z_upper = first.z.try_clone()?;

            for point in points {
                let BezierPoint::Point3(point) = point else {
                    return Err(CadFailure::ControlPointDimensionMismatch);
                };
                update_extrema(&mut x_lower, &mut x_upper, &point.x)?;
                update_extrema(&mut y_lower, &mut y_upper, &point.y)?;
                update_extrema(&mut z_lower, &mut z_upper, &point.z)?;
            }

            Ok(ConvexHull::Point3(Point3ConvexHull {
                x: CoordinateInterval {
                    lower: x_lower,
                    upper: x_upper,
                },
                y: CoordinateInterval {
                    lower: y_lower,
                    upper: y_upper,
                },
                z: CoordinateInterval {
                    lower: z_lower,
                    upper: z_upper,
                },
            }))
        }
    }
}

fn update_extrema(
    lower: &mut RealAlgebraic,
    upper: &mut RealAlgebraic,
    value: &RealAlgebraic,
) -> Result<(), CadFailure> {
    if value.compare(lower)? == Ordering::Less {
        *lower = value.try_clone()?;
    }
    if value.compare(upper)? == Ordering::Greater {
        *upper = value.try_clone()?;
    }
    Ok(())
}
