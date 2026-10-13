# neco-arrangement2d

[日本語](README-ja.md)

`neco-arrangement2d` constructs the arrangement induced by a finite set of planar curves.

## Features

**Arrangement Construction**

The arrangement consists of vertices, faces, holes, and curve segments derived from curve intersections.

**Parameter Association**

The intersection set of two curves consists of intersection points and overlapping spans. Each connected component of that set is associated with parameter values and endpoints on the curves.

**Typed Failures**

Incomplete associations of intersection points or overlapping spans return typed failures. Face and hole boundaries are traced as sequences of directed curve segments. A sequence that fails to return to its start yields `NonClosedWalk`, and a hole declared as its own containing outer boundary yields `ContainingOuterWalkSelfReference`.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-arrangement2d = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-arrangement2d = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-geometry`](https://docs.rs/neco-geometry): Curve charts and intersection sets.
* [`neco-calculus`](https://docs.rs/neco-calculus): Validated evaluation and root evidence.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
