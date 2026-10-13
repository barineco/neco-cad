use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, ReducedRational, Sign};
use neco_calculus::SurfacePolynomial;

/// Dense region polynomial over rationals.
pub type RTerm = (ReducedRational, u32, u32, u32);
pub type RPoly = Vec<RTerm>;

/// Dense surface polynomial over rationals.
pub type STerm = (ReducedRational, u32, u32);
pub type SPoly = Vec<STerm>;

#[derive(Debug)]
pub enum AlgebraError {
    Bigint,
    NonRational,
    NotDivisible,
}

impl From<neco_bigint::BigintError> for AlgebraError {
    fn from(_: neco_bigint::BigintError) -> Self {
        Self::Bigint
    }
}

pub fn rat(value: i64) -> Result<ReducedRational, AlgebraError> {
    Ok(ReducedRational::from_bigint(BigInt::try_from(value)?)?)
}

pub fn as_rational(value: &RealAlgebraic) -> Result<ReducedRational, AlgebraError> {
    let coefficients = value.minimal_polynomial().polynomial().coefficients();
    if coefficients.len() != 2 {
        return Err(AlgebraError::NonRational);
    }
    let mut numerator = coefficients[0].negated()?;
    if coefficients[1].sign() == Sign::Negative {
        numerator = numerator.negated()?;
    }
    if coefficients[1].sign() == Sign::Zero {
        return Err(AlgebraError::NonRational);
    }
    let denominator = coefficients[1].magnitude().try_clone()?;
    Ok(neco_bigint::RawRational::new(numerator, denominator)
        .reduce()?
        .into_reduced())
}

pub fn s_normalize(mut terms: SPoly) -> Result<SPoly, AlgebraError> {
    terms.sort_unstable_by_key(|t| (t.1, t.2));
    let mut out: SPoly = Vec::with_capacity(terms.len());
    for (c, u, v) in terms {
        if c.is_zero() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.1 == u && last.2 == v {
                last.0 = last.0.add(&c)?;
                if last.0.is_zero() {
                    out.pop();
                }
                continue;
            }
        }
        out.push((c, u, v));
    }
    Ok(out)
}

pub fn s_clone(a: &SPoly) -> Result<SPoly, AlgebraError> {
    a.iter()
        .map(|(c, u, v)| Ok((c.try_clone()?, *u, *v)))
        .collect()
}

pub fn s_add(a: &SPoly, b: &SPoly) -> Result<SPoly, AlgebraError> {
    let mut terms = s_clone(a)?;
    for (c, u, v) in b {
        terms.push((c.try_clone()?, *u, *v));
    }
    s_normalize(terms)
}

pub fn s_neg(a: &SPoly) -> Result<SPoly, AlgebraError> {
    a.iter()
        .map(|(c, u, v)| -> Result<STerm, AlgebraError> { Ok((rat(0)?.sub(c)?, *u, *v)) })
        .collect()
}

pub fn s_sub(a: &SPoly, b: &SPoly) -> Result<SPoly, AlgebraError> {
    s_add(a, &s_neg(b)?)
}

pub fn s_mul(a: &SPoly, b: &SPoly) -> Result<SPoly, AlgebraError> {
    let mut terms = Vec::with_capacity(a.len() * b.len());
    for (c1, u1, v1) in a {
        for (c2, u2, v2) in b {
            terms.push((c1.mul(c2)?, u1 + u2, v1 + v2));
        }
    }
    s_normalize(terms)
}

pub fn s_pow(a: &SPoly, n: u32) -> Result<SPoly, AlgebraError> {
    let mut acc = vec![(rat(1)?, 0, 0)];
    for _ in 0..n {
        acc = s_mul(&acc, a)?;
    }
    Ok(acc)
}

pub fn from_surface(p: &SurfacePolynomial) -> Result<SPoly, AlgebraError> {
    p.terms()
        .iter()
        .map(|t| Ok((t.coefficient.try_clone()?, t.exponent_u, t.exponent_v)))
        .collect()
}

pub fn r_normalize(mut terms: RPoly) -> Result<RPoly, AlgebraError> {
    terms.sort_unstable_by_key(|t| (t.1, t.2, t.3));
    let mut out: RPoly = Vec::with_capacity(terms.len());
    for (c, x, y, z) in terms {
        if c.is_zero() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.1 == x && last.2 == y && last.3 == z {
                last.0 = last.0.add(&c)?;
                if last.0.is_zero() {
                    out.pop();
                }
                continue;
            }
        }
        out.push((c, x, y, z));
    }
    Ok(out)
}

pub fn r_clone(a: &RPoly) -> Result<RPoly, AlgebraError> {
    a.iter()
        .map(|(c, x, y, z)| Ok((c.try_clone()?, *x, *y, *z)))
        .collect()
}

