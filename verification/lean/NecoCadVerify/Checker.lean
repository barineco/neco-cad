import NecoLean.Digest
import NecoCadVerify.DeclarationBindings
import NecoCadVerify.Semantics

set_option autoImplicit false

namespace NecoCadVerify.Checker

open NecoLean.Json
open NecoAlgebraVerify
open NecoCadVerify
open NecoCadVerify.Protocol

private def jsonNumberRat? (value : Value) : Option Rat := do
  let text ← match value with
    | .number number => some (Number.toString number)
    | _ => none
  let (mantissa, exponent) ← match (text.replace "E" "e").splitOn "e" with
    | [mantissa] => some (mantissa, (0 : Int))
    | [mantissa, exponent] => do
        let exponent := if exponent.startsWith "+" then
          String.ofList (exponent.toList.drop 1) else exponent
        pure (mantissa, ← exponent.toInt?)
    | _ => none
  let negative := mantissa.startsWith "-"
  let unsigned := if negative then String.ofList (mantissa.toList.drop 1) else mantissa
  let (digits, fractionalDigits) ← match unsigned.splitOn "." with
    | [whole] => some (whole, 0)
    | [whole, fraction] => some (whole ++ fraction, fraction.length)
    | _ => none
  let natural ← digits.toNat?
  let coefficient : Rat := if negative then -(natural : Rat) else (natural : Rat)
  if coefficient == 0 then pure 0
  else
    let shift := exponent - Int.ofNat fractionalDigits
    if shift.natAbs > digits.length + 1 then none
    else if shift ≥ 0 then pure (coefficient * (10 : Rat) ^ shift.toNat)
    else pure (coefficient / (10 : Rat) ^ shift.natAbs)

private def numberFieldEquals (fields : JsonFields) (field : String) (expected : Rat) : Bool :=
  ((jsonField? fields field) >>= jsonNumberRat?) == some expected

private def validCaseInput (input : Value) : Bool :=
  match jsonFields? input with
  | none => false
  | some fields =>
      match jsonField? fields "primitiveExtent" with
      | none => true
      | some value =>
          match jsonNumberRat? value with
          | some extent => [1, 1 / 2, 1 / 4, 1 / 8].contains extent
          | none => false

private def boundaryFailureInput (input : Value) : Bool :=
  match jsonFields? input with
  | none => false
  | some fields =>
      numberFieldEquals fields "primitiveExtent" 1 &&
        numberFieldEquals fields "scale" (5 / 4) &&
        jsonStringField? fields "surfaceFamily" == some "sphere-south" &&
        match jsonField? fields "translation" with
        | some (.array [x, y, z]) =>
            jsonNumberRat? x == some (-1 / 2) && jsonNumberRat? y == some 0 &&
              jsonNumberRat? z == some 0
        | _ => false

private def decodeTypedFailure (caseId : String) (input value : Value) : Option CadObservation := do
  let fields ← jsonObjectWithExactFields? value ["failureKind", "provenance", "reason"]
  let failureKind ← NecoCalculusVerify.decodeStringIn fields "failureKind"
    ["engineRejection", "boundaryRejection"]
  let reason ← jsonStringField? fields "reason"
  if reason.isEmpty then none
  let provenance ← jsonObjectWithExactFields? (← jsonField? fields "provenance")
    ["caseId", "failingOperation", "input", "violatedCondition"]
  if jsonStringField? provenance "caseId" != some caseId ||
      jsonField? provenance "input" != some input then none
  let operation ← jsonStringField? provenance "failingOperation"
  let condition ← jsonStringField? provenance "violatedCondition"
  let inputFields ← jsonFields? input
  let valid := match failureKind with
    | "engineRejection" =>
        operation == "construct" && condition == "transform.scale must be positive" &&
          numberFieldEquals inputFields "scale" 0
    | "boundaryRejection" =>
        operation == "derive_complete_boundary" &&
          condition == "no exact boundary seed in integer lattice [-3,3]^3" &&
          boundaryFailureInput input
    | _ => false
  if valid then pure (.typedFailure failureKind reason) else none

