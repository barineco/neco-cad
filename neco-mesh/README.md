# neco-mesh

[日本語](README-ja.md)

`neco-mesh` is a crate that provides validated surface and triangle mesh types. Inputs that fail validation return typed failures.

## Features

**Surface Meshes**

A surface mesh consists of vertex positions and, for each triangle, the indices of the vertices it references. Construction validates that every index is less than the number of vertices and that no triangle is degenerate.

**Triangle Meshes**

A triangle mesh is a two-dimensional mesh of triangles used by triangulation and tessellation.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-mesh = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-mesh = { version = "0.3", default-features = false }
```

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
