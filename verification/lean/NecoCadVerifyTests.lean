import NecoCadVerify

open NecoLean.Json

#print axioms NecoCadVerify.acceptExactChart_sound
#print axioms NecoCadVerify.acceptFaceAtomCovering_sound
#print axioms NecoCadVerify.acceptFiniteSurface_sound
#print axioms NecoCadVerify.acceptChordBound_sound
#print axioms NecoCadVerify.acceptMeshJudgment_sound
#print axioms NecoCadVerify.verdictOf_sound
#print axioms NecoCadVerify.checkPatchAgainstAtom_checkedRange_sound
#print axioms NecoCadVerify.integerEdgePolynomialRange_contains
#print axioms NecoCadVerify.arcDenominator_nonzero

private def sphereAtom : List NecoGeometryVerify.RegionTerm :=
  NecoGeometryVerify.RegionPolynomial.sub
    (NecoGeometryVerify.RegionPolynomial.add
      (NecoGeometryVerify.RegionPolynomial.pow (NecoGeometryVerify.RegionPolynomial.variablePolynomial 0) 2)
      (NecoGeometryVerify.RegionPolynomial.add
        (NecoGeometryVerify.RegionPolynomial.pow (NecoGeometryVerify.RegionPolynomial.variablePolynomial 1) 2)
        (NecoGeometryVerify.RegionPolynomial.pow (NecoGeometryVerify.RegionPolynomial.variablePolynomial 2) 2)))
    (NecoGeometryVerify.RegionPolynomial.constant 1)

private def spherePatch (index : Nat) (sign : Rat) : NecoCadVerify.CoveringPatch :=
  let u : List NecoCalculusVerify.SurfaceTerm := NecoCalculusVerify.SurfacePolynomial.variableU
  let v : List NecoCalculusVerify.SurfaceTerm := NecoCalculusVerify.SurfacePolynomial.variableV
  let radial := NecoCalculusVerify.SurfacePolynomial.add
    (NecoCalculusVerify.SurfacePolynomial.pow u 2) (NecoCalculusVerify.SurfacePolynomial.pow v 2)
  let g := NecoGeometryVerify.RegionPolynomial.rawScale sign
    (NecoGeometryVerify.RegionPolynomial.variablePolynomial 2)
  let d := NecoGeometryVerify.RegionPolynomial.add (NecoGeometryVerify.RegionPolynomial.constant 1) g
  {
    patchIndex := index
    chart := {
      x := NecoCalculusVerify.SurfacePolynomial.scale 2 u
      y := NecoCalculusVerify.SurfacePolynomial.scale 2 v
      z := NecoCalculusVerify.SurfacePolynomial.scale sign
        (NecoCalculusVerify.SurfacePolynomial.sub (NecoCalculusVerify.SurfacePolynomial.constant 1) radial)
      denominator := NecoCalculusVerify.SurfacePolynomial.add
        (NecoCalculusVerify.SurfacePolynomial.constant 1) radial
    }
    domain := { u := { lower := -1, upper := 1 }, v := { lower := -1, upper := 1 } }
    disk := true
    denominatorBound := { constant := 1, domainPowers := [], squares := [(1, u), (1, v)] }
    inverse := {
      uNumerator := NecoGeometryVerify.RegionPolynomial.variablePolynomial 0
      uDenominator := d
      vNumerator := NecoGeometryVerify.RegionPolynomial.variablePolynomial 1
      vDenominator := d
    }
  }

private def sphereClause (index : Nat) (sign : Rat) : NecoCadVerify.CoveringClause :=
  let g := NecoGeometryVerify.RegionPolynomial.rawScale sign
    (NecoGeometryVerify.RegionPolynomial.variablePolynomial 2)
  let d := NecoGeometryVerify.RegionPolynomial.add (NecoGeometryVerify.RegionPolynomial.constant 1) g
  let inverseBound : NecoCadVerify.BoundForm :=
    { constant := 1, guardPowers := [([1], 1)], squares := [], atomQuotient := [] }
  {
    patch := index
    guards := [g]
    inverseDenominatorBounds := (inverseBound, inverseBound)
    chartDenominatorBound := {
      constant := 2
      guardPowers := [([1], 6), ([2], 6), ([3], 2)]
      squares := []
      atomQuotient := NecoGeometryVerify.RegionPolynomial.pow d 2
    }
    diskRange := {
      constant := 0
      guardPowers := [([1], 2), ([2], 6), ([3], 6), ([4], 2)]
      squares := []
      atomQuotient := NecoGeometryVerify.RegionPolynomial.neg (NecoGeometryVerify.RegionPolynomial.pow d 2)
    }
    imageQuotients :=
      (NecoGeometryVerify.RegionPolynomial.neg
         (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.variablePolynomial 0)
           (NecoGeometryVerify.RegionPolynomial.pow d 3)),
       NecoGeometryVerify.RegionPolynomial.neg
         (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.variablePolynomial 1)
           (NecoGeometryVerify.RegionPolynomial.pow d 3)),
       NecoGeometryVerify.RegionPolynomial.rawScale (-sign) (NecoGeometryVerify.RegionPolynomial.pow d 7))
  }

