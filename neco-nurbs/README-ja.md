# neco-nurbs

[English](README.md)

`neco-nurbs` は、NURBS 曲線・曲面の厳密な評価と当てはめを提供します。

## 主要機能

**厳密な評価**

厳密な重みと節点列を用いて、有理式と多項式の NURBS を評価します。

**曲線の当てはめ**

厳密な連立一次方程式を解き、標本点に NURBS 曲線を当てはめます。

**検証済みの宣言**

構成時には、正でない重みと不正な節点列を拒否します。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-nurbs = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-nurbs = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-bezier`](https://docs.rs/neco-bezier): Bézier の制御点と評価結果
* [`neco-linear-exact`](https://docs.rs/neco-linear-exact): 当てはめで解く厳密な連立一次方程式

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
