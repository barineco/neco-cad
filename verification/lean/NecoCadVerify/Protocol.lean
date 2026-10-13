import NecoCadVerify.Decode
import NecoCadVerify.Observation

set_option autoImplicit false

namespace NecoCadVerify.Protocol

open NecoLean.Json
open NecoAlgebraVerify

inductive Verdict where
  | verified | refuted | undetermined | invalid
  deriving Repr, BEq, DecidableEq

structure CaseResult where
  caseId : String
  verdict : Verdict
  deriving Repr, BEq

inductive CheckResult where
  | batchInvalid (digest : String)
  | cases (digest : String) (items : List CaseResult)
  deriving Repr, BEq

/-- The `publicObservation` envelope: `{"kind":..., "value":...}`. -/
def decodeObservationEnvelope (value : Value) : Option (String × Value) := do
  let fields ← jsonObjectWithExactFields? value ["kind", "value"]
  let kind ← jsonStringField? fields "kind"
  pure (kind, ← jsonField? fields "value")

/-- A declaration reference has a lowercase SHA-256 digest and a nonempty identity. -/
def decodeVersionedRef (value : Value) : Option (String × String) := do
  let fields ← jsonObjectWithExactFields? value ["contentDigest", "versionIdentity"]
  let contentDigest ← jsonStringField? fields "contentDigest"
  let versionIdentity ← jsonStringField? fields "versionIdentity"
  if contentDigest.length != 64 || versionIdentity.isEmpty ||
      !(contentDigest.toList.all fun c => ('0' ≤ c && c ≤ '9') || ('a' ≤ c && c ≤ 'f')) then none
  pure (contentDigest, versionIdentity)

end NecoCadVerify.Protocol
