import NecoGeometryVerify.Soundness
import NecoGeometryVerify.RegionPolynomial
import NecoCalculusVerify.SurfacePolynomial
import NecoCalculusVerify.RationalPolynomialSoundness
import NecoCalculusVerify.RationalPolynomialBridge

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify


-- ============================================================
-- Surface bound forms over a parameter box.
-- ============================================================

/-- `lhs = constant + Σ c·∏ domainBounds^e + Σ c·q²` where
    `domainBounds = [u−lowerU, upperU−u, v−lowerV, upperV−v]`. -/
structure SurfaceBoundForm where
  constant : Rat
  domainPowers : List (List Nat × Rat)
  squares : List (Rat × List SurfaceTerm)
  deriving Repr, BEq

def surfaceBoundExpansion (form : SurfaceBoundForm)
    (domainBounds : List (List SurfaceTerm)) : List SurfaceTerm :=
  let fromConstant := NecoCalculusVerify.SurfacePolynomial.constant form.constant
  let fromPowers := form.domainPowers.foldl (fun acc (powers, coefficient) =>
    let product := (powers.zip domainBounds).foldl (fun poly (exponent, bound) =>
      NecoCalculusVerify.SurfacePolynomial.mul poly (NecoCalculusVerify.SurfacePolynomial.pow bound exponent))
      [{ coefficient := 1, exponentU := 0, exponentV := 0 }]
    NecoCalculusVerify.SurfacePolynomial.add acc (NecoCalculusVerify.SurfacePolynomial.scale coefficient product)) []
  let fromSquares := form.squares.foldl (fun acc (coefficient, poly) =>
    NecoCalculusVerify.SurfacePolynomial.add acc (NecoCalculusVerify.SurfacePolynomial.scale coefficient (NecoCalculusVerify.SurfacePolynomial.mul poly poly))) []
  NecoCalculusVerify.SurfacePolynomial.normalize (fromConstant ++ fromPowers ++ fromSquares)

def surfaceBoundHolds (lhs : List SurfaceTerm) (form : SurfaceBoundForm)
    (domainBounds : List (List SurfaceTerm)) : Bool :=
  decide (NecoCalculusVerify.SurfacePolynomial.normalize lhs = surfaceBoundExpansion form domainBounds)

def surfaceBoundNonnegative (form : SurfaceBoundForm) : Bool :=
  decide (form.constant ≥ 0) &&
    form.domainPowers.all (fun (_, coefficient) => decide (coefficient ≥ 0)) &&
    form.squares.all (fun (coefficient, _) => decide (coefficient ≥ 0))

-- ============================================================
-- Bound forms: lhs = constant + Σ c·∏ guards^e + Σ c·q² + q·atom
-- ============================================================

structure BoundForm where
  constant : Rat
  guardPowers : List (List Nat × Rat)
  squares : List (Rat × (List NecoGeometryVerify.RegionTerm))
  atomQuotient : (List NecoGeometryVerify.RegionTerm)
  deriving Repr, BEq

/-- Expand a bound form into the polynomial it claims `lhs` equals.
    Powers beyond the guard list length are ignored (zip semantics). -/
def boundFormExpansion (form : BoundForm) (guards : List (List NecoGeometryVerify.RegionTerm)) (atom : (List NecoGeometryVerify.RegionTerm)) :
    (List NecoGeometryVerify.RegionTerm) :=
  let fromConstant := NecoGeometryVerify.RegionPolynomial.constant form.constant
  let fromGuards := form.guardPowers.foldl (fun acc (powers, coefficient) =>
    let product := (powers.zip guards).foldl (fun poly (exponent, guard) =>
      NecoGeometryVerify.RegionPolynomial.mul poly (NecoGeometryVerify.RegionPolynomial.pow guard exponent))
      [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 0 }]
    NecoGeometryVerify.RegionPolynomial.add acc (NecoGeometryVerify.RegionPolynomial.rawScale coefficient product)) []
  let fromSquares := form.squares.foldl (fun acc (coefficient, poly) =>
    NecoGeometryVerify.RegionPolynomial.add acc (NecoGeometryVerify.RegionPolynomial.rawScale coefficient (NecoGeometryVerify.RegionPolynomial.mul poly poly))) []
  let fromAtom := NecoGeometryVerify.RegionPolynomial.mul form.atomQuotient atom
  NecoGeometryVerify.RegionPolynomial.normalize (fromConstant ++ fromGuards ++ fromSquares ++ fromAtom)

