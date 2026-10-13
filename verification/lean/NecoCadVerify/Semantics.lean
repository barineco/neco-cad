import NecoCadVerify.ExactChart
import NecoCadVerify.FiniteSurface
import NecoCadVerify.FaceAtomCovering
import NecoCadVerify.Mesh
import NecoCadVerify.ChordBound
import NecoCadVerify.Protocol

set_option autoImplicit false

namespace NecoCadVerify.Semantics

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify
open NecoCadVerify.Protocol

-- ============================================================
-- Declarative semantics per observation family
-- ============================================================

/-- Exact chart: on the whole parameter domain the chart denominator is
    nonzero and every chart point lies on the implicit atom. -/
def chartRepresentsAtom (patch : PatchData) (atom : (List NecoGeometryVerify.RegionTerm)) : Prop :=
  ∀ u v, domainContains patch.domain patch.disk u v →
    surfaceValue patch.chart.denominator u v ≠ 0 ∧
      NecoGeometryVerify.RegionPolynomial.eval atom (chartPoint patch.chart u v).1
        (chartPoint patch.chart u v).2.1 (chartPoint patch.chart u v).2.2 = 0

/-- Finite surface: every mesh vertex referenced by a face's triangles is
    covered by a chart point within `surfaceBound`, and its implicit residual
    is within `face.residualBound`. Every face patch is an exact chart. -/
def faceVerticesCovered (face : FaceTriangles) (mesh : MeshData)
    (surfaceBound : Rat) : Prop :=
  ∀ triangleIndex ∈ face.triangleIndices,
    ∀ (i j k : Nat), mesh.triangles[triangleIndex]? = some (i, j, k) →
      ∀ vertexIndex ∈ [i, j, k],
        ∀ vertex, mesh.vertices[vertexIndex]? = some vertex →
          (∃ param ∈ face.vertexParameters,
            ∃ patch ∈ face.patches, patch.patchIndex = param.patchIndex ∧
              domainContains patch.domain patch.disk param.u param.v ∧
              surfaceValue patch.chart.denominator param.u param.v ≠ 0 ∧
              chartPoint patch.chart param.u param.v = param.point ∧
              vdot (vsub param.point vertex.toVec3)
                (vsub param.point vertex.toVec3) ≤ surfaceBound * surfaceBound) ∧
          |NecoGeometryVerify.RegionPolynomial.eval face.atom vertex.x vertex.y vertex.z| ≤ face.residualBound

def finiteSurfaceRepresents (observation : FiniteSurfaceObservation) : Prop :=
  0 ≤ observation.establishedSurfaceError ∧
    ∀ face ∈ observation.faceTriangles, 0 ≤ face.residualBound ∧
      (∀ patch ∈ face.patches, chartRepresentsAtom patch face.atom) ∧
      faceVerticesCovered face observation.mesh observation.establishedSurfaceError

/-- One covering clause covers its guard region: every atom point where the
    clause's guards are nonnegative is the chart image of a domain point of
    the referenced (disk) patch. -/
def clauseCovers (atom : (List NecoGeometryVerify.RegionTerm)) (patch : CoveringPatch) (clause : CoveringClause) :
    Prop :=
  patch.patchIndex = clause.patch ∧ patch.disk ∧
    ∀ x y z, NecoGeometryVerify.RegionPolynomial.eval atom x y z = 0 →
      (∀ g ∈ clause.guards, 0 ≤ NecoGeometryVerify.RegionPolynomial.eval g x y z) →
      ∃ u v, domainContains patch.domain true u v ∧
        chartPoint patch.chart u v = (x, y, z)

/-- Face/atom/covering correspondence: every patch is an exact chart and the
    clauses jointly cover the atom region. Every atom point satisfying the
    outer guards is the chart image of some clause's patch domain point. -/
