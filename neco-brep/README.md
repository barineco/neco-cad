# neco-brep

[日本語](README-ja.md)

`neco-brep` builds analytic boundary representations from a boundary split by dimension into parts corresponding to vertices, edges, and faces.

## Features

**Materialization**

Constructs vertices, edges, faces, trims, shells, and their incidences from the parts of the boundary.

**Assignments**

Assigns each part of the boundary a vertex, segment, or face of the arrangement on a chart. Edges and faces are also assigned a NURBS curve or surface representation.

**Witnesses**

The result retains the source revision and the derivation witnesses. The source revision is the revision string that the caller attaches to the materialization request. The derivation witnesses consist of the incidence count, the list of closed shells, and a topology witness for each shell (closure, orientation, and manifoldness).

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-brep = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-brep = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-geometry`](https://docs.rs/neco-geometry): The split of a boundary by dimension, and charts.
* [`neco-nurbs`](https://docs.rs/neco-nurbs): Curve and surface representations.
* [`neco-arrangement2d`](https://docs.rs/neco-arrangement2d): Planar arrangements used by trims.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
