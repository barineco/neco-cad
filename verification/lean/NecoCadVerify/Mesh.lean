import NecoCadVerify.Decode

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

-- ============================================================
-- Combinatorial mesh properties
-- ============================================================

/-- Directed edges `(a,b)` of a triangle. -/
def triangleEdges (t : Nat × Nat × Nat) : List (Nat × Nat) :=
  [(t.1, t.2.1), (t.2.1, t.2.2), (t.2.2, t.1)]

/-- All directed edges of the mesh, in order. -/
def meshDirectedEdges (mesh : MeshData) : List (Nat × Nat) :=
  mesh.triangles.foldl (fun acc t => acc ++ triangleEdges t) []

/-- Incidence count of the undirected edge `{a,b}` (both directions). -/
def undirectedIncidence (edges : List (Nat × Nat)) (a b : Nat) : Nat :=
  (edges.filter (fun (x, y) => (x == a && y == b) || (x == b && y == a))).length

/-- Direction count of the directed edge `(a,b)`. -/
def directedIncidence (edges : List (Nat × Nat)) (a b : Nat) : Nat :=
  (edges.filter (fun (x, y) => x == a && y == b)).length

/-- Undirected edge set of the mesh (each unordered pair once). -/
def meshUndirectedEdges (mesh : MeshData) : List (Nat × Nat) :=
  let directed := meshDirectedEdges mesh
  (directed.map (fun (a, b) => (min a b, max a b))).eraseDups

/-- Every undirected edge is used by exactly two directed half-edges with
    opposite orientations: closed and consistently oriented. -/
def meshClosedOriented (mesh : MeshData) : Bool :=
  let directed := meshDirectedEdges mesh
  meshUndirectedEdges mesh |>.all fun (a, b) =>
    directedIncidence directed a b == 1 && directedIncidence directed b a == 1

/-- Every undirected edge has at most two incident half-edges: manifold. -/
def meshManifold (mesh : MeshData) : Bool :=
  let directed := meshDirectedEdges mesh
  meshUndirectedEdges mesh |>.all fun (a, b) =>
    undirectedIncidence directed a b ≤ 2

/-- Triangles have distinct vertex indices within range. -/
def meshWellFormed (mesh : MeshData) : Bool :=
  mesh.triangles.all fun (a, b, c) =>
    a != b && b != c && a != c &&
      a < mesh.vertices.length && b < mesh.vertices.length &&
        c < mesh.vertices.length

/-- Triangles are non-degenerate geometrically (nonzero normal). -/
def meshNonDegenerate (mesh : MeshData) : Bool :=
  mesh.triangles.all fun (a, b, c) =>
    match mesh.vertices[a]?, mesh.vertices[b]?, mesh.vertices[c]? with
    | some va, some vb, some vc =>
        let n := vcross (vsub vb.toVec3 va.toVec3) (vsub vc.toVec3 va.toVec3)
        decide (vdot n n ≠ 0)
    | _, _, _ => false

/-- Shared index count of two triangles. -/
def sharedIndexCount (t1 t2 : Nat × Nat × Nat) : Nat :=
  let l1 := [t1.1, t1.2.1, t1.2.2]
  (l1.filter (fun i => i == t2.1 || i == t2.2.1 || i == t2.2.2)).length

/-- Shared vertex coordinates of two triangles. -/
def sharedVertices (mesh : MeshData) (t1 t2 : Nat × Nat × Nat) : List Vec3 :=
  let l1 := [t1.1, t1.2.1, t1.2.2]
  let l2 := [t2.1, t2.2.1, t2.2.2]
  (l1.filter fun i => l2.contains i).filterMap fun i => mesh.vertices[i]?.map (·.toVec3)

/-- Exact reclassification of one triangle pair of the mesh. -/
def classifyMeshPair (mesh : MeshData) (i j : Nat) : Option PairKind :=
  match mesh.triangles[i]?, mesh.triangles[j]? with
  | some t1, some t2 =>
      match t1, t2 with
      | (a0, a1, a2), (b0, b1, b2) =>
          match mesh.vertices[a0]?, mesh.vertices[a1]?, mesh.vertices[a2]?,
            mesh.vertices[b0]?, mesh.vertices[b1]?, mesh.vertices[b2]? with
          | some va0, some va1, some va2, some vb0, some vb1, some vb2 =>
              some (classifyPair
                va0.toVec3 va1.toVec3 va2.toVec3
                vb0.toVec3 vb1.toVec3 vb2.toVec3
                (sharedIndexCount t1 t2) (sharedVertices mesh t1 t2))
          | _, _, _, _, _, _ => none
  | _, _ => none