/-- The certificate claim: `lhs` is literally the bound-form expansion. -/
def boundFormHolds (lhs : (List NecoGeometryVerify.RegionTerm)) (form : BoundForm)
    (guards : List (List NecoGeometryVerify.RegionTerm)) (atom : (List NecoGeometryVerify.RegionTerm)) : Bool :=
  decide (NecoGeometryVerify.RegionPolynomial.normalize lhs = boundFormExpansion form guards atom)

/-- All summand coefficients of the form are nonnegative, so on the region
    `guards ≥ 0 ∧ atom = 0` the expansion is at least `form.constant`. -/
def boundFormNonnegative (form : BoundForm) : Bool :=
  decide (form.constant ≥ 0) &&
    form.guardPowers.all (fun (_, coefficient) => decide (coefficient ≥ 0)) &&
    form.squares.all (fun (coefficient, _) => decide (coefficient ≥ 0))

-- ============================================================
-- Exact rational 3-vectors and triangle-pair classification
-- ============================================================

abbrev Vec3 := Rat × Rat × Rat

def vsub (a b : Vec3) : Vec3 := (a.1 - b.1, a.2.1 - b.2.1, a.2.2 - b.2.2)

def vdot (a b : Vec3) : Rat := a.1 * b.1 + a.2.1 * b.2.1 + a.2.2 * b.2.2

def vcross (a b : Vec3) : Vec3 :=
  (a.2.1 * b.2.2 - a.2.2 * b.2.1,
   a.2.2 * b.1 - a.1 * b.2.2,
   a.1 * b.2.1 - a.2.1 * b.1)

def vscale (s : Rat) (a : Vec3) : Vec3 := (s * a.1, s * a.2.1, s * a.2.2)

def vadd (a b : Vec3) : Vec3 := (a.1 + b.1, a.2.1 + b.2.1, a.2.2 + b.2.2)

/-- Plane offsets of a triangle's vertices relative to a plane through `origin`
    with `normal`: `n·(p−origin)` for each vertex. -/
def triangleOffsets (normal origin : Vec3) (a b c : Vec3) : Vec3 :=
  (vdot normal (vsub a origin), vdot normal (vsub b origin),
   vdot normal (vsub c origin))

/-- All three offsets share one strict sign: the triangle does not touch the
    plane. -/
def offsetsStrictlySeparated (offsets : Vec3) : Bool :=
  (offsets.1 > 0 && offsets.2.1 > 0 && offsets.2.2 > 0) ||
    (offsets.1 < 0 && offsets.2.1 < 0 && offsets.2.2 < 0)

/-- All three offsets are zero: the triangle lies in the plane. -/
def offsetsAllZero (offsets : Vec3) : Bool :=
  offsets.1 == 0 && offsets.2.1 == 0 && offsets.2.2 == 0

/-- Barycentric coordinates of `p` in triangle `(a,b,c)` with nondegenerate
    Gram determinant; `none` when the triangle is degenerate. -/
def barycentric (a b c p : Vec3) : Option (Rat × Rat) :=
  let e0 := vsub b a
  let e1 := vsub c a
  let d := vsub p a
  let d00 := vdot e0 e0
  let d01 := vdot e0 e1
  let d11 := vdot e1 e1
  let d20 := vdot d e0
  let d21 := vdot d e1
  let denom := d00 * d11 - d01 * d01
  if denom == 0 then none
  else
    let beta := (d11 * d20 - d01 * d21) / denom
    let gamma := (d00 * d21 - d01 * d20) / denom
    some (beta, gamma)

/-- `p` lies in the closed triangle `(a,b,c)` assuming `p` is in its plane:
    barycentric coordinates are in the unit triangle and the reconstructed
    point equals `p`. -/
def pointInTriangle (a b c p : Vec3) : Bool :=
  match barycentric a b c p with
  | none => p == a || p == b || p == c
  | some (beta, gamma) =>
      beta ≥ 0 && gamma ≥ 0 && beta + gamma ≤ 1 &&
        vadd a (vadd (vscale beta (vsub b a)) (vscale gamma (vsub c a))) == p