private def planePatch : NecoCadVerify.PatchData :=
  {
    patchIndex := 0
    chart := {
      x := NecoCalculusVerify.SurfacePolynomial.variableU
      y := NecoCalculusVerify.SurfacePolynomial.variableV
      z := []
      denominator := NecoCalculusVerify.SurfacePolynomial.constant 1
    }
    domain := { u := { lower := 0, upper := 1 }, v := { lower := 0, upper := 1 } }
    disk := false
    denominatorBound := { constant := 1, domainPowers := [], squares := [] }
  }

private def finitePlane : NecoCadVerify.FiniteSurfaceObservation :=
  {
    identity := "plane-mesh"
    sourceRevision := "independent-plane"
    mesh := { vertices := [⟨0, 0, 0⟩, ⟨1, 0, 0⟩, ⟨0, 1, 0⟩], triangles := [(0, 1, 2)] }
    faceTriangles := [{
      faceId := "plane"
      atomId := "z"
      atomRelation := "zero"
      atom := NecoGeometryVerify.RegionPolynomial.variablePolynomial 2
      triangleIndices := [0]
      patches := [planePatch]
      vertexParameters := [⟨0, 0, 0, (0, 0, 0)⟩, ⟨0, 1, 0, (1, 0, 0)⟩, ⟨0, 0, 1, (0, 1, 0)⟩]
      residualBound := 0
    }]
    requestedChordError := 0
    establishedChordError := 0
    requestedSurfaceError := 0
    establishedSurfaceError := 0
  }

private def straightChord : NecoCadVerify.ChordBoundObservation :=
  {
    faceId := "plane"
    atomId := "z"
    atomRelation := "zero"
    patches := [planePatch]
    edges := [{
      patchIndex := 0
      left := ⟨0, 0, (0, 0, 0)⟩
      right := ⟨1, 0, (1, 0, 0)⟩
      midpointU := 1 / 2
      midpointV := 0
      parameterization := .line 0 1
      establishedSquaredBound := 0
      subdivisionDepth := 0
    }]
    triangleEdgeIndices := []
    establishedSquaredBound := 0
    requestedChordError := 0
  }

private def tetrahedron : NecoCadVerify.MeshData :=
  { vertices := [⟨0, 0, 0⟩, ⟨1, 0, 0⟩, ⟨0, 1, 0⟩, ⟨0, 0, 1⟩]
    triangles := [(0, 2, 1), (0, 1, 3), (1, 2, 3), (2, 0, 3)] }

private def tetrahedronJudgment : NecoCadVerify.MeshJudgmentObservation :=
  { mesh := tetrahedron
    outcome := .validated 6 6 6
      ([(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)].map fun (first, second) =>
        { first, second, sharedIndexCount := 2, kind := .sharedEdgeSegment }) }

private def number (text : String) : IO Value :=
  match Number.create text with
  | .ok value => pure (.number value)
  | .error _ => throw (IO.userError "invalid independent test number")

private def expectSemantic (name : String) (expected : NecoCadVerify.Protocol.Verdict)
    (observation : NecoCadVerify.CadObservation) : IO Unit :=
  if NecoCadVerify.Semantics.verdictOf observation == expected then pure ()
  else throw (IO.userError s!"{name}: semantic verdict mismatch")

