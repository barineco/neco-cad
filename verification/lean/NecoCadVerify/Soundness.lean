import NecoCadVerify.Semantics
import NecoCadVerify.Compose

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

-- ============================================================
-- List and evaluation helpers
-- ============================================================

private theorem find?_mem_pred {α : Type _} (p : α → Bool) (l : List α) (a : α)
    (h : l.find? p = some a) : p a = true ∧ a ∈ l := by
  induction l with
  | nil => simp at h
  | cons x rest ih =>
      simp only [List.find?_cons] at h
      split at h
      · rename_i hx
        injection h with heq
        subst heq
        exact ⟨hx, List.mem_cons_self⟩
      · obtain ⟨h1, h2⟩ := ih h
        exact ⟨h1, List.mem_cons_of_mem _ h2⟩

private theorem getElem?_of_mem {α : Type _} (l : List α) (i : Nat) {a : α}
    (h : l[i]? = some a) : a ∈ l := by
  induction l generalizing i with
  | nil => simp at h
  | cons x rest ih =>
      cases i with
      | zero =>
          simp only [List.getElem?_cons_zero] at h
          injection h with heq
          subst heq
          exact List.mem_cons_self
      | succ k =>
          simp only [List.getElem?_cons_succ] at h
          exact List.mem_cons_of_mem _ (ih k h)

private theorem length_one_singleton {α : Type _} (l : List α) (h : l.length = 1) :
    ∃ a, l = [a] := by
  cases l with
  | nil => simp at h
  | cons a rest =>
      cases rest with
      | nil => exact ⟨a, rfl⟩
      | cons => simp at h

private theorem nonempty_of_not_isEmpty {α : Type _} (l : List α)
    (h : (!l.isEmpty) = true) : l ≠ [] := by
  intro he
  rw [he] at h
  simp [List.isEmpty] at h

private theorem exists_le_of_map_sum_le {α : Type _} (l : List α) (f : α → Rat)
    (h : l ≠ []) (hle : 0 ≤ (l.map f).sum) : ∃ a ∈ l, 0 ≤ f a := by
  induction l with
  | nil => exact absurd rfl h
  | cons x rest ih =>
      simp only [List.map_cons, List.sum_cons] at hle
      by_cases hx : 0 ≤ f x
      · exact ⟨x, List.mem_cons_self, hx⟩
      · have hx : f x < 0 := lt_of_not_ge hx
        have hsum : 0 < (rest.map f).sum := by linarith
        have hne : rest ≠ [] := by
          intro he
          rw [he] at hsum
          simp at hsum
        obtain ⟨a, ha, hf⟩ := ih hne (le_of_lt hsum)
        exact ⟨a, List.mem_cons_of_mem _ ha, hf⟩

-- ============================================================
-- Patch soundness
-- ============================================================

/-- The four domain bound polynomials are nonnegative on the domain. -/
theorem domainBoundPolys_nonneg (domain : PatchDomain) (disk : Bool) (u v : Rat)
    (h : domainContains domain disk u v) :
    ∀ b ∈ domainBoundPolys domain, 0 ≤ surfaceValue b u v := by
  intro b hb
  simp only [domainBoundPolys, List.mem_cons, List.not_mem_nil,
    or_false] at hb
  rcases hb with rfl | rfl | rfl | rfl
  · rw [NecoCalculusVerify.SurfacePolynomial.sub_eval, NecoCalculusVerify.SurfacePolynomial.variableU_eval, NecoCalculusVerify.SurfacePolynomial.constant_eval]
    linarith [h.1.1]
  · rw [NecoCalculusVerify.SurfacePolynomial.sub_eval, NecoCalculusVerify.SurfacePolynomial.variableU_eval, NecoCalculusVerify.SurfacePolynomial.constant_eval]
    linarith [h.1.2]
  · rw [NecoCalculusVerify.SurfacePolynomial.sub_eval, NecoCalculusVerify.SurfacePolynomial.variableV_eval, NecoCalculusVerify.SurfacePolynomial.constant_eval]
    linarith [h.2.1.1]
  · rw [NecoCalculusVerify.SurfacePolynomial.sub_eval, NecoCalculusVerify.SurfacePolynomial.variableV_eval, NecoCalculusVerify.SurfacePolynomial.constant_eval]
    linarith [h.2.1.2]

/-- A checked patch bound certifies a nonzero denominator on the domain. -/
theorem checkPatchBound_sound (patch : PatchData)
    (h : checkPatchBound patch = true) :
    ∀ u v, domainContains patch.domain patch.disk u v →
      surfaceValue patch.chart.denominator u v ≠ 0 := by
  unfold checkPatchBound at h
  rw [Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true,
    Bool.and_eq_true] at h
  obtain ⟨⟨⟨⟨hordU, hordV⟩, hholds⟩, hnonneg⟩, hconst⟩ := h
  intro u v hdom
  have hbounds := domainBoundPolys_nonneg patch.domain patch.disk u v hdom
  have hlower := surfaceBound_lower patch.denominatorBound
    (domainBoundPolys patch.domain) u v hnonneg hbounds
  have heval := surfaceBoundHolds_eval patch.chart.denominator
    patch.denominatorBound (domainBoundPolys patch.domain) u v hholds
  have hpos : 0 < patch.denominatorBound.constant := of_decide_eq_true hconst
  rw [heval]
  exact ne_of_gt (lt_of_lt_of_le hpos hlower)

/-- An accepted patch is an exact chart: nonzero denominator and the chart
    maps into the atom's zero set on the whole domain. -/
theorem checkPatchAgainstAtom_sound (atom : (List NecoGeometryVerify.RegionTerm)) (patch : PatchData)
    (h : checkPatchAgainstAtom atom patch = true) :
    Semantics.chartRepresentsAtom patch atom := by
  unfold checkPatchAgainstAtom at h
  rw [Bool.and_eq_true] at h
  obtain ⟨hbound, hcomp⟩ := h
  intro u v hdom
  have hd : surfaceValue patch.chart.denominator u v ≠ 0 := by
    cases checked : NecoGeometryVerify.surfaceCheckedRange patch.chart.x
        patch.chart.y patch.chart.z patch.chart.denominator
        patch.domain.u patch.domain.v with
    | none => exact checkPatchBound_sound patch hbound u v hdom
    | some output =>
        exact (chartPoint_checkedRange_contains patch.chart patch.domain output
          checked u v hdom.1 hdom.2.1).1
  refine ⟨hd, ?_⟩
  have hcompNil : composeAtomNumerator atom patch.chart = [] := by
    cases hc : composeAtomNumerator atom patch.chart with
    | nil => rfl
    | cons =>
        rw [hc] at hcomp
        simp [List.isEmpty] at hcomp
  have hcompEval := composeAtomNumerator_exactValue_eval atom patch.chart u v
    (chartPoint patch.chart u v) hd (chartPoint_exactValue patch.chart u v hd)
  rw [hcompNil] at hcompEval
  have hzero : surfaceValue ([] : List SurfaceTerm) u v = 0 := rfl
  rw [hzero] at hcompEval
  have hdv : surfaceValue patch.chart.denominator u v ^
      (atom.foldl (fun d t => max d
        (t.exponentX + t.exponentY + t.exponentZ)) 0) ≠ 0 :=
    pow_ne_zero _ hd
  rcases mul_eq_zero.mp hcompEval.symm with hz | hz
  · exact hz
  · exact absurd hz hdv

theorem acceptExactChart_sound (observation : ExactChartObservation)
    (accepted : acceptExactChart observation = true) :
    Semantics.chartRepresentsAtom observation.patch observation.atom :=
  checkPatchAgainstAtom_sound observation.atom observation.patch
    (of_decide_eq_true accepted)

theorem checkPatchAgainstAtom_checkedRange_sound
    (atom : List NecoGeometryVerify.RegionTerm) (patch : PatchData)
    (accepted : checkPatchAgainstAtom atom patch = true)
    (output : CoordinateIntervals)
    (checked : NecoGeometryVerify.surfaceCheckedRange patch.chart.x patch.chart.y
      patch.chart.z patch.chart.denominator patch.domain.u patch.domain.v = some output)
    (u v : Rat) (contained : domainContains patch.domain patch.disk u v) :
    NecoGeometryVerify.surfaceExactValue patch.chart.x patch.chart.y patch.chart.z
      patch.chart.denominator u v = some (chartPoint patch.chart u v) ∧
      NecoGeometryVerify.RegionPolynomial.eval atom (chartPoint patch.chart u v).1
        (chartPoint patch.chart u v).2.1 (chartPoint patch.chart u v).2.2 = 0 ∧
      output.x.Contains (chartPoint patch.chart u v).1 ∧
      output.y.Contains (chartPoint patch.chart u v).2.1 ∧
      output.z.Contains (chartPoint patch.chart u v).2.2 := by
  have represents := checkPatchAgainstAtom_sound atom patch accepted u v contained
  have rangeContains := chartPoint_checkedRange_contains patch.chart patch.domain
    output checked u v contained.1 contained.2.1
  exact ⟨chartPoint_exactValue patch.chart u v represents.1,
    represents.2, rangeContains.2⟩

