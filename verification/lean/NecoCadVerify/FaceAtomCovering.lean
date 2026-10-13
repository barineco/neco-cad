import NecoCadVerify.ExactChart

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

/-- Compose a bivariate surface polynomial `p(u,v)` with rational arguments
    `u = un/ud`, `v = vn/vd` (trivariate polynomials in `x,y,z`), returning the
    homogenized pair `(N, D)` with
    `N(p⃗)/D(p⃗) = p(un(p⃗)/ud(p⃗), vn(p⃗)/vd(p⃗))`
    whenever `ud(p⃗)·vd(p⃗) ≠ 0`. -/
def composeAtomArgs (terms : List SurfaceTerm) (un ud vn vd : (List NecoGeometryVerify.RegionTerm)) :
    (List NecoGeometryVerify.RegionTerm) × (List NecoGeometryVerify.RegionTerm) :=
  let degreeU := terms.foldl (fun d t => max d t.exponentU) 0
  let degreeV := terms.foldl (fun d t => max d t.exponentV) 0
  let numerator := NecoGeometryVerify.RegionPolynomial.normalize (terms.foldl (fun acc term =>
    NecoGeometryVerify.RegionPolynomial.add acc (NecoGeometryVerify.RegionPolynomial.rawScale term.coefficient
      (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow un term.exponentU)
        (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow vn term.exponentV)
          (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow ud (degreeU - term.exponentU))
            (NecoGeometryVerify.RegionPolynomial.pow vd (degreeV - term.exponentV))))))) [])
  (numerator, NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow ud degreeU) (NecoGeometryVerify.RegionPolynomial.pow vd degreeV))

/-- `ud²·vd² − un²·vd² − vn²·ud²`: the disk-condition numerator.
    `(u,v) = (un/ud, vn/vd)` satisfies `u²+v² ≤ 1` iff this is `≥ 0` (when
    `ud·vd ≠ 0`). -/
def diskConditionNumerator (inverse : InverseChart) : (List NecoGeometryVerify.RegionTerm) :=
  NecoGeometryVerify.RegionPolynomial.sub
    (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow inverse.uDenominator 2) (NecoGeometryVerify.RegionPolynomial.pow inverse.vDenominator 2))
    (NecoGeometryVerify.RegionPolynomial.add
      (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow inverse.uNumerator 2) (NecoGeometryVerify.RegionPolynomial.pow inverse.vDenominator 2))
      (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.pow inverse.vNumerator 2) (NecoGeometryVerify.RegionPolynomial.pow inverse.uDenominator 2)))

/-- Composed chart numerator and denominator for axis or the denominator
    polynomial of `chart` through `inverse`. -/
def composedChart (chart : List SurfaceTerm) (inverse : InverseChart) :
    (List NecoGeometryVerify.RegionTerm) × (List NecoGeometryVerify.RegionTerm) :=
  composeAtomArgs chart inverse.uNumerator inverse.uDenominator
    inverse.vNumerator inverse.vDenominator

/-- Residual certifying `χ_axis(un/ud, vn/vd) = p_axis` on the atom:
    `A_axis·D_d − var_axis·A_d·D_axis`. -/
def imageResidual (axis : Nat) (axisTerms denominator : List SurfaceTerm)
    (inverse : InverseChart) : (List NecoGeometryVerify.RegionTerm) :=
  let (aAxis, dAxis) := composedChart axisTerms inverse
  let (aD, dD) := composedChart denominator inverse
  NecoGeometryVerify.RegionPolynomial.sub (NecoGeometryVerify.RegionPolynomial.mul aAxis dD)
    (NecoGeometryVerify.RegionPolynomial.mul (NecoGeometryVerify.RegionPolynomial.variablePolynomial axis) (NecoGeometryVerify.RegionPolynomial.mul aD dAxis))

