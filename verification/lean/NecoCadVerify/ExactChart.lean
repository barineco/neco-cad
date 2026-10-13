import NecoCadVerify.Decode

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

/-- Compose the implicit atom `atom(x,y,z)` with the rational chart
    `χ = (x,y,z)/d`: returns the homogenized numerator
    `Σ c·X^a·Y^b·Z^c·D^(T−a−b−c)` as a bivariate polynomial, where
    `T` is the maximum total degree of the atom. The composed numerator
    evaluates to `atom(χ(u,v))·d(u,v)^T`. -/
def composeAtomNumerator (atom : (List NecoGeometryVerify.RegionTerm)) (chart : ChartPolynomials) :
    List SurfaceTerm :=
  let total := atom.foldl
    (fun degree term =>
      max degree (term.exponentX + term.exponentY + term.exponentZ)) 0
  NecoCalculusVerify.SurfacePolynomial.normalize (atom.foldl (fun acc term =>
    NecoCalculusVerify.SurfacePolynomial.add acc (NecoCalculusVerify.SurfacePolynomial.scale term.coefficient
      (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.x term.exponentX)
        (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.y term.exponentY)
          (NecoCalculusVerify.SurfacePolynomial.mul (NecoCalculusVerify.SurfacePolynomial.pow chart.z term.exponentZ)
            (NecoCalculusVerify.SurfacePolynomial.pow chart.denominator
              (total - term.exponentX - term.exponentY - term.exponentZ)))))))
    [])

def decodeExactChartObservation (value : Value) : Option ExactChartObservation := do
  let fields ← jsonObjectWithExactFields? value
    ["atom", "atomId", "atomRelation", "chart", "chartDenominatorBound",
      "disk", "domain", "faceId", "patchIndex"]
  let faceId ← jsonStringField? fields "faceId"
  let atomId ← jsonStringField? fields "atomId"
  let atomRelation ← decodeStringIn fields "atomRelation"
    ["negative", "positive", "zero"]
  let patchIndex ← decodeNatAtMost fields "patchIndex" 64
  let atom ← decodeAtomPoly (← jsonField? fields "atom")
  let chart ← decodeChart (← jsonField? fields "chart")
  let domain ← decodePatchDomain (← jsonField? fields "domain")
  let disk ← jsonBoolField? fields "disk"
  let denominatorBound ← decodeSurfaceBoundForm
    (← jsonField? fields "chartDenominatorBound")
  pure { faceId, atomId, atomRelation, patchIndex, atom, patch := { patchIndex, chart, domain, disk, denominatorBound } }

/-- Certified denominator bound: ordered domain and a nonnegative bound form
    with a strictly positive constant, certifying `denominator ≥ c > 0` on
    the domain. -/
def checkPatchBound (patch : PatchData) : Bool :=
  decide (patch.domain.u.lower ≤ patch.domain.u.upper) &&
  decide (patch.domain.v.lower ≤ patch.domain.v.upper) &&
  surfaceBoundHolds patch.chart.denominator patch.denominatorBound
    (domainBoundPolys patch.domain) &&
  surfaceBoundNonnegative patch.denominatorBound &&
  decide (patch.denominatorBound.constant > 0)

/-- Structural checks for one patch: ordered domain, certified positive
    denominator bound, and `atom ∘ χ ≡ 0`. -/
def checkPatchAgainstAtom (atom : (List NecoGeometryVerify.RegionTerm)) (patch : PatchData) : Bool :=
  checkPatchBound patch && (composeAtomNumerator atom patch.chart).isEmpty

def checkExactChart (observation : ExactChartObservation) : Bool :=
  checkPatchAgainstAtom observation.atom observation.patch

def ExactChartObservation.Valid (observation : ExactChartObservation) : Prop :=
  checkExactChart observation = true

instance decidableExactChartValid (observation : ExactChartObservation) : Decidable observation.Valid := by
  unfold ExactChartObservation.Valid checkExactChart
  infer_instance

def acceptExactChart (observation : ExactChartObservation) : Bool :=
  decide observation.Valid

end NecoCadVerify