/-- The intersection point of segment `(p0,p1)` with a plane, when the segment
    straddles or touches it. Returns the plane point when it exists on the
    closed segment. -/
def segmentPlanePoint (normal origin p0 p1 : Vec3) : Option Vec3 :=
  let d0 := vdot normal (vsub p0 origin)
  let d1 := vdot normal (vsub p1 origin)
  if d0 == 0 && d1 == 0 then none
  else if d0 == 0 then some p0
  else if d1 == 0 then some p1
  else if (d0 > 0 && d1 < 0) || (d0 < 0 && d1 > 0) then
    let t := d0 / (d0 - d1)
    some (vadd p0 (vscale t (vsub p1 p0)))
  else none

/-- Whether segment `(p0,p1)` intersects triangle `(a,b,c)` in a point. -/
def segmentIntersectsTriangle (a b c p0 p1 : Vec3) : Bool :=
  let normal := vcross (vsub b a) (vsub c a)
  match segmentPlanePoint normal a p0 p1 with
  | some point => pointInTriangle a b c point
  | none =>
      -- Segment lies in or off the plane: only coplanar contact possible.
      vdot normal (vsub p0 a) == 0 && vdot normal (vsub p1 a) == 0 &&
        (pointInTriangle a b c p0 || pointInTriangle a b c p1)

/-- 2D orientation of point `p` relative to directed segment `(s0,s1)` in
    coordinates `i,j`: sign of the 2D cross product. -/
def orient2D (i j : Nat) (s0 s1 p : Vec3) : Rat :=
  let coord (v : Vec3) (axis : Nat) : Rat :=
    match axis with
    | 0 => v.1
    | 1 => v.2.1
    | _ => v.2.2
  (coord s1 i - coord s0 i) * (coord p j - coord s0 j) -
    (coord s1 j - coord s0 j) * (coord p i - coord s0 i)

/-- Whether 2D segments `(s0,s1)` and `(t0,t1)` cross in their interiors
    (proper crossing, both endpoint pairs strictly on opposite sides). -/
def segmentSegmentCrossing2D (i j : Nat) (s0 s1 t0 t1 : Vec3) : Bool :=
  let a := orient2D i j s0 s1 t0
  let b := orient2D i j s0 s1 t1
  let c := orient2D i j t0 t1 s0
  let d := orient2D i j t0 t1 s1
  ((a > 0 && b < 0) || (a < 0 && b > 0)) && ((c > 0 && d < 0) || (c < 0 && d > 0))

/-- Whether 2D point `p` lies inside-or-on triangle `(a,b,c)` projected to
    coordinates `i,j`: all three edge orientations agree (with degenerate
    edge orientations handled by strict sign consistency). -/
def pointInTriangle2D (i j : Nat) (a b c p : Vec3) : Bool :=
  let d0 := orient2D i j a b p
  let d1 := orient2D i j b c p
  let d2 := orient2D i j c a p
  (d0 ≥ 0 && d1 ≥ 0 && d2 ≥ 0) || (d0 ≤ 0 && d1 ≤ 0 && d2 ≤ 0)

/-- Two coplanar triangles overlap beyond shared vertices. `normal` is a
    nonzero normal of the shared plane; the pair is projected onto the two
    coordinates where `normal` has its largest-magnitude component dropped. -/
def coplanarViolation (a0 a1 a2 b0 b1 b2 : Vec3) (normal : Vec3) (shared : List Vec3) :
    Bool :=
  let ax := |normal.1|
  let ay := |normal.2.1|
  let az := |normal.2.2|
  let (i, j) := if ax ≥ ay && ax ≥ az then (1, 2)
    else if ay ≥ az then (0, 2) else (0, 1)
  let edgesA := [(a0, a1), (a1, a2), (a2, a0)]
  let edgesB := [(b0, b1), (b1, b2), (b2, b0)]
  -- A vertex of either triangle inside the other but not shared, or a proper
  -- interior edge crossing, is a coplanar violation.
  ([b0, b1, b2].any (fun p => pointInTriangle2D i j a0 a1 a2 p && !shared.contains p)) ||
    ([a0, a1, a2].any (fun p => pointInTriangle2D i j b0 b1 b2 p && !shared.contains p)) ||
    edgesA.any (fun (s0, s1) =>
      edgesB.any (fun (t0, t1) => segmentSegmentCrossing2D i j s0 s1 t0 t1))

