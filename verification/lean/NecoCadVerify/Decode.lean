import NecoCadVerify.Observation

set_option autoImplicit false

namespace NecoCadVerify

open NecoLean.Json
open NecoAlgebraVerify
open NecoCalculusVerify

-- ============================================================
-- Shared JSON decoders (strict exact-field decoding)
-- ============================================================

def jsonBoolField? (fields : JsonFields) (name : String) : Option Bool := do
  match ← jsonField? fields name with
  | .bool value => some value
  | _ => none

/-- Bivariate surface polynomial term list, normalized after decoding. -/
def decodeSurfaceTerms (value : Value) : Option (List SurfaceTerm) := do
  let values ← decodeValueArray value
  if values.length > 4096 then none
  let decoded ← values.mapM fun term => do
    let fields ← jsonObjectWithExactFields? term ["coefficient", "exponentU", "exponentV"]
    let coefficient ← decodeRationalSchema (← jsonField? fields "coefficient")
    let exponentU ← decodeNatAtMost fields "exponentU" 256
    let exponentV ← decodeNatAtMost fields "exponentV" 256
    pure ({ coefficient, exponentU, exponentV : SurfaceTerm })
  pure (NecoCalculusVerify.SurfacePolynomial.normalize decoded)

/-- Trivariate atom polynomial, normalized after decoding. -/
def decodeAtomPoly (value : Value) : Option (List NecoGeometryVerify.RegionTerm) := do
  let values ← decodeValueArray value
  if values.length > 4096 then none
  let decoded ← values.mapM fun term => do
    let fields ← jsonObjectWithExactFields? term
      ["coefficient", "exponentX", "exponentY", "exponentZ"]
    let coefficient ← decodeRationalSchema (← jsonField? fields "coefficient")
    let exponentX ← decodeNatAtMost fields "exponentX" 256
    let exponentY ← decodeNatAtMost fields "exponentY" 256
    let exponentZ ← decodeNatAtMost fields "exponentZ" 256
    pure ({ coefficient, exponentX, exponentY, exponentZ : NecoGeometryVerify.RegionTerm })
  pure (NecoGeometryVerify.RegionPolynomial.normalize decoded)

/-- Guard power list `{powers : [Nat], coefficient : rational}`. -/
private def decodePowersList (value : Value) : Option (List (List Nat × Rat)) := do
  let values ← decodeValueArray value
  if values.length > 4096 then none
  values.mapM fun entry => do
    let fields ← jsonObjectWithExactFields? entry ["coefficient", "powers"]
    let powerValues ← decodeValueArray (← jsonField? fields "powers")
    if powerValues.length > 8 then none
    let powers ← powerValues.mapM fun item => jsonNat? item
    if powers.any (· > 256) then none
    let coefficient ← decodeRationalSchema (← jsonField? fields "coefficient")
    pure (powers, coefficient)

/-- 3-var bound form `{constant, guardPowers, squares, atomQuotient}`. -/
def decodeBoundForm (value : Value) : Option BoundForm := do
  let fields ← jsonObjectWithExactFields? value
    ["atomQuotient", "constant", "guardPowers", "squares"]
  let constant ← decodeRationalSchema (← jsonField? fields "constant")
  let guardPowers ← decodePowersList (← jsonField? fields "guardPowers")
  let squareValues ← decodeValueArray (← jsonField? fields "squares")
  if squareValues.length > 64 then none
  let squares ← squareValues.mapM fun entry => do
    let entryFields ← jsonObjectWithExactFields? entry ["coefficient", "polynomial"]
    let coefficient ← decodeRationalSchema (← jsonField? entryFields "coefficient")
    let polynomial ← decodeAtomPoly (← jsonField? entryFields "polynomial")
    pure (coefficient, polynomial)
  let atomQuotient ← decodeAtomPoly (← jsonField? fields "atomQuotient")
  pure { constant, guardPowers, squares, atomQuotient }

