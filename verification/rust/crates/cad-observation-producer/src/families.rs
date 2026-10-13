use neco_bigint::{Dyadic, ReducedRational, Sign};
use neco_cad::*;
use neco_cad::{ChartPatch, FaceExactSurface, RegionFunction, RegionPolynomial3, SignRelation};
use neco_mesh::{
    validate_closed_oriented, validate_surface_mesh, MeshFailure, MeshSource, SurfaceMesh,
};
use serde_json::{json, Value};

use crate::algebra::*;
use crate::encode::*;
use crate::engine::*;

#[derive(Debug)]
pub enum FamilyError {
    Engine(EngineError),
    Algebra(AlgebraError),
    Bigint(neco_bigint::BigintError),
    Unsupported(&'static str),
    Certificate,
}

impl std::fmt::Display for FamilyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(error) => std::fmt::Display::fmt(error, formatter),
            Self::Algebra(error) => write!(formatter, "exact certificate arithmetic: {error:?}"),
            Self::Bigint(error) => write!(formatter, "exact bigint operation: {error:?}"),
            Self::Unsupported(form) => write!(formatter, "unsupported certificate input form: {form}"),
            Self::Certificate => formatter.write_str("constructed certificate failed its exact identity check"),
        }
    }
}

impl From<EngineError> for FamilyError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}
impl From<AlgebraError> for FamilyError {
    fn from(e: AlgebraError) -> Self {
        Self::Algebra(e)
    }
}
impl From<neco_bigint::BigintError> for FamilyError {
    fn from(error: neco_bigint::BigintError) -> Self {
        Self::Bigint(error)
    }
}

type FResult<T> = Result<T, FamilyError>;

fn unsupported<T>(what: &'static str) -> FResult<T> {
    Err(FamilyError::Unsupported(what))
}

fn atom_polynomial(face: &FaceExactSurface) -> FResult<(String, &'static str, RPoly)> {
    let atom = face.exact_surface();
    let relation = match atom.relation() {
        SignRelation::Negative => "negative",
        SignRelation::Zero => "zero",
        SignRelation::Positive => "positive",
    };
    match atom.function() {
        RegionFunction::Polynomial(p) => Ok((
            String::from(atom.atom_id()),
            relation,
            region_poly_to_rpoly(p)?,
        )),
        RegionFunction::RationalFunction(_) => unsupported("rational-function atom"),
    }
}

fn region_poly_to_rpoly(p: &RegionPolynomial3) -> FResult<RPoly> {
    let mut terms = Vec::new();
    for term in p.terms() {
        terms.push((
            as_rational(term.coefficient())?,
            term.exponent_x(),
            term.exponent_y(),
            term.exponent_z(),
        ));
    }
    Ok(r_normalize(terms)?)
}

fn patch_is_disk(patch: &ChartPatch) -> bool {
    matches!(patch, ChartPatch::Full { disk: Some(_), .. })
}

fn domain_json_of(patch: &ChartPatch) -> FResult<Value> {
    let conv = |a: &neco_algnum::RealAlgebraic| -> ReducedRational {
        as_rational(a).expect("rational domain bound")
    };
    Ok(domain_json(patch.domain(), &conv))
}

fn patch_domain_bounds(patch: &ChartPatch) -> FResult<[ReducedRational; 4]> {
    let domain = patch.domain();
    Ok([
        as_rational(domain.u().lower())?,
        as_rational(domain.u().upper())?,
        as_rational(domain.v().lower())?,
        as_rational(domain.v().upper())?,
    ])
}

