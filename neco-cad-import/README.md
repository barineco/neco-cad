# neco-cad-import

[日本語](README-ja.md)

`neco-cad-import` constructs certified regions from closed oriented meshes and inspected analytic shells.

## Features

**Mesh import**

Accepts a closed oriented surface mesh together with a requested error bound and a ray crossing policy, and returns a certified region.

**Shell import**

Inspects a shell selected by name from an analytic boundary representation and returns a certified region when evidence is present for all five of closure, orientation, manifoldness, trims, and crossings. The caller supplies three values for exact decisions on expressions. Every target tested for crossings with the ray (each mesh triangle or each shell face) retains these values unchanged.

- `OwnerId`: identifier of the owner of an expression set.
- `ExactExpressionRequirement`: request for an exact decision on expressions. It consists of the consumer identifier (the place that receives the result), the list of target expressions, the decision (its kind and the expressions from the list that it decides), and the precision.
- `NecoImplementationSource`: version of the exact computation implementation neco-expr and its public API identifier.

**Membership**

Classifies a point with finite coordinates as interior to the region, exterior to it, or on its boundary, using a ray from the point in the positive X direction and the crossing policy given in the request. Each call supplies the three values listed above. The result holds the classification, the hit count, and a typed witness for the case where the classification is undetermined.

## Runtime Configuration

The default configuration uses the standard library.

```toml
[dependencies]
neco-cad-import = "0.3"
```

Disabling the default `std` feature leaves dynamic memory allocation as the only runtime requirement, so the crate can be used in environments without the standard library.

```toml
[dependencies]
neco-cad-import = { version = "0.3", default-features = false }
```

## License

MIT License (See the repository [LICENSE](../LICENSE) for details).