private def testKnown : IO Unit := do
  let north := spherePatch 0 1
  expectSemantic "sphere chart" .verified (.exactChart
    { faceId := "sphere", atomId := "unit-sphere", atomRelation := "zero", patchIndex := 0,
      atom := sphereAtom, patch := north.toPatchData })
  expectSemantic "sphere covering" .verified (.faceAtomCovering
    { faceId := "sphere", atomId := "unit-sphere", atomRelation := "zero", atom := sphereAtom,
      outerGuards := [], patches := [north, spherePatch 1 (-1)],
      clauses := [sphereClause 0 1, sphereClause 1 (-1)],
      coverageSelections := [{ constant := 0, guardPowers := [], squares := [], atomQuotient := [] }] })
  expectSemantic "finite plane" .verified (.finiteSurfaceRepresentation finitePlane)
  expectSemantic "straight chord" .verified (.certifiedChordBound straightChord)
  expectSemantic "closed tetrahedron" .verified (.meshJudgment tetrahedronJudgment)
  let openMesh := { tetrahedron with triangles := tetrahedron.triangles.take 3 }
  let openFailure := [("firstVertex", ← number "0"), ("secondVertex", ← number "2")]
  expectSemantic "open tetrahedron failure" .verified
    (.meshJudgment { mesh := openMesh, outcome := .rejected "openEdge" openFailure })
  expectSemantic "false closure failure" .refuted
    (.meshJudgment { mesh := tetrahedron, outcome := .rejected "openEdge" openFailure })
  expectSemantic "wrong implicit atom" .refuted (.exactChart
    { faceId := "plane", atomId := "offset", atomRelation := "zero", patchIndex := 0,
      atom := NecoGeometryVerify.RegionPolynomial.constant 1, patch := planePatch })
  expectSemantic "typed failure" .undetermined (.typedFailure "engineRejection" "scale is zero")
  IO.println "five semantic families and independent mathematical controls passed"

private def refJson (reference : String × String) : Value :=
  .object [("contentDigest", .string reference.1), ("versionIdentity", .string reference.2)]

private def encode (value : Value) : IO ByteArray :=
  match NecoLean.Json.encode value with
  | .ok bytes => pure bytes
  | .error _ => throw (IO.userError "independent test encoding failed")

private def batch (cases : List Value) (wrongProof : Bool := false)
    (wrongDigest : Bool := false) : IO ByteArray := do
  let proof ← match NecoCadVerify.DeclarationBindings.proofSubjectRef with
    | some proof => pure proof
    | none => throw (IO.userError "compiled proof declaration missing")
  let tests ← match NecoCadVerify.DeclarationBindings.testSetRef with
    | some tests => pure tests
    | none => throw (IO.userError "compiled test declaration missing")
  let proof := if wrongProof then (proof.1, proof.2 ++ "-wrong") else proof
  let proof := if wrongDigest then
      ((if proof.1.startsWith "0" then "1" else "0") ++ String.ofList (proof.1.toList.drop 1), proof.2)
    else proof
  encode (.object [("cases", .array cases), ("proofSubjectRef", refJson proof),
    ("schema", .string "neco.cad.observation-batch.v1"), ("testSetRef", refJson tests)])

private def failureCase (caseId : String) (extent scale : Value)
    (provenanceId : String := "independent-failure") : Value :=
  let input := Value.object [("primitiveExtent", extent), ("scale", scale)]
  .object [("caseId", .string caseId), ("input", input), ("operationFamily", .string "exact-chart"),
    ("publicObservation", .object [("kind", .string "typedFailure"), ("value", .object [
      ("failureKind", .string "engineRejection"), ("reason", .string "scale is zero"),
      ("provenance", .object [("caseId", .string provenanceId),
        ("failingOperation", .string "construct"), ("input", input),
        ("violatedCondition", .string "transform.scale must be positive")])])])]

private def replaceField (value : Value) (name : String) (replacement : Value) : IO Value :=
  match value with
  | .object fields =>
      pure (.object ((fields.filter fun field => field.1 != name) ++ [(name, replacement)]))
  | _ => throw (IO.userError "independent control requires an object")

private def boundaryFailureCase : IO Value := do
  let input := Value.object [("primitiveExtent", ← number "1.0"), ("scale", ← number "1.25"),
    ("surfaceFamily", .string "sphere-south"),
    ("translation", .array [← number "-0.5", ← number "0.0", ← number "0.0"])]
  pure (.object [("caseId", .string "independent-boundary"), ("input", input),
    ("operationFamily", .string "exact-chart"),
    ("publicObservation", .object [("kind", .string "typedFailure"), ("value", .object [
      ("failureKind", .string "boundaryRejection"), ("reason", .string "no boundary seed"),
      ("provenance", .object [("caseId", .string "independent-boundary"),
        ("failingOperation", .string "derive_complete_boundary"), ("input", input),
        ("violatedCondition", .string "no exact boundary seed in integer lattice [-3,3]^3")])])])])

