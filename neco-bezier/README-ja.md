# neco-bezier

[English](README.md)

`neco-bezier` は、実代数的数を座標とする Bézier 曲線・曲面の厳密な評価を提供します。

## 主要機能

**厳密な評価**

多項式と有理式による Bézier 曲線・曲面を、浮動小数点の誤差なく評価します。

**分割**

分割は厳密なパラメータの値で行うため、分割した各部分を合わせた形状は元の形状と同一です。

**包含判定**

点が曲線または曲面の上にあるかを、厳密な比較で判定します。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-bezier = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-bezier = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-algnum`](https://docs.rs/neco-algnum): 厳密な実代数的座標
* [`neco-geometry`](https://docs.rs/neco-geometry): 2 次元と 3 次元の点の型
* [`neco-calculus`](https://docs.rs/neco-calculus): 検証済みの評価

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
