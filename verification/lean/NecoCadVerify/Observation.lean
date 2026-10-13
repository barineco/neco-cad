import NecoCadVerify.Geometry

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

-- ============================================================
-- Decoded observation structures
-- ============================================================

structure ChartPolynomials where
  x : List SurfaceTerm
  y : List SurfaceTerm
  z : List SurfaceTerm
  denominator : List SurfaceTerm
  deriving Repr, BEq, DecidableEq

structure PatchDomain where
  u : NecoAlgebraVerify.Interval
  v : NecoAlgebraVerify.Interval
  deriving Repr, BEq, DecidableEq

/-- A chart patch: its rational chart, parameter domain box, disk flag, and
    the bound form certifying `denominator ≥ constant > 0` on the box. -/
structure PatchData where
  patchIndex : Nat
  chart : ChartPolynomials
  domain : PatchDomain
  disk : Bool
  denominatorBound : SurfaceBoundForm
  deriving Repr, BEq

/-- `p ∈ domain`: box membership, plus `u²+v² ≤ 1` when `disk`. -/
def domainContains (domain : PatchDomain) (disk : Bool) (u v : Rat) : Prop :=
  domain.u.Contains u ∧ domain.v.Contains v ∧
    (disk → u * u + v * v ≤ 1)

instance decidableDomainContains (domain : PatchDomain) (disk : Bool) (u v : Rat) :
    Decidable (domainContains domain disk u v) := by
  unfold domainContains
  infer_instance

/-- The rational point `χ(u,v)` when the denominator is nonzero. -/
def chartPoint (chart : ChartPolynomials) (u v : Rat) : Vec3 :=
  let divisor := surfaceValue chart.denominator u v
  (surfaceValue chart.x u v / divisor,
   surfaceValue chart.y u v / divisor,
   surfaceValue chart.z u v / divisor)

theorem chartPoint_exactValue (chart : ChartPolynomials) (u v : Rat)
    (nonzero : surfaceValue chart.denominator u v ≠ 0) :
    NecoGeometryVerify.surfaceExactValue chart.x chart.y chart.z
      chart.denominator u v = some (chartPoint chart u v) :=
  NecoGeometryVerify.surfaceExactValue_of_nonzero chart.x chart.y chart.z
    chart.denominator u v nonzero

theorem chartPoint_checkedRange_contains (chart : ChartPolynomials)
    (domain : PatchDomain) (output : CoordinateIntervals)
    (checked : NecoGeometryVerify.surfaceCheckedRange chart.x chart.y chart.z
      chart.denominator domain.u domain.v = some output)
    (u v : Rat) (uContained : domain.u.Contains u) (vContained : domain.v.Contains v) :
    surfaceValue chart.denominator u v ≠ 0 ∧
      output.x.Contains (chartPoint chart u v).1 ∧
      output.y.Contains (chartPoint chart u v).2.1 ∧
      output.z.Contains (chartPoint chart u v).2.2 :=
  NecoGeometryVerify.surfaceCheckedRange_contains chart.x chart.y chart.z
    chart.denominator domain.u domain.v output checked u v uContained vContained

-- ---------- exactChart ----------

structure ExactChartObservation where
  faceId : String
  atomId : String
  atomRelation : String
  patchIndex : Nat
  atom : (List NecoGeometryVerify.RegionTerm)
  patch : PatchData
  deriving Repr, BEq

-- ---------- faceAtomCovering ----------

/-- A rational inverse chart `u = un/ud`, `v = vn/vd` as 3-var polynomials. -/
structure InverseChart where
  uNumerator : (List NecoGeometryVerify.RegionTerm)
  uDenominator : (List NecoGeometryVerify.RegionTerm)
  vNumerator : (List NecoGeometryVerify.RegionTerm)
  vDenominator : (List NecoGeometryVerify.RegionTerm)
  deriving Repr, BEq

structure CoveringPatch extends PatchData where
  inverse : InverseChart
  deriving Repr, BEq