private def rationalJson (numerator : String) : Value :=
  .object [("denominator", .string "1"), ("numerator", .string numerator)]

private def planeCase : IO Value := do
  let zero ← number "0"
  let one ← number "1"
  let surfaceTerm (exponentU exponentV : Value) := Value.object [
    ("coefficient", rationalJson "1"), ("exponentU", exponentU), ("exponentV", exponentV)]
  let interval := Value.object [("lower", rationalJson "0"), ("lowerClosed", .bool true),
    ("upper", rationalJson "1"), ("upperClosed", .bool true)]
  pure (.object [("caseId", .string "independent-plane"), ("input", .object []),
    ("operationFamily", .string "exact-chart"), ("publicObservation", .object [
      ("kind", .string "exactChart"), ("value", .object [
        ("atom", .array [.object [("coefficient", rationalJson "1"),
          ("exponentX", zero), ("exponentY", zero), ("exponentZ", one)]]),
        ("atomId", .string "z"), ("atomRelation", .string "zero"),
        ("chart", .object [("denominator", .array [surfaceTerm zero zero]),
          ("x", .array [surfaceTerm one zero]), ("y", .array [surfaceTerm zero one]), ("z", .array [])]),
        ("chartDenominatorBound", .object [("constant", rationalJson "1"),
          ("domainPowers", .array []), ("squares", .array [])]),
        ("disk", .bool false), ("domain", .object [("u", interval), ("v", interval)]),
        ("faceId", .string "plane"), ("patchIndex", zero)])])])

private def meshFailureCase (failureKind : String) (secondVertex : String := "1") : IO Value := do
  let zero ← number "0"
  let one ← number "1"
  let two ← number "2"
  let dyadic (integer : String) := Value.object [("exponent", zero), ("integer", .string integer)]
  let vertex (x y z : String) := Value.object [("x", dyadic x), ("y", dyadic y), ("z", dyadic z)]
  pure (.object [("caseId", .string "independent-mesh-failure"), ("input", .object []),
    ("operationFamily", .string "mesh-judgment"), ("publicObservation", .object [
      ("kind", .string "meshJudgment"), ("value", .object [
        ("mesh", .object [("vertices", .array [vertex "0" "0" "0", vertex "1" "0" "0", vertex "0" "1" "0"]),
          ("triangles", .array [.array [zero, one, two]])]),
        ("outcome", .object [("kind", .string "rejected"), ("failureKind", .string failureKind),
          ("failure", .object [("firstVertex", zero), ("secondVertex", ← number secondVertex)])])])])])

private def expectVerdict (name : String) (expected : NecoCadVerify.Protocol.Verdict) :
    NecoCadVerify.Protocol.CheckResult → IO Unit
  | .cases _ [item] =>
      if item.verdict == expected then pure ()
      else throw (IO.userError s!"{name}: unexpected verdict")
  | _ => throw (IO.userError s!"{name}: expected one case verdict")

private def expectBatchInvalid (name : String) : NecoCadVerify.Protocol.CheckResult → IO Unit
  | .batchInvalid _ => pure ()
  | .cases _ _ => throw (IO.userError s!"{name}: expected batch-invalid result")