fn surface_bound_form_json(form: &SurfaceBoundForm) -> Value {
    object(&[
        ("constant", rational_json(&form.constant)),
        (
            "domainPowers",
            Value::Array(
                form.domain_powers
                    .iter()
                    .map(|(powers, coefficient)| {
                        object(&[
                            ("powers", json!(powers)),
                            ("coefficient", rational_json(coefficient)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "squares",
            Value::Array(
                form.squares
                    .iter()
                    .map(|(coefficient, poly)| {
                        object(&[
                            ("coefficient", rational_json(coefficient)),
                            (
                                "polynomial",
                                Value::Array(
                                    poly.iter()
                                        .map(|(c, u, v)| {
                                            object(&[
                                                ("coefficient", rational_json(c)),
                                                ("exponentU", json!(u)),
                                                ("exponentV", json!(v)),
                                            ])
                                        })
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

/// Bound form certifying `chart.denominator(u,v) ≠ 0` on the patch domain box.
fn chart_denominator_form(patch: &ChartPatch) -> FResult<SurfaceBoundForm> {
    let denominator = from_surface(&patch.chart().surface().denominator)?;
    let bounds = patch_domain_bounds(patch)?;
    let domain_bounds = domain_bound_polys(&bounds[0], &bounds[1], &bounds[2], &bounds[3])?;
    let one = rat(1)?;
    let candidate = if denominator.len() == 1 && denominator[0].1 == 0 && denominator[0].2 == 0 {
        SurfaceBoundForm {
            constant: denominator[0].0.try_clone()?,
            domain_powers: Vec::new(),
            squares: Vec::new(),
        }
    } else {
        SurfaceBoundForm {
            constant: one.try_clone()?,
            domain_powers: Vec::new(),
            squares: vec![
                (one.try_clone()?, vec![(one.try_clone()?, 1, 0)]),
                (one.try_clone()?, vec![(one.try_clone()?, 0, 1)]),
            ],
        }
    };
    if !verify_surface_bound_form(&denominator, &candidate, &domain_bounds)?
        || candidate.constant.numerator().sign() != Sign::Positive
    {
        return unsupported("chart denominator bound form");
    }
    Ok(candidate)
}

fn patch_json(patch: &ChartPatch, patch_index: usize) -> FResult<Value> {
    if matches!(patch, ChartPatch::Trimmed { .. }) {
        return unsupported("trimmed patch");
    }
    let inverse = inverse_for(patch)?;
    let denominator_form = chart_denominator_form(patch)?;
    Ok(object(&[
        ("patchIndex", json!(patch_index)),
        ("chart", chart_json(patch.chart().surface())),
        ("domain", domain_json_of(patch)?),
        ("disk", json!(patch_is_disk(patch))),
        (
            "chartDenominatorBound",
            surface_bound_form_json(&denominator_form),
        ),
        (
            "inverse",
            match &inverse {
                Some(inv) => inverse_json(inv),
                None => Value::Null,
            },
        ),
    ]))
}

struct Inverse {
    un: RPoly,
    ud: RPoly,
    vn: RPoly,
    vd: RPoly,
}

fn one_poly() -> RPoly {
    vec![(rat(1).expect("one"), 0, 0, 0)]
}

/// Known inverse rational maps for the engine's chart families, keyed by the
/// chart domain identity. Returns None when the chart family is unknown.
fn inverse_for(patch: &ChartPatch) -> FResult<Option<Inverse>> {
    let domain_id = patch.chart().identity().domain_id().as_str();
    match domain_id {
        "south-from" => Ok(Some(Inverse {
            un: r_variable(0),
            ud: r_add(&one_poly(), &r_variable(2))?,
            vn: r_variable(1),
            vd: r_add(&one_poly(), &r_variable(2))?,
        })),
        "north-from" => Ok(Some(Inverse {
            un: r_variable(0),
            ud: r_sub(&one_poly(), &r_variable(2))?,
            vn: r_variable(1),
            vd: r_sub(&one_poly(), &r_variable(2))?,
        })),
        "affine-plane" => {
            let surface = patch.chart().surface();
            let axes = [&surface.x, &surface.y, &surface.z];
            let mut singles = Vec::new();
            for (axis, poly) in axes.iter().enumerate() {
                let sp = from_surface(poly)?;
                if sp.len() == 1 && sp[0].1 + sp[0].2 == 1 && sp[0].0 == rat(1)? {
                    singles.push((axis, sp[0].1 == 1));
                }
            }
            if singles.len() != 2 || !singles[0].1 || singles[1].1 {
                return Ok(None);
            }
            Ok(Some(Inverse {
                un: r_variable(singles[0].0),
                ud: one_poly(),
                vn: r_variable(singles[1].0),
                vd: one_poly(),
            }))
        }
        _ => Ok(None),
    }
}

fn inverse_json(inverse: &Inverse) -> Value {
    object(&[
        ("uNumerator", rational_terms_json(&inverse.un)),
        ("uDenominator", rational_terms_json(&inverse.ud)),
        ("vNumerator", rational_terms_json(&inverse.vn)),
        ("vDenominator", rational_terms_json(&inverse.vd)),
    ])
}

fn bound_form_json(form: &BoundForm) -> Value {
    object(&[
        ("constant", rational_json(&form.constant)),
        (
            "guardPowers",
            Value::Array(
                form.guard_powers
                    .iter()
                    .map(|(powers, coefficient)| {
                        object(&[
                            ("powers", json!(powers)),
                            ("coefficient", rational_json(coefficient)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "squares",
            Value::Array(
                form.squares
                    .iter()
                    .map(|(coefficient, poly)| {
                        object(&[
                            ("coefficient", rational_json(coefficient)),
                            ("polynomial", rational_terms_json(poly)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("atomQuotient", rational_terms_json(&form.atom_quotient)),
    ])
}

fn form(constant: i64, powers: &[(u32, i64)], atom_quotient: RPoly) -> FResult<BoundForm> {
    Ok(BoundForm {
        constant: rat(constant)?,
        guard_powers: powers
            .iter()
            .map(|(e, c)| Ok((vec![*e], rat(*c)?)))
            .collect::<FResult<Vec<_>>>()?,
        squares: Vec::new(),
        atom_quotient,
    })
}

/// Assert that a constructed bound form actually satisfies its identity.
fn checked_form(
    lhs: &RPoly,
    form: BoundForm,
    guards: &[RPoly],
    atom: &RPoly,
) -> FResult<BoundForm> {
    if !verify_bound_form(lhs, &form, guards, atom)? || !bound_form_nonnegative(&form) {
        return Err(FamilyError::Certificate);
    }
    Ok(form)
}

/// Bound-form certificates for a stereographic clause's single guard.
fn stereographic_forms(
    inverse: &Inverse,
    surface: &neco_calculus::RationalSurfacePolynomials,
    guard: &RPoly,
    atom: &RPoly,
) -> FResult<(BoundForm, BoundForm, BoundForm)> {
    let ud_form = checked_form(
        &inverse.ud,
        form(1, &[(1, 1)], Vec::new())?,
        std::slice::from_ref(guard),
        atom,
    )?;
    let sp_d = from_surface(&surface.denominator)?;
    let (du_d, dv_d) = surface_max_degrees(&sp_d);
    let a_d = compose_surface_by_rational_args(
        &sp_d,
        &inverse.un,
        &inverse.ud,
        &inverse.vn,
        &inverse.vd,
    )?;
    let sq = r_pow(&inverse.ud, 2)?;
    let a_d_form = checked_form(
        &a_d,
        BoundForm {
            constant: rat(2)?,
            guard_powers: vec![
                (vec![1], rat(6)?),
                (vec![2], rat(6)?),
                (vec![3], rat(2)?),
            ],
            squares: Vec::new(),
            atom_quotient: sq,
        },
        std::slice::from_ref(guard),
        atom,
    )?;
    let _ = (du_d, dv_d);
    let lhs = r_sub(
        &r_sub(
            &r_mul(&r_pow(&inverse.ud, 2)?, &r_pow(&inverse.vd, 2)?)?,
            &r_mul(&r_pow(&inverse.un, 2)?, &r_pow(&inverse.vd, 2)?)?,
        )?,
        &r_mul(&r_pow(&inverse.vn, 2)?, &r_pow(&inverse.ud, 2)?)?,
    )?;
    let disk_form = checked_form(
        &lhs,
        BoundForm {
            constant: rat(0)?,
            guard_powers: vec![
                (vec![1], rat(2)?),
                (vec![2], rat(6)?),
                (vec![3], rat(6)?),
                (vec![4], rat(2)?),
            ],
            squares: Vec::new(),
            atom_quotient: r_neg(&r_pow(&inverse.ud, 2)?)?,
        },
        std::slice::from_ref(guard),
        atom,
    )?;
    Ok((ud_form, a_d_form, disk_form))
}

/// For each chart axis, the composed residual must equal a multiple of the
/// atom. Returns the three quotients.
fn image_quotients(
    surface: &neco_calculus::RationalSurfacePolynomials,
    inverse: &Inverse,
    atom: &RPoly,
) -> FResult<(RPoly, RPoly, RPoly)> {
    let sp_x = from_surface(&surface.x)?;
    let sp_y = from_surface(&surface.y)?;
    let sp_z = from_surface(&surface.z)?;
    let sp_d = from_surface(&surface.denominator)?;

    let composed = |p: &SPoly| -> FResult<(RPoly, RPoly)> {
        let (du, dv) = surface_max_degrees(p);
        let numerator = compose_surface_by_rational_args(
            p,
            &inverse.un,
            &inverse.ud,
            &inverse.vn,
            &inverse.vd,
        )?;
        let denominator = r_mul(&r_pow(&inverse.ud, du)?, &r_pow(&inverse.vd, dv)?)?;
        Ok((numerator, denominator))
    };

    let (a_d, d_d) = composed(&sp_d)?;
    let mut quotients = Vec::new();
    for (axis, sp) in [&sp_x, &sp_y, &sp_z].iter().enumerate() {
        let (a_axis, d_axis) = composed(sp)?;
        let residual = r_sub(
            &r_mul(&a_axis, &d_d)?,
            &r_mul(&r_variable(axis), &r_mul(&a_d, &d_axis)?)?,
        )?;
        let quotient = if residual.is_empty() {
            Vec::new()
        } else {
            let (q, rem) = r_div_by_quadratic_x(&residual, atom)?;
            if !rem.is_empty() {
                return Err(FamilyError::Certificate);
            }
            q
        };
        quotients.push(quotient);
    }
    let qz = quotients.pop().expect("z");
    let qy = quotients.pop().expect("y");
    let qx = quotients.pop().expect("x");
    Ok((qx, qy, qz))
}

fn mesh_json(mesh: &SurfaceMesh) -> Value {
    object(&[
        (
            "vertices",
            Value::Array(
                mesh.vertices()
                    .iter()
                    .map(|v| {
                        object(&[
                            ("x", f64_dyadic_json(v[0])),
                            ("y", f64_dyadic_json(v[1])),
                            ("z", f64_dyadic_json(v[2])),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("triangles", json!(mesh.triangles())),
    ])
}

/// `exact-chart` case: emit the atom and chart of a selected face patch.
pub fn exact_chart(input: &Value) -> FResult<Value> {
    let family = input
        .get("surfaceFamily")
        .and_then(Value::as_str)
        .unwrap_or("");
    let derived = chart_derived(input)?;
    let surfaces = derived.face_surfaces();
    let (face, patch_index) = match family {
        "plane" => (surfaces.first().ok_or(EngineError::MissingSurface)?, 0usize),
        "sphere-south" => (surfaces.first().ok_or(EngineError::MissingSurface)?, 0usize),
        "sphere-north" => (surfaces.first().ok_or(EngineError::MissingSurface)?, 1usize),
        _ => return unsupported("surfaceFamily"),
    };
    let (atom_id, atom_relation, atom) = atom_polynomial(face)?;
    let patch = &face.covering().patches()[patch_index];
    Ok(object(&[
        ("faceId", json!(face.face_id())),
        ("atomId", json!(atom_id)),
        ("atomRelation", json!(atom_relation)),
        ("patchIndex", json!(patch_index)),
        ("atom", rational_terms_json(&atom)),
        ("chart", chart_json(patch.chart().surface())),
        ("domain", domain_json_of(patch)?),
        ("disk", json!(patch_is_disk(patch))),
        (
            "chartDenominatorBound",
            surface_bound_form_json(&chart_denominator_form(patch)?),
        ),
    ]))
}

/// `face-atom-covering` case: emit the covering with inverse charts and the
/// clause certificates for the unit sphere face.
pub fn face_atom_covering(input: &Value) -> FResult<Value> {
    let derived = sphere_derived(revision(input)?)?;
    let surfaces = derived.face_surfaces();
    let face = surfaces.first().ok_or(EngineError::MissingSurface)?;
    let (atom_id, atom_relation, atom) = atom_polynomial(face)?;
    let original_patches = face.covering().patches();
    let first = input["firstPatch"].as_u64().ok_or(FamilyError::Unsupported("firstPatch"))? as usize;
    if first >= original_patches.len() {
        return unsupported("firstPatch");
    }
    let patches: Vec<_> = original_patches.iter().cycle().skip(first)
        .take(original_patches.len()).collect();

    let mut patches_json = Vec::new();
    let mut inverses = Vec::new();
    for (index, patch) in patches.iter().enumerate() {
        let inverse = inverse_for(patch)?.ok_or(FamilyError::Unsupported(
            "unknown chart family for inverse",
        ))?;
        patches_json.push(patch_json(patch, index)?);
        inverses.push(inverse);
    }

    let mut clauses = Vec::new();
    for (index, patch) in patches.iter().enumerate() {
        let inverse = &inverses[index];
        let domain_id = patch.chart().identity().domain_id().as_str();
        let guard = match domain_id {
            "south-from" => r_variable(2),
            "north-from" => r_neg(&r_variable(2))?,
            _ => return unsupported("clause guard"),
        };
        let (ud_form, a_d_form, disk_form) = stereographic_forms(
            inverse,
            patch.chart().surface(),
            &guard,
            &atom,
        )?;
        let (qx, qy, qz) = image_quotients(patch.chart().surface(), inverse, &atom)?;
        clauses.push(object(&[
            ("patch", json!(index)),
            ("guards", json!([rational_terms_json(&guard)])),
            (
                "inverseDenominatorBounds",
                object(&[
                    ("u", bound_form_json(&ud_form)),
                    ("v", bound_form_json(&ud_form)),
                ]),
            ),
            ("chartDenominatorBound", bound_form_json(&a_d_form)),
            ("diskRange", bound_form_json(&disk_form)),
            (
                "imageQuotients",
                object(&[
                    ("x", rational_terms_json(&qx)),
                    ("y", rational_terms_json(&qy)),
                    ("z", rational_terms_json(&qz)),
                ]),
            ),
        ]));
    }

    let coverage_form = BoundForm {
        constant: rat(0)?,
        guard_powers: Vec::new(),
        squares: Vec::new(),
        atom_quotient: Vec::new(),
    };
    if !verify_bound_form(&Vec::new(), &coverage_form, &[], &atom)? {
        return Err(FamilyError::Certificate);
    }

    Ok(object(&[
        ("faceId", json!(face.face_id())),
        ("atomId", json!(atom_id)),
        ("atomRelation", json!(atom_relation)),
        ("atom", rational_terms_json(&atom)),
        ("outerGuards", json!([])),
        ("patches", Value::Array(patches_json)),
        ("clauses", Value::Array(clauses)),
        (
            "coverage",
            object(&[("selections", json!([bound_form_json(&coverage_form)]))]),
        ),
    ]))
}

/// `finite-surface-representation` case: run adaptive tessellation of the
/// unit sphere and emit the mesh with per-face atoms and residuals.
/// Each face residual is the exact maximum of |atom| over the face's triangle vertices.
pub fn finite_surface(input: &Value) -> FResult<Value> {
    let tessellation = sphere_tessellation(input)?;
    let mesh = tessellation.validated_surface_mesh().mesh();

    let derived = sphere_derived(revision(input)?)?;
    let surfaces = derived.face_surfaces();
    let mut face_entries = Vec::new();
    for tessellated in tessellation.face_triangle_correspondence() {
        let face = surfaces
            .iter()
            .find(|f| f.face_id() == tessellated.face_id())
            .ok_or(EngineError::MissingFace)?;
        let (atom_id, atom_relation, atom) = atom_polynomial(face)?;
        let mut max_num = neco_bigint::BigUint::zero();
        let mut max_den = neco_bigint::BigUint::one()?;
        for &tri in tessellated.triangle_indices() {
            for &vi in &mesh.triangles()[tri] {
                let v = &mesh.vertices()[vi];
                let val = r_evaluate(
                    &atom,
                    &dyadic_to_rational(&Dyadic::from_f64_exact(v[0]).expect("dyadic"))?,
                    &dyadic_to_rational(&Dyadic::from_f64_exact(v[1]).expect("dyadic"))?,
                    &dyadic_to_rational(&Dyadic::from_f64_exact(v[2]).expect("dyadic"))?,
                )?;
                let num = val.numerator().magnitude().try_clone()?;
                let den = val.denominator().try_clone()?;
                let lhs = num.mul(&max_den)?;
                let rhs = max_num.mul(&den)?;
                if lhs.cmp(&rhs) == core::cmp::Ordering::Greater {
                    max_num = num;
                    max_den = den;
                }
            }
        }
        let bound = neco_bigint::RawRational::new(
            neco_bigint::BigInt::from_sign_magnitude(neco_bigint::Sign::Positive, max_num),
            max_den,
        )
        .reduce()?
        .into_reduced()
        .dyadic_ceil(48)?;
        let mut patches_json = Vec::new();
        for (index, patch) in face.covering().patches().iter().enumerate() {
            patches_json.push(patch_json(patch, index)?);
        }
        let mut vertex_parameters = Vec::new();
        let mut seen_parameters = std::collections::BTreeSet::new();
        for witness in tessellation.chord_bound_witnesses() {
            if witness.face_id() != tessellated.face_id() {
                continue;
            }
            for edge in witness.edges() {
                for endpoint in [edge.left(), edge.right()] {
                    let key = (
                        edge.patch_index(),
                        rational_json(endpoint.u()).to_string()
                            + &rational_json(endpoint.v()).to_string(),
                    );
                    if seen_parameters.insert(key) {
                        vertex_parameters.push(object(&[
                            ("patchIndex", json!(edge.patch_index())),
                            ("u", rational_json(endpoint.u())),
                            ("v", rational_json(endpoint.v())),
                            ("point", point_json(endpoint.point())),
                        ]));
                    }
                }
            }
        }
        face_entries.push(object(&[
            ("faceId", json!(tessellated.face_id())),
            ("atomId", json!(atom_id)),
            ("atomRelation", json!(atom_relation)),
            ("atom", rational_terms_json(&atom)),
            ("triangleIndices", json!(tessellated.triangle_indices())),
            ("patches", Value::Array(patches_json)),
            ("vertexParameters", Value::Array(vertex_parameters)),
            ("residualBound", dyadic_json(&bound)),
        ]));
    }

    let precision = tessellation.precision_witness();
    Ok(object(&[
        ("identity", json!(tessellation.identity().as_str())),
        (
            "sourceRevision",
            json!(tessellation.source_revision().as_str()),
        ),
        ("mesh", mesh_json(mesh)),
        ("faceTriangles", Value::Array(face_entries)),
        (
            "precision",
            object(&[
                (
                    "requestedChordError",
                    f64_dyadic_json(precision.requested_maximum_chord_error()),
                ),
                (
                    "establishedChordError",
                    f64_dyadic_json(precision.established_maximum_chord_error()),
                ),
                (
                    "requestedSurfaceError",
                    f64_dyadic_json(precision.requested_maximum_surface_error()),
                ),
                (
                    "establishedSurfaceError",
                    dyadic_json(precision.established_maximum_surface_error()),
                ),
            ]),
        ),
    ]))
}

/// Univariate edge parameterization for the chord-bound checker: line edges interpolate on
/// `t ∈ [0,1]`; arc edges use the rational circle parameterization from the pole that the arc,
/// swept in the direction given by the sign of cross(left, right), does not contain.
fn edge_parameterization(edge: &ChordBoundEdgeEvaluation) -> FResult<Value> {
    if edge.shape().arc_box().is_none() {
        return Ok(object(&[
            ("kind", json!("line")),
            ("tLower", rational_json(&rat(0)?)),
            ("tUpper", rational_json(&rat(1)?)),
        ]));
    }
    let left = edge.left();
    let right = edge.right();
    let zero = rat(0)?;
    let one = rat(1)?;
    let sweep = left.u().mul(right.v())?.sub(&left.v().mul(right.u())?)?;
    if sweep.is_zero() {
        return unsupported("antipodal arc parameterization");
    }
    let dir_pos = sweep.numerator().sign() == Sign::Positive;
    let contains = |pu: &ReducedRational, pv: &ReducedRational| -> FResult<bool> {
        let before = left.u().mul(pv)?.sub(&left.v().mul(pu)?)?;
        let after = pu.mul(right.v())?.sub(&pv.mul(right.u())?)?;
        let ok = |s: &ReducedRational| {
            s.is_zero() || (s.numerator().sign() == Sign::Positive) == dir_pos
        };
        Ok(ok(&before) && ok(&after))
    };
    let mut minus = left.u().add(right.u())?.cmp(&zero) == core::cmp::Ordering::Less;
    let neg_one = zero.sub(&one)?;
    if minus && contains(&one, &zero)? {
        minus = false;
    } else if !minus && contains(&neg_one, &zero)? {
        minus = true;
    }
    let tau = |u: &ReducedRational, v: &ReducedRational| -> FResult<ReducedRational> {
        if minus {
            Ok(v.div(&one.sub(u)?)?)
        } else {
            Ok(v.div(&one.add(u)?)?)
        }
    };
    let tau_left = tau(left.u(), left.v())?;
    let tau_right = tau(right.u(), right.v())?;
    let (lower, upper) = if tau_left.cmp(&tau_right) == core::cmp::Ordering::Greater {
        (tau_right, tau_left)
    } else {
        (tau_left, tau_right)
    };
    Ok(object(&[
        ("kind", json!("arc")),
        ("branch", json!(if minus { "minusPole" } else { "plusPole" })),
        ("tLower", rational_json(&lower)),
        ("tUpper", rational_json(&upper)),
    ]))
}

/// `certified-chord-bound` case: emit the chord-bound witness graph for one
/// face of the sphere tessellation.
pub fn chord_bound(input: &Value) -> FResult<Value> {
    let tessellation = sphere_tessellation(input)?;
    let witnesses = tessellation.chord_bound_witnesses();
    let witness = witnesses.first().ok_or(EngineError::MissingFace)?;
    let derived = sphere_derived(revision(input)?)?;
    let surfaces = derived.face_surfaces();
    let face = surfaces
        .iter()
        .find(|f| f.face_id() == witness.face_id())
        .ok_or(EngineError::MissingFace)?;
    let (atom_id, atom_relation, _atom) = atom_polynomial(face)?;

    let mut patches_json = Vec::new();
    for (index, patch) in face.covering().patches().iter().enumerate() {
        patches_json.push(patch_json(patch, index)?);
    }

    let endpoint_json = |e: &ChordBoundEndpoint| -> Value {
        object(&[
            ("u", rational_json(e.u())),
            ("v", rational_json(e.v())),
            ("point", point_json(e.point())),
        ])
    };

    let mut edges = Vec::new();
    for edge in witness.edges() {
        let shape = match edge.shape() {
            ChordBoundEdgeShape::Line => object(&[("kind", json!("line"))]),
            ChordBoundEdgeShape::Arc(range) => object(&[
                ("kind", json!("arc")),
                ("uLower", rational_json(range.u_lower())),
                ("uUpper", rational_json(range.u_upper())),
                ("vLower", rational_json(range.v_lower())),
                ("vUpper", rational_json(range.v_upper())),
            ]),
        };
        let max_subdivision_depth = match edge.shape() {
            ChordBoundEdgeShape::Line => 4,
            ChordBoundEdgeShape::Arc(_) => 8,
        };
        edges.push(object(&[
            ("patchIndex", json!(edge.patch_index())),
            ("left", endpoint_json(edge.left())),
            ("right", endpoint_json(edge.right())),
            ("midpointU", rational_json(edge.midpoint_u())),
            ("midpointV", rational_json(edge.midpoint_v())),
            ("shape", shape),
            (
                "parameterization",
                edge_parameterization(edge)?,
            ),
            ("subdivisionDepth", json!(max_subdivision_depth)),
            (
                "establishedSquaredBound",
                match edge.squared_bound() {
                    Some(d) => dyadic_json(d),
                    None => Value::Null,
                },
            ),
        ]));
    }
    let reverse_edges = input["edgeOrder"].as_u64() == Some(1);
    if reverse_edges {
        edges.reverse();
    }
    let triangles: Vec<Value> = witness
        .triangles()
        .iter()
        .map(|t| {
            object(&[
                ("patchIndex", json!(t.patch_index())),
                ("edgeIndices", json!(t.edge_indices().map(|index| {
                    if reverse_edges { edges.len() - 1 - index } else { index }
                }))),
            ])
        })
        .collect();

    Ok(object(&[
        ("faceId", json!(witness.face_id())),
        ("atomId", json!(atom_id)),
        ("atomRelation", json!(atom_relation)),
        ("patches", Value::Array(patches_json)),
        ("edges", Value::Array(edges)),
        ("triangles", Value::Array(triangles)),
        (
            "establishedSquaredBound",
            match witness.established_squared_bound() {
                Some(d) => dyadic_json(d),
                None => Value::Null,
            },
        ),
        (
            "requestedChordError",
            f64_dyadic_json(
                tessellation
                    .precision_witness()
                    .requested_maximum_chord_error(),
            ),
        ),
    ]))
}

/// `mesh-judgment` case: build the requested mesh and run the mesh validators.
pub fn mesh_judgment(input: &Value) -> FResult<Value> {
    let mesh_name = input
        .get("mesh")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (mut vertices, mut triangles): (Vec<[f64; 3]>, Vec<[usize; 3]>) = match mesh_name {
        "tetrahedron" | "reversed-face-tetrahedron" | "degenerate-tetrahedron" => (
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        ),
        "open-tetrahedron" => (
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            vec![[0, 2, 1], [0, 1, 3], [0, 3, 2]],
        ),
        _ => return unsupported("mesh name"),
    };
    if mesh_name == "reversed-face-tetrahedron" {
        triangles[0].swap(0, 1);
    } else if mesh_name == "degenerate-tetrahedron" {
        vertices[1] = vertices[0];
    }
    let scale = input["scale"].as_f64().filter(|s| *s > 0.0)
        .ok_or(FamilyError::Unsupported("scale"))?;
    let translation = input["translation"].as_array()
        .ok_or(FamilyError::Unsupported("translation"))?;
    if translation.len() != 3 {
        return unsupported("translation");
    }
    for vertex in &mut vertices {
        for (coordinate, offset) in vertex.iter_mut().zip(translation) {
            *coordinate = *coordinate * scale
                + offset.as_f64().ok_or(FamilyError::Unsupported("translation"))?;
        }
    }
    let mesh = SurfaceMesh::new(
        vertices,
        triangles,
        Vec::new(),
    )
    .map_err(|_| FamilyError::Certificate)?;
    let mesh_value = mesh_json(&mesh);

    let validated = match validate_surface_mesh(mesh) {
        Ok(v) => v,
        Err(failure) => {
            return Ok(mesh_failure_observation(mesh_value, &failure));
        }
    };
    let source = MeshSource::new(
        String::from("cad-verification-mesh"),
        String::from(revision(input)?),
    )
    .map_err(|_| FamilyError::Certificate)?;
    match validate_closed_oriented(validated, source) {
        Ok(closed) => {
            let pairs: Vec<Value> = closed
                .examined_triangle_pairs()
                .iter()
                .map(|p| {
                    object(&[
                        ("first", json!(p.first_triangle())),
                        ("second", json!(p.second_triangle())),
                        ("sharedIndexCount", json!(p.shared_index_count())),
                        (
                            "kind",
                            json!(match p.intersection_kind() {
                                neco_mesh::AllowedSharedIndexIntersectionKind::Empty => "empty",
                                neco_mesh::AllowedSharedIndexIntersectionKind::SharedVertex =>
                                    "sharedVertex",
                                neco_mesh::AllowedSharedIndexIntersectionKind::SharedEdgeSegment =>
                                    "sharedEdgeSegment",
                            }),
                        ),
                    ])
                })
                .collect();
            Ok(object(&[
                ("mesh", mesh_value),
                (
                    "outcome",
                    object(&[
                        ("kind", json!("validated")),
                        (
                            "closure",
                            object(&[(
                                "edgeCount",
                                json!(closed.closure_witness().edge_count()),
                            )]),
                        ),
                        (
                            "orientation",
                            object(&[(
                                "edgeCount",
                                json!(closed.orientation_witness().edge_count()),
                            )]),
                        ),
                        (
                            "manifold",
                            object(&[(
                                "edgeCount",
                                json!(closed.manifold_witness().edge_count()),
                            )]),
                        ),
                        ("examinedPairs", Value::Array(pairs)),
                    ]),
                ),
            ]))
        }
        Err(failure) => Ok(mesh_failure_observation(mesh_value, &failure)),
    }
}

fn mesh_failure_observation(mesh_value: Value, failure: &MeshFailure) -> Value {
    let (kind, detail) = match failure {
        MeshFailure::AttributeCount {
            attribute_count,
            triangle_count,
        } => ("attributeCount", json!({"attributeCount": attribute_count, "triangleCount": triangle_count})),
        MeshFailure::NonFiniteVertex { vertex_index } => {
            ("nonFiniteVertex", json!({"vertexIndex": vertex_index}))
        }
        MeshFailure::TriangleIndexOutOfBounds {
            triangle_index,
            vertex_index,
        } => (
            "triangleIndexOutOfBounds",
            json!({"triangleIndex": triangle_index, "vertexIndex": vertex_index}),
        ),
        MeshFailure::DegenerateTriangle { triangle_index } => {
            ("degenerateTriangle", json!({"triangleIndex": triangle_index}))
        }
        MeshFailure::OpenEdge {
            first_vertex,
            second_vertex,
        } => (
            "openEdge",
            json!({"firstVertex": first_vertex, "secondVertex": second_vertex}),
        ),
        MeshFailure::NonManifoldEdge {
            first_vertex,
            second_vertex,
            incidence_count,
        } => (
            "nonManifoldEdge",
            json!({"firstVertex": first_vertex, "secondVertex": second_vertex, "incidenceCount": incidence_count}),
        ),
        MeshFailure::OrientationMismatch {
            first_vertex,
            second_vertex,
        } => (
            "orientationMismatch",
            json!({"firstVertex": first_vertex, "secondVertex": second_vertex}),
        ),
        MeshFailure::EmptySourceIdentity => ("emptySourceIdentity", json!({})),
        MeshFailure::EmptySourceRevision => ("emptySourceRevision", json!({})),
        MeshFailure::SelfIntersection(witness) => (
            "selfIntersection",
            json!({
                "first": witness.first_triangle(),
                "second": witness.second_triangle(),
                "intersectionKind": match witness.kind() {
                    neco_mesh::MeshPairIntersectionKind::NonPermittedCrossing => "nonPermittedCrossing",
                    neco_mesh::MeshPairIntersectionKind::CoplanarOverlap => "coplanarOverlap",
                    neco_mesh::MeshPairIntersectionKind::DistinctIndexCoincidentContact => "distinctIndexCoincidentContact",
                    neco_mesh::MeshPairIntersectionKind::DuplicateFace => "duplicateFace",
                    neco_mesh::MeshPairIntersectionKind::Indeterminate => "indeterminate",
                    _ => "other",
                },
            }),
        ),
    };
    object(&[
        ("mesh", mesh_value),
        (
            "outcome",
            object(&[
                ("kind", json!("rejected")),
                ("failureKind", json!(kind)),
                ("failure", detail),
            ]),
        ),
    ])
}
