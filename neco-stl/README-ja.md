# neco-stl

[English](README.md)

`neco-stl` は、検証済みの曲面メッシュと STL 形式のデータを相互に変換するクレートです。

## 主要機能

**読み込み**

バイナリ形式の STL のバイト列から曲面メッシュを作ります。読み込みの際は、ヘッダに書かれた三角形の数と、バイト列の長さが整合することを検証します。

**書き出し**

検証済みの曲面メッシュは、バイナリ形式と ASCII 形式のどちらの STL としても書き出せます。

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-stl = "0.3"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。

```toml
[dependencies]
neco-stl = { version = "0.3", default-features = false }
```

## 依存クレート

* [`neco-mesh`](https://docs.rs/neco-mesh): 検証済みの曲面メッシュの型

## ライセンス

MIT License ( 詳細はリポジトリの [LICENSE](../LICENSE) を参照 )
