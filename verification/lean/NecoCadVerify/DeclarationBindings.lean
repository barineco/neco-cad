import Lean
import NecoLean.Digest
import NecoCadVerify.Protocol

set_option autoImplicit false

namespace NecoCadVerify.DeclarationBindings

open NecoLean.Json
open NecoAlgebraVerify
open NecoCadVerify.Protocol

elab "cadDeclaration%" file:str : term => do
  let name := file.getString
  unless ["cad-proof-subject.json", "cad-finite-test-set.json", "cad-semantics.json",
      "cad-transition-class-declaration.json", "cad-generated-request-declaration.json"].contains name do
    throwError "unknown CAD declaration"
  let modulePath := System.FilePath.mk (← Lean.getFileName)
  let directory ← match modulePath.parent with
    | some directory => pure directory
    | none => throwError "CAD declaration module has no source directory"
  let bytes ← IO.FS.readBinFile (directory / ".." / ".." / "manifest" / name)
  let text ← match String.fromUTF8? bytes with
    | some text => pure text
    | none => throwError "CAD declaration is not UTF-8"
  let value ← match NecoLean.Json.decode bytes with
    | .ok value => pure value
    | .error _ => throwError "CAD declaration is not JSON"
  match NecoLean.Json.encode value with
  | .ok canonical =>
      unless canonical == bytes do throwError "CAD declaration bytes are not canonical"
  | .error _ => throwError "CAD declaration cannot be encoded"
  pure (Lean.mkStrLit text)

def proofSubjectBytes : ByteArray := (cadDeclaration% "cad-proof-subject.json").toUTF8

def testSetBytes : ByteArray := (cadDeclaration% "cad-finite-test-set.json").toUTF8

def semanticsBytes : ByteArray := (cadDeclaration% "cad-semantics.json").toUTF8

def transitionClassBytes : ByteArray := (cadDeclaration% "cad-transition-class-declaration.json").toUTF8

def generatedRequestBytes : ByteArray := (cadDeclaration% "cad-generated-request-declaration.json").toUTF8

private def declarationFields (bytes : ByteArray) : Option JsonFields := do
  let value ← (NecoLean.Json.decode bytes).toOption
  jsonFields? value

private def declarationRef (bytes : ByteArray) (idField : String) : Option (String × String) := do
  let fields ← declarationFields bytes
  let identity ← jsonStringField? fields idField
  if identity.isEmpty then none
  pure (NecoLean.Digest.toHex (NecoLean.Digest.sha256 bytes), identity)

def proofSubjectRef : Option (String × String) :=
  declarationRef proofSubjectBytes "proofSubjectId"

def testSetRef : Option (String × String) :=
  declarationRef testSetBytes "testSetId"

private def declaredDependenciesValid : Bool :=
  match declarationFields proofSubjectBytes, declarationFields testSetBytes,
      declarationRef semanticsBytes "revision", proofSubjectRef,
      declarationRef transitionClassBytes "transitionClassId",
      declarationRef generatedRequestBytes "adapterRevision" with
  | some proof, some tests, some semantics, some subject, some transition, some generated =>
      ((jsonField? proof "semanticsRef") >>= decodeVersionedRef) == some semantics &&
        ((jsonField? tests "proofSubjectRef") >>= decodeVersionedRef) == some subject &&
        ((jsonField? tests "transitionClassRef") >>= decodeVersionedRef) == some transition &&
        ((jsonField? tests "generatedRequestRef") >>= decodeVersionedRef) == some generated &&
        jsonField? proof "obligations" == some (.array [
          .string "exactChartRepresentsImplicitAtom",
          .string "finiteRepresentationRepresentsExactSurface",
          .string "faceAtomCoveringCorrespondence",
          .string "certifiedChordBoundCoversEdgeDeviation",
          .string "meshClosureOrientationManifoldnessAndSelfIntersection"])
  | _, _, _, _, _, _ => false

def rootRefsMatch (fields : JsonFields) : Bool :=
  match proofSubjectRef, testSetRef with
  | some proof, some tests =>
      declaredDependenciesValid &&
        ((jsonField? fields "proofSubjectRef") >>= decodeVersionedRef) == some proof &&
        ((jsonField? fields "testSetRef") >>= decodeVersionedRef) == some tests
  | _, _ => false

end NecoCadVerify.DeclarationBindings