pub fn r_add(a: &RPoly, b: &RPoly) -> Result<RPoly, AlgebraError> {
    let mut terms = r_clone(a)?;
    for (c, x, y, z) in b {
        terms.push((c.try_clone()?, *x, *y, *z));
    }
    r_normalize(terms)
}

pub fn r_neg(a: &RPoly) -> Result<RPoly, AlgebraError> {
    a.iter()
        .map(|(c, x, y, z)| -> Result<RTerm, AlgebraError> { Ok((rat(0)?.sub(c)?, *x, *y, *z)) })
        .collect()
}

pub fn r_sub(a: &RPoly, b: &RPoly) -> Result<RPoly, AlgebraError> {
    r_add(a, &r_neg(b)?)
}

pub fn r_mul(a: &RPoly, b: &RPoly) -> Result<RPoly, AlgebraError> {
    let mut terms = Vec::with_capacity(a.len() * b.len());
    for (c1, x1, y1, z1) in a {
        for (c2, x2, y2, z2) in b {
            terms.push((c1.mul(c2)?, x1 + x2, y1 + y2, z1 + z2));
        }
    }
    r_normalize(terms)
}

pub fn r_pow(a: &RPoly, n: u32) -> Result<RPoly, AlgebraError> {
    let mut acc = vec![(rat(1)?, 0, 0, 0)];
    for _ in 0..n {
        acc = r_mul(&acc, a)?;
    }
    Ok(acc)
}

pub fn r_monomial(c: ReducedRational, x: u32, y: u32, z: u32) -> RPoly {
    if c.is_zero() {
        Vec::new()
    } else {
        vec![(c, x, y, z)]
    }
}

pub fn r_variable(axis: usize) -> RPoly {
    match axis {
        0 => vec![(rat(1).expect("one"), 1, 0, 0)],
        1 => vec![(rat(1).expect("one"), 0, 1, 0)],
        _ => vec![(rat(1).expect("one"), 0, 0, 1)],
    }
}

pub fn r_evaluate(
    a: &RPoly,
    x: &ReducedRational,
    y: &ReducedRational,
    z: &ReducedRational,
) -> Result<ReducedRational, AlgebraError> {
    let mut acc = rat(0)?;
    for (c, ex, ey, ez) in a {
        let term = c
            .mul(&x.pow_i32(*ex as i32)?)?
            .mul(&y.pow_i32(*ey as i32)?)?
            .mul(&z.pow_i32(*ez as i32)?)?;
        acc = acc.add(&term)?;
    }
    Ok(acc)
}

/// Divide `p` by the monic-in-x² sphere atom; remainder has x-degree ≤ 1.
pub fn r_div_by_quadratic_x(p: &RPoly, atom: &RPoly) -> Result<(RPoly, RPoly), AlgebraError> {
    let lead = atom.iter().find(|t| t.1 == 2 && t.2 == 0 && t.3 == 0);
    let Some((lead_c, _, _, _)) = lead else {
        return Err(AlgebraError::NotDivisible);
    };
    if *lead_c != rat(1)? {
        return Err(AlgebraError::NotDivisible);
    }
    let mut quotient: RPoly = Vec::new();
    let mut remainder = r_clone(p)?;
    loop {
        let candidate = remainder
            .iter()
            .filter(|t| t.1 >= 2)
            .max_by_key(|t| (t.1, t.2, t.3));
        let Some(&(ref c, ex, ey, ez)) = candidate else { break };
        let c = c.try_clone()?;
        let mono = r_monomial(c, ex - 2, ey, ez);
        let sub = r_mul(&mono, atom)?;
        remainder = r_sub(&remainder, &sub)?;
        quotient = r_add(&quotient, &mono)?;
    }
    Ok((r_normalize(quotient)?, r_normalize(remainder)?))
}

/// Homogenized numerator of `P` composed with the two rational args over the
/// region ring; the implicit denominator is the per-axis max-power product.
pub fn compose_surface_by_rational_args(
    p: &SPoly,
    un: &RPoly,
    ud: &RPoly,
    vn: &RPoly,
    vd: &RPoly,
) -> Result<RPoly, AlgebraError> {
    let du = p.iter().map(|t| t.1).max().unwrap_or(0);
    let dv = p.iter().map(|t| t.2).max().unwrap_or(0);
    let mut acc: RPoly = Vec::new();
    for (c, a, b) in p {
        let mut term = r_monomial(c.try_clone()?, 0, 0, 0);
        term = r_mul(&term, &r_pow(un, *a)?)?;
        term = r_mul(&term, &r_pow(vn, *b)?)?;
        term = r_mul(&term, &r_pow(ud, du - a)?)?;
        term = r_mul(&term, &r_pow(vd, dv - b)?)?;
        acc = r_add(&acc, &term)?;
    }
    r_normalize(acc)
}

