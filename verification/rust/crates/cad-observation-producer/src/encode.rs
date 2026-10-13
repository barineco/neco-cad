use neco_bigint::{BigInt, BigUint, Dyadic, ReducedRational, Sign};
use neco_calculus::{RationalSurfacePolynomials, SurfacePolynomial};
use neco_geometry::{ParameterInterval, SurfaceParameterDomain};
use serde_json::{json, Map, Value};

pub fn uint_to_decimal(value: &BigUint) -> String {
    if value.is_zero() {
        return String::from("0");
    }
    let base = BigUint::try_from(1_000_000_000_u64).expect("base");
    let mut remaining = value.try_clone().expect("bigint clone");
    let mut parts = Vec::new();
    while !remaining.is_zero() {
        let (quotient, remainder) = remaining.div_rem(&base).expect("div_rem");
        parts.push(remainder.to_u32().expect("limb"));
        remaining = quotient;
    }
    let mut text = String::new();
    for (index, part) in parts.iter().rev().enumerate() {
        if index == 0 {
            text.push_str(&part.to_string());
        } else {
            text.push_str(&format!("{part:09}"));
        }
    }
    text
}

pub fn int_to_decimal(value: &BigInt) -> String {
    let magnitude = uint_to_decimal(value.magnitude());
    match value.sign() {
        Sign::Negative => format!("-{magnitude}"),
        _ => magnitude,
    }
}

pub fn rational_json(value: &ReducedRational) -> Value {
    json!({
        "numerator": int_to_decimal(value.numerator()),
        "denominator": uint_to_decimal(value.denominator()),
    })
}

pub fn dyadic_json(value: &Dyadic) -> Value {
    json!({
        "integer": int_to_decimal(value.integer()),
        "exponent": value.exponent(),
    })
}

pub fn f64_dyadic_json(value: f64) -> Value {
    dyadic_json(&Dyadic::from_f64_exact(value).expect("finite f64"))
}

pub fn surface_terms_json(polynomial: &SurfacePolynomial) -> Value {
    Value::Array(
        polynomial
            .terms()
            .iter()
            .map(|term| {
                json!({
                    "coefficient": rational_json(&term.coefficient),
                    "exponentU": term.exponent_u,
                    "exponentV": term.exponent_v,
                })
            })
            .collect(),
    )
}

pub fn chart_json(chart: &RationalSurfacePolynomials) -> Value {
    json!({
        "x": surface_terms_json(&chart.x),
        "y": surface_terms_json(&chart.y),
        "z": surface_terms_json(&chart.z),
        "denominator": surface_terms_json(&chart.denominator),
    })
}

pub fn interval_json(interval: &ParameterInterval, as_rational: &dyn Fn(&neco_algnum::RealAlgebraic) -> ReducedRational) -> Value {
    json!({
        "lower": rational_json(&as_rational(interval.lower())),
        "upper": rational_json(&as_rational(interval.upper())),
        "lowerClosed": interval.lower_closed(),
        "upperClosed": interval.upper_closed(),
    })
}

pub fn domain_json(domain: &SurfaceParameterDomain, as_rational: &dyn Fn(&neco_algnum::RealAlgebraic) -> ReducedRational) -> Value {
    json!({
        "u": interval_json(domain.u(), as_rational),
        "v": interval_json(domain.v(), as_rational),
    })
}

pub fn rational_terms_json(terms: &[(ReducedRational, u32, u32, u32)]) -> Value {
    Value::Array(
        terms
            .iter()
            .map(|(coefficient, x, y, z)| {
                json!({
                    "coefficient": rational_json(coefficient),
                    "exponentX": x,
                    "exponentY": y,
                    "exponentZ": z,
                })
            })
            .collect(),
    )
}

pub fn point_json(point: &[ReducedRational; 3]) -> Value {
    json!({
        "x": rational_json(&point[0]),
        "y": rational_json(&point[1]),
        "z": rational_json(&point[2]),
    })
}

pub fn object(entries: &[(&str, Value)]) -> Value {
    let mut map = Map::new();
    for (key, value) in entries {
        map.insert(String::from(*key), value.clone());
    }
    Value::Object(map)
}
