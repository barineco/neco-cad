# neco-stl

[日本語](README-ja.md)

`neco-stl` is a crate that converts between validated surface meshes and STL data.

## Features

**Reading**

Builds a surface mesh from a binary STL byte sequence. Reading validates that the triangle count in the header is consistent with the length of the byte sequence.

**Writing**

A validated surface mesh can be written as either binary or ASCII STL.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-stl = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-stl = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-mesh`](https://docs.rs/neco-mesh): Validated surface mesh types.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