/-- Pair classification kinds matching `neco-mesh` vocabulary. -/
inductive PairKind where
  | empty | sharedVertex | sharedEdgeSegment
  | nonPermittedCrossing | coplanarOverlap | coincidentContact
  deriving Repr, BEq, DecidableEq

/-- Exact rational classification of a triangle pair. `shared` is the list of
    vertex coordinates shared by index (the caller computes index overlap). -/
def classifyPair (a0 a1 a2 b0 b1 b2 : Vec3) (sharedCount : Nat) (shared : List Vec3) :
    PairKind :=
  let normalA := vcross (vsub a1 a0) (vsub a2 a0)
  let normalB := vcross (vsub b1 b0) (vsub b2 b0)
  let offB := triangleOffsets normalA a0 b0 b1 b2
  let offA := triangleOffsets normalB b0 a0 a1 a2
  if sharedCount ≥ 2 then .sharedEdgeSegment
  else if offsetsStrictlySeparated offB || offsetsStrictlySeparated offA then .empty
  else if offsetsAllZero offB then
    if coplanarViolation a0 a1 a2 b0 b1 b2 normalA shared then .coplanarOverlap
    else .empty
  else if sharedCount == 1 then
    -- B touches plane A only at the shared vertex iff exactly one offset is
    -- zero and the other two share a strict sign.
    let offsets := [offB.1, offB.2.1, offB.2.2]
    let zeros := offsets.filter (· == 0)
    let nonzero := offsets.filter (· != 0)
    if zeros.length == 1 &&
        ((nonzero.all fun x => decide (x > 0)) ||
          (nonzero.all fun x => decide (x < 0))) then .sharedVertex
    else .nonPermittedCrossing
  else
    -- sharedCount == 0, non-coplanar: any edge of either triangle meeting the
    -- other means a crossing; otherwise empty.
    let edgesB := [(b0, b1), (b1, b2), (b2, b0)]
    let edgesA := [(a0, a1), (a1, a2), (a2, a0)]
    if edgesB.any (fun (p0, p1) => segmentIntersectsTriangle a0 a1 a2 p0 p1) ||
        edgesA.any (fun (p0, p1) => segmentIntersectsTriangle b0 b1 b2 p0 p1) then
      .nonPermittedCrossing
    else .empty

/-- Whether a pair kind is permitted inside a closed oriented surface mesh. -/
def PairKind.permitted : PairKind → Bool
  | .empty | .sharedVertex | .sharedEdgeSegment => true
  | _ => false

-- ============================================================
-- Bound-form expansion evaluation
-- ============================================================

/-- Evaluate the product term built from zipped (exponent, guard) pairs. -/
private theorem boundProduct_eval (pairs : List (Nat × (List NecoGeometryVerify.RegionTerm))) (x y z : Rat) :
    ∀ acc, NecoGeometryVerify.RegionPolynomial.eval (pairs.foldl (fun poly (e, g) =>
        NecoGeometryVerify.RegionPolynomial.mul poly (NecoGeometryVerify.RegionPolynomial.pow g e)) acc) x y z =
      NecoGeometryVerify.RegionPolynomial.eval acc x y z *
        (pairs.map (fun (e, g) => NecoGeometryVerify.RegionPolynomial.eval g x y z ^ e)).prod := by
  induction pairs with
  | nil => intro acc; simp
  | cons pair rest ih =>
      intro acc
      obtain ⟨e, g⟩ := pair
      simp only [List.foldl_cons, List.map_cons, List.prod_cons]
      rw [ih (NecoGeometryVerify.RegionPolynomial.mul acc (NecoGeometryVerify.RegionPolynomial.pow g e)), NecoGeometryVerify.RegionPolynomial.mul_eval, NecoGeometryVerify.RegionPolynomial.pow_eval]
      ring