-- ============================================================
-- Covering soundness
-- ============================================================

/-- Evaluation of an image residual:
    `R_axis = A_axis·D_d − var_axis·A_d·D_axis`. -/
theorem imageResidual_eval (axis : Nat) (axisTerms denominator : List SurfaceTerm)
    (inverse : InverseChart) (x y z : Rat) :
    NecoGeometryVerify.RegionPolynomial.eval (imageResidual axis axisTerms denominator inverse) x y z =
      NecoGeometryVerify.RegionPolynomial.eval (composedChart axisTerms inverse).1 x y z *
          NecoGeometryVerify.RegionPolynomial.eval (composedChart denominator inverse).2 x y z -
        NecoGeometryVerify.RegionPolynomial.eval (NecoGeometryVerify.RegionPolynomial.variablePolynomial axis) x y z *
          (NecoGeometryVerify.RegionPolynomial.eval (composedChart denominator inverse).1 x y z *
            NecoGeometryVerify.RegionPolynomial.eval (composedChart axisTerms inverse).2 x y z) := by
  unfold imageResidual composedChart
  rw [NecoGeometryVerify.RegionPolynomial.sub_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, NecoGeometryVerify.RegionPolynomial.mul_eval]

/-- Evaluation of the composed-chart denominator `.2` part. -/
private theorem composedChart_den_eval (terms : List SurfaceTerm)
    (inverse : InverseChart) (x y z : Rat)
    (hud : NecoGeometryVerify.RegionPolynomial.eval inverse.uDenominator x y z ≠ 0)
    (hvd : NecoGeometryVerify.RegionPolynomial.eval inverse.vDenominator x y z ≠ 0) :
    NecoGeometryVerify.RegionPolynomial.eval (composedChart terms inverse).2 x y z ≠ 0 ∧
      NecoGeometryVerify.RegionPolynomial.eval (composedChart terms inverse).1 x y z /
          NecoGeometryVerify.RegionPolynomial.eval (composedChart terms inverse).2 x y z =
        surfaceValue terms
          (NecoGeometryVerify.RegionPolynomial.eval inverse.uNumerator x y z / NecoGeometryVerify.RegionPolynomial.eval inverse.uDenominator x y z)
          (NecoGeometryVerify.RegionPolynomial.eval inverse.vNumerator x y z / NecoGeometryVerify.RegionPolynomial.eval inverse.vDenominator x y z) :=
  composeAtomArgs_eval terms inverse.uNumerator inverse.uDenominator
    inverse.vNumerator inverse.vDenominator x y z hud hvd

/-- One verified clause covers its guard region: every atom point where the
    clause's single guard is nonnegative is a chart image of a disk point of
    the referenced patch. -/
