# neco-nurbs

[日本語](README-ja.md)

`neco-nurbs` provides NURBS curves and surfaces with exact evaluation and fitting.

## Features

**Exact Evaluation**

Evaluates rational and polynomial NURBS with exact weights and knot vectors.

**Curve Fitting**

Fits a NURBS curve to sample points by solving an exact linear system.

**Validated Declarations**

Construction rejects non-positive weights and malformed knot vectors.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-nurbs = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-nurbs = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-bezier`](https://docs.rs/neco-bezier): Bézier control points and evaluation results.
* [`neco-linear-exact`](https://docs.rs/neco-linear-exact): Exact linear systems used by fitting.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