/-- Fold a family of atom-polynomial addends: evaluation distributes to a sum. -/
private theorem atomEntrySum_eval (entries : List (List Nat × Rat))
    (guards : List (List NecoGeometryVerify.RegionTerm)) (x y z : Rat) :
    ∀ acc, NecoGeometryVerify.RegionPolynomial.eval (entries.foldl (fun poly (powers, coefficient) =>
        let product := (powers.zip guards).foldl (fun p (e, g) =>
          NecoGeometryVerify.RegionPolynomial.mul p (NecoGeometryVerify.RegionPolynomial.pow g e))
          [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 0 }]
        NecoGeometryVerify.RegionPolynomial.add poly (NecoGeometryVerify.RegionPolynomial.rawScale coefficient product)) acc) x y z =
      NecoGeometryVerify.RegionPolynomial.eval acc x y z +
        (entries.map (fun (powers, coefficient) => coefficient *
          ((powers.zip guards).map (fun (e, g) => NecoGeometryVerify.RegionPolynomial.eval g x y z ^ e)).prod)).sum := by
  induction entries with
  | nil => intro acc; simp
  | cons entry rest ih =>
      intro acc
      obtain ⟨powers, coefficient⟩ := entry
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, NecoGeometryVerify.RegionPolynomial.add_eval, NecoGeometryVerify.RegionPolynomial.rawScale_eval, boundProduct_eval]
      have hone : NecoGeometryVerify.RegionPolynomial.eval [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 0 }] x y z = 1 := by
        simp [NecoGeometryVerify.RegionPolynomial.eval, List.foldl_cons, NecoGeometryVerify.RegionPolynomial.monomial]
      rw [hone]
      ring

private theorem atomSquareSum_eval (entries : List (Rat × (List NecoGeometryVerify.RegionTerm))) (x y z : Rat) :
    ∀ acc, NecoGeometryVerify.RegionPolynomial.eval (entries.foldl (fun poly (coefficient, q) =>
        NecoGeometryVerify.RegionPolynomial.add poly (NecoGeometryVerify.RegionPolynomial.rawScale coefficient (NecoGeometryVerify.RegionPolynomial.mul q q))) acc) x y z =
      NecoGeometryVerify.RegionPolynomial.eval acc x y z +
        (entries.map (fun (coefficient, q) =>
          coefficient * NecoGeometryVerify.RegionPolynomial.eval q x y z ^ 2)).sum := by
  induction entries with
  | nil => intro acc; simp
  | cons entry rest ih =>
      intro acc
      obtain ⟨coefficient, q⟩ := entry
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, NecoGeometryVerify.RegionPolynomial.add_eval, NecoGeometryVerify.RegionPolynomial.rawScale_eval, NecoGeometryVerify.RegionPolynomial.mul_eval]
      ring

theorem boundFormExpansion_eval (form : BoundForm) (guards : List (List NecoGeometryVerify.RegionTerm))
    (atom : (List NecoGeometryVerify.RegionTerm)) (x y z : Rat) :
    NecoGeometryVerify.RegionPolynomial.eval (boundFormExpansion form guards atom) x y z =
      form.constant +
        (form.guardPowers.map (fun (powers, coefficient) => coefficient *
          ((powers.zip guards).map (fun (e, g) => NecoGeometryVerify.RegionPolynomial.eval g x y z ^ e)).prod)).sum +
        (form.squares.map (fun (coefficient, q) =>
          coefficient * NecoGeometryVerify.RegionPolynomial.eval q x y z ^ 2)).sum +
        NecoGeometryVerify.RegionPolynomial.eval form.atomQuotient x y z * NecoGeometryVerify.RegionPolynomial.eval atom x y z := by
  unfold boundFormExpansion
  rw [NecoGeometryVerify.RegionPolynomial.normalize_eval, NecoGeometryVerify.RegionPolynomial.eval_append, NecoGeometryVerify.RegionPolynomial.eval_append, NecoGeometryVerify.RegionPolynomial.eval_append,
    NecoGeometryVerify.RegionPolynomial.constant_eval, atomEntrySum_eval, atomSquareSum_eval, NecoGeometryVerify.RegionPolynomial.mul_eval]
  simp [NecoGeometryVerify.RegionPolynomial.eval]

private theorem boundProduct_nonneg (pairs : List (Nat × (List NecoGeometryVerify.RegionTerm))) (x y z : Rat)
    (guards_nonneg : ∀ g ∈ pairs.map Prod.snd, 0 ≤ NecoGeometryVerify.RegionPolynomial.eval g x y z) :
    0 ≤ (pairs.map (fun (e, g) => NecoGeometryVerify.RegionPolynomial.eval g x y z ^ e)).prod := by
  induction pairs with
  | nil => simp
  | cons pair rest ih =>
      obtain ⟨e, g⟩ := pair
      simp only [List.map_cons, List.prod_cons]
      apply mul_nonneg
      · exact pow_nonneg (guards_nonneg g (by simp)) e
      · exact ih (fun g' hg' => guards_nonneg g' (by simp [hg']))