theorem checkClause_sound (atom : (List NecoGeometryVerify.RegionTerm)) (patch : CoveringPatch)
    (clause : CoveringClause)
    (h : checkClause atom patch clause = true) :
    Semantics.clauseCovers atom patch clause := by
  unfold checkClause at h
  dsimp only at h
  rw [Bool.and_eq_true] at h
  obtain ⟨hrest, hresidZ⟩ := h
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hresidY⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hresidX⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hdisknonneg⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hdiskform⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hconstD⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hDnonneg⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hDholds⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hconstV⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hconstU⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hVnonneg⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hUnonneg⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hVholds⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hUholds⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hguardsLen⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hboxV2⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hboxV⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hboxU2⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hboxU⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hpatchEq, hdisk⟩ := hrest
  refine ⟨of_decide_eq_true hpatchEq |>.symm, hdisk, ?_⟩
  intro x y z hatom hguards
  obtain ⟨g, hg⟩ := length_one_singleton clause.guards
    (of_decide_eq_true hguardsLen)
  have hg0 : 0 ≤ NecoGeometryVerify.RegionPolynomial.eval g x y z := hguards g (by rw [hg]; simp)
  have hguard_nn : ∀ gi ∈ clause.guards, 0 ≤ NecoGeometryVerify.RegionPolynomial.eval gi x y z := hguards
  -- inverse denominators are positive on the guard region
  have hud : NecoGeometryVerify.RegionPolynomial.eval patch.inverse.uDenominator x y z ≠ 0 := by
    have hlower := boundForm_lower clause.inverseDenominatorBounds.1
      clause.guards atom x y z hUnonneg hguard_nn hatom
    have heval := boundFormHolds_eval patch.inverse.uDenominator
      clause.inverseDenominatorBounds.1 clause.guards atom x y z hUholds
    have hpos : 0 < clause.inverseDenominatorBounds.1.constant :=
      of_decide_eq_true hconstU
    rw [heval]
    exact ne_of_gt (lt_of_lt_of_le hpos hlower)
  have hvd : NecoGeometryVerify.RegionPolynomial.eval patch.inverse.vDenominator x y z ≠ 0 := by
    have hlower := boundForm_lower clause.inverseDenominatorBounds.2
      clause.guards atom x y z hVnonneg hguard_nn hatom
    have heval := boundFormHolds_eval patch.inverse.vDenominator
      clause.inverseDenominatorBounds.2 clause.guards atom x y z hVholds
    have hpos : 0 < clause.inverseDenominatorBounds.2.constant :=
      of_decide_eq_true hconstV
    rw [heval]
    exact ne_of_gt (lt_of_lt_of_le hpos hlower)
  -- composed denominator aD > 0
  have haD : NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator
      patch.inverse).1 x y z ≠ 0 := by
    have hlower := boundForm_lower clause.chartDenominatorBound
      clause.guards atom x y z hDnonneg hguard_nn hatom
    have heval := boundFormHolds_eval (composedChart patch.chart.denominator
        patch.inverse).1 clause.chartDenominatorBound clause.guards atom
      x y z hDholds
    have hpos : 0 < clause.chartDenominatorBound.constant :=
      of_decide_eq_true hconstD
    rw [heval]
    exact ne_of_gt (lt_of_lt_of_le hpos hlower)
  -- disk-range numerator ≥ 0
  have hdiskNum : 0 ≤ NecoGeometryVerify.RegionPolynomial.eval (diskConditionNumerator patch.inverse) x y z := by
    have hlower := boundForm_lower clause.diskRange
      clause.guards atom x y z hdisknonneg hguard_nn hatom
    have heval := boundFormHolds_eval (diskConditionNumerator patch.inverse)
      clause.diskRange clause.guards atom x y z hdiskform
    have hconst : 0 ≤ clause.diskRange.constant := by
      unfold boundFormNonnegative at hdisknonneg
      rw [Bool.and_eq_true, Bool.and_eq_true] at hdisknonneg
      exact of_decide_eq_true hdisknonneg.1.1
    rw [heval]
    exact le_trans hconst hlower
  -- set up the parameter point (u,v)
  set unv := NecoGeometryVerify.RegionPolynomial.eval patch.inverse.uNumerator x y z
  set udv := NecoGeometryVerify.RegionPolynomial.eval patch.inverse.uDenominator x y z
  set vnv := NecoGeometryVerify.RegionPolynomial.eval patch.inverse.vNumerator x y z
  set vdv := NecoGeometryVerify.RegionPolynomial.eval patch.inverse.vDenominator x y z
  have hdiskEval := diskConditionNumerator_eval patch.inverse x y z
  rw [hdiskEval] at hdiskNum
  have huv : unv * unv / (udv * udv) + vnv * vnv / (vdv * vdv) ≤ 1 := by
    have hden : udv * udv * (vdv * vdv) ≠ 0 :=
      mul_ne_zero (mul_ne_zero hud hud) (mul_ne_zero hvd hvd)
    have hdiv : unv * unv / (udv * udv) + vnv * vnv / (vdv * vdv) - 1 =
        (unv * unv * (vdv * vdv) + vnv * vnv * (udv * udv) -
          udv * udv * (vdv * vdv)) / (udv * udv * (vdv * vdv)) := by
      field_simp
    have hnum : unv * unv * (vdv * vdv) + vnv * vnv * (udv * udv) -
        udv * udv * (vdv * vdv) ≤ 0 := by
      linarith [hdiskNum]
    have hdenpos : 0 < udv * udv * (vdv * vdv) :=
      mul_pos (mul_self_pos.mpr hud) (mul_self_pos.mpr hvd)
    have : unv * unv / (udv * udv) + vnv * vnv / (vdv * vdv) - 1 ≤ 0 := by
      rw [hdiv]
      exact div_nonpos_of_nonpos_of_nonneg hnum (le_of_lt hdenpos)
    linarith
  have hu2 : unv * unv / (udv * udv) = (unv / udv) * (unv / udv) := by
    field_simp
  have hv2 : vnv * vnv / (vdv * vdv) = (vnv / vdv) * (vnv / vdv) := by
    field_simp
  rw [hu2, hv2] at huv
  refine ⟨unv / udv, vnv / vdv, ⟨⟨?_, ?_⟩, ⟨?_, ?_⟩, fun _ => huv⟩, ?_⟩
  · have hbox : patch.domain.u.lower ≤ -1 := of_decide_eq_true hboxU
    have habs : -1 ≤ unv / udv ∧ unv / udv ≤ 1 := by
      have hsq : (unv / udv) * (unv / udv) ≤ 1 := by
        have hvnn : 0 ≤ (vnv / vdv) * (vnv / vdv) := mul_self_nonneg _
        linarith
      constructor <;> nlinarith [hsq]
    linarith
  · have hbox : 1 ≤ patch.domain.u.upper := of_decide_eq_true hboxU2
    have habs : unv / udv ≤ 1 := by
      have hsq : (unv / udv) * (unv / udv) ≤ 1 := by
        have hvnn : 0 ≤ (vnv / vdv) * (vnv / vdv) := mul_self_nonneg _
        linarith
      nlinarith [hsq]
    linarith
  · have hbox : patch.domain.v.lower ≤ -1 := of_decide_eq_true hboxV
    have habs : -1 ≤ vnv / vdv := by
      have hsq : (vnv / vdv) * (vnv / vdv) ≤ 1 := by
        have hunn : 0 ≤ (unv / udv) * (unv / udv) := mul_self_nonneg _
        linarith
      nlinarith [hsq]
    linarith
  · have hbox : 1 ≤ patch.domain.v.upper := of_decide_eq_true hboxV2
    have habs : vnv / vdv ≤ 1 := by
      have hsq : (vnv / vdv) * (vnv / vdv) ≤ 1 := by
        have hunn : 0 ≤ (unv / udv) * (unv / udv) := mul_self_nonneg _
        linarith
      nlinarith [hsq]
    linarith
  · -- chartPoint equality via the three residuals
    have hxEval := composedChart_den_eval patch.chart.x patch.inverse x y z hud hvd
    have hyEval := composedChart_den_eval patch.chart.y patch.inverse x y z hud hvd
    have hzEval := composedChart_den_eval patch.chart.z patch.inverse x y z hud hvd
    have hdEval := composedChart_den_eval patch.chart.denominator patch.inverse
      x y z hud hvd
    have hresX := imageResidual_eval 0 patch.chart.x patch.chart.denominator
      patch.inverse x y z
    have hresY := imageResidual_eval 1 patch.chart.y patch.chart.denominator
      patch.inverse x y z
    have hresZ := imageResidual_eval 2 patch.chart.z patch.chart.denominator
      patch.inverse x y z
    have hnormX : NecoGeometryVerify.RegionPolynomial.normalize (imageResidual 0 patch.chart.x
        patch.chart.denominator patch.inverse) =
        NecoGeometryVerify.RegionPolynomial.normalize (NecoGeometryVerify.RegionPolynomial.mul clause.imageQuotients.1 atom) :=
      of_decide_eq_true hresidX
    have hnormY : NecoGeometryVerify.RegionPolynomial.normalize (imageResidual 1 patch.chart.y
        patch.chart.denominator patch.inverse) =
        NecoGeometryVerify.RegionPolynomial.normalize (NecoGeometryVerify.RegionPolynomial.mul clause.imageQuotients.2.1 atom) :=
      of_decide_eq_true hresidY
    have hnormZ : NecoGeometryVerify.RegionPolynomial.normalize (imageResidual 2 patch.chart.z
        patch.chart.denominator patch.inverse) =
        NecoGeometryVerify.RegionPolynomial.normalize (NecoGeometryVerify.RegionPolynomial.mul clause.imageQuotients.2.2 atom) :=
      of_decide_eq_true hresidZ
    have heqX := congrArg (fun p => NecoGeometryVerify.RegionPolynomial.eval p x y z) hnormX
    rw [NecoGeometryVerify.RegionPolynomial.normalize_eval, NecoGeometryVerify.RegionPolynomial.normalize_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, hatom, mul_zero,
      hresX] at heqX
    have heqY := congrArg (fun p => NecoGeometryVerify.RegionPolynomial.eval p x y z) hnormY
    rw [NecoGeometryVerify.RegionPolynomial.normalize_eval, NecoGeometryVerify.RegionPolynomial.normalize_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, hatom, mul_zero,
      hresY] at heqY
    have heqZ := congrArg (fun p => NecoGeometryVerify.RegionPolynomial.eval p x y z) hnormZ
    rw [NecoGeometryVerify.RegionPolynomial.normalize_eval, NecoGeometryVerify.RegionPolynomial.normalize_eval, NecoGeometryVerify.RegionPolynomial.mul_eval, hatom, mul_zero,
      hresZ] at heqZ
    simp only [NecoGeometryVerify.RegionPolynomial.variable_eval] at heqX heqY heqZ
    -- residual equations: aAxis·dD = p_axis·aD·dAxis
    have hrx : NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.x patch.inverse).1 x y z *
        NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z =
        x * (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z *
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.x patch.inverse).2 x y z) := by
      linarith [heqX]
    have hry : NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.y patch.inverse).1 x y z *
        NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z =
        y * (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z *
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.y patch.inverse).2 x y z) := by
      linarith [heqY]
    have hrz : NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.z patch.inverse).1 x y z *
        NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z =
        z * (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z *
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.z patch.inverse).2 x y z) := by
      linarith [heqZ]
    -- chartPoint components: χ_axis = aAxis·dD/(dAxis·aD) = p_axis
    have hχx : surfaceValue patch.chart.x (unv / udv) (vnv / vdv) /
        surfaceValue patch.chart.denominator (unv / udv) (vnv / vdv) = x := by
      rw [← hxEval.2, ← hdEval.2]
      have hdiv : NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.x patch.inverse).1 x y z /
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.x patch.inverse).2 x y z /
            (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z /
              NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z) =
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.x patch.inverse).1 x y z *
            NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z /
            (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.x patch.inverse).2 x y z *
              NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z) := by
        field_simp
      rw [hdiv, div_eq_iff (mul_ne_zero hxEval.1 haD)]
      rw [hrx]
      ring
    have hχy : surfaceValue patch.chart.y (unv / udv) (vnv / vdv) /
        surfaceValue patch.chart.denominator (unv / udv) (vnv / vdv) = y := by
      rw [← hyEval.2, ← hdEval.2]
      have hdiv : NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.y patch.inverse).1 x y z /
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.y patch.inverse).2 x y z /
            (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z /
              NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z) =
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.y patch.inverse).1 x y z *
            NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z /
            (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.y patch.inverse).2 x y z *
              NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z) := by
        field_simp
      rw [hdiv, div_eq_iff (mul_ne_zero hyEval.1 haD)]
      rw [hry]
      ring
    have hχz : surfaceValue patch.chart.z (unv / udv) (vnv / vdv) /
        surfaceValue patch.chart.denominator (unv / udv) (vnv / vdv) = z := by
      rw [← hzEval.2, ← hdEval.2]
      have hdiv : NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.z patch.inverse).1 x y z /
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.z patch.inverse).2 x y z /
            (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z /
              NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z) =
          NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.z patch.inverse).1 x y z *
            NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).2 x y z /
            (NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.z patch.inverse).2 x y z *
              NecoGeometryVerify.RegionPolynomial.eval (composedChart patch.chart.denominator patch.inverse).1 x y z) := by
        field_simp
      rw [hdiv, div_eq_iff (mul_ne_zero hzEval.1 haD)]
      rw [hrz]
      ring
    unfold chartPoint
    dsimp only
    simp only [Prod.mk.injEq]
    exact ⟨hχx, hχy, hχz⟩

