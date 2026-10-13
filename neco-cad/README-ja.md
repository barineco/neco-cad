# neco-cad

[English](README.md)

`neco-cad` は、代数的領域を起点に、境界表現、面分割、メッシュなどの CAD の形状データを構成する、CAD エンジンの公開 API です。

## 主要機能

**領域からの構成**

領域の境界を、頂点、辺、面にあたる 0 次元、1 次元、2 次元の部分に分け、そこから境界表現の頂点、辺、面を導出します。

**ブール演算**

二つの領域の原子式を合わせ、指定した論理式で新しい領域を作ります。立体どうしの演算では、両方の入力について、領域の種類が閉じた立体 ( `RegularClosedVolume` ) であることを確かめ、結果の領域の種類も閉じた立体にします。

**結果の来歴**

結果は、構築要求と、その導出の過程を保持します。

## 公開 API の処理の流れ

エンジンは、構築要求から証明付きの領域まで、値の所有権を次の順に移します。

```text
ConstructionRequest
  -> ConstructionResult::into_outputs
  -> derive_complete_boundary
  -> derive_analytic_brep
  -> DerivedAnalyticBrep::into_adaptive_tessellation_request
  -> tessellate_adaptive
  -> AdaptiveTessellation::into_closed_oriented_surface_mesh
  -> region_from_mesh
  -> CertifiedRegion
```

`derive_analytic_brep` が返す `DerivedAnalyticBrep` は、境界表現と各面の曲面の情報をまとめた一つの値です。

* 解析的な境界表現
* 各面 ( Face ) の厳密な曲面を表す原子式 ( atom、関数の符号条件 )
* 各面の曲面の被覆
* 境界表現の導出元の版 ( source revision、呼び出し側が構築要求に付ける版の文字列 )

メッシュの識別子 ( source identity ) は、適応面分割の要求で呼び出し側が付ける、生成メッシュの名前です。メッシュから取り込み直した領域は、この識別子と導出元の版の両方を保持します。

## 現在の制限

### 曲面の被覆

厳密な曲面を導出できる面は、平面と球面です。導出に対応する形状は、平面と球面そのものと、一部の立体の面のうち平面または球面であるものに限ります。

* 一般のアフィン平面
* 一般の中心と半径を持つ球面
* 直方体の面 ( 平面 )
* 線分から成る輪郭を押し出した立体の、平面である面
* 内部が交わらない球の和集合 ( 互いに離れているもの、または外接するもの )

上の一覧に無い曲面と、曲線によるトリムを必要とする曲面には、分類済みの失敗 `UnsupportedSurfaceCovering` を返します。曲面の種類のほかに、ブール演算の種類によっても、入力は未対応になります。

* 円柱、円錐、一般の楕円体、トーラス
* 一般の回転体、重なり合う球どうしのブール演算

### 境界の導出

境界の導出では、有界な整数格子の点の中から、境界の候補を求める起点となる点 ( 以下、境界の種 ) を探索します。
境界の導出に固有の失敗 ( `BoundaryDerivationCondition` ) は、導出の処理の順に判定します。

1. 空の領域: 領域に原子式が無い場合、または原子式を組み合わせて領域を定める論理式が、正規化すると偽 ( `False` ) になる場合は `EmptyRegion`
2. 走査: 原子式を一つずつ走査し、有理関数の符号条件で定めた原子式に達した時点で `SeamUnsupported`。走査中に格子点が領域に属するかを判定できない場合は `IncompleteCoverage`
3. 特異点: 走査の後、ある原子式の格子上の零点に特異点 ( 勾配が 0 になる点 ) が含まれ、その原子式から境界の種が見つからなければ `SingularityUnsupported`
4. 境界の種: 境界の種が一つも見つからなければ `IncompleteCoverage`
5. 閉じた立体の境界: 閉じた立体の領域で、いずれかの原子式について、局所的な証明が付き、かつその原子式に由来する候補を含む境界の部分が一つも無ければ `IncompleteCoverage`

### 構築操作

構築要求の型は、スイープ、ロフト、フィレット、面取りを表せます。ただし現在の実装は、これらの要求に分類済みの失敗を返します。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-cad = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-cad = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-brep`](https://docs.rs/neco-brep): 解析的な境界表現
* [`neco-geometry`](https://docs.rs/neco-geometry): 代数的領域と、境界の次元ごとの分割
* [`neco-calculus`](https://docs.rs/neco-calculus): 検証済みの評価

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
