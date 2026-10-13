import NecoCadVerify.ExactChart

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

def decodeVertexParameter (value : Value) : Option VertexParameter := do
  let fields ← jsonObjectWithExactFields? value ["patchIndex", "point", "u", "v"]
  let patchIndex ← decodeNatAtMost fields "patchIndex" 64
  let u ← decodeRationalSchema (← jsonField? fields "u")
  let v ← decodeRationalSchema (← jsonField? fields "v")
  let point ← decodeVec3Point (← jsonField? fields "point")
  pure { patchIndex, u, v, point }

def decodeFaceTriangles (value : Value) : Option FaceTriangles := do
  let fields ← jsonObjectWithExactFields? value
    ["atom", "atomId", "atomRelation", "faceId", "patches", "residualBound",
      "triangleIndices", "vertexParameters"]
  let faceId ← jsonStringField? fields "faceId"
  let atomId ← jsonStringField? fields "atomId"
  let atomRelation ← decodeStringIn fields "atomRelation"
    ["negative", "positive", "zero"]
  let atom ← decodeAtomPoly (← jsonField? fields "atom")
  let triangleIndices ← decodeNatList (← jsonField? fields "triangleIndices") 65536
  let patchValues ← decodeValueArray (← jsonField? fields "patches")
  if patchValues.length > 64 || patchValues.isEmpty then none
  let patches ← patchValues.mapM decodePatchBody
  let parameterValues ← decodeValueArray (← jsonField? fields "vertexParameters")
  if parameterValues.length > 65536 then none
  let vertexParameters ← parameterValues.mapM decodeVertexParameter
  let residualBound ← decodeDyadicRatSchema (← jsonField? fields "residualBound")
  pure { faceId, atomId, atomRelation, atom, triangleIndices, patches, vertexParameters, residualBound }

def decodeFiniteSurfaceObservation (value : Value) :
    Option FiniteSurfaceObservation := do
  let fields ← jsonObjectWithExactFields? value
    ["faceTriangles", "identity", "mesh", "precision", "sourceRevision"]
  let identity ← jsonStringField? fields "identity"
  let sourceRevision ← jsonStringField? fields "sourceRevision"
  let mesh ← decodeMesh (← jsonField? fields "mesh")
  let faceValues ← decodeValueArray (← jsonField? fields "faceTriangles")
  if faceValues.length > 64 || faceValues.isEmpty then none
  let faceTriangles ← faceValues.mapM decodeFaceTriangles
  let precisionFields ← jsonObjectWithExactFields? (← jsonField? fields "precision")
    ["establishedChordError", "establishedSurfaceError",
      "requestedChordError", "requestedSurfaceError"]
  let requestedChordError ← decodeDyadicRatSchema
    (← jsonField? precisionFields "requestedChordError")
  let establishedChordError ← decodeDyadicRatSchema
    (← jsonField? precisionFields "establishedChordError")
  let requestedSurfaceError ← decodeDyadicRatSchema
    (← jsonField? precisionFields "requestedSurfaceError")
  let establishedSurfaceError ← decodeDyadicRatSchema
    (← jsonField? precisionFields "establishedSurfaceError")
  pure { identity, sourceRevision, mesh, faceTriangles, requestedChordError, establishedChordError, requestedSurfaceError, establishedSurfaceError }

/-- Squared distance bound check: `|a−b|² ≤ bound²`. -/
def squaredWithin (a b : Vec3) (bound : Rat) : Bool :=
  let d := vsub a b
  decide (vdot d d ≤ bound * bound)

/-- One vertex parameter entry certifies `vertex`:
    the patch exists, `(u,v)` lies in the patch domain, the chart evaluates
    exactly to the declared point, and the point is within `bound` of the
    vertex. -/
def vertexCovered (patches : List PatchData) (bound : Rat)
    (vertex : Vec3) (param : VertexParameter) : Bool :=
  match patches[param.patchIndex]? with
  | none => false
  | some patch =>
      decide (patch.patchIndex = param.patchIndex) &&
      decide (domainContains patch.domain patch.disk param.u param.v) &&
      decide (surfaceValue patch.chart.denominator param.u param.v ≠ 0) &&
      decide (chartPoint patch.chart param.u param.v = param.point) &&
      squaredWithin param.point vertex bound

/-- All mesh vertices referenced by the face's triangles are covered by some
    vertex parameter within `establishedSurfaceError`, and each vertex's
    implicit residual is within `residualBound`. -/
def checkFaceVertices (face : FaceTriangles) (mesh : MeshData)
    (surfaceBound : Rat) : Bool :=
  face.triangleIndices.all fun triangleIndex =>
    match mesh.triangles[triangleIndex]? with
    | none => false
    | some (i, j, k) =>
        [i, j, k].all fun vertexIndex =>
          match mesh.vertices[vertexIndex]? with
          | none => false
          | some vertex =>
              let point := vertex.toVec3
              face.vertexParameters.any
                (vertexCovered face.patches surfaceBound point) &&
              decide (|NecoGeometryVerify.RegionPolynomial.eval face.atom point.1 point.2.1 point.2.2| ≤
                face.residualBound)

def checkFiniteSurface (observation : FiniteSurfaceObservation) : Bool :=
  decide (0 ≤ observation.establishedSurfaceError) &&
  observation.faceTriangles.all fun face =>
    decide (0 ≤ face.residualBound) &&
    face.patches.all (checkPatchAgainstAtom face.atom) &&
    checkFaceVertices face observation.mesh observation.establishedSurfaceError

def FiniteSurfaceObservation.Valid (observation : FiniteSurfaceObservation) : Prop :=
  checkFiniteSurface observation = true

instance decidableFiniteSurfaceValid (observation : FiniteSurfaceObservation) : Decidable observation.Valid := by
  unfold FiniteSurfaceObservation.Valid checkFiniteSurface
  infer_instance

def acceptFiniteSurface (observation : FiniteSurfaceObservation) : Bool :=
  decide observation.Valid

end NecoCadVerify