theorem acceptFaceAtomCovering_sound (observation : FaceAtomCoveringObservation)
    (accepted : acceptFaceAtomCovering observation = true) :
    Semantics.coveringRepresentsFace observation := by
  have h : checkFaceAtomCovering observation = true := of_decide_eq_true accepted
  unfold checkFaceAtomCovering at h
  dsimp only at h
  rw [Bool.and_eq_true] at h
  obtain ⟨hrest, hlen⟩ := h
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hrest, hcoverage⟩ := hrest
  rw [Bool.and_eq_true] at hrest
  obtain ⟨hpatchesOk, hclausesOk⟩ := hrest
  rw [Bool.and_eq_true, Bool.and_eq_true] at hcoverage
  obtain ⟨⟨hclausesNE, hnonempty⟩, hselections⟩ := hcoverage
  constructor
  · intro patch hmem
    have hpk := List.all_eq_true.mp hpatchesOk patch hmem
    rw [Bool.and_eq_true, Bool.and_eq_true] at hpk
    exact checkPatchAgainstAtom_sound observation.atom patch.toPatchData hpk.1.1
  · intro x y z hatom houter
    -- coverage: some selection certifies `Σ∏ ≥ 0`
    obtain ⟨selection, hselmem⟩ :=
      List.exists_mem_of_ne_nil _ (nonempty_of_not_isEmpty
        observation.coverageSelections hnonempty)
    have hsel := List.all_eq_true.mp hselections selection hselmem
    rw [Bool.and_eq_true] at hsel
    obtain ⟨hholds, hnonneg⟩ := hsel
    have hlower := boundForm_lower selection [] observation.atom x y z
      hnonneg (by simp) hatom
    have heval := boundFormHolds_eval (coverageSum observation.clauses)
      selection [] observation.atom x y z hholds
    have hconst : 0 ≤ selection.constant := by
      unfold boundFormNonnegative at hnonneg
      rw [Bool.and_eq_true, Bool.and_eq_true] at hnonneg
      exact of_decide_eq_true hnonneg.1.1
    have hsum : 0 ≤ NecoGeometryVerify.RegionPolynomial.eval (coverageSum observation.clauses) x y z := by
      rw [heval]
      exact le_trans hconst hlower
    rw [coverageSum_eval] at hsum
    obtain ⟨clause, hcmem, hprod⟩ := exists_le_of_map_sum_le
      observation.clauses (fun clause =>
        (clause.guards.map fun g => NecoGeometryVerify.RegionPolynomial.eval g x y z).prod)
      (nonempty_of_not_isEmpty observation.clauses hclausesNE) hsum
    refine ⟨clause, hcmem, ?_⟩
    -- the clause's patch and clauseCovers
    have hcl := List.all_eq_true.mp hclausesOk clause hcmem
    cases hfind : observation.patches.find?
        (fun p => p.patchIndex == clause.patch) with
    | none =>
        rw [hfind] at hcl
        simp at hcl
    | some patch =>
        rw [hfind] at hcl
        obtain ⟨hpeq, hpmem⟩ := find?_mem_pred _ _ _ hfind
        -- single guard: the product equals that guard's value
        have hlen1 : clause.guards.length = 1 := by
          have hcl' := hcl
          unfold checkClause at hcl'
          dsimp only at hcl'
          simp only [Bool.and_eq_true] at hcl'
          exact of_decide_eq_true hcl'.1.1.1.1.1.1.1.1.1.1.1.1.1.1.2
        obtain ⟨g, hg⟩ := length_one_singleton clause.guards hlen1
        have hg0 : 0 ≤ NecoGeometryVerify.RegionPolynomial.eval g x y z := by
          rw [hg] at hprod
          simpa using hprod
        have hguards : ∀ gi ∈ clause.guards, 0 ≤ NecoGeometryVerify.RegionPolynomial.eval gi x y z := by
          intro gi hgi
          rw [hg, List.mem_singleton] at hgi
          subst hgi
          exact hg0
        exact ⟨patch, hpmem, hguards,
          checkClause_sound observation.atom patch clause hcl⟩

theorem vertexCovered_sound (face : FaceTriangles) (bound : Rat)
    (vertex : MeshVertex) (param : VertexParameter)
    (h : vertexCovered face.patches bound vertex.toVec3 param = true) :
    ∃ patch ∈ face.patches, patch.patchIndex = param.patchIndex ∧
      domainContains patch.domain patch.disk param.u param.v ∧
      surfaceValue patch.chart.denominator param.u param.v ≠ 0 ∧
      chartPoint patch.chart param.u param.v = param.point ∧
      vdot (vsub param.point vertex.toVec3) (vsub param.point vertex.toVec3) ≤
        bound * bound := by
  unfold vertexCovered at h
  cases hp : face.patches[param.patchIndex]? with
  | none =>
      rw [hp] at h
      simp at h
  | some patch =>
      rw [hp] at h
      dsimp only at h
      rw [Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true,
        Bool.and_eq_true] at h
      obtain ⟨⟨⟨⟨hidx, hdom⟩, hden⟩, hpt⟩, hsq⟩ := h
      exact ⟨patch, getElem?_of_mem _ _ hp, of_decide_eq_true hidx,
        of_decide_eq_true hdom, of_decide_eq_true hden,
        of_decide_eq_true hpt, of_decide_eq_true hsq⟩

theorem checkFaceVertices_sound (face : FaceTriangles) (mesh : MeshData)
    (surfaceBound : Rat)
    (h : checkFaceVertices face mesh surfaceBound = true) :
    Semantics.faceVerticesCovered face mesh surfaceBound := by
  unfold checkFaceVertices at h
  intro ti hti i j k htri vi hvi vertex hvert
  have hall := List.all_eq_true.mp h ti hti
  rw [htri] at hall
  dsimp only at hall
  have hvi' := List.all_eq_true.mp hall vi hvi
  rw [hvert] at hvi'
  dsimp only at hvi'
  rw [Bool.and_eq_true] at hvi'
  obtain ⟨hany, hres⟩ := hvi'
  obtain ⟨param, hpmem, hcover⟩ := List.any_eq_true.mp hany
  obtain ⟨patch, hpm, hidx, hdom, hden, hpt, hsq⟩ :=
    vertexCovered_sound face _ vertex param hcover
  refine ⟨⟨param, hpmem, patch, hpm, hidx, hdom, hden, hpt, hsq⟩,
    of_decide_eq_true hres⟩

theorem checkFiniteSurface_sound (observation : FiniteSurfaceObservation)
    (h : checkFiniteSurface observation = true) :
    Semantics.finiteSurfaceRepresents observation := by
  unfold checkFiniteSurface at h
  rw [Bool.and_eq_true] at h
  obtain ⟨herr, hfaces⟩ := h
  refine ⟨of_decide_eq_true herr, ?_⟩
  intro face hfmem
  have hf := List.all_eq_true.mp hfaces face hfmem
  rw [Bool.and_eq_true, Bool.and_eq_true] at hf
  obtain ⟨⟨hrb, hpat⟩, hverts⟩ := hf
  refine ⟨of_decide_eq_true hrb, ?_, ?_⟩
  · intro patch hpmem
    exact checkPatchAgainstAtom_sound face.atom patch
      (List.all_eq_true.mp hpat patch hpmem)
  · exact checkFaceVertices_sound _ _ _ hverts

theorem acceptFiniteSurface_sound (observation : FiniteSurfaceObservation)
    (accepted : acceptFiniteSurface observation = true) :
    Semantics.finiteSurfaceRepresents observation :=
  checkFiniteSurface_sound observation (of_decide_eq_true accepted)

-- ============================================================
-- Chord bound soundness
-- ============================================================

theorem integerEdgePolynomialRange_contains (coefficients : List Int)
    (domain : NecoAlgebraVerify.Interval) (t : Rat) (contained : domain.Contains t) :
    (NecoCalculusVerify.RationalPolynomial.range (coefficients.map Rat.ofInt) domain).Contains
      (NecoCalculusVerify.RationalPolynomial.eval (coefficients.map Rat.ofInt) t) := by
  rw [NecoCalculusVerify.RationalPolynomial.range_intCoefficients,
    NecoCalculusVerify.RationalPolynomial.eval_intCoefficients]
  exact NecoAlgebraVerify.intervalEval_sound (contained.1.trans contained.2) contained

theorem arcDenominator_nonzero (t : Rat) :
    NecoCalculusVerify.RationalPolynomial.eval [1, 0, 1] t ≠ 0 := by
  have contained := integerEdgePolynomialRange_contains [1, 0, 1]
    { lower := t, upper := t } t ⟨le_rfl, le_rfl⟩
  have lowerPositive : 0 <
      (NecoCalculusVerify.RationalPolynomial.range
        (([1, 0, 1] : List Int).map Rat.ofInt) { lower := t, upper := t }).lower := by
    norm_num [NecoCalculusVerify.RationalPolynomial.range,
      NecoAlgebraVerify.Interval.zero, NecoAlgebraVerify.Interval.constant,
      NecoAlgebraVerify.Interval.add, NecoAlgebraVerify.Interval.mul] <;>
      nlinarith [mul_self_nonneg t]
  exact ne_of_gt (lt_of_lt_of_le lowerPositive contained.1)

private theorem mapDiskDen_eval (uD vD : List Rat) (t : Rat) :
    NecoCalculusVerify.RationalPolynomial.eval (NecoCalculusVerify.RationalPolynomial.normalize (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow uD 2) (NecoCalculusVerify.RationalPolynomial.pow vD 2))) t =
      NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2 := by
  simp only [NecoCalculusVerify.RationalPolynomial.normalize_eval, NecoCalculusVerify.RationalPolynomial.mul_eval, NecoCalculusVerify.RationalPolynomial.pow_eval]