pub fn surface_max_degrees(p: &SPoly) -> (u32, u32) {
    (
        p.iter().map(|t| t.1).max().unwrap_or(0),
        p.iter().map(|t| t.2).max().unwrap_or(0),
    )
}

/// Bound-form witness: a constant plus guard powers, squares, and an atom
/// multiple.
#[derive(Debug)]
pub struct BoundForm {
    pub constant: ReducedRational,
    pub guard_powers: Vec<(Vec<u32>, ReducedRational)>,
    pub squares: Vec<(ReducedRational, RPoly)>,
    pub atom_quotient: RPoly,
}

/// Verify the bound-form polynomial identity exactly.
pub fn verify_bound_form(
    lhs: &RPoly,
    form: &BoundForm,
    guards: &[RPoly],
    atom: &RPoly,
) -> Result<bool, AlgebraError> {
    let mut rhs = r_monomial(form.constant.try_clone()?, 0, 0, 0);
    for (powers, coefficient) in &form.guard_powers {
        let mut mono = r_monomial(coefficient.try_clone()?, 0, 0, 0);
        for (g, e) in guards.iter().zip(powers.iter()) {
            mono = r_mul(&mono, &r_pow(g, *e)?)?;
        }
        rhs = r_add(&rhs, &mono)?;
    }
    for (coefficient, sq) in &form.squares {
        let term = r_mul(&r_monomial(coefficient.try_clone()?, 0, 0, 0), &r_pow(sq, 2)?)?;
        rhs = r_add(&rhs, &term)?;
    }
    if !form.atom_quotient.is_empty() {
        rhs = r_add(&rhs, &r_mul(&form.atom_quotient, atom)?)?;
    }
    Ok(r_sub(lhs, &rhs)?.is_empty())
}

fn non_negative(value: &ReducedRational) -> bool {
    value.numerator().sign() != Sign::Negative
}

pub fn bound_form_nonnegative(form: &BoundForm) -> bool {
    non_negative(&form.constant)
        && form.guard_powers.iter().all(|(_, c)| non_negative(c))
        && form.squares.iter().all(|(c, _)| non_negative(c))
}

pub fn dyadic_to_rational(value: &neco_bigint::Dyadic) -> Result<ReducedRational, AlgebraError> {
    let denominator = neco_bigint::BigUint::one()?.shl_bits(value.exponent() as usize)?;
    Ok(neco_bigint::RawRational::new(value.integer().try_clone()?, denominator)
        .reduce()?
        .into_reduced())
}

/// Surface-level bound form: a constant plus domain-bound powers and squares.
pub struct SurfaceBoundForm {
    pub constant: ReducedRational,
    pub domain_powers: Vec<([u32; 4], ReducedRational)>,
    pub squares: Vec<(ReducedRational, SPoly)>,
}

pub fn verify_surface_bound_form(
    lhs: &SPoly,
    form: &SurfaceBoundForm,
    domain_bounds: &[SPoly; 4],
) -> Result<bool, AlgebraError> {
    let mut rhs: SPoly = vec![(form.constant.try_clone()?, 0, 0)];
    for (powers, coefficient) in &form.domain_powers {
        let mut mono: SPoly = vec![(coefficient.try_clone()?, 0, 0)];
        for (g, e) in domain_bounds.iter().zip(powers.iter()) {
            mono = s_mul(&mono, &s_pow(g, *e)?)?;
        }
        rhs = s_add(&rhs, &mono)?;
    }
    for (coefficient, sq) in &form.squares {
        let term = s_mul(
            &vec![(coefficient.try_clone()?, 0, 0)],
            &s_pow(sq, 2)?,
        )?;
        rhs = s_add(&rhs, &term)?;
    }
    Ok(s_sub(lhs, &rhs)?.is_empty())
}

/// Domain-bound polys from a domain encoding.
pub fn domain_bound_polys(
    u_lower: &ReducedRational,
    u_upper: &ReducedRational,
    v_lower: &ReducedRational,
    v_upper: &ReducedRational,
) -> Result<[SPoly; 4], AlgebraError> {
    Ok([
        vec![(rat(0)?.sub(u_lower)?, 0, 0), (rat(1)?, 1, 0)],
        vec![(u_upper.try_clone()?, 0, 0), (rat(-1)?, 1, 0)],
        vec![(rat(0)?.sub(v_lower)?, 0, 0), (rat(1)?, 0, 1)],
        vec![(v_upper.try_clone()?, 0, 0), (rat(-1)?, 0, 1)],
    ])
}
