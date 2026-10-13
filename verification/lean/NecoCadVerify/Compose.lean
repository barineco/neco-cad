import NecoCadVerify.FaceAtomCovering
import NecoCadVerify.ChordBound

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

-- ============================================================
-- composeAtomNumerator evaluation
-- ============================================================

private theorem composedChartTerm_eval (term : NecoGeometryVerify.RegionTerm) (chart : ChartPolynomials)
    (total : Nat) (u v : Rat) :
    surfaceValue (NecoCalculusVerify.SurfacePolynomial.scale term.coefficient
      (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.x term.exponentX)
        (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.y term.exponentY)
          (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.z term.exponentZ)
            (NecoCalculusVerify.SurfacePolynomial.pow chart.denominator
              (total - term.exponentX - term.exponentY - term.exponentZ)))))) u v =
    term.coefficient * surfaceValue chart.x u v ^ term.exponentX *
      surfaceValue chart.y u v ^ term.exponentY *
      surfaceValue chart.z u v ^ term.exponentZ *
      surfaceValue chart.denominator u v ^
        (total - term.exponentX - term.exponentY - term.exponentZ) := by
  rw [NecoCalculusVerify.SurfacePolynomial.scale_eval, NecoCalculusVerify.SurfacePolynomial.mul_eval, NecoCalculusVerify.SurfacePolynomial.mul_eval, NecoCalculusVerify.SurfacePolynomial.mul_eval,
    NecoCalculusVerify.SurfacePolynomial.pow_eval, NecoCalculusVerify.SurfacePolynomial.pow_eval, NecoCalculusVerify.SurfacePolynomial.pow_eval, NecoCalculusVerify.SurfacePolynomial.pow_eval]
  ring

private theorem composedChartFoldl_eval (terms : List NecoGeometryVerify.RegionTerm)
    (chart : ChartPolynomials) (total : Nat) (u v : Rat) :
    ∀ acc, surfaceValue (terms.foldl (fun acc term =>
        NecoCalculusVerify.SurfacePolynomial.add acc (NecoCalculusVerify.SurfacePolynomial.scale term.coefficient
          (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.x term.exponentX)
            (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.y term.exponentY)
              (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.z term.exponentZ)
                (NecoCalculusVerify.SurfacePolynomial.pow chart.denominator
                  (total - term.exponentX - term.exponentY - term.exponentZ)))))))
        acc) u v =
      surfaceValue acc u v + (terms.map fun term =>
        term.coefficient * surfaceValue chart.x u v ^ term.exponentX *
          surfaceValue chart.y u v ^ term.exponentY *
          surfaceValue chart.z u v ^ term.exponentZ *
          surfaceValue chart.denominator u v ^
            (total - term.exponentX - term.exponentY - term.exponentZ)).sum := by
  induction terms with
  | nil => intro acc; simp
  | cons term rest ih =>
      intro acc
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, NecoCalculusVerify.SurfacePolynomial.add_eval, composedChartTerm_eval]
      ring

/-- Evaluation of the homogenized composition numerator:
    `N(u,v) = atom(χ(u,v)) · d(u,v)^T` whenever `d(u,v) ≠ 0`. -/