private theorem mapDiskNum_eval (uN uD vN vD : List Rat) (t : Rat) :
    NecoCalculusVerify.RationalPolynomial.eval (NecoCalculusVerify.RationalPolynomial.normalize (NecoCalculusVerify.RationalPolynomial.sub
      (NecoCalculusVerify.RationalPolynomial.add (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow uN 2) (NecoCalculusVerify.RationalPolynomial.pow vD 2)) (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow vN 2) (NecoCalculusVerify.RationalPolynomial.pow uD 2)))
      (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow uD 2) (NecoCalculusVerify.RationalPolynomial.pow vD 2)))) t =
      NecoCalculusVerify.RationalPolynomial.eval uN t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2 + NecoCalculusVerify.RationalPolynomial.eval vN t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 -
        NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2 := by
  simp only [NecoCalculusVerify.RationalPolynomial.normalize_eval, NecoCalculusVerify.RationalPolynomial.sub_eval, NecoCalculusVerify.RationalPolynomial.add_eval, NecoCalculusVerify.RationalPolynomial.mul_eval, NecoCalculusVerify.RationalPolynomial.pow_eval]

/-- `mapStaysInDisk` soundness: the parameter map's image stays in the
    unit disk over `[t0,t1]`. -/
theorem mapStaysInDisk_sound (uN uD vN vD : List Rat) (t0 t1 : Rat) (depth : Nat)
    (h : mapStaysInDisk uN uD vN vD t0 t1 depth = true) (t : Rat)
    (ht : t0 ≤ t ∧ t ≤ t1) :
    NecoCalculusVerify.RationalPolynomial.eval uD t ≠ 0 ∧ NecoCalculusVerify.RationalPolynomial.eval vD t ≠ 0 ∧
      (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t) * (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t) +
        (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) * (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) ≤ 1 := by
  unfold mapStaysInDisk at h
  dsimp only at h
  obtain ⟨hden, hnum'⟩ := NecoCalculusVerify.RationalPolynomial.nonpositiveSub_sound _ _ depth t0 t1 h t ht
  rw [mapDiskDen_eval] at hden
  have hdu : NecoCalculusVerify.RationalPolynomial.eval uD t ≠ 0 := by
    by_contra hz
    rw [hz] at hden
    simp at hden
  have hdv : NecoCalculusVerify.RationalPolynomial.eval vD t ≠ 0 := by
    by_contra hz
    rw [hz] at hden
    simp at hden
  refine ⟨hdu, hdv, ?_⟩
  have hnum : NecoCalculusVerify.RationalPolynomial.eval uN t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2 +
      NecoCalculusVerify.RationalPolynomial.eval vN t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 -
        NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2 ≤ 0 := by
    rw [mapDiskNum_eval] at hnum'
    exact hnum'
  have hdenpos : 0 < NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2 :=
    mul_pos (sq_pos_of_ne_zero hdu) (sq_pos_of_ne_zero hdv)
  have hdiv : (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t) * (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t) +
      (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) * (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) - 1 =
      (NecoCalculusVerify.RationalPolynomial.eval uN t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2 + NecoCalculusVerify.RationalPolynomial.eval vN t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 -
        NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2) /
        (NecoCalculusVerify.RationalPolynomial.eval uD t ^ 2 * NecoCalculusVerify.RationalPolynomial.eval vD t ^ 2) := by
    field_simp
  have hle1 : (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t) * (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t) +
      (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) * (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) - 1 ≤ 0 := by
    rw [hdiv]
    exact div_nonpos_of_nonpos_of_nonneg hnum (le_of_lt hdenpos)
  linarith

private theorem lineMap_eval (x0 x1 t0 t1 t : Rat) :
    NecoCalculusVerify.RationalPolynomial.eval [x0 * t1 - x1 * t0, x1 - x0] t / NecoCalculusVerify.RationalPolynomial.eval [t1 - t0] t =
      (x0 * (t1 - t) + x1 * (t - t0)) / (t1 - t0) := by
  rw [NecoCalculusVerify.RationalPolynomial.eval_two, NecoCalculusVerify.RationalPolynomial.eval_single]
  have hn : x0 * t1 - x1 * t0 + t * (x1 - x0) =
      x0 * (t1 - t) + x1 * (t - t0) := by ring
  rw [hn]

private theorem affineConvex_bounds (x0 x1 lo hi t0 t1 t : Rat)
    (hspan : 0 < t1 - t0) (ht0 : t0 ≤ t) (ht1 : t ≤ t1)
    (hx0 : lo ≤ x0) (hx0' : x0 ≤ hi) (hx1 : lo ≤ x1) (hx1' : x1 ≤ hi) :
    lo ≤ (x0 * (t1 - t) + x1 * (t - t0)) / (t1 - t0) ∧
      (x0 * (t1 - t) + x1 * (t - t0)) / (t1 - t0) ≤ hi := by
  have hw1 : 0 ≤ t1 - t := by linarith
  have hw2 : 0 ≤ t - t0 := by linarith
  constructor
  · rw [le_div_iff₀ hspan]
    have h1 : lo * (t1 - t) ≤ x0 * (t1 - t) :=
      mul_le_mul_of_nonneg_right hx0 hw1
    have h2 : lo * (t - t0) ≤ x1 * (t - t0) :=
      mul_le_mul_of_nonneg_right hx1 hw2
    linarith
  · rw [div_le_iff₀ hspan]
    have h1 : x0 * (t1 - t) ≤ hi * (t1 - t) :=
      mul_le_mul_of_nonneg_right hx0' hw1
    have h2 : x1 * (t - t0) ≤ hi * (t - t0) :=
      mul_le_mul_of_nonneg_right hx1' hw2
    linarith

/-- The circle parameterization stays on the unit circle:
    `u² + v² = 1` whenever `uNum² = (1−t²)²`. -/
