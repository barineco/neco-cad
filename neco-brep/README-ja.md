# neco-brep

[English](README.md)

`neco-brep` は、頂点、辺、面にあたる次元ごとの部分に分けた境界から、解析的な境界表現を構成します。

## 主要機能

**構成**

境界の各部分から、頂点、辺、面、トリム、シェルと、それらの接続関係を作ります。

**割り当て**

境界の各部分に、局所座標系 ( chart ) 上の交差構造の頂点、区間、面を割り当てます。辺と面には、NURBS による曲線または曲面の表現も割り当てます。

**証拠**

構成した結果は、導出元の版 ( source revision ) と導出の証拠を保持します。導出元の版は、呼び出し側が構成の要求に付ける版の文字列です。導出の証拠は、接続関係の数、閉じたシェルの一覧、シェルごとの位相の証拠 ( 閉じているか、向き、多様体であるか ) から成ります。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-brep = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-brep = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-geometry`](https://docs.rs/neco-geometry): 境界の次元ごとの分割と局所座標系
* [`neco-nurbs`](https://docs.rs/neco-nurbs): 曲線と曲面の表現
* [`neco-arrangement2d`](https://docs.rs/neco-arrangement2d): トリムで使う平面上の交差構造

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
