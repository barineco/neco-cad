# neco-tessellation

[English](README.md)

`neco-tessellation` は、解析的な境界表現を、宣言された精度で面分割します。

## 主要機能

**用途ごとの精度**

精度の要求は、面分割の用途 ( 表示、解析、要素の特定 ) を添えて受け取り、達成した精度を証拠として記録します。

**要素の対応**

面分割後の各要素を、元の境界表現の要素に対応付けます。

**メッシュの出力**

出力は、面分割の結果から作る検証済みの曲面メッシュです。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-tessellation = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-tessellation = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-brep`](https://docs.rs/neco-brep): 解析的な境界表現の入力
* [`neco-mesh`](https://docs.rs/neco-mesh): 検証済みの曲面メッシュの出力

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