theorem boundForm_lower (form : BoundForm) (guards : List (List NecoGeometryVerify.RegionTerm)) (atom : (List NecoGeometryVerify.RegionTerm))
    (x y z : Rat)
    (nonneg : boundFormNonnegative form = true)
    (guards_nonneg : ∀ g ∈ guards, 0 ≤ NecoGeometryVerify.RegionPolynomial.eval g x y z)
    (atom_zero : NecoGeometryVerify.RegionPolynomial.eval atom x y z = 0) :
    form.constant ≤ NecoGeometryVerify.RegionPolynomial.eval (boundFormExpansion form guards atom) x y z := by
  rw [boundFormNonnegative, Bool.and_eq_true, Bool.and_eq_true] at nonneg
  obtain ⟨⟨hconst, hpall⟩, hsall⟩ := nonneg
  rw [boundFormExpansion_eval, atom_zero, mul_zero, add_zero]
  have hpowers : ∀ (powers : List Nat) (coefficient : Rat),
      (powers, coefficient) ∈ form.guardPowers → 0 ≤ coefficient := by
    intro powers coefficient hmem
    have h := List.all_eq_true.mp hpall (powers, coefficient) hmem
    exact of_decide_eq_true h
  have hsquares : ∀ (coefficient : Rat) (q : (List NecoGeometryVerify.RegionTerm)),
      (coefficient, q) ∈ form.squares → 0 ≤ coefficient := by
    intro coefficient q hmem
    have h := List.all_eq_true.mp hsall (coefficient, q) hmem
    exact of_decide_eq_true h
  have hterm1 : ∀ (powers : List Nat) (coefficient : Rat),
      (powers, coefficient) ∈ form.guardPowers →
      0 ≤ coefficient *
        ((powers.zip guards).map (fun (e, g) => NecoGeometryVerify.RegionPolynomial.eval g x y z ^ e)).prod := by
    intro powers coefficient hmem
    apply mul_nonneg (hpowers powers coefficient hmem)
    apply boundProduct_nonneg
    intro g hg
    simp only [List.mem_map] at hg
    obtain ⟨⟨e, g'⟩, hgmem, rfl⟩ := hg
    exact guards_nonneg g' (List.of_mem_zip hgmem |>.2)
  have hterm2 : ∀ (coefficient : Rat) (q : (List NecoGeometryVerify.RegionTerm)),
      (coefficient, q) ∈ form.squares →
      0 ≤ coefficient * NecoGeometryVerify.RegionPolynomial.eval q x y z ^ 2 := by
    intro coefficient q hmem
    exact mul_nonneg (hsquares coefficient q hmem) (sq_nonneg _)
  have hsum1 : 0 ≤ (form.guardPowers.map (fun (powers, coefficient) => coefficient *
      ((powers.zip guards).map (fun (e, g) => NecoGeometryVerify.RegionPolynomial.eval g x y z ^ e)).prod)).sum := by
    apply List.sum_nonneg
    intro v hv
    simp only [List.mem_map] at hv
    obtain ⟨⟨powers, coefficient⟩, hmem, rfl⟩ := hv
    exact hterm1 powers coefficient hmem
  have hsum2 : 0 ≤ (form.squares.map (fun (coefficient, q) =>
      coefficient * NecoGeometryVerify.RegionPolynomial.eval q x y z ^ 2)).sum := by
    apply List.sum_nonneg
    intro v hv
    simp only [List.mem_map] at hv
    obtain ⟨⟨coefficient, q⟩, hmem, rfl⟩ := hv
    exact hterm2 coefficient q hmem
  linarith