theorem composeAtomNumerator_eval (atom : (List NecoGeometryVerify.RegionTerm)) (chart : ChartPolynomials)
    (u v : Rat) (hd : surfaceValue chart.denominator u v ≠ 0) :
    surfaceValue (composeAtomNumerator atom chart) u v =
      NecoGeometryVerify.RegionPolynomial.eval atom (chartPoint chart u v).1 (chartPoint chart u v).2.1
        (chartPoint chart u v).2.2 *
        surfaceValue chart.denominator u v ^
          (atom.foldl (fun d t => max d
            (t.exponentX + t.exponentY + t.exponentZ)) 0) := by
  set total := atom.foldl (fun d t => max d
    (t.exponentX + t.exponentY + t.exponentZ)) 0
  have hbound : ∀ t ∈ atom, t.exponentX + t.exponentY + t.exponentZ ≤ total :=
    fun t ht => foldl_max_ge_mem
      (fun t => t.exponentX + t.exponentY + t.exponentZ) atom t ht 0
  unfold composeAtomNumerator
  rw [NecoCalculusVerify.SurfacePolynomial.normalize_eval]
  have hfold := composedChartFoldl_eval atom chart total u v []
  rw [hfold]
  have hz : surfaceValue ([] : List SurfaceTerm) u v = 0 := rfl
  rw [hz, zero_add, NecoGeometryVerify.RegionPolynomial.eval_sum]
  unfold chartPoint
  dsimp only
  set xv := surfaceValue chart.x u v
  set yv := surfaceValue chart.y u v
  set zv := surfaceValue chart.z u v
  set dv := surfaceValue chart.denominator u v
  rw [mul_comm ((atom.map _).sum) (dv ^ total)]
  have hmul := NecoGeometryVerify.RegionPolynomial.map_sum_mul_left
    (fun t => NecoGeometryVerify.RegionPolynomial.monomial t (xv / dv) (yv / dv)
      (zv / dv)) (dv ^ total) atom
  rw [hmul]
  apply congrArg List.sum
  apply List.map_congr_left
  intro t ht
  have htot := hbound t ht
  have hpow : dv ^ total = dv ^ t.exponentX * dv ^ t.exponentY * dv ^ t.exponentZ *
      dv ^ (total - t.exponentX - t.exponentY - t.exponentZ) := by
    rw [← pow_add, ← pow_add, ← pow_add]
    congr 1
    omega
  simp only [NecoGeometryVerify.RegionPolynomial.monomial]
  rw [hpow, div_pow, div_pow, div_pow]
  have hdx : dv ^ t.exponentX ≠ 0 := pow_ne_zero _ hd
  have hdy : dv ^ t.exponentY ≠ 0 := pow_ne_zero _ hd
  have hdz : dv ^ t.exponentZ ≠ 0 := pow_ne_zero _ hd
  field_simp

theorem composeAtomNumerator_exactValue_eval
    (atom : List NecoGeometryVerify.RegionTerm) (chart : ChartPolynomials)
    (u v : Rat) (point : Vec3)
    (nonzero : surfaceValue chart.denominator u v ≠ 0)
    (observed : NecoGeometryVerify.surfaceExactValue chart.x chart.y chart.z
      chart.denominator u v = some point) :
    surfaceValue (composeAtomNumerator atom chart) u v =
      NecoGeometryVerify.RegionPolynomial.eval atom point.1 point.2.1 point.2.2 *
        surfaceValue chart.denominator u v ^
          (atom.foldl (fun d t => max d
            (t.exponentX + t.exponentY + t.exponentZ)) 0) := by
  have pointEq : chartPoint chart u v = point :=
    Option.some.inj ((chartPoint_exactValue chart u v nonzero).symm.trans observed)
  rw [composeAtomNumerator_eval atom chart u v nonzero, pointEq]

-- ============================================================
-- composeAtomArgs evaluation
-- ============================================================

private theorem composedArgsTerm_eval (term : SurfaceTerm) (un ud vn vd : (List NecoGeometryVerify.RegionTerm))
    (degreeU degreeV : Nat) (x y z : Rat) :
    NecoGeometryVerify.RegionPolynomial.eval (NecoGeometryVerify.RegionPolynomial.rawScale term.coefficient
      (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow un term.exponentU)
        (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow vn term.exponentV)
          (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow ud (degreeU - term.exponentU))
            (NecoGeometryVerify.RegionPolynomial.pow vd (degreeV - term.exponentV)))))) x y z =
    term.coefficient * NecoGeometryVerify.RegionPolynomial.eval un x y z ^ term.exponentU *
      NecoGeometryVerify.RegionPolynomial.eval vn x y z ^ term.exponentV *
      NecoGeometryVerify.RegionPolynomial.eval ud x y z ^ (degreeU - term.exponentU) *
      NecoGeometryVerify.RegionPolynomial.eval vd x y z ^ (degreeV - term.exponentV) := by
  rw [NecoGeometryVerify.RegionPolynomial.rawScale_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, NecoGeometryVerify.RegionPolynomial.mul_eval,
    NecoGeometryVerify.RegionPolynomial.pow_eval, NecoGeometryVerify.RegionPolynomial.pow_eval, NecoGeometryVerify.RegionPolynomial.pow_eval, NecoGeometryVerify.RegionPolynomial.pow_eval]
  ring

