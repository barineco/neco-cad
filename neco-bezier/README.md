# neco-bezier

[日本語](README-ja.md)

`neco-bezier` provides Bézier curves and surfaces with exact evaluation over real algebraic coordinates.

## Features

**Exact Evaluation**

Evaluates polynomial and rational Bézier curves and surfaces without floating-point error.

**Splitting**

Splitting uses an exact parameter value, so the pieces together are identical to the original geometry.

**Containment**

Decides whether a point lies on a curve or surface using exact comparison.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-bezier = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-bezier = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-algnum`](https://docs.rs/neco-algnum): Exact real algebraic coordinates.
* [`neco-geometry`](https://docs.rs/neco-geometry): Two- and three-dimensional point types.
* [`neco-calculus`](https://docs.rs/neco-calculus): Validated evaluation.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
