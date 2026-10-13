import NecoCadVerify.Decode
import NecoCadVerify.ExactChart

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

-- ============================================================
-- Edge parameterization: univariate rational maps t ↦ (u(t), v(t))
-- ============================================================

/-- Full univariate maps `(uN, uD, vN, vD, tLower, tUpper)` for an edge.
    Line edges interpolate the endpoint parameters; arc edges use the
    rational circle parameterization of the declared branch. -/
def edgeMaps (edge : ChordEdge) :
    List Rat × List Rat × List Rat × List Rat × Rat × Rat :=
  match edge.parameterization with
  | .line tLower tUpper =>
      let span := tUpper - tLower
      let du := edge.right.u - edge.left.u
      let dv := edge.right.v - edge.left.v
      ([edge.left.u * tUpper - edge.right.u * tLower, du], [span],
       [edge.left.v * tUpper - edge.right.v * tLower, dv], [span],
       tLower, tUpper)
  | .arc minusPole tLower tUpper =>
      if minusPole then
        -- `τ = v/(1−u)` inverse: `u = (τ²−1)/(τ²+1)`, `v = 2τ/(τ²+1)`
        ([-1, 0, 1], [1, 0, 1], [0, 2], [1, 0, 1], tLower, tUpper)
      else
        -- `τ = v/(1+u)` inverse: `u = (1−τ²)/(τ²+1)`, `v = 2τ/(τ²+1)`
        ([1, 0, -1], [1, 0, 1], [0, 2], [1, 0, 1], tLower, tUpper)

/-- Parameter interval of an edge. -/
def edgeParameterInterval (edge : ChordEdge) : Rat × Rat :=
  match edge.parameterization with
  | .line tLower tUpper => (tLower, tUpper)
  | .arc _ tLower tUpper => (tLower, tUpper)

/-- `u(t)` and `v(t)` of an edge at parameter `t`. -/
def edgeParamsAt (edge : ChordEdge) (t : Rat) : Rat × Rat :=
  let (uN, uD, vN, vD, _, _) := edgeMaps edge
  (NecoCalculusVerify.RationalPolynomial.eval uN t / NecoCalculusVerify.RationalPolynomial.eval uD t, NecoCalculusVerify.RationalPolynomial.eval vN t / NecoCalculusVerify.RationalPolynomial.eval vD t)

/-- The linear chord `c(t)` between the endpoint 3D points over `[t0,t1]`. -/
def chordNumerator (p0 p1 t0 t1 : Rat) : List Rat :=
  [p0 * t1 - p1 * t0, p1 - p0]

def chordDenominator (t0 t1 : Rat) : List Rat := [t1 - t0]

def chordAxisAt (p0 p1 t0 t1 t : Rat) : Rat :=
  NecoCalculusVerify.RationalPolynomial.eval (chordNumerator p0 p1 t0 t1) t / NecoCalculusVerify.RationalPolynomial.eval (chordDenominator t0 t1) t

/-- Per-axis deviation polynomials `dev(t) = χ_axis(u(t),v(t)) − c_axis(t)`
    as `devNum/devDen`. -/
def deviationPolys (axisTerms denominator : List SurfaceTerm)
    (uN uD vN vD : List Rat) (p0 p1 t0 t1 : Rat) : List Rat × List Rat :=
  let (sN, sD) := NecoCalculusVerify.RationalPolynomial.composeSurface axisTerms uN uD vN vD
  let (dN, dD) := NecoCalculusVerify.RationalPolynomial.composeSurface denominator uN uD vN vD
  -- s_axis = (sN/sD)/(dN/dD) = sN·dD/(sD·dN)
  let sNum := NecoCalculusVerify.RationalPolynomial.mul sN dD
  let sDen := NecoCalculusVerify.RationalPolynomial.mul sD dN
  let cNum := chordNumerator p0 p1 t0 t1
  let cDen := chordDenominator t0 t1
  (NecoCalculusVerify.RationalPolynomial.normalize (NecoCalculusVerify.RationalPolynomial.sub (NecoCalculusVerify.RationalPolynomial.mul sNum cDen) (NecoCalculusVerify.RationalPolynomial.mul cNum sDen)), NecoCalculusVerify.RationalPolynomial.normalize (NecoCalculusVerify.RationalPolynomial.mul sDen cDen))

/-- Whether `(u(t),v(t))` stays in the unit disk over `[t0,t1]`: every
    sub-interval reached by bisection up to `depth` either has a denominator
    interval excluding zero and a nonpositive upper bound of
    `uN²vD²+vN²uD²−uD²vD²`, or is split further. -/