def coveringRepresentsFace (observation : FaceAtomCoveringObservation) : Prop :=
  (∀ patch ∈ observation.patches, chartRepresentsAtom patch.toPatchData
      observation.atom) ∧
    ∀ x y z, NecoGeometryVerify.RegionPolynomial.eval observation.atom x y z = 0 →
      (∀ g ∈ observation.outerGuards, 0 ≤ NecoGeometryVerify.RegionPolynomial.eval g x y z) →
      ∃ clause ∈ observation.clauses,
        ∃ patch ∈ observation.patches,
          (∀ g ∈ clause.guards, 0 ≤ NecoGeometryVerify.RegionPolynomial.eval g x y z) ∧
            clauseCovers observation.atom patch clause

/-- Chord bound: along each declared edge parameter interval the deviation
    between the exact chart surface point and the linear chord stays within
    `edge.establishedSquaredBound`. -/
def edgeDeviationWithin (patch : PatchData) (edge : ChordEdge) : Prop :=
  let (t0, t1) := edgeParameterInterval edge
  t0 < t1 ∧
    ∀ t, t0 ≤ t ∧ t ≤ t1 →
      let (u, v) := edgeParamsAt edge t
      surfaceValue patch.chart.denominator u v ≠ 0 ∧
        let s := chartPoint patch.chart u v
        let c := (chordAxisAt edge.left.point.1 edge.right.point.1 t0 t1 t,
          chordAxisAt edge.left.point.2.1 edge.right.point.2.1 t0 t1 t,
          chordAxisAt edge.left.point.2.2 edge.right.point.2.2 t0 t1 t)
        vdot (vsub s c) (vsub s c) ≤ edge.establishedSquaredBound

def chordBoundHolds (observation : ChordBoundObservation) : Prop :=
  0 ≤ observation.establishedSquaredBound ∧
    ∀ edge ∈ observation.edges,
      ∃ patch ∈ observation.patches, patch.patchIndex = edge.patchIndex ∧
        edgeDeviationWithin patch edge ∧
        edge.establishedSquaredBound ≤ observation.establishedSquaredBound

/-- Mathematical content of a supported, independently checkable mesh failure.
    Unsupported kinds have no admitted claim in this predicate and are rejected
    by the wire decoder; False does not refute an external provenance report. -/
def meshFailureClaim (mesh : MeshData) (failureKind : String)
    (failure : JsonFields) : Prop :=
  let edges := meshDirectedEdges mesh
  match failureKind with
  | "openEdge" =>
      ∃ a b, jsonNatField? failure "firstVertex" = some a ∧
        jsonNatField? failure "secondVertex" = some b ∧
        undirectedIncidence edges a b = 1
  | "nonManifoldEdge" =>
      ∃ a b count, jsonNatField? failure "firstVertex" = some a ∧
        jsonNatField? failure "secondVertex" = some b ∧
        jsonNatField? failure "incidenceCount" = some count ∧
        undirectedIncidence edges a b = count ∧ count ≥ 3
  | "orientationMismatch" =>
      ∃ a b, jsonNatField? failure "firstVertex" = some a ∧
        jsonNatField? failure "secondVertex" = some b ∧
        (directedIncidence edges a b ≥ 2 ∨ directedIncidence edges b a ≥ 2)
  | "degenerateTriangle" =>
      ∃ t a b c, jsonNatField? failure "triangleIndex" = some t ∧
        mesh.triangles[t]? = some (a, b, c) ∧
        (a = b ∨ b = c ∨ a = c ∨
          ∃ va vb vc, mesh.vertices[a]? = some va ∧ mesh.vertices[b]? = some vb ∧
            mesh.vertices[c]? = some vc ∧
            vdot (vcross (vsub vb.toVec3 va.toVec3) (vsub vc.toVec3 va.toVec3))
              (vcross (vsub vb.toVec3 va.toVec3) (vsub vc.toVec3 va.toVec3)) = 0)
  | "triangleIndexOutOfBounds" =>
      ∃ t v i a b c, jsonNatField? failure "triangleIndex" = some t ∧
        jsonNatField? failure "vertexIndex" = some v ∧
        mesh.triangles[t]? = some (a, b, c) ∧
        [a, b, c][v]? = some i ∧ i ≥ mesh.vertices.length
  | "selfIntersection" =>
      ∃ first second computed,
        jsonNatField? failure "first" = some first ∧
        jsonNatField? failure "second" = some second ∧
        classifyMeshPair mesh first second = some computed ∧
        computed.permitted = false ∧
        (jsonStringField? failure "intersectionKind" = some "nonPermittedCrossing" ∧
          computed = PairKind.nonPermittedCrossing ∨
          jsonStringField? failure "intersectionKind" = some "coplanarOverlap" ∧
            computed = PairKind.coplanarOverlap ∨
          jsonStringField? failure "intersectionKind" = some "distinctIndexCoincidentContact" ∧
            computed = PairKind.coincidentContact)
  | "attributeCount" =>
      ∃ attributes triangles,
        jsonNatField? failure "attributeCount" = some attributes ∧
        jsonNatField? failure "triangleCount" = some triangles ∧
        triangles = mesh.triangles.length ∧ attributes ≠ 3 * triangles
  | "emptySourceIdentity" | "emptySourceRevision" => False
  | _ => False