/-- All triangle pairs `(i,j)` with `i < j`. -/
def meshPairs (count : Nat) : List (Nat × Nat) :=
  (List.range count).flatMap fun i =>
    ((List.range count).filter (i < ·)).map (i, ·)

/-- Whether the emitted pair list covers every pair exactly once and every
    claimed kind matches the exact reclassification. -/
def pairsVerified (mesh : MeshData) (pairs : List ExaminedPair) : Bool :=
  pairs.length == (meshPairs mesh.triangles.length).length &&
  (meshPairs mesh.triangles.length).all fun (i, j) =>
    match pairs.find? (fun p => p.first == i && p.second == j),
        classifyMeshPair mesh i j with
    | some pair, some kind =>
        decide (pair.sharedIndexCount =
            (match mesh.triangles[i]?, mesh.triangles[j]? with
             | some t1, some t2 => sharedIndexCount t1 t2
             | _, _ => 0)) &&
        decide (pair.kind = kind) && pair.kind.permitted
    | _, _ => false

/-- Per-kind verification of a rejected outcome's failure claim. -/
def failureVerified (mesh : MeshData) (failureKind : String) (failure : JsonFields) :
    Bool :=
  let edges := meshDirectedEdges mesh
  match failureKind with
  | "openEdge" =>
      match jsonNatField? failure "firstVertex",
          jsonNatField? failure "secondVertex" with
      | some a, some b =>
          -- genuinely a boundary edge: exactly one directed use
          undirectedIncidence edges a b == 1
      | _, _ => false
  | "nonManifoldEdge" =>
      match jsonNatField? failure "firstVertex",
          jsonNatField? failure "secondVertex",
          jsonNatField? failure "incidenceCount" with
      | some a, some b, some count =>
          undirectedIncidence edges a b == count && count ≥ 3
      | _, _, _ => false
  | "orientationMismatch" =>
      match jsonNatField? failure "firstVertex",
          jsonNatField? failure "secondVertex" with
      | some a, some b =>
          -- same direction used twice: not consistently oriented
          directedIncidence edges a b ≥ 2 || directedIncidence edges b a ≥ 2
      | _, _ => false
  | "degenerateTriangle" =>
      match jsonNatField? failure "triangleIndex" with
      | some t =>
          match mesh.triangles[t]? with
          | some (a, b, c) =>
              a == b || b == c || a == c ||
              (match mesh.vertices[a]?, mesh.vertices[b]?, mesh.vertices[c]? with
               | some va, some vb, some vc =>
                   let n := vcross (vsub vb.toVec3 va.toVec3)
                     (vsub vc.toVec3 va.toVec3)
                   decide (vdot n n = 0)
               | _, _, _ => false)
          | none => false
      | none => false
  | "triangleIndexOutOfBounds" =>
      match jsonNatField? failure "triangleIndex",
          jsonNatField? failure "vertexIndex" with
      | some t, some v =>
          match mesh.triangles[t]? with
          | some (a, b, c) =>
              let index := [a, b, c][v]?
              match index with
              | some i => i ≥ mesh.vertices.length
              | none => false
          | none => false
      | _, _ => false
  | "nonFiniteVertex" =>
      -- Dyadic-decoded vertices are always finite: the claim is unverifiable.
      false
  | "selfIntersection" =>
      match jsonNatField? failure "first", jsonNatField? failure "second",
          jsonStringField? failure "intersectionKind" with
      | some first, some second, some kind =>
          match classifyMeshPair mesh first second with
          | some computed =>
              let claimed := match kind with
                | "nonPermittedCrossing" => some PairKind.nonPermittedCrossing
                | "coplanarOverlap" => some PairKind.coplanarOverlap
                | "distinctIndexCoincidentContact" => some PairKind.coincidentContact
                | _ => none
              decide (claimed = some computed) && !computed.permitted
          | none => false
      | _, _, _ => false
  | "attributeCount" =>
      match jsonNatField? failure "attributeCount",
          jsonNatField? failure "triangleCount" with
      | some attributes, some triangles =>
          triangles == mesh.triangles.length && attributes != 3 * triangles
      | _, _ => false
  | "emptySourceIdentity" | "emptySourceRevision" => false
  | _ => false