private theorem arcParam_circle (uNum t : Rat)
    (hu2 : uNum * uNum = (1 - t * t) * (1 - t * t)) :
    (uNum / (1 + t * t)) * (uNum / (1 + t * t)) +
      (2 * t / (1 + t * t)) * (2 * t / (1 + t * t)) = 1 := by
  have hd : (1 + t * t) ≠ 0 := by
    have hs := mul_self_nonneg t
    positivity
  have hkey : uNum * uNum + (2 * t) * (2 * t) =
      (1 + t * t) * (1 + t * t) := by
    rw [hu2]; ring
  have h' : (uNum / (1 + t * t)) * (uNum / (1 + t * t)) +
      (2 * t / (1 + t * t)) * (2 * t / (1 + t * t)) =
      (uNum * uNum + (2 * t) * (2 * t)) / ((1 + t * t) * (1 + t * t)) := by
    field_simp
  rw [h', hkey]
  exact div_self (mul_ne_zero hd hd)

/-- On the unit circle `|u| ≤ 1`. -/
private theorem arcParam_abs_le (uNum t : Rat)
    (hcircle : (uNum / (1 + t * t)) * (uNum / (1 + t * t)) +
      (2 * t / (1 + t * t)) * (2 * t / (1 + t * t)) = 1) :
    |uNum / (1 + t * t)| ≤ 1 := by
  have h2 : (uNum / (1 + t * t)) * (uNum / (1 + t * t)) ≤ 1 := by
    have hv2 : 0 ≤ (2 * t / (1 + t * t)) * (2 * t / (1 + t * t)) :=
      mul_self_nonneg _
    linarith
  rw [abs_le]
  constructor <;> nlinarith [h2,
    sq_nonneg (uNum / (1 + t * t) - 1), sq_nonneg (uNum / (1 + t * t) + 1)]

/-- On the unit circle `|v| ≤ 1`. -/
private theorem arcParam_v_abs_le (uNum t : Rat)
    (hcircle : (uNum / (1 + t * t)) * (uNum / (1 + t * t)) +
      (2 * t / (1 + t * t)) * (2 * t / (1 + t * t)) = 1) :
    |2 * t / (1 + t * t)| ≤ 1 := by
  have h2 : (2 * t / (1 + t * t)) * (2 * t / (1 + t * t)) ≤ 1 := by
    have hu2 : 0 ≤ (uNum / (1 + t * t)) * (uNum / (1 + t * t)) :=
      mul_self_nonneg _
    linarith
  rw [abs_le]
  constructor <;> nlinarith [h2,
    sq_nonneg (2 * t / (1 + t * t) - 1), sq_nonneg (2 * t / (1 + t * t) + 1)]

/-- Shared tail of chord-edge soundness: from the parameter maps, per-`t`
    denominator and domain facts, and the `boundOk` certificate, derive the
    declarative deviation semantics. -/
private theorem chordEdgeTail_sound (patch : PatchData) (faceBound : Rat)
    (edge : ChordEdge) (uN uD vN vD : List Rat) (t0 t1 : Rat)
    (hmaps : edgeMaps edge = (uN, uD, vN, vD, t0, t1))
    (hint : edgeParameterInterval edge = (t0, t1))
    (hspan : t0 < t1)
    (hpatch : checkPatchBound patch = true)
    (hden_uv : ∀ t, t0 ≤ t → t ≤ t1 → NecoCalculusVerify.RationalPolynomial.eval uD t ≠ 0 ∧ NecoCalculusVerify.RationalPolynomial.eval vD t ≠ 0)
    (hdom_t : ∀ t, t0 ≤ t → t ≤ t1 →
      domainContains patch.domain patch.disk (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t)
        (NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t))
    (hbound : (decide (0 ≤ edge.establishedSquaredBound) &&
        decide (edge.establishedSquaredBound ≤ faceBound) &&
        NecoCalculusVerify.RationalPolynomial.squaredSumWithinSub
          (deviationPolys patch.chart.x patch.chart.denominator uN uD vN vD
            edge.left.point.1 edge.right.point.1 t0 t1).1
          (deviationPolys patch.chart.x patch.chart.denominator uN uD vN vD
            edge.left.point.1 edge.right.point.1 t0 t1).2
          (deviationPolys patch.chart.y patch.chart.denominator uN uD vN vD
            edge.left.point.2.1 edge.right.point.2.1 t0 t1).1
          (deviationPolys patch.chart.y patch.chart.denominator uN uD vN vD
            edge.left.point.2.1 edge.right.point.2.1 t0 t1).2
          (deviationPolys patch.chart.z patch.chart.denominator uN uD vN vD
            edge.left.point.2.2 edge.right.point.2.2 t0 t1).1
          (deviationPolys patch.chart.z patch.chart.denominator uN uD vN vD
            edge.left.point.2.2 edge.right.point.2.2 t0 t1).2
          edge.establishedSquaredBound t0 t1 edge.subdivisionDepth) = true) :
    Semantics.edgeDeviationWithin patch edge ∧
      edge.establishedSquaredBound ≤ faceBound := by
  rw [Bool.and_eq_true, Bool.and_eq_true] at hbound
  obtain ⟨⟨_hB0, hBF⟩, hsplit⟩ := hbound
  refine ⟨?_, of_decide_eq_true hBF⟩
  unfold Semantics.edgeDeviationWithin
  rw [hint]
  dsimp only
  refine ⟨hspan, ?_⟩
  intro t ht
  have hparams' : edgeParamsAt edge t =
      (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t, NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t) := by
    unfold edgeParamsAt
    rw [hmaps]
  rw [hparams']
  dsimp only
  obtain ⟨huD, hvD⟩ := hden_uv t ht.1 ht.2
  have hdom' := hdom_t t ht.1 ht.2
  have hden := checkPatchBound_sound patch hpatch _ _ hdom'
  refine ⟨hden, ?_⟩
  obtain ⟨_, _, _, hsum⟩ :=
    NecoCalculusVerify.RationalPolynomial.squaredSumWithinSub_sound _ _ _ _ _ _ _
      edge.subdivisionDepth t0 t1 hsplit t ⟨ht.1, ht.2⟩
  rw [deviationPolys_eval patch.chart.x patch.chart.denominator uN uD vN vD
      edge.left.point.1 edge.right.point.1 t0 t1 t huD hvD hden (ne_of_lt hspan),
    deviationPolys_eval patch.chart.y patch.chart.denominator uN uD vN vD
      edge.left.point.2.1 edge.right.point.2.1 t0 t1 t huD hvD hden (ne_of_lt hspan),
    deviationPolys_eval patch.chart.z patch.chart.denominator uN uD vN vD
      edge.left.point.2.2 edge.right.point.2.2 t0 t1 t huD hvD hden (ne_of_lt hspan)] at hsum
  unfold chartPoint vsub vdot
  dsimp only
  linarith [hsum]

/-- One checked edge satisfies its declarative deviation semantics. -/
theorem checkChordEdge_sound (patch : PatchData) (faceBound : Rat)
    (edge : ChordEdge)
    (hpatch : checkPatchBound patch = true)
    (h : checkChordEdge patch faceBound edge = true) :
    Semantics.edgeDeviationWithin patch edge ∧
      edge.establishedSquaredBound ≤ faceBound := by
  unfold checkChordEdge at h
  cases hparam : edge.parameterization with
  | line tL tU =>
      have hmaps : edgeMaps edge =
          ([edge.left.u * tU - edge.right.u * tL, edge.right.u - edge.left.u],
            [tU - tL],
            [edge.left.v * tU - edge.right.v * tL, edge.right.v - edge.left.v],
            [tU - tL], tL, tU) := by
        unfold edgeMaps
        rw [hparam]
      have hint : edgeParameterInterval edge = (tL, tU) := by
        unfold edgeParameterInterval
        rw [hparam]
      rw [hmaps, hparam] at h
      dsimp only at h
      rw [Bool.and_eq_true, Bool.and_eq_true] at h
      obtain ⟨⟨hparams, hdom⟩, hbound⟩ := h
      rw [Bool.and_eq_true] at hparams
      obtain ⟨_, hspan⟩ := hparams
      have hspan' : tL < tU := of_decide_eq_true hspan
      rw [Bool.and_eq_true, Bool.and_eq_true] at hdom
      obtain ⟨⟨hu_affine, hv_affine⟩, hdisk_or⟩ := hdom
      unfold affineInInterval at hu_affine hv_affine
      rw [Bool.and_eq_true] at hu_affine hv_affine
      obtain ⟨hu0, hu1⟩ := hu_affine
      obtain ⟨hv0, hv1⟩ := hv_affine
      obtain ⟨hu0lo, hu0hi⟩ := of_decide_eq_true hu0
      obtain ⟨hu1lo, hu1hi⟩ := of_decide_eq_true hu1
      obtain ⟨hv0lo, hv0hi⟩ := of_decide_eq_true hv0
      obtain ⟨hv1lo, hv1hi⟩ := of_decide_eq_true hv1
      apply chordEdgeTail_sound patch faceBound edge _ _ _ _ tL tU hmaps hint
        hspan' hpatch
      · intro t _ht0 _ht1
        have hspan0 : tU - tL ≠ 0 := sub_ne_zero.mpr (ne_of_gt hspan')
        constructor <;> · rw [NecoCalculusVerify.RationalPolynomial.eval_single]; exact hspan0
      · intro t ht0 ht1
        have hub := affineConvex_bounds edge.left.u edge.right.u
          patch.domain.u.lower patch.domain.u.upper tL tU t
          (sub_pos.mpr hspan') ht0 ht1 hu0lo hu0hi hu1lo hu1hi
        have hvb := affineConvex_bounds edge.left.v edge.right.v
          patch.domain.v.lower patch.domain.v.upper tL tU t
          (sub_pos.mpr hspan') ht0 ht1 hv0lo hv0hi hv1lo hv1hi
        refine ⟨⟨?_, ?_⟩, ⟨?_, ?_⟩, ?_⟩
        · rw [lineMap_eval]; exact hub.1
        · rw [lineMap_eval]; exact hub.2
        · rw [lineMap_eval]; exact hvb.1
        · rw [lineMap_eval]; exact hvb.2
        · intro hdisk
          have hms : mapStaysInDisk
              [edge.left.u * tU - edge.right.u * tL,
                edge.right.u - edge.left.u]
              [tU - tL]
              [edge.left.v * tU - edge.right.v * tL,
                edge.right.v - edge.left.v]
              [tU - tL] tL tU edge.subdivisionDepth = true := by
            simp only [Bool.or_eq_true] at hdisk_or
            rcases hdisk_or with hnl | hms
            · rw [hdisk] at hnl; simp at hnl
            · exact hms
          exact (mapStaysInDisk_sound _ _ _ _ tL tU edge.subdivisionDepth
            hms t ⟨ht0, ht1⟩).2.2
      · exact hbound
  | arc minusPole tL tU =>
      have hint : edgeParameterInterval edge = (tL, tU) := by
        unfold edgeParameterInterval
        rw [hparam]
      cases hmp : minusPole with
      | false =>
          have hmaps : edgeMaps edge =
              ([1, 0, -1], [1, 0, 1], [0, 2], [1, 0, 1], tL, tU) := by
            unfold edgeMaps
            rw [hparam, hmp]
            rfl
          rw [hmaps, hparam] at h
          dsimp only at h
          rw [Bool.and_eq_true, Bool.and_eq_true] at h
          obtain ⟨⟨hparams, hdom⟩, hbound⟩ := h
          rw [Bool.and_eq_true] at hparams
          obtain ⟨_, hspan⟩ := hparams
          have hspan' : tL < tU := of_decide_eq_true hspan
          rw [Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true,
            Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true] at hdom
          obtain ⟨⟨⟨⟨⟨⟨_pd, hulo⟩, huhi⟩, hvlo⟩, hvhi⟩, _domL⟩, _domR⟩ := hdom
          apply chordEdgeTail_sound patch faceBound edge _ _ _ _ tL tU hmaps
            hint hspan' hpatch
          · intro t _ht0 _ht1
            exact ⟨arcDenominator_nonzero t, arcDenominator_nonzero t⟩
          · intro t _ht0 _ht1
            have hdval : NecoCalculusVerify.RationalPolynomial.eval [1, 0, 1] t = 1 + t * t := by
              rw [NecoCalculusVerify.RationalPolynomial.eval_three]; ring
            have huval : NecoCalculusVerify.RationalPolynomial.eval [1, 0, -1] t = 1 - t * t := by
              rw [NecoCalculusVerify.RationalPolynomial.eval_three]; ring
            have hvval : NecoCalculusVerify.RationalPolynomial.eval [0, 2] t = 2 * t := by
              rw [NecoCalculusVerify.RationalPolynomial.eval_two]; ring
            have hu2 : NecoCalculusVerify.RationalPolynomial.eval [1, 0, -1] t * NecoCalculusVerify.RationalPolynomial.eval [1, 0, -1] t =
                (1 - t * t) * (1 - t * t) := by
              rw [huval]
            have hcircle := arcParam_circle (NecoCalculusVerify.RationalPolynomial.eval [1, 0, -1] t) t hu2
            have habs := arcParam_abs_le _ _ hcircle
            have hvabs := arcParam_v_abs_le _ _ hcircle
            rw [← hdval] at habs hvabs hcircle
            rw [← hvval] at hvabs hcircle
            rw [abs_le] at habs hvabs
            refine ⟨⟨?_, ?_⟩, ⟨?_, ?_⟩, ?_⟩
            · exact le_trans (of_decide_eq_true hulo) habs.1
            · exact le_trans habs.2 (of_decide_eq_true huhi)
            · exact le_trans (of_decide_eq_true hvlo) hvabs.1
            · exact le_trans hvabs.2 (of_decide_eq_true hvhi)
            · intro _
              exact le_of_eq hcircle
          · exact hbound
      | true =>
          have hmaps : edgeMaps edge =
              ([-1, 0, 1], [1, 0, 1], [0, 2], [1, 0, 1], tL, tU) := by
            unfold edgeMaps
            rw [hparam, hmp]
            rfl
          rw [hmaps, hparam] at h
          dsimp only at h
          rw [Bool.and_eq_true, Bool.and_eq_true] at h
          obtain ⟨⟨hparams, hdom⟩, hbound⟩ := h
          rw [Bool.and_eq_true] at hparams
          obtain ⟨_, hspan⟩ := hparams
          have hspan' : tL < tU := of_decide_eq_true hspan
          rw [Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true,
            Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true] at hdom
          obtain ⟨⟨⟨⟨⟨⟨_pd, hulo⟩, huhi⟩, hvlo⟩, hvhi⟩, _domL⟩, _domR⟩ := hdom
          apply chordEdgeTail_sound patch faceBound edge _ _ _ _ tL tU hmaps
            hint hspan' hpatch
          · intro t _ht0 _ht1
            exact ⟨arcDenominator_nonzero t, arcDenominator_nonzero t⟩
          · intro t _ht0 _ht1
            have hdval : NecoCalculusVerify.RationalPolynomial.eval [1, 0, 1] t = 1 + t * t := by
              rw [NecoCalculusVerify.RationalPolynomial.eval_three]; ring
            have huval : NecoCalculusVerify.RationalPolynomial.eval [-1, 0, 1] t = -1 + t * t := by
              rw [NecoCalculusVerify.RationalPolynomial.eval_three]; ring
            have hvval : NecoCalculusVerify.RationalPolynomial.eval [0, 2] t = 2 * t := by
              rw [NecoCalculusVerify.RationalPolynomial.eval_two]; ring
            have hu2 : NecoCalculusVerify.RationalPolynomial.eval [-1, 0, 1] t * NecoCalculusVerify.RationalPolynomial.eval [-1, 0, 1] t =
                (1 - t * t) * (1 - t * t) := by
              rw [huval]; ring
            have hcircle := arcParam_circle (NecoCalculusVerify.RationalPolynomial.eval [-1, 0, 1] t) t hu2
            have habs := arcParam_abs_le _ _ hcircle
            have hvabs := arcParam_v_abs_le _ _ hcircle
            rw [← hdval] at habs hvabs hcircle
            rw [← hvval] at hvabs hcircle
            rw [abs_le] at habs hvabs
            refine ⟨⟨?_, ?_⟩, ⟨?_, ?_⟩, ?_⟩
            · exact le_trans (of_decide_eq_true hulo) habs.1
            · exact le_trans habs.2 (of_decide_eq_true huhi)
            · exact le_trans (of_decide_eq_true hvlo) hvabs.1
            · exact le_trans hvabs.2 (of_decide_eq_true hvhi)
            · intro _
              exact le_of_eq hcircle
          · exact hbound

theorem checkChordBound_sound (observation : ChordBoundObservation)
    (h : checkChordBound observation = true) :
    Semantics.chordBoundHolds observation := by
  unfold checkChordBound at h
  rw [Bool.and_eq_true, Bool.and_eq_true] at h
  obtain ⟨⟨hB, hpat⟩, hedges⟩ := h
  refine ⟨of_decide_eq_true hB, ?_⟩
  intro edge hemem
  have he := List.all_eq_true.mp hedges edge hemem
  cases hp : observation.patches[edge.patchIndex]? with
  | none => rw [hp] at he; simp at he
  | some patch =>
      rw [hp] at he
      dsimp only at he
      rw [Bool.and_eq_true] at he
      obtain ⟨hidx, hcheck⟩ := he
      have hpb : checkPatchBound patch = true :=
        List.all_eq_true.mp hpat patch (getElem?_of_mem _ _ hp)
      obtain ⟨hdevW, hbF⟩ := checkChordEdge_sound patch
        observation.establishedSquaredBound edge hpb hcheck
      exact ⟨patch, getElem?_of_mem _ _ hp, of_decide_eq_true hidx,
        hdevW, hbF⟩

theorem acceptChordBound_sound (observation : ChordBoundObservation)
    (accepted : acceptChordBound observation = true) :
    Semantics.chordBoundHolds observation :=
  checkChordBound_sound observation (of_decide_eq_true accepted)

-- ============================================================
-- Mesh soundness
-- ============================================================

private theorem meshPairs_mem (count i j : Nat) (hij : i < j) (hj : j < count) :
    (i, j) ∈ meshPairs count := by
  unfold meshPairs
  rw [List.mem_flatMap]
  refine ⟨i, List.mem_range.mpr (lt_trans hij hj), ?_⟩
  rw [List.mem_map]
  refine ⟨j, ?_, rfl⟩
  rw [List.mem_filter]
  exact ⟨List.mem_range.mpr hj, by simp [hij]⟩

private theorem pairsVerified_sound (mesh : MeshData) (pairs : List ExaminedPair)
    (h : pairsVerified mesh pairs = true) :
    ∀ i j, i < j → j < mesh.triangles.length →
      ∃ pair ∈ pairs, pair.first = i ∧ pair.second = j ∧
        pair.sharedIndexCount =
          (match mesh.triangles[i]?, mesh.triangles[j]? with
           | some t1, some t2 => sharedIndexCount t1 t2
           | _, _ => 0) ∧
        pair.kind = (classifyMeshPair mesh i j).getD .empty ∧
        pair.kind.permitted = true := by
  unfold pairsVerified at h
  rw [Bool.and_eq_true] at h
  obtain ⟨hlen, hall⟩ := h
  intro i j hij hj
  have hmem := meshPairs_mem _ _ _ hij hj
  have h := List.all_eq_true.mp hall (i, j) hmem
  dsimp only at h
  cases hf : pairs.find? (fun p => p.first == i && p.second == j) with
  | none =>
      rw [hf] at h
      cases hc : classifyMeshPair mesh i j <;> rw [hc] at h <;> simp at h
  | some pair =>
      rw [hf] at h
      cases hc : classifyMeshPair mesh i j with
      | none => rw [hc] at h; simp at h
      | some kind =>
          rw [hc] at h
          dsimp only at h
          rw [Bool.and_eq_true, Bool.and_eq_true] at h
          obtain ⟨⟨hsc, hk⟩, hperm⟩ := h
          obtain ⟨hfind, hpmem⟩ := find?_mem_pred _ _ _ hf
          rw [Bool.and_eq_true] at hfind
          obtain ⟨hf1, hf2⟩ := hfind
          exact ⟨pair, hpmem, beq_iff_eq.mp hf1, beq_iff_eq.mp hf2,
            of_decide_eq_true hsc, of_decide_eq_true hk, hperm⟩

theorem failureVerified_sound (mesh : MeshData) (failureKind : String)
    (failure : JsonFields)
    (h : failureVerified mesh failureKind failure = true) :
    Semantics.meshFailureClaim mesh failureKind failure := by
  unfold failureVerified at h
  unfold Semantics.meshFailureClaim
  dsimp only at h ⊢
  split at h
  · cases ha : jsonNatField? failure "firstVertex" with
    | none => rw [ha] at h; simp at h
    | some a =>
        cases hb : jsonNatField? failure "secondVertex" with
        | none => rw [ha, hb] at h; simp at h
        | some b =>
            rw [ha, hb] at h
            dsimp only at h
            exact ⟨a, b, rfl, rfl, beq_iff_eq.mp h⟩
  · cases ha : jsonNatField? failure "firstVertex" with
    | none => rw [ha] at h; simp at h
    | some a =>
        cases hb : jsonNatField? failure "secondVertex" with
        | none => rw [ha, hb] at h; simp at h
        | some b =>
            cases hc : jsonNatField? failure "incidenceCount" with
            | none => rw [ha, hb, hc] at h; simp at h
            | some count =>
                rw [ha, hb, hc] at h
                dsimp only at h
                rw [Bool.and_eq_true] at h
                obtain ⟨hinc, hge⟩ := h
                exact ⟨a, b, count, rfl, rfl, rfl, beq_iff_eq.mp hinc,
                  of_decide_eq_true hge⟩
  · cases ha : jsonNatField? failure "firstVertex" with
    | none => rw [ha] at h; simp at h
    | some a =>
        cases hb : jsonNatField? failure "secondVertex" with
        | none => rw [ha, hb] at h; simp at h
        | some b =>
            rw [ha, hb] at h
            dsimp only at h
            rw [Bool.or_eq_true] at h
            rcases h with h1 | h2
            · exact ⟨a, b, rfl, rfl, Or.inl (of_decide_eq_true h1)⟩
            · exact ⟨a, b, rfl, rfl, Or.inr (of_decide_eq_true h2)⟩
  · cases ht : jsonNatField? failure "triangleIndex" with
    | none => rw [ht] at h; simp at h
    | some t =>
        rw [ht] at h
        dsimp only at h
        cases htri : mesh.triangles[t]? with
        | none => rw [htri] at h; simp at h
        | some tri =>
            rw [htri] at h
            dsimp only at h
            obtain ⟨a, b, c⟩ := tri
            refine ⟨t, a, b, c, rfl, htri, ?_⟩
            rw [Bool.or_eq_true] at h
            rcases h with habc | hnorm
            · cases hab : a == b <;> cases hbc : b == c <;>
                cases hac : a == c <;> simp_all
            · cases hva : mesh.vertices[a]? with
              | none => rw [hva] at hnorm; simp at hnorm
              | some va =>
                  cases hvb : mesh.vertices[b]? with
                  | none => rw [hva, hvb] at hnorm; simp at hnorm
                  | some vb =>
                      cases hvc : mesh.vertices[c]? with
                      | none => rw [hva, hvb, hvc] at hnorm; simp at hnorm
                      | some vc =>
                          rw [hva, hvb, hvc] at hnorm
                          dsimp only at hnorm
                          exact Or.inr (Or.inr (Or.inr
                            ⟨va, vb, vc, rfl, rfl, rfl,
                              of_decide_eq_true hnorm⟩))
  · cases ht : jsonNatField? failure "triangleIndex" with
    | none => rw [ht] at h; simp at h
    | some t =>
        cases hv : jsonNatField? failure "vertexIndex" with
        | none => rw [ht, hv] at h; simp at h
        | some v =>
            rw [ht, hv] at h
            dsimp only at h
            cases htri : mesh.triangles[t]? with
            | none => rw [htri] at h; simp at h
            | some tri =>
                rw [htri] at h
                dsimp only at h
                obtain ⟨a, b, c⟩ := tri
                cases hi : [a, b, c][v]? with
                | none => rw [hi] at h; simp at h
                | some i =>
                    rw [hi] at h
                    dsimp only at h
                    exact ⟨t, v, i, a, b, c, rfl, rfl, htri, hi,
                      of_decide_eq_true h⟩
  · -- "nonFiniteVertex": the check is unconditionally false.
    simp at h
  · cases hf : jsonNatField? failure "first" with
    | none => rw [hf] at h; simp at h
    | some first =>
        cases hs : jsonNatField? failure "second" with
        | none => rw [hf, hs] at h; simp at h
        | some second =>
            cases hks : jsonStringField? failure "intersectionKind" with
            | none => rw [hf, hs, hks] at h; simp at h
            | some kind =>
                rw [hf, hs, hks] at h
                dsimp only at h
                cases hc : classifyMeshPair mesh first second with
                | none => rw [hc] at h; simp at h
                | some computed =>
                    rw [hc] at h
                    dsimp only at h
                    rw [Bool.and_eq_true] at h
                    obtain ⟨hclaim, hperm⟩ := h
                    have hclaimed := of_decide_eq_true hclaim
                    have hperm' : computed.permitted = false := by
                      cases hp : computed.permitted <;> simp_all
                    refine ⟨first, second, computed, rfl, rfl, hc, hperm', ?_⟩
                    split at hclaimed
                    · exact Or.inl ⟨rfl, (Option.some.inj hclaimed).symm⟩
                    · exact Or.inr (Or.inl ⟨rfl,
                        (Option.some.inj hclaimed).symm⟩)
                    · exact Or.inr (Or.inr ⟨rfl,
                        (Option.some.inj hclaimed).symm⟩)
                    · simp at hclaimed
  · cases ha : jsonNatField? failure "attributeCount" with
    | none => rw [ha] at h; simp at h
    | some attributes =>
        cases ht : jsonNatField? failure "triangleCount" with
        | none => rw [ha, ht] at h; simp at h
        | some triangles =>
            rw [ha, ht] at h
            dsimp only at h
            rw [Bool.and_eq_true] at h
            obtain ⟨heq, hne⟩ := h
            exact ⟨attributes, triangles, rfl, rfl, beq_iff_eq.mp heq,
              bne_iff_ne.mp hne⟩
  · simp at h
  · simp at h
  · -- unknown failure kind: the check is unconditionally false.
    simp at h

theorem checkMeshJudgment_sound (observation : MeshJudgmentObservation)
    (h : checkMeshJudgment observation = true) :
    Semantics.meshJudgmentHolds observation := by
  unfold checkMeshJudgment at h
  unfold Semantics.meshJudgmentHolds
  cases hout : observation.outcome with
  | validated closureEdges orientationEdges manifoldEdges pairs =>
      rw [hout] at h
      dsimp only at h
      rw [Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true,
        Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true,
        Bool.and_eq_true] at h
      obtain ⟨⟨⟨⟨⟨⟨⟨hWF, hND⟩, hCO⟩, hM⟩, hcl⟩, hor⟩, hma⟩, hpv⟩ := h
      refine ⟨hWF, hND, hCO, hM, beq_iff_eq.mp hcl, beq_iff_eq.mp hor,
        beq_iff_eq.mp hma, ?_⟩
      exact pairsVerified_sound _ _ hpv
  | rejected failureKind failure =>
      rw [hout] at h
      exact failureVerified_sound _ _ _ h

theorem acceptMeshJudgment_sound (observation : MeshJudgmentObservation)
    (accepted : acceptMeshJudgment observation = true) :
    Semantics.meshJudgmentHolds observation :=
  checkMeshJudgment_sound observation (of_decide_eq_true accepted)

/-- A `verified` verdict certifies the observation's declarative semantics. -/
theorem verdictOf_sound (observation : CadObservation)
    (h : Semantics.verdictOf observation = .verified) :
    Semantics.CadObservation.Holds observation := by
  cases observation with
  | exactChart o =>
      unfold Semantics.verdictOf at h
      dsimp only at h
      by_cases hacc : acceptExactChart o = true
      · rw [if_pos hacc] at h
        exact acceptExactChart_sound o hacc
      · rw [if_neg hacc] at h
        exact absurd h (by simp)
  | faceAtomCovering o =>
      unfold Semantics.verdictOf at h
      dsimp only at h
      by_cases hacc : acceptFaceAtomCovering o = true
      · rw [if_pos hacc] at h
        exact acceptFaceAtomCovering_sound o hacc
      · rw [if_neg hacc] at h
        exact absurd h (by simp)
  | finiteSurfaceRepresentation o =>
      unfold Semantics.verdictOf at h
      dsimp only at h
      by_cases hacc : acceptFiniteSurface o = true
      · rw [if_pos hacc] at h
        exact acceptFiniteSurface_sound o hacc
      · rw [if_neg hacc] at h
        exact absurd h (by simp)
  | certifiedChordBound o =>
      unfold Semantics.verdictOf at h
      dsimp only at h
      by_cases hacc : acceptChordBound o = true
      · rw [if_pos hacc] at h
        exact acceptChordBound_sound o hacc
      · rw [if_neg hacc] at h
        exact absurd h (by simp)
  | meshJudgment o =>
      unfold Semantics.verdictOf at h
      dsimp only at h
      by_cases hacc : acceptMeshJudgment o = true
      · rw [if_pos hacc] at h
        exact acceptMeshJudgment_sound o hacc
      · rw [if_neg hacc] at h
        exact absurd h (by simp)
  | typedFailure _ _ =>
      simp [Semantics.verdictOf] at h

end NecoCadVerify