private def testClassification : IO Unit := do
  let extent ← number "1.0"
  let zero ← number "0.0"
  let valid := failureCase "independent-failure" extent zero
  let bytes ← batch [valid]
  expectVerdict "protocol v2 typed failure" .undetermined (NecoCadVerify.Checker.checkBytes bytes)
  expectVerdict "independent plane wire" .verified (NecoCadVerify.Checker.checkBytes (← batch [← planeCase]))
  expectVerdict "protocol v2 boundary rejection" .undetermined
    (NecoCadVerify.Checker.checkBytes (← batch [← boundaryFailureCase]))
  expectVerdict "checkable open edge" .verified (NecoCadVerify.Checker.checkBytes (← batch [← meshFailureCase "openEdge"]))
  expectVerdict "false open edge claim" .refuted
    (NecoCadVerify.Checker.checkBytes (← batch [← meshFailureCase "openEdge" "3"]))
  expectVerdict "unsupported source identity claim" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [← meshFailureCase "emptySourceIdentity"]))
  expectVerdict "unsupported source revision claim" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [← meshFailureCase "emptySourceRevision"]))
  expectVerdict "unsupported nonfinite claim" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [← meshFailureCase "nonFiniteVertex"]))
  expectVerdict "unknown mesh failure kind" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [← meshFailureCase "unknown"] ))
  expectVerdict "unknown operation family" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [← replaceField valid "operationFamily" (.string "unknown")]))
  expectVerdict "unknown observation kind" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [← replaceField valid "publicObservation"
      (.object [("kind", .string "unknown"), ("value", .object [])])]))
  expectBatchInvalid "unknown case field"
    (NecoCadVerify.Checker.checkBytes (← batch [← replaceField valid "unexpected" .null]))
  expectVerdict "fractional primitive extent" .undetermined
    (NecoCadVerify.Checker.checkBytes (← batch [failureCase "independent-failure" (← number "0.125") zero]))
  expectVerdict "scientific JSON number" .undetermined
    (NecoCadVerify.Checker.checkBytes (← batch [failureCase "independent-failure" (← number "1e0") zero]))
  expectVerdict "invalid primitive extent" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [failureCase "independent-failure" (← number "0.75") zero]))
  expectVerdict "string primitive extent" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [failureCase "independent-failure" (.string "1.0") zero]))
  expectVerdict "false failure condition" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [failureCase "independent-failure" extent (← number "1.0")]))
  expectVerdict "wrong failure provenance" .invalid
    (NecoCadVerify.Checker.checkBytes (← batch [failureCase "independent-failure" extent zero "wrong-case"]))
  expectBatchInvalid "duplicate caseId" (NecoCadVerify.Checker.checkBytes (← batch [valid, valid]))
  expectBatchInvalid "sentinel caseId"
    (NecoCadVerify.Checker.checkBytes (← batch [failureCase "__digest__" extent zero "__digest__"]))
  expectBatchInvalid "empty batch" (NecoCadVerify.Checker.checkBytes (← batch []))
  expectBatchInvalid "wrong declaration reference" (NecoCadVerify.Checker.checkBytes (← batch [valid] true))
  expectBatchInvalid "wrong source declaration digest" (NecoCadVerify.Checker.checkBytes (← batch [valid] false true))
  expectBatchInvalid "noncanonical batch" (NecoCadVerify.Checker.checkBytes (" ".toUTF8 ++ bytes))
  expectBatchInvalid "batch without declaration references"
    (NecoCadVerify.Checker.checkBytes "{\"cases\":[],\"schema\":\"neco.cad.observation-batch.v1\"}".toUTF8)
  let canonicalText ← match String.fromUTF8? bytes with
    | some text => pure text
    | none => throw (IO.userError "independent canonical control is not UTF-8")
  let duplicateKey := canonicalText.replace "{\"cases\":"
    "{\"schema\":\"neco.cad.observation-batch.v1\",\"cases\":"
  expectBatchInvalid "duplicate object key" (NecoCadVerify.Checker.checkBytes duplicateKey.toUTF8)
  IO.println "independent protocol controls passed"

private def testSoundness : IO Unit := do
  testKnown
  IO.println "kernel soundness declarations compiled for the exercised semantic families"

private def checkPath (path : String) : IO Unit := do
  let bytes ← if path == "-" then
    (← IO.getStdin).readBinToEnd
  else IO.FS.readBinFile path
  let output ← match NecoCadVerify.Checker.resultBytes (NecoCadVerify.Checker.checkBytes bytes) with
    | .ok output => pure output
    | .error _ => throw (IO.userError "checker result serialization failed")
  (← IO.getStdout).write output

def main (arguments : List String) : IO Unit :=
  let command := if arguments.head? == some "--" then arguments.drop 1 else arguments
  match command with
  | ["known"] => testKnown
  | ["classification"] => testClassification
  | ["soundness"] => testSoundness
  | ["all"] => do testSoundness; testClassification
  | ["check", path] => checkPath path
  | _ => throw (IO.userError
      "usage: neco-cad-checker <known|classification|soundness|all|check PATH>")