def decodeExaminedPair (value : Value) : Option ExaminedPair := do
  let fields ← jsonObjectWithExactFields? value
    ["first", "kind", "second", "sharedIndexCount"]
  let first ← decodeNatAtMost fields "first" 65536
  let second ← decodeNatAtMost fields "second" 65536
  let sharedIndexCount ← decodeNatAtMost fields "sharedIndexCount" 3
  let kindText ← decodeStringIn fields "kind"
    ["empty", "sharedEdgeSegment", "sharedVertex"]
  let kind := match kindText with
    | "empty" => PairKind.empty
    | "sharedVertex" => PairKind.sharedVertex
    | _ => PairKind.sharedEdgeSegment
  pure { first, second, sharedIndexCount, kind }

def decodeMeshJudgmentObservation (value : Value) :
    Option MeshJudgmentObservation := do
  let fields ← jsonObjectWithExactFields? value ["mesh", "outcome"]
  let mesh ← decodeMesh (← jsonField? fields "mesh")
  let outcomeValue ← jsonField? fields "outcome"
  let outcomeFields ← jsonFields? outcomeValue
  let kind ← jsonStringField? outcomeFields "kind"
  match kind with
  | "validated" => do
      let exact ← jsonObjectWithExactFields? outcomeValue
        ["closure", "examinedPairs", "kind", "manifold", "orientation"]
      let edgeCountOf := fun name => do
        let inner ← jsonObjectWithExactFields? (← jsonField? exact name) ["edgeCount"]
        decodeNatAtMost inner "edgeCount" 4294967295
      let closureEdges ← edgeCountOf "closure"
      let orientationEdges ← edgeCountOf "orientation"
      let manifoldEdges ← edgeCountOf "manifold"
      let pairValues ← decodeValueArray (← jsonField? exact "examinedPairs")
      if pairValues.length > 65536 then none
      let pairs ← pairValues.mapM decodeExaminedPair
      pure { mesh, outcome := .validated closureEdges orientationEdges manifoldEdges pairs }
  | "rejected" => do
      let exact ← jsonObjectWithExactFields? outcomeValue
        ["failure", "failureKind", "kind"]
      let failureKind ← decodeStringIn exact "failureKind"
        ["openEdge", "nonManifoldEdge", "orientationMismatch", "degenerateTriangle",
          "triangleIndexOutOfBounds", "selfIntersection", "attributeCount"]
      let failure ← jsonFields? (← jsonField? exact "failure")
      pure { mesh, outcome := .rejected failureKind failure }
  | _ => none

def checkMeshJudgment (observation : MeshJudgmentObservation) : Bool :=
  match observation.outcome with
  | .validated closureEdges orientationEdges manifoldEdges pairs =>
      let mesh := observation.mesh
      meshWellFormed mesh && meshNonDegenerate mesh &&
      meshClosedOriented mesh && meshManifold mesh &&
      closureEdges == (meshUndirectedEdges mesh).length &&
      orientationEdges == (meshUndirectedEdges mesh).length &&
      manifoldEdges == (meshUndirectedEdges mesh).length &&
      pairsVerified mesh pairs
  | .rejected failureKind failure =>
      failureVerified observation.mesh failureKind failure

def MeshJudgmentObservation.Valid (observation : MeshJudgmentObservation) : Prop :=
  checkMeshJudgment observation = true

instance decidableMeshJudgmentValid (observation : MeshJudgmentObservation) : Decidable observation.Valid := by
  unfold MeshJudgmentObservation.Valid checkMeshJudgment
  infer_instance

def acceptMeshJudgment (observation : MeshJudgmentObservation) : Bool :=
  decide observation.Valid

end NecoCadVerify
