# neco-cdt

[English](README.md)

`neco-cdt` は、検証済みの平面入力に対して、制約付き Delaunay 三角形分割を行います。

## 主要機能

**制約付き三角形分割**

宣言された制約辺を保ちながら、平面領域を三角形へ分割します。

**型付きの失敗**

退化した入力と制約違反を、型付きの失敗として返します。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-cdt = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-cdt = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-mesh`](https://docs.rs/neco-mesh): 検証済みの三角形メッシュ

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
