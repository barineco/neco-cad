# neco-tessellation

[日本語](README-ja.md)

`neco-tessellation` tessellates analytic boundary representations with declared precision.

## Features

**Precision by Use**

Accepts a precision request labeled with the use of the tessellation (display, analysis, or picking) and records the achieved precision as a witness.

**Element Correspondence**

Maps each tessellated element back to its boundary representation element.

**Mesh Output**

The output is a validated surface mesh built from the tessellated faces.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-tessellation = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-tessellation = { version = "0.3", default-features = false }
```

## Dependencies

* [`neco-brep`](https://docs.rs/neco-brep): Analytic boundary representation input.
* [`neco-mesh`](https://docs.rs/neco-mesh): Validated surface mesh output.

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