/-- Evaluation of the surface bound-form expansion. -/
private theorem surfaceBoundProduct_eval (pairs : List (Nat × List SurfaceTerm))
    (u v : Rat) :
    ∀ acc, surfaceValue (pairs.foldl (fun poly (e, b) =>
        NecoCalculusVerify.SurfacePolynomial.mul poly (NecoCalculusVerify.SurfacePolynomial.pow b e)) acc) u v =
      surfaceValue acc u v *
        (pairs.map (fun (e, b) => surfaceValue b u v ^ e)).prod := by
  induction pairs with
  | nil => intro acc; simp
  | cons pair rest ih =>
      intro acc
      obtain ⟨e, b⟩ := pair
      simp only [List.foldl_cons, List.map_cons, List.prod_cons]
      rw [ih (NecoCalculusVerify.SurfacePolynomial.mul acc (NecoCalculusVerify.SurfacePolynomial.pow b e)), NecoCalculusVerify.SurfacePolynomial.mul_eval, NecoCalculusVerify.SurfacePolynomial.pow_eval]
      ring

private theorem surfaceEntrySum_eval (entries : List (List Nat × Rat))
    (bounds : List (List SurfaceTerm)) (u v : Rat) :
    ∀ acc, surfaceValue (entries.foldl (fun poly (powers, coefficient) =>
        let product := (powers.zip bounds).foldl (fun p (e, b) =>
          NecoCalculusVerify.SurfacePolynomial.mul p (NecoCalculusVerify.SurfacePolynomial.pow b e))
          [{ coefficient := 1, exponentU := 0, exponentV := 0 }]
        NecoCalculusVerify.SurfacePolynomial.add poly (NecoCalculusVerify.SurfacePolynomial.scale coefficient product)) acc) u v =
      surfaceValue acc u v +
        (entries.map (fun (powers, coefficient) => coefficient *
          ((powers.zip bounds).map (fun (e, b) => surfaceValue b u v ^ e)).prod)).sum := by
  induction entries with
  | nil => intro acc; simp
  | cons entry rest ih =>
      intro acc
      obtain ⟨powers, coefficient⟩ := entry
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, NecoCalculusVerify.SurfacePolynomial.add_eval, NecoCalculusVerify.SurfacePolynomial.scale_eval, surfaceBoundProduct_eval]
      have hone : surfaceValue [{ coefficient := 1, exponentU := 0, exponentV := 0 }] u v = 1 := by
        simp [surfaceValue, List.foldl_cons, monomial]
      rw [hone]
      ring

private theorem surfaceSquareSum_eval (entries : List (Rat × List SurfaceTerm))
    (u v : Rat) :
    ∀ acc, surfaceValue (entries.foldl (fun poly (coefficient, q) =>
        NecoCalculusVerify.SurfacePolynomial.add poly (NecoCalculusVerify.SurfacePolynomial.scale coefficient (NecoCalculusVerify.SurfacePolynomial.mul q q))) acc) u v =
      surfaceValue acc u v +
        (entries.map (fun (coefficient, q) =>
          coefficient * surfaceValue q u v ^ 2)).sum := by
  induction entries with
  | nil => intro acc; simp
  | cons entry rest ih =>
      intro acc
      obtain ⟨coefficient, q⟩ := entry
      simp only [List.foldl_cons, List.map_cons, List.sum_cons]
      rw [ih, NecoCalculusVerify.SurfacePolynomial.add_eval, NecoCalculusVerify.SurfacePolynomial.scale_eval, NecoCalculusVerify.SurfacePolynomial.mul_eval]
      ring

theorem surfaceBoundExpansion_eval (form : SurfaceBoundForm)
    (bounds : List (List SurfaceTerm)) (u v : Rat) :
    surfaceValue (surfaceBoundExpansion form bounds) u v =
      form.constant +
        (form.domainPowers.map (fun (powers, coefficient) => coefficient *
          ((powers.zip bounds).map (fun (e, b) => surfaceValue b u v ^ e)).prod)).sum +
        (form.squares.map (fun (coefficient, q) =>
          coefficient * surfaceValue q u v ^ 2)).sum := by
  unfold surfaceBoundExpansion
  rw [NecoCalculusVerify.SurfacePolynomial.normalize_eval, NecoCalculusVerify.SurfacePolynomial.value_append, NecoCalculusVerify.SurfacePolynomial.value_append,
    NecoCalculusVerify.SurfacePolynomial.constant_eval, surfaceEntrySum_eval, surfaceSquareSum_eval]
  simp [surfaceValue]