def decodeCadObservation (caseId family kind : String) (input value : Value) :
    Option CadObservation :=
  match family, kind with
  | "exact-chart", "exactChart" =>
      (decodeExactChartObservation value).map .exactChart
  | "face-atom-covering", "faceAtomCovering" =>
      (decodeFaceAtomCoveringObservation value).map .faceAtomCovering
  | "finite-surface-representation", "finiteSurfaceRepresentation" =>
      (decodeFiniteSurfaceObservation value).map .finiteSurfaceRepresentation
  | "certified-chord-bound", "certifiedChordBound" =>
      (decodeChordBoundObservation value).map .certifiedChordBound
  | "mesh-judgment", "meshJudgment" =>
      (decodeMeshJudgmentObservation value).map .meshJudgment
  | "exact-chart", "typedFailure"
  | "face-atom-covering", "typedFailure"
  | "finite-surface-representation", "typedFailure"
  | "certified-chord-bound", "typedFailure"
  | "mesh-judgment", "typedFailure" => decodeTypedFailure caseId input value
  | _, _ => none

structure DecodedCase where
  caseId : String
  observation : CadObservation
  deriving Repr

def decodeCase (value : Value) : Option DecodedCase := do
  let fields ← jsonObjectWithExactFields? value
    ["caseId", "input", "operationFamily", "publicObservation"]
  let caseId ← jsonStringField? fields "caseId"
  if caseId.isEmpty || caseId == "__digest__" then none
  let input ← jsonField? fields "input"
  if !validCaseInput input then none
  let family ← jsonStringField? fields "operationFamily"
  let (kind, payload) ← decodeObservationEnvelope
    (← jsonField? fields "publicObservation")
  let observation ← decodeCadObservation caseId family kind input payload
  pure { caseId, observation }

private def encodedCaseId (value : Value) : String :=
  match jsonObjectWithExactFields? value
      ["caseId", "input", "operationFamily", "publicObservation"] with
  | some fields => (jsonStringField? fields "caseId").getD ""
  | none => ""

def checkCase (value : Value) : CaseResult :=
  match decodeCase value with
  | none => { caseId := encodedCaseId value, verdict := .invalid }
  | some decoded =>
      { caseId := decoded.caseId, verdict := Semantics.verdictOf decoded.observation }

private def uniqueCaseIds : List CaseResult → Bool
  | [] => true
  | item :: remaining =>
      !item.caseId.isEmpty && item.caseId != "__digest__" &&
        !(remaining.any fun candidate => candidate.caseId == item.caseId) &&
        uniqueCaseIds remaining

private def checkedBatch (digest : String) (cases : List CaseResult) : CheckResult :=
  if !cases.isEmpty && uniqueCaseIds cases then .cases digest cases
  else .batchInvalid digest

def checkBytes (bytes : ByteArray) : CheckResult :=
  let digest := NecoLean.Digest.toHex (NecoLean.Digest.sha256 bytes)
  match NecoLean.Json.decode bytes with
  | .ok value =>
      let canonical := match NecoLean.Json.encode value with
        | .ok encoded => encoded == bytes
        | .error _ => false
      match jsonObjectWithExactFields? value ["cases", "proofSubjectRef", "schema", "testSetRef"] with
      | some fields =>
          if !canonical || jsonStringField? fields "schema" != some "neco.cad.observation-batch.v1" ||
              !DeclarationBindings.rootRefsMatch fields then .batchInvalid digest
          else match jsonField? fields "cases" with
            | some (.array cases) => checkedBatch digest (cases.map checkCase)
            | _ => .batchInvalid digest
      | none => .batchInvalid digest
  | .error _ => .batchInvalid digest

private def verdictText : Verdict → String
  | .verified => "verified"
  | .refuted => "refuted"
  | .undetermined => "undetermined"
  | .invalid => "invalid"

def resultJson : CheckResult → Value
  | .batchInvalid digest =>
      .object [("batchVerdict", .string "invalid"), ("digest", .string digest)]
  | .cases digest items =>
      .object [
        ("cases", .array (items.map fun item =>
          .object [("caseId", .string item.caseId), ("verdict", .string (verdictText item.verdict))])),
        ("digest", .string digest)
      ]

def resultBytes (result : CheckResult) : Except NecoLean.Json.Failure ByteArray :=
  NecoLean.Json.encode (resultJson result)

end NecoCadVerify.Checker