/-- Per-clause certificate checks. `atom` is the face atom. -/
def checkClause (atom : (List NecoGeometryVerify.RegionTerm)) (patch : CoveringPatch) (clause : CoveringClause) :
    Bool :=
  let inverse := patch.inverse
  let (aD, _) := composedChart patch.chart.denominator inverse
  let residualX := imageResidual 0 patch.chart.x patch.chart.denominator inverse
  let residualY := imageResidual 1 patch.chart.y patch.chart.denominator inverse
  let residualZ := imageResidual 2 patch.chart.z patch.chart.denominator inverse
  decide (clause.patch = patch.patchIndex) &&
  patch.disk &&
  -- The parameter box contains the unit disk's `[-1,1]²` hull.
  decide (patch.domain.u.lower ≤ -1) && decide (1 ≤ patch.domain.u.upper) &&
  decide (patch.domain.v.lower ≤ -1) && decide (1 ≤ patch.domain.v.upper) &&
  decide (clause.guards.length = 1) &&
  -- inverse denominators: `ud = 1 + g`, `vd = 1 + g` (bounds certify
  -- `ud,vd ≥ 1` on `g ≥ 0`, hence nonzero where the clause applies).
  boundFormHolds inverse.uDenominator clause.inverseDenominatorBounds.1
    clause.guards atom &&
  boundFormHolds inverse.vDenominator clause.inverseDenominatorBounds.2
    clause.guards atom &&
  boundFormNonnegative clause.inverseDenominatorBounds.1 &&
  boundFormNonnegative clause.inverseDenominatorBounds.2 &&
  decide (clause.inverseDenominatorBounds.1.constant > 0) &&
  decide (clause.inverseDenominatorBounds.2.constant > 0) &&
  -- composed chart denominator `A_d > 0` on `g ≥ 0`.
  boundFormHolds aD clause.chartDenominatorBound clause.guards atom &&
  boundFormNonnegative clause.chartDenominatorBound &&
  decide (clause.chartDenominatorBound.constant > 0) &&
  -- disk-range certificate.
  boundFormHolds (diskConditionNumerator inverse) clause.diskRange
    clause.guards atom &&
  boundFormNonnegative clause.diskRange &&
  -- image quotients: residuals are atom multiples.
  decide (NecoGeometryVerify.RegionPolynomial.normalize residualX =
    NecoGeometryVerify.RegionPolynomial.normalize (NecoGeometryVerify.RegionPolynomial.mul clause.imageQuotients.1 atom)) &&
  decide (NecoGeometryVerify.RegionPolynomial.normalize residualY =
    NecoGeometryVerify.RegionPolynomial.normalize (NecoGeometryVerify.RegionPolynomial.mul clause.imageQuotients.2.1 atom)) &&
  decide (NecoGeometryVerify.RegionPolynomial.normalize residualZ =
    NecoGeometryVerify.RegionPolynomial.normalize (NecoGeometryVerify.RegionPolynomial.mul clause.imageQuotients.2.2 atom))

/-- Sum of per-clause guard products: the coverage left-hand side. -/
def coverageSum (clauses : List CoveringClause) : (List NecoGeometryVerify.RegionTerm) :=
  NecoGeometryVerify.RegionPolynomial.normalize (clauses.foldl (fun acc clause =>
    NecoGeometryVerify.RegionPolynomial.add acc (clause.guards.foldl (fun product guard =>
      NecoGeometryVerify.RegionPolynomial.mul product guard)
      [{ coefficient := 1, exponentX := 0, exponentY := 0, exponentZ := 0 }])) [])

def decodeClause (value : Value) : Option CoveringClause := do
  let fields ← jsonObjectWithExactFields? value
    ["chartDenominatorBound", "diskRange", "guards",
      "imageQuotients", "inverseDenominatorBounds", "patch"]
  let patch ← decodeNatAtMost fields "patch" 64
  let guardValues ← decodeValueArray (← jsonField? fields "guards")
  if guardValues.length > 8 then none
  let guards ← guardValues.mapM decodeAtomPoly
  let denominatorFields ← jsonObjectWithExactFields?
    (← jsonField? fields "inverseDenominatorBounds") ["u", "v"]
  let boundU ← decodeBoundForm (← jsonField? denominatorFields "u")
  let boundV ← decodeBoundForm (← jsonField? denominatorFields "v")
  let chartDenominatorBound ← decodeBoundForm
    (← jsonField? fields "chartDenominatorBound")
  let diskRange ← decodeBoundForm (← jsonField? fields "diskRange")
  let quotientFields ← jsonObjectWithExactFields?
    (← jsonField? fields "imageQuotients") ["x", "y", "z"]
  let quotientX ← decodeAtomPoly (← jsonField? quotientFields "x")
  let quotientY ← decodeAtomPoly (← jsonField? quotientFields "y")
  let quotientZ ← decodeAtomPoly (← jsonField? quotientFields "z")
  pure { patch, guards, inverseDenominatorBounds := (boundU, boundV), chartDenominatorBound, diskRange, imageQuotients := (quotientX, quotientY, quotientZ) }