/-- 2-var bound form `{constant, domainPowers, squares}`. -/
def decodeSurfaceBoundForm (value : Value) : Option SurfaceBoundForm := do
  let fields ← jsonObjectWithExactFields? value ["constant", "domainPowers", "squares"]
  let constant ← decodeRationalSchema (← jsonField? fields "constant")
  let domainPowers ← decodePowersList (← jsonField? fields "domainPowers")
  let squareValues ← decodeValueArray (← jsonField? fields "squares")
  if squareValues.length > 64 then none
  let squares ← squareValues.mapM fun entry => do
    let entryFields ← jsonObjectWithExactFields? entry ["coefficient", "polynomial"]
    let coefficient ← decodeRationalSchema (← jsonField? entryFields "coefficient")
    let polynomial ← decodeSurfaceTerms (← jsonField? entryFields "polynomial")
    pure (coefficient, polynomial)
  pure { constant, domainPowers, squares }

/-- Chart `{x, y, z, denominator}` as bivariate term lists. -/
def decodeChart (value : Value) : Option ChartPolynomials := do
  let fields ← jsonObjectWithExactFields? value ["denominator", "x", "y", "z"]
  let x ← decodeSurfaceTerms (← jsonField? fields "x")
  let y ← decodeSurfaceTerms (← jsonField? fields "y")
  let z ← decodeSurfaceTerms (← jsonField? fields "z")
  let denominator ← decodeSurfaceTerms (← jsonField? fields "denominator")
  pure { x, y, z, denominator }

/-- Closed rational interval `{lower, upper, lowerClosed, upperClosed}`;
    open bounds are not accepted: semantics uses the closed interval. -/
def decodeClosedInterval (value : Value) : Option NecoAlgebraVerify.Interval := do
  let fields ← jsonObjectWithExactFields? value
    ["lower", "lowerClosed", "upper", "upperClosed"]
  let lower ← decodeRationalSchema (← jsonField? fields "lower")
  let upper ← decodeRationalSchema (← jsonField? fields "upper")
  let lowerClosed ← jsonBoolField? fields "lowerClosed"
  let upperClosed ← jsonBoolField? fields "upperClosed"
  if !(lowerClosed && upperClosed) then none
  pure { lower, upper }

def decodePatchDomain (value : Value) : Option PatchDomain := do
  let fields ← jsonObjectWithExactFields? value ["u", "v"]
  let u ← decodeClosedInterval (← jsonField? fields "u")
  let v ← decodeClosedInterval (← jsonField? fields "v")
  pure { u, v }

/-- The four domain bound polynomials
    `[u−lowerU, upperU−u, v−lowerV, upperV−v]`. -/
def domainBoundPolys (domain : PatchDomain) : List (List SurfaceTerm) :=
  [ NecoCalculusVerify.SurfacePolynomial.sub NecoCalculusVerify.SurfacePolynomial.variableU (NecoCalculusVerify.SurfacePolynomial.constant domain.u.lower),
    NecoCalculusVerify.SurfacePolynomial.sub (NecoCalculusVerify.SurfacePolynomial.constant domain.u.upper) NecoCalculusVerify.SurfacePolynomial.variableU,
    NecoCalculusVerify.SurfacePolynomial.sub NecoCalculusVerify.SurfacePolynomial.variableV (NecoCalculusVerify.SurfacePolynomial.constant domain.v.lower),
    NecoCalculusVerify.SurfacePolynomial.sub (NecoCalculusVerify.SurfacePolynomial.constant domain.v.upper) NecoCalculusVerify.SurfacePolynomial.variableV ]

def decodeInverseChart (value : Value) : Option InverseChart := do
  let fields ← jsonObjectWithExactFields? value
    ["uDenominator", "uNumerator", "vDenominator", "vNumerator"]
  let uNumerator ← decodeAtomPoly (← jsonField? fields "uNumerator")
  let uDenominator ← decodeAtomPoly (← jsonField? fields "uDenominator")
  let vNumerator ← decodeAtomPoly (← jsonField? fields "vNumerator")
  let vDenominator ← decodeAtomPoly (← jsonField? fields "vDenominator")
  pure { uNumerator, uDenominator, vNumerator, vDenominator }