def mapStaysInDisk (uN uD vN vD : List Rat) (t0 t1 : Rat) (depth : Nat) : Bool :=
  let wNum := NecoCalculusVerify.RationalPolynomial.normalize (NecoCalculusVerify.RationalPolynomial.sub
    (NecoCalculusVerify.RationalPolynomial.add (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow uN 2) (NecoCalculusVerify.RationalPolynomial.pow vD 2)) (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow vN 2) (NecoCalculusVerify.RationalPolynomial.pow uD 2)))
    (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow uD 2) (NecoCalculusVerify.RationalPolynomial.pow vD 2)))
  let wDen := NecoCalculusVerify.RationalPolynomial.normalize (NecoCalculusVerify.RationalPolynomial.mul (NecoCalculusVerify.RationalPolynomial.pow uD 2) (NecoCalculusVerify.RationalPolynomial.pow vD 2))
  NecoCalculusVerify.RationalPolynomial.nonpositiveSub wNum wDen t0 t1 depth

/-- Whether the affine map stays in the domain box: endpoints suffice since
    the map is affine in `t`. -/
def affineInInterval (x0 x1 : Rat) (bound : NecoAlgebraVerify.Interval) : Bool :=
  decide (bound.lower ≤ x0 ∧ x0 ≤ bound.upper) &&
    decide (bound.lower ≤ x1 ∧ x1 ≤ bound.upper)

/-- One edge's checks: parameterization consistency, endpoint/midpoint chart
    fidelity, domain containment, and the adaptively subdivided deviation bound
    (`subdivisionDepth` is the maximum depth). -/
def checkChordEdge (patch : PatchData) (faceBound : Rat) (edge : ChordEdge) : Bool :=
  let (uN, uD, vN, vD, t0, t1) := edgeMaps edge
  let domain := patch.domain
  -- Line maps are built from the declared endpoints so `t0 → left`,
  -- `t1 → right` by construction. Arc τ can traverse either direction along
  -- left → right (e.g. plusPole where τ decreases): accept both orders.
  let endpointsCovered := match edge.parameterization with
    | .line _ _ =>
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt uN uD t0 = edge.left.u) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt vN vD t0 = edge.left.v) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt uN uD t1 = edge.right.u) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt vN vD t1 = edge.right.v)
    | .arc _ _ _ =>
        (decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt uN uD t0 = edge.left.u) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt vN vD t0 = edge.left.v) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt uN uD t1 = edge.right.u) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt vN vD t1 = edge.right.v)) ||
        (decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt uN uD t0 = edge.right.u) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt vN vD t0 = edge.right.v) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt uN uD t1 = edge.left.u) &&
        decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt vN vD t1 = edge.left.v))
  let paramsOk := endpointsCovered &&
    decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt uN uD ((t0 + t1) / 2) = edge.midpointU) &&
    decide (NecoCalculusVerify.RationalPolynomial.rationalValueAt vN vD ((t0 + t1) / 2) = edge.midpointV) &&
    -- Endpoint points equal the exact chart evaluation.
    decide (surfaceValue patch.chart.denominator edge.left.u edge.left.v ≠ 0) &&
    decide (chartPoint patch.chart edge.left.u edge.left.v = edge.left.point) &&
    decide (surfaceValue patch.chart.denominator edge.right.u edge.right.v ≠ 0) &&
    decide (chartPoint patch.chart edge.right.u edge.right.v = edge.right.point) &&
    decide (t0 < t1)
  let domainOk := match edge.parameterization with
    | .line _ _ =>
        -- Affine map into a convex box: endpoints decide.
        affineInInterval edge.left.u edge.right.u domain.u &&
          affineInInterval edge.left.v edge.right.v domain.v &&
          (!patch.disk || mapStaysInDisk uN uD vN vD t0 t1 edge.subdivisionDepth)
    | .arc _ _ _ =>
        -- Arc parameters lie on the unit circle; require the disk flag and a
        -- box containing `[-1,1]²`, plus endpoints in domain.
        patch.disk &&
        decide (domain.u.lower ≤ -1) && decide (domain.u.upper ≥ 1) &&
        decide (domain.v.lower ≤ -1) && decide (domain.v.upper ≥ 1) &&
        decide (domainContains domain true edge.left.u edge.left.v) &&
        decide (domainContains domain true edge.right.u edge.right.v)
  let axisDeviation (axisTerms : List SurfaceTerm) (p0 p1 : Rat) : List Rat × List Rat :=
    deviationPolys axisTerms patch.chart.denominator uN uD vN vD p0 p1 t0 t1
  let devX := axisDeviation patch.chart.x edge.left.point.1 edge.right.point.1
  let devY := axisDeviation patch.chart.y edge.left.point.2.1 edge.right.point.2.1
  let devZ := axisDeviation patch.chart.z edge.left.point.2.2 edge.right.point.2.2
  let boundOk := decide (0 ≤ edge.establishedSquaredBound) &&
    decide (edge.establishedSquaredBound ≤ faceBound) &&
    NecoCalculusVerify.RationalPolynomial.squaredSumWithinSub
      devX.1 devX.2 devY.1 devY.2 devZ.1 devZ.2
      edge.establishedSquaredBound t0 t1 edge.subdivisionDepth
  paramsOk && domainOk && boundOk

def decodeEndpoint (value : Value) : Option ChordEndpoint := do
  let fields ← jsonObjectWithExactFields? value ["point", "u", "v"]
  let u ← decodeRationalSchema (← jsonField? fields "u")
  let v ← decodeRationalSchema (← jsonField? fields "v")
  let point ← decodeVec3Point (← jsonField? fields "point")
  pure { u, v, point }

