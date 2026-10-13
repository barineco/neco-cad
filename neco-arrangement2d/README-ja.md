# neco-arrangement2d

[English](README.md)

`neco-arrangement2d` は、有限個の平面曲線が作る交差構造 ( arrangement ) を構成します。

## 主要機能

**交差構造の構成**

交差構造の要素は、曲線の交差から導出した頂点、面、穴と、曲線の区間 ( segment ) です。

**パラメータの対応付け**

二つの曲線の交差集合は、交点または重なる区間から成ります。その各連結成分を、曲線上のパラメータの値と端点に対応付けます。

**型付きの失敗**

交点や重なる区間の対応付けが不完全な場合は、型付きの失敗を返します。面と穴の境界は、向きを付けた曲線の区間を順に辿った列として求めます。この列が出発点に戻らない場合の失敗は `NonClosedWalk` で、穴を囲む外周に穴自身を指定した場合の失敗は `ContainingOuterWalkSelfReference` です。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-arrangement2d = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-arrangement2d = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-geometry`](https://docs.rs/neco-geometry): 曲線の局所座標系 ( chart ) と交差集合
* [`neco-calculus`](https://docs.rs/neco-calculus): 検証済みの評価と、根の検証結果

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
