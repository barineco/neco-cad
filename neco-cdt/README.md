# neco-cdt

[日本語](README-ja.md)

`neco-cdt` performs constrained Delaunay triangulation over validated planar inputs.

## Features

**Constrained Triangulation**

Triangulates a planar region while preserving declared constraint edges.

**Typed Failures**

Reports degenerate inputs and constraint violations as typed failures.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-cdt = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-cdt = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-mesh`](https://docs.rs/neco-mesh): Validated triangle mesh output.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