def decodeFaceAtomCoveringObservation (value : Value) :
    Option FaceAtomCoveringObservation := do
  let fields ← jsonObjectWithExactFields? value
    ["atom", "atomId", "atomRelation", "clauses", "coverage", "faceId",
      "outerGuards", "patches"]
  let faceId ← jsonStringField? fields "faceId"
  let atomId ← jsonStringField? fields "atomId"
  let atomRelation ← decodeStringIn fields "atomRelation"
    ["negative", "positive", "zero"]
  let atom ← decodeAtomPoly (← jsonField? fields "atom")
  let outerValues ← decodeValueArray (← jsonField? fields "outerGuards")
  if outerValues.length > 8 then none
  let outerGuards ← outerValues.mapM decodeAtomPoly
  let patchValues ← decodeValueArray (← jsonField? fields "patches")
  if patchValues.length > 64 || patchValues.isEmpty then none
  let patches ← patchValues.mapM decodeCoveringPatch
  let clauseValues ← decodeValueArray (← jsonField? fields "clauses")
  if clauseValues.length > 64 then none
  let clauses ← clauseValues.mapM decodeClause
  let coverageFields ← jsonObjectWithExactFields? (← jsonField? fields "coverage")
    ["selections"]
  let selectionValues ← decodeValueArray (← jsonField? coverageFields "selections")
  if selectionValues.length > 64 || selectionValues.isEmpty then none
  let coverageSelections ← selectionValues.mapM decodeBoundForm
  pure { faceId, atomId, atomRelation, atom, outerGuards, patches, clauses, coverageSelections }

/-- Whole covering check: each patch is a valid chart of the atom, each
    clause's certificates hold against its referenced patch, disk patches
    cover `[-1,1]²` in their domain box, and the coverage selections prove
    `Σ clause-guard-products ≥ 0` on the atom. -/
def checkFaceAtomCovering (observation : FaceAtomCoveringObservation) : Bool :=
  let patchesOk := observation.patches.all fun patch =>
    checkPatchAgainstAtom observation.atom patch.toPatchData &&
      patch.patchIndex < observation.patches.length &&
      (!patch.disk ||
        (decide (patch.domain.u.lower ≤ -1) && decide (patch.domain.u.upper ≥ 1) &&
          decide (patch.domain.v.lower ≤ -1) && decide (patch.domain.v.upper ≥ 1)))
  let clausesOk := observation.clauses.all fun clause =>
    match observation.patches.find? (fun p => p.patchIndex == clause.patch) with
    | some patch => checkClause observation.atom patch clause
    | none => false
  let lhs := coverageSum observation.clauses
  let coverageOk := !observation.clauses.isEmpty &&
    !observation.coverageSelections.isEmpty &&
    observation.coverageSelections.all fun selection =>
      boundFormHolds lhs selection [] observation.atom &&
        boundFormNonnegative selection
  patchesOk && clausesOk && coverageOk &&
    observation.clauses.length == observation.patches.length

def FaceAtomCoveringObservation.Valid (observation : FaceAtomCoveringObservation) :
    Prop :=
  checkFaceAtomCovering observation = true

instance decidableFaceAtomCoveringValid (observation : FaceAtomCoveringObservation) :
    Decidable observation.Valid := by
  unfold FaceAtomCoveringObservation.Valid checkFaceAtomCovering
  infer_instance

def acceptFaceAtomCovering (observation : FaceAtomCoveringObservation) : Bool :=
  decide observation.Valid

end NecoCadVerify
