# neco-cad-view

[日本語](README-ja.md)

`neco-cad-view` provides a view model for displaying CAD results and a reference projection computed on the CPU. The view model records the source revision of the boundary representation. The source revision is the revision string that the caller attaches to the construction request.

## Features

**View Model**

The view model combines the tessellated geometry and the element identities together with their revision.

**Reference Projection**

As a reference implementation, projects the view model on the CPU.

**Picking**

Identifies the boundary representation element that corresponds to a projected coordinate.

## Precision and Identity

A view model is derived from the result of either the basic or the adaptive tessellation and holds the mesh, the element correspondences, and the precision status. Derivation first checks the source revision and the precision requests, then applies shared element-correspondence rules. The model exposes requested precision values separately from certified bounds.

* `CadViewModelRequest`: derivation request that combines the tessellation input, labeled with its kind, and the viewer state.
* `Basic`: tessellation kind that records only the requested chord error and has no certified bound.
* `NotEstablished`: basic precision status that holds only the requested chord error.
* `TessellationResult`: mesh and element correspondences produced by basic tessellation.
* `TessellationRequest`: chord-error request passed to basic tessellation.
* `Adaptive`: tessellation kind that carries a certified precision witness.
* `AdaptiveTessellation`: mesh, element correspondences, and witness produced by adaptive tessellation.
* `AdaptivePrecision`: the use of the tessellation (display, analysis, or picking), the chord-error limit, and the surface-residual limit.
* `SourceRevision`: source revision of the boundary representation, checked during view model derivation.
* `SourceIdentity`: mesh identity, the name of the generated mesh that the caller supplies in the adaptive tessellation request.
* `ViewModelIdentity`: value that exposes the source revision and the optional mesh identity separately.
* Adaptive tessellation witness: a chord-error upper bound for each triangle edge and an exact dyadic upper bound on the surface residual of each vertex.

The surface residual is the absolute value, at a vertex, of the function $f$ that defines the face surface as $f = 0$. Its bound is computed over dyadic intervals that enclose the exact vertex coordinates.

CPU reference projection takes the derived view model as input. Picking takes the result of that reference projection as input.

## Camera Numerical Specification

The perspective field of view $\theta$ is the full vertical angle in radians.
Cameras accept finite values satisfying $0 < \theta < \pi$.
At depth $z$, the frustum half-height is $z \tan(\theta/2)$, and the half-width is that value multiplied by the viewport aspect ratio.
Ray generation, CPU projection, and clipping against the top, bottom, left, and right frustum planes all use the same tangent computed once during camera construction.

Pixel coordinates include an offset of 0.5 for the pixel center.
For example, with a 100 × 100 viewport, a full field of view of $\pi/2$, and a camera facing negative z, the ray direction and the screen edges take these values:

* The ray through pixel ( 0, 49.5 ) has direction ratio x/z = 0.99.
* The screen edges correspond to coordinates -0.5 and 99.5.

Both runtime configurations use the same double-precision arithmetic. Angles approaching $\pi$ take the same arithmetic path and evaluate the tangent of the supplied double-precision angle. When the arithmetic fails, the returned value identifies whether the failure occurred in projection, the field of view, or the ray.

* Arithmetic: `libm` tangent and square root.
* Projection failure: `NonFiniteCoordinate` when the tangent rounds to zero or a projected coordinate becomes nonfinite.
* Angle failure: `InvalidPerspectiveFieldOfView` for nonfinite or out-of-domain angles.
* Ray failure: preserves the underlying `GeometryFailure` for nonfinite values.

The adaptive precision witness covers only the chord-error and surface-residual bounds of the tessellated geometry.
Camera floating-point arithmetic carries its own rounding error, separate from that witness.
The zero tolerance supplied during ray construction is used to check that the numerical direction is a nonzero vector.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-cad-view = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-cad-view = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-cad`](https://docs.rs/neco-cad): Adaptive tessellation results and mesh identities.
* [`neco-bigint`](https://docs.rs/neco-bigint): Exact dyadic numbers for surface-residual bounds.
* [`neco-brep`](https://docs.rs/neco-brep): Boundary representation elements.
* [`neco-tessellation`](https://docs.rs/neco-tessellation): Tessellated geometry.
* [`neco-mesh`](https://docs.rs/neco-mesh): Validated surface meshes.
* [`neco-geometry`](https://docs.rs/neco-geometry): Checked rays and geometry failures.
* [`neco-nurbs`](https://docs.rs/neco-nurbs): Surface control points and identities.
* [`libm`](https://docs.rs/libm): Shared tangent and square-root arithmetic in both runtime configurations.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