/-- Mesh judgment semantics: a validated outcome is the independently
    re-derived closed-oriented-manifold judgment with an exact pair
    classification; a rejected outcome is a verified failure claim. -/
def meshJudgmentHolds (observation : MeshJudgmentObservation) : Prop :=
  match observation.outcome with
  | .validated closureEdges orientationEdges manifoldEdges pairs =>
      meshWellFormed observation.mesh = true ∧
      meshNonDegenerate observation.mesh = true ∧
      meshClosedOriented observation.mesh = true ∧
      meshManifold observation.mesh = true ∧
      closureEdges = (meshUndirectedEdges observation.mesh).length ∧
      orientationEdges = (meshUndirectedEdges observation.mesh).length ∧
      manifoldEdges = (meshUndirectedEdges observation.mesh).length ∧
      ∀ i j, i < j → j < observation.mesh.triangles.length →
        ∃ pair ∈ pairs, pair.first = i ∧ pair.second = j ∧
          pair.sharedIndexCount =
            (match observation.mesh.triangles[i]?, observation.mesh.triangles[j]? with
             | some t1, some t2 => sharedIndexCount t1 t2
             | _, _ => 0) ∧
          pair.kind = (classifyMeshPair observation.mesh i j).getD .empty ∧
          pair.kind.permitted = true
  | .rejected failureKind failure =>
      meshFailureClaim observation.mesh failureKind failure

-- ============================================================
-- Unified observation semantics and verdict dispatch
-- ============================================================

/-- This predicate describes the mathematical claim certified by `verified`.
    For `typedFailure`, False means that no verified mathematical claim is
    represented here; it does not assert that the reported API failure is false.
    A well-formed typed failure is classified as `undetermined`. -/
def CadObservation.Holds : CadObservation → Prop
  | .exactChart observation =>
      chartRepresentsAtom observation.patch observation.atom
  | .faceAtomCovering observation => coveringRepresentsFace observation
  | .finiteSurfaceRepresentation observation => finiteSurfaceRepresents observation
  | .certifiedChordBound observation => chordBoundHolds observation
  | .meshJudgment observation => meshJudgmentHolds observation
  | .typedFailure _ _ => False

/-- The verdict of a decoded observation. Semantic families are `verified`
    or `refuted`; a well-formed `typedFailure` is `undetermined`. -/
def verdictOf : CadObservation → Protocol.Verdict
  | .exactChart observation =>
      if acceptExactChart observation then .verified else .refuted
  | .faceAtomCovering observation =>
      if acceptFaceAtomCovering observation then .verified else .refuted
  | .finiteSurfaceRepresentation observation =>
      if acceptFiniteSurface observation then .verified else .refuted
  | .certifiedChordBound observation =>
      if acceptChordBound observation then .verified else .refuted
  | .meshJudgment observation =>
      if acceptMeshJudgment observation then .verified else .refuted
  | .typedFailure _ _ => .undetermined

end NecoCadVerify.Semantics
