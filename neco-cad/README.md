# neco-cad

[日本語](README-ja.md)

`neco-cad` is the public API of a CAD engine that constructs CAD shape data, such as boundary representations, tessellations, and meshes, starting from algebraic regions.

## Features

**Region Construction**

Splits the boundary of a region into 0-, 1-, and 2-dimensional parts, corresponding to vertices, edges, and faces, and derives the vertices, edges, and faces of the boundary representation from them.

**Boolean Operations**

Combines the atoms of two regions and builds a new region from a given logical formula. Solid operations check that the region kind of both inputs is a closed solid (`RegularClosedVolume`) and set the region kind of the result to a closed solid.

**Result Provenance**

The result retains the construction request and its derivation.

## Public API Pipeline

The engine transfers ownership of values from a construction request to a certified region in this order:

```text
ConstructionRequest
  -> ConstructionResult::into_outputs
  -> derive_complete_boundary
  -> derive_analytic_brep
  -> DerivedAnalyticBrep::into_adaptive_tessellation_request
  -> tessellate_adaptive
  -> AdaptiveTessellation::into_closed_oriented_surface_mesh
  -> region_from_mesh
  -> CertifiedRegion
```

`DerivedAnalyticBrep`, returned by `derive_analytic_brep`, is one value that combines the boundary representation and the surface information of each Face:

* The analytic boundary representation
* The atom (a sign condition on a function) that represents the exact surface of each Face
* The surface covering of each Face
* The B-Rep source revision, a revision string that the caller attaches to the construction request

The mesh identity is the name of the generated mesh, which the caller supplies in the adaptive tessellation request. The region re-imported from the mesh retains both the identity and the source revision.

## Current Limitations

### Surface Coverage

Exact surfaces can be derived for planar and spherical faces. Support is limited to planes and spheres themselves and to the planar and spherical faces of some solids:

* General affine planes
* Spheres with general centers and radii
* Faces of boxes (planar)
* Planar Faces of extrusions from profiles made of line segments
* Sphere unions with disjoint interiors (separated or externally tangent)

Surfaces other than those listed above, and surfaces that require curved trims, return the classified failure `UnsupportedSurfaceCovering`. Besides surface kinds, the kinds of Boolean operations can also make an input unsupported.

* Cylinders, cones, general ellipsoids, and tori
* General revolutions and Booleans of overlapping spheres

### Boundary Derivation

Boundary derivation searches the points of a bounded integer lattice for boundary seeds, the points from which boundary candidates are computed.
The failures specific to boundary derivation (`BoundaryDerivationCondition`) are determined in the order of the derivation steps.

1. When the region has no atom or its normalized formula is false (`False`), the failure is `EmptyRegion`.
2. The atoms are scanned one by one. The failure `SeamUnsupported` is returned when the scan reaches an atom defined by a sign condition on a rational function. When the scan cannot classify whether a lattice point belongs to the region, the failure is `IncompleteCoverage`.
3. After the scan, if the zeros of an atom found on the lattice include a singular point (a point where the gradient is 0) and the search finds no boundary seed from that atom, the failure `SingularityUnsupported` is returned.
4. When the search finds no boundary seed at all, the failure is `IncompleteCoverage`.
5. For a closed solid, `IncompleteCoverage` is also returned when, for some atom, no part of the boundary both has a local proof and contains a candidate derived from that atom.

### Construction Operations

Construction request types can represent sweep, loft, fillet, and chamfer. The current implementation returns classified failures for these requests.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-cad = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-cad = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-brep`](https://docs.rs/neco-brep): Analytic boundary representation.
* [`neco-geometry`](https://docs.rs/neco-geometry): Algebraic regions and the split of boundaries by dimension.
* [`neco-calculus`](https://docs.rs/neco-calculus): Validated evaluation.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