def decodeParameterization (value : Value) : Option EdgeParameterization := do
  let fields ← jsonFields? value
  let kind ← jsonStringField? fields "kind"
  match kind with
  | "line" => do
      let exact ← jsonObjectWithExactFields? value ["kind", "tLower", "tUpper"]
      let tLower ← decodeRationalSchema (← jsonField? exact "tLower")
      let tUpper ← decodeRationalSchema (← jsonField? exact "tUpper")
      pure (.line tLower tUpper)
  | "arc" => do
      let exact ← jsonObjectWithExactFields? value ["branch", "kind", "tLower", "tUpper"]
      let branch ← decodeStringIn exact "branch" ["minusPole", "plusPole"]
      let tLower ← decodeRationalSchema (← jsonField? exact "tLower")
      let tUpper ← decodeRationalSchema (← jsonField? exact "tUpper")
      pure (.arc (branch == "minusPole") tLower tUpper)
  | _ => none

def decodeChordEdge (value : Value) : Option ChordEdge := do
  let fields ← jsonObjectWithExactFields? value
    ["establishedSquaredBound", "left", "midpointU", "midpointV",
      "parameterization", "patchIndex", "right", "shape", "subdivisionDepth"]
  let patchIndex ← decodeNatAtMost fields "patchIndex" 64
  let left ← decodeEndpoint (← jsonField? fields "left")
  let right ← decodeEndpoint (← jsonField? fields "right")
  let midpointU ← decodeRationalSchema (← jsonField? fields "midpointU")
  let midpointV ← decodeRationalSchema (← jsonField? fields "midpointV")
  let shapeFields ← jsonFields? (← jsonField? fields "shape")
  let shapeKind ← jsonStringField? shapeFields "kind"
  let parameterization ← decodeParameterization
    (← jsonField? fields "parameterization")
  let kindsMatch := match shapeKind, parameterization with
    | "line", .line _ _ => true
    | "arc", .arc _ _ _ => true
    | _, _ => false
  if !kindsMatch then none
  let establishedSquaredBound ← decodeDyadicRatSchema
    (← jsonField? fields "establishedSquaredBound")
  let subdivisionDepth ← decodeNatAtMost fields "subdivisionDepth" 8
  pure { patchIndex, left, right, midpointU, midpointV, parameterization, establishedSquaredBound, subdivisionDepth }

def decodeChordBoundObservation (value : Value) : Option ChordBoundObservation := do
  let fields ← jsonObjectWithExactFields? value
    ["atomId", "atomRelation", "edges", "establishedSquaredBound", "faceId",
      "patches", "requestedChordError", "triangles"]
  let faceId ← jsonStringField? fields "faceId"
  let atomId ← jsonStringField? fields "atomId"
  let atomRelation ← decodeStringIn fields "atomRelation"
    ["negative", "positive", "zero"]
  let patchValues ← decodeValueArray (← jsonField? fields "patches")
  if patchValues.length > 64 || patchValues.isEmpty then none
  let patches ← patchValues.mapM decodePatchBody
  let edgeValues ← decodeValueArray (← jsonField? fields "edges")
  if edgeValues.length > 65536 then none
  let edges ← edgeValues.mapM decodeChordEdge
  let triangleValues ← decodeValueArray (← jsonField? fields "triangles")
  if triangleValues.length > 65536 then none
  let triangleEdgeIndices ← triangleValues.mapM fun item => do
    let itemFields ← jsonObjectWithExactFields? item ["edgeIndices", "patchIndex"]
    let patchIndex ← decodeNatAtMost itemFields "patchIndex" 64
    let (a, b, c) ← decodeNatTriple (← jsonField? itemFields "edgeIndices")
    pure (patchIndex, a, b, c)
  let establishedSquaredBound ← decodeDyadicRatSchema
    (← jsonField? fields "establishedSquaredBound")
  let requestedChordError ← decodeDyadicRatSchema
    (← jsonField? fields "requestedChordError")
  pure { faceId, atomId, atomRelation, patches, edges, triangleEdgeIndices, establishedSquaredBound, requestedChordError }

def checkChordBound (observation : ChordBoundObservation) : Bool :=
  decide (0 ≤ observation.establishedSquaredBound) &&
  observation.patches.all checkPatchBound &&
  observation.edges.all fun edge =>
    match observation.patches[edge.patchIndex]? with
    | none => false
    | some patch =>
        decide (patch.patchIndex = edge.patchIndex) &&
        checkChordEdge patch observation.establishedSquaredBound edge

def ChordBoundObservation.Valid (observation : ChordBoundObservation) : Prop :=
  checkChordBound observation = true

instance decidableChordBoundValid (observation : ChordBoundObservation) : Decidable observation.Valid := by
  unfold ChordBoundObservation.Valid checkChordBound
  infer_instance

def acceptChordBound (observation : ChordBoundObservation) : Bool :=
  decide observation.Valid

end NecoCadVerify