/-- A patch validates a null inverse or a complete inverse chart. -/
def decodePatchBody (value : Value) : Option PatchData := do
  let fields ← jsonObjectWithExactFields? value
    ["chart", "chartDenominatorBound", "disk", "domain", "inverse", "patchIndex"]
  let patchIndex ← decodeNatAtMost fields "patchIndex" 64
  let chart ← decodeChart (← jsonField? fields "chart")
  let domain ← decodePatchDomain (← jsonField? fields "domain")
  let disk ← jsonBoolField? fields "disk"
  let denominatorBound ← decodeSurfaceBoundForm
    (← jsonField? fields "chartDenominatorBound")
  let inverse ← jsonField? fields "inverse"
  match inverse with
  | .null => pure ()
  | _ => do
      let _ ← decodeInverseChart inverse
      pure ()
  pure { patchIndex, chart, domain, disk, denominatorBound }

/-- Covering patch: patch body plus the inverse chart. -/
def decodeCoveringPatch (value : Value) : Option CoveringPatch := do
  let fields ← jsonObjectWithExactFields? value
    ["chart", "chartDenominatorBound", "disk", "domain", "inverse", "patchIndex"]
  let patchIndex ← decodeNatAtMost fields "patchIndex" 64
  let chart ← decodeChart (← jsonField? fields "chart")
  let domain ← decodePatchDomain (← jsonField? fields "domain")
  let disk ← jsonBoolField? fields "disk"
  let denominatorBound ← decodeSurfaceBoundForm
    (← jsonField? fields "chartDenominatorBound")
  let inverse ← decodeInverseChart (← jsonField? fields "inverse")
  pure { patchIndex, chart, domain, disk, denominatorBound, inverse }

def decodeVec3Point (value : Value) : Option Vec3 := do
  let fields ← jsonObjectWithExactFields? value ["x", "y", "z"]
  let x ← decodeRationalSchema (← jsonField? fields "x")
  let y ← decodeRationalSchema (← jsonField? fields "y")
  let z ← decodeRationalSchema (← jsonField? fields "z")
  pure (x, y, z)

/-- Mesh vertex `{x, y, z}` of dyadic coordinates. -/
def decodeMeshVertex (value : Value) : Option MeshVertex := do
  let fields ← jsonObjectWithExactFields? value ["x", "y", "z"]
  let x ← decodeDyadicRatSchema (← jsonField? fields "x")
  let y ← decodeDyadicRatSchema (← jsonField? fields "y")
  let z ← decodeDyadicRatSchema (← jsonField? fields "z")
  pure { x, y, z }

def decodeNatTriple (value : Value) : Option (Nat × Nat × Nat) := do
  let values ← decodeValueArray value
  match values with
  | [a, b, c] => do
      let na ← jsonNat? a
      let nb ← jsonNat? b
      let nc ← jsonNat? c
      pure (na, nb, nc)
  | _ => none

def decodeMesh (value : Value) : Option MeshData := do
  let fields ← jsonObjectWithExactFields? value ["triangles", "vertices"]
  let vertexValues ← decodeValueArray (← jsonField? fields "vertices")
  if vertexValues.length > 65536 then none
  let vertices ← vertexValues.mapM decodeMeshVertex
  let triangleValues ← decodeValueArray (← jsonField? fields "triangles")
  if triangleValues.length > 65536 then none
  let triangles ← triangleValues.mapM decodeNatTriple
  pure { vertices, triangles }

def decodeNatList (value : Value) (bound : Nat) : Option (List Nat) := do
  let values ← decodeValueArray value
  if values.length > bound then none
  values.mapM jsonNat?

end NecoCadVerify