private theorem composedArgsFoldl_eval (terms : List SurfaceTerm)
    (un ud vn vd : (List NecoGeometryVerify.RegionTerm)) (degreeU degreeV : Nat) (x y z : Rat) :
    ∀ acc, NecoGeometryVerify.RegionPolynomial.eval (terms.foldl (fun acc term =>
        NecoGeometryVerify.RegionPolynomial.add acc (NecoGeometryVerify.RegionPolynomial.rawScale term.coefficient
          (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow un term.exponentU)
            (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow vn term.exponentV)
              (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow ud (degreeU - term.exponentU))
                (NecoGeometryVerify.RegionPolynomial.pow vd (degreeV - term.exponentV))))))) acc) x y z =
      NecoGeometryVerify.RegionPolynomial.eval acc x y z + (terms.map fun term =>
        term.coefficient * NecoGeometryVerify.RegionPolynomial.eval un x y z ^ term.exponentU *
          NecoGeometryVerify.RegionPolynomial.eval vn x y z ^ term.exponentV *
          NecoGeometryVerify.RegionPolynomial.eval ud x y z ^ (degreeU - term.exponentU) *
          NecoGeometryVerify.RegionPolynomial.eval vd x y z ^ (degreeV - term.exponentV)).sum := by
  induction terms with
  | nil => intro acc; simp
  | cons term rest ih =>
      intro acc
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, NecoGeometryVerify.RegionPolynomial.add_eval, composedArgsTerm_eval]
      ring

/-- Evaluation of `composeAtomArgs`: with `ud·vd ≠ 0` at the point, the
    composed fraction equals the surface polynomial at `(un/ud, vn/vd)`. -/