structure CoveringClause where
  patch : Nat
  guards : List (List NecoGeometryVerify.RegionTerm)
  inverseDenominatorBounds : BoundForm × BoundForm
  chartDenominatorBound : BoundForm
  diskRange : BoundForm
  imageQuotients : (List NecoGeometryVerify.RegionTerm) × (List NecoGeometryVerify.RegionTerm) × (List NecoGeometryVerify.RegionTerm)
  deriving Repr, BEq

structure FaceAtomCoveringObservation where
  faceId : String
  atomId : String
  atomRelation : String
  atom : (List NecoGeometryVerify.RegionTerm)
  outerGuards : List (List NecoGeometryVerify.RegionTerm)
  patches : List CoveringPatch
  clauses : List CoveringClause
  coverageSelections : List BoundForm
  deriving Repr, BEq

-- ---------- finiteSurfaceRepresentation ----------

structure MeshVertex where
  x : Rat
  y : Rat
  z : Rat
  deriving Repr, BEq, DecidableEq

def MeshVertex.toVec3 (v : MeshVertex) : Vec3 := (v.x, v.y, v.z)

structure MeshData where
  vertices : List MeshVertex
  triangles : List (Nat × Nat × Nat)
  deriving Repr, BEq

structure VertexParameter where
  patchIndex : Nat
  u : Rat
  v : Rat
  point : Vec3
  deriving Repr, BEq

structure FaceTriangles where
  faceId : String
  atomId : String
  atomRelation : String
  atom : (List NecoGeometryVerify.RegionTerm)
  triangleIndices : List Nat
  patches : List PatchData
  vertexParameters : List VertexParameter
  residualBound : Rat
  deriving Repr, BEq

structure FiniteSurfaceObservation where
  identity : String
  sourceRevision : String
  mesh : MeshData
  faceTriangles : List FaceTriangles
  requestedChordError : Rat
  establishedChordError : Rat
  requestedSurfaceError : Rat
  establishedSurfaceError : Rat
  deriving Repr, BEq

-- ---------- certifiedChordBound ----------

structure ChordEndpoint where
  u : Rat
  v : Rat
  point : Vec3
  deriving Repr, BEq

inductive EdgeParameterization where
  | line (tLower tUpper : Rat)
  | arc (minusPole : Bool) (tLower tUpper : Rat)
  deriving Repr, BEq

structure ChordEdge where
  patchIndex : Nat
  left : ChordEndpoint
  right : ChordEndpoint
  midpointU : Rat
  midpointV : Rat
  parameterization : EdgeParameterization
  establishedSquaredBound : Rat
  subdivisionDepth : Nat
  deriving Repr, BEq

structure ChordBoundObservation where
  faceId : String
  atomId : String
  atomRelation : String
  patches : List PatchData
  edges : List ChordEdge
  triangleEdgeIndices : List (Nat × Nat × Nat × Nat)
  establishedSquaredBound : Rat
  requestedChordError : Rat
  deriving Repr, BEq

-- ---------- meshJudgment ----------

structure ExaminedPair where
  first : Nat
  second : Nat
  sharedIndexCount : Nat
  kind : PairKind
  deriving Repr, BEq

inductive MeshOutcome where
  | validated
      (closureEdges orientationEdges manifoldEdges : Nat)
      (pairs : List ExaminedPair)
  | rejected (failureKind : String) (failure : JsonFields)
  deriving Repr, BEq

structure MeshJudgmentObservation where
  mesh : MeshData
  outcome : MeshOutcome
  deriving Repr, BEq

-- ---------- batch ----------

inductive CadObservation where
  | exactChart (observation : ExactChartObservation)
  | faceAtomCovering (observation : FaceAtomCoveringObservation)
  | finiteSurfaceRepresentation (observation : FiniteSurfaceObservation)
  | certifiedChordBound (observation : ChordBoundObservation)
  | meshJudgment (observation : MeshJudgmentObservation)
  | typedFailure (failureKind : String) (reason : String)
  deriving Repr, BEq

end NecoCadVerify