theorem surfaceBound_lower (form : SurfaceBoundForm)
    (bounds : List (List SurfaceTerm)) (u v : Rat)
    (nonneg : surfaceBoundNonnegative form = true)
    (bounds_nonneg : ∀ b ∈ bounds, 0 ≤ surfaceValue b u v) :
    form.constant ≤ surfaceValue (surfaceBoundExpansion form bounds) u v := by
  rw [surfaceBoundNonnegative, Bool.and_eq_true, Bool.and_eq_true] at nonneg
  obtain ⟨⟨hconst, hpall⟩, hsall⟩ := nonneg
  rw [surfaceBoundExpansion_eval]
  have hpowers : ∀ (powers : List Nat) (coefficient : Rat),
      (powers, coefficient) ∈ form.domainPowers → 0 ≤ coefficient := by
    intro powers coefficient hmem
    have h := List.all_eq_true.mp hpall (powers, coefficient) hmem
    exact of_decide_eq_true h
  have hsquares : ∀ (coefficient : Rat) (q : List SurfaceTerm),
      (coefficient, q) ∈ form.squares → 0 ≤ coefficient := by
    intro coefficient q hmem
    have h := List.all_eq_true.mp hsall (coefficient, q) hmem
    exact of_decide_eq_true h
  have hprod : ∀ pairs : List (Nat × List SurfaceTerm),
      (∀ b ∈ pairs.map Prod.snd, 0 ≤ surfaceValue b u v) →
      0 ≤ (pairs.map (fun (e, b) => surfaceValue b u v ^ e)).prod := by
    intro pairs hnn
    induction pairs with
    | nil => simp
    | cons pair rest ih' =>
        obtain ⟨e, b⟩ := pair
        simp only [List.map_cons, List.prod_cons]
        apply mul_nonneg
        · exact pow_nonneg (hnn b (by simp)) e
        · exact ih' (fun b' hb' => hnn b' (by simp [hb']))
  have hsum1 : 0 ≤ (form.domainPowers.map (fun (powers, coefficient) => coefficient *
      ((powers.zip bounds).map (fun (e, b) => surfaceValue b u v ^ e)).prod)).sum := by
    apply List.sum_nonneg
    intro w hw
    simp only [List.mem_map] at hw
    obtain ⟨⟨powers, coefficient⟩, hmem, rfl⟩ := hw
    apply mul_nonneg (hpowers powers coefficient hmem)
    apply hprod
    intro b hb
    simp only [List.mem_map] at hb
    obtain ⟨⟨e, b'⟩, hbmem, rfl⟩ := hb
    exact bounds_nonneg b' (List.of_mem_zip hbmem |>.2)
  have hsum2 : 0 ≤ (form.squares.map (fun (coefficient, q) =>
      coefficient * surfaceValue q u v ^ 2)).sum := by
    apply List.sum_nonneg
    intro w hw
    simp only [List.mem_map] at hw
    obtain ⟨⟨coefficient, q⟩, hmem, rfl⟩ := hw
    exact mul_nonneg (hsquares coefficient q hmem) (sq_nonneg _)
  linarith

/-- `boundFormHolds` transfers the expansion's evaluation to `lhs`. -/
theorem boundFormHolds_eval (lhs : (List NecoGeometryVerify.RegionTerm)) (form : BoundForm)
    (guards : List (List NecoGeometryVerify.RegionTerm)) (atom : (List NecoGeometryVerify.RegionTerm)) (x y z : Rat)
    (holds : boundFormHolds lhs form guards atom = true) :
    NecoGeometryVerify.RegionPolynomial.eval lhs x y z = NecoGeometryVerify.RegionPolynomial.eval (boundFormExpansion form guards atom) x y z := by
  have h := of_decide_eq_true holds
  have := congrArg (fun t => NecoGeometryVerify.RegionPolynomial.eval t x y z) h
  simpa [NecoGeometryVerify.RegionPolynomial.normalize_eval] using this

theorem surfaceBoundHolds_eval (lhs : List SurfaceTerm) (form : SurfaceBoundForm)
    (bounds : List (List SurfaceTerm)) (u v : Rat)
    (holds : surfaceBoundHolds lhs form bounds = true) :
    surfaceValue lhs u v = surfaceValue (surfaceBoundExpansion form bounds) u v := by
  have h := of_decide_eq_true holds
  have := congrArg (fun t => surfaceValue t u v) h
  simpa [NecoCalculusVerify.SurfacePolynomial.normalize_eval] using this

end NecoCadVerify