theorem composeAtomArgs_eval (terms : List SurfaceTerm) (un ud vn vd : (List NecoGeometryVerify.RegionTerm))
    (x y z : Rat)
    (hud : NecoGeometryVerify.RegionPolynomial.eval ud x y z ≠ 0) (hvd : NecoGeometryVerify.RegionPolynomial.eval vd x y z ≠ 0) :
    NecoGeometryVerify.RegionPolynomial.eval (composeAtomArgs terms un ud vn vd).2 x y z ≠ 0 ∧
      NecoGeometryVerify.RegionPolynomial.eval (composeAtomArgs terms un ud vn vd).1 x y z /
          NecoGeometryVerify.RegionPolynomial.eval (composeAtomArgs terms un ud vn vd).2 x y z =
        surfaceValue terms
          (NecoGeometryVerify.RegionPolynomial.eval un x y z / NecoGeometryVerify.RegionPolynomial.eval ud x y z)
          (NecoGeometryVerify.RegionPolynomial.eval vn x y z / NecoGeometryVerify.RegionPolynomial.eval vd x y z) := by
  set degreeU := terms.foldl (fun d t => max d t.exponentU) 0
  set degreeV := terms.foldl (fun d t => max d t.exponentV) 0
  have hboundU : ∀ t ∈ terms, t.exponentU ≤ degreeU :=
    fun t ht => foldl_max_ge_mem (fun t => t.exponentU) terms t ht 0
  have hboundV : ∀ t ∈ terms, t.exponentV ≤ degreeV :=
    fun t ht => foldl_max_ge_mem (fun t => t.exponentV) terms t ht 0
  unfold composeAtomArgs
  dsimp only
  have hfold := composedArgsFoldl_eval terms un ud vn vd degreeU degreeV x y z []
  set unv := NecoGeometryVerify.RegionPolynomial.eval un x y z
  set udv := NecoGeometryVerify.RegionPolynomial.eval ud x y z
  set vnv := NecoGeometryVerify.RegionPolynomial.eval vn x y z
  set vdv := NecoGeometryVerify.RegionPolynomial.eval vd x y z
  rw [NecoGeometryVerify.RegionPolynomial.mul_eval, NecoGeometryVerify.RegionPolynomial.pow_eval, NecoGeometryVerify.RegionPolynomial.pow_eval]
  have hdU : udv ^ degreeU ≠ 0 := pow_ne_zero _ hud
  have hdV : vdv ^ degreeV ≠ 0 := pow_ne_zero _ hvd
  have hden : udv ^ degreeU * vdv ^ degreeV ≠ 0 := mul_ne_zero hdU hdV
  refine ⟨hden, ?_⟩
  rw [NecoGeometryVerify.RegionPolynomial.normalize_eval]
  rw [hfold]
  have hz : NecoGeometryVerify.RegionPolynomial.eval ([] : (List NecoGeometryVerify.RegionTerm)) x y z = 0 := rfl
  rw [hz, zero_add, NecoCalculusVerify.SurfacePolynomial.value_sum]
  have hmul := NecoCalculusVerify.SurfacePolynomial.map_sum_mul_left
    (fun t => monomial t (unv / udv) (vnv / vdv))
    (udv ^ degreeU * vdv ^ degreeV) terms
  rw [div_eq_iff hden]
  rw [mul_comm ((terms.map _).sum) (udv ^ degreeU * vdv ^ degreeV), hmul]
  apply congrArg List.sum
  apply List.map_congr_left
  intro t ht
  have hbU := hboundU t ht
  have hbV := hboundV t ht
  have hpowU : udv ^ degreeU = udv ^ t.exponentU * udv ^ (degreeU - t.exponentU) := by
    rw [← pow_add, Nat.add_sub_cancel' hbU]
  have hpowV : vdv ^ degreeV = vdv ^ t.exponentV * vdv ^ (degreeV - t.exponentV) := by
    rw [← pow_add, Nat.add_sub_cancel' hbV]
  simp only [monomial]
  rw [hpowU, hpowV, div_pow, div_pow]
  have hdu : udv ^ t.exponentU ≠ 0 := pow_ne_zero _ hud
  have hdv : vdv ^ t.exponentV ≠ 0 := pow_ne_zero _ hvd
  field_simp

-- ============================================================
-- deviationPolys evaluation
-- ============================================================

/-- Evaluation of the deviation fraction:
    `dev(t) = χ_axis(u(t),v(t)) − c_axis(t)`. -/
theorem deviationPolys_eval (axisTerms denominator : List SurfaceTerm)
    (uN uD vN vD : List Rat) (p0 p1 t0 t1 t : Rat)
    (huD : NecoCalculusVerify.RationalPolynomial.eval uD t ≠ 0) (hvD : NecoCalculusVerify.RationalPolynomial.eval vD t ≠ 0)
    (hden : surfaceValue denominator (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t)
      (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) ≠ 0)
    (hspan : t0 ≠ t1) :
    NecoCalculusVerify.RationalPolynomial.eval (deviationPolys axisTerms denominator uN uD vN vD p0 p1 t0 t1).1 t /
        NecoCalculusVerify.RationalPolynomial.eval (deviationPolys axisTerms denominator uN uD vN vD p0 p1 t0 t1).2 t =
      surfaceValue axisTerms (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t)
          (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) /
        surfaceValue denominator (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t)
          (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) -
        chordAxisAt p0 p1 t0 t1 t := by
  unfold deviationPolys
  dsimp only
  obtain ⟨hsD, hsEval⟩ := NecoCalculusVerify.RationalPolynomial.composeSurface_eval axisTerms uN uD vN vD t huD hvD
  obtain ⟨hdD, hdEval⟩ := NecoCalculusVerify.RationalPolynomial.composeSurface_eval denominator uN uD vN vD t huD hvD
  have hdN : NecoCalculusVerify.RationalPolynomial.eval (NecoCalculusVerify.RationalPolynomial.composeSurface denominator uN uD vN vD).1 t ≠ 0 := by
    have h : NecoCalculusVerify.RationalPolynomial.eval (NecoCalculusVerify.RationalPolynomial.composeSurface denominator uN uD vN vD).1 t /
        NecoCalculusVerify.RationalPolynomial.eval (NecoCalculusVerify.RationalPolynomial.composeSurface denominator uN uD vN vD).2 t ≠ 0 := by
      rw [hdEval]
      exact hden
    exact (div_ne_zero_iff.mp h).1
  have hcDen : NecoCalculusVerify.RationalPolynomial.eval (chordDenominator t0 t1) t ≠ 0 := by
    unfold chordDenominator
    have hsingle : NecoCalculusVerify.RationalPolynomial.eval [t1 - t0] t = t1 - t0 := by simp [NecoCalculusVerify.RationalPolynomial.eval]
    rw [hsingle]
    exact sub_ne_zero.mpr hspan.symm
  simp only [NecoCalculusVerify.RationalPolynomial.normalize_eval, NecoCalculusVerify.RationalPolynomial.sub_eval, NecoCalculusVerify.RationalPolynomial.mul_eval]
  unfold chordAxisAt
  rw [← hsEval, ← hdEval]
  field_simp

-- ============================================================
-- diskConditionNumerator and coverageSum evaluation
-- ============================================================

theorem diskConditionNumerator_eval (inverse : InverseChart) (x y z : Rat) :
    NecoGeometryVerify.RegionPolynomial.eval (diskConditionNumerator inverse) x y z =
      NecoGeometryVerify.RegionPolynomial.eval inverse.uDenominator x y z ^ 2 * NecoGeometryVerify.RegionPolynomial.eval inverse.vDenominator x y z ^ 2 -
        (NecoGeometryVerify.RegionPolynomial.eval inverse.uNumerator x y z ^ 2 * NecoGeometryVerify.RegionPolynomial.eval inverse.vDenominator x y z ^ 2 +
          NecoGeometryVerify.RegionPolynomial.eval inverse.vNumerator x y z ^ 2 * NecoGeometryVerify.RegionPolynomial.eval inverse.uDenominator x y z ^ 2) := by
  unfold diskConditionNumerator
  simp only [NecoGeometryVerify.RegionPolynomial.sub_eval, NecoGeometryVerify.RegionPolynomial.add_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, NecoGeometryVerify.RegionPolynomial.pow_eval]

/-- Evaluate the per-clause guard product. -/
private theorem guardProduct_eval (guards : List (List NecoGeometryVerify.RegionTerm)) (x y z : Rat) :
    ∀ acc, NecoGeometryVerify.RegionPolynomial.eval (guards.foldl (fun product guard =>
        NecoGeometryVerify.RegionPolynomial.mul product guard) acc) x y z =
      NecoGeometryVerify.RegionPolynomial.eval acc x y z * (guards.map fun g => NecoGeometryVerify.RegionPolynomial.eval g x y z).prod := by
  induction guards with
  | nil => intro acc; simp
  | cons g rest ih =>
      intro acc
      simp only [List.foldl_cons, List.map_cons, List.prod_cons]
      rw [ih (NecoGeometryVerify.RegionPolynomial.mul acc g), NecoGeometryVerify.RegionPolynomial.mul_eval]
      ring

theorem coverageSum_eval (clauses : List CoveringClause) (x y z : Rat) :
    NecoGeometryVerify.RegionPolynomial.eval (coverageSum clauses) x y z =
      (clauses.map fun clause =>
        (clause.guards.map fun g => NecoGeometryVerify.RegionPolynomial.eval g x y z).prod).sum := by
  unfold coverageSum
  rw [NecoGeometryVerify.RegionPolynomial.normalize_eval]
  suffices h : ∀ acc, NecoGeometryVerify.RegionPolynomial.eval (clauses.foldl (fun acc clause =>
      NecoGeometryVerify.RegionPolynomial.add acc (clause.guards.foldl (fun product guard =>
        NecoGeometryVerify.RegionPolynomial.mul product guard)
        [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 0 }]))
      acc) x y z =
      NecoGeometryVerify.RegionPolynomial.eval acc x y z + (clauses.map fun clause =>
        (clause.guards.map fun g => NecoGeometryVerify.RegionPolynomial.eval g x y z).prod).sum by
    rw [h []]
    have hz : NecoGeometryVerify.RegionPolynomial.eval ([] : (List NecoGeometryVerify.RegionTerm)) x y z = 0 := rfl
    rw [hz, zero_add]
  induction clauses with
  | nil => intro acc; simp
  | cons clause rest ih =>
      intro acc
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, NecoGeometryVerify.RegionPolynomial.add_eval, guardProduct_eval]
      have hone : NecoGeometryVerify.RegionPolynomial.eval
          [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 0 }]
          x y z = 1 := by
        simp [NecoGeometryVerify.RegionPolynomial.eval, NecoGeometryVerify.RegionPolynomial.monomial]
      rw [hone]
      ring

end NecoCadVerify
