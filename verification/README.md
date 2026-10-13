# CAD 演算の検証

この検証では、CAD の公開 API が返した値が数学的に正しいかを、Lean で書いた検査器で確かめます。
検査器が受理した値について各性質が成り立つことは、検査器の Lean プロジェクトで定理として証明しています。
本書の手順に従えば、この検証を手元で再現できます。
以下では、公開 API に渡した入力と、そこから得た値の組を「観測」と呼びます。

検証には、三つのプログラムを使います。観測生成器が公開 API を呼んで観測を作り、検査器がその観測に判定値 ( verdict ) を付け、照合器が要求、観測、判定値の対応を確かめます。

- 観測生成器 ( producer ): `verification/rust/`
- 検査器 ( checker ): `verification/lean/`
- 照合器 ( observer ): `verification/observer/`。公開メッシュ API を直接呼ぶメッシュ計測器 ( control probe ) も、同じパッケージから構築する

## 検証の対象と判定値

- 厳密な局所座標系 ( `exact-chart` ): 公開 API が構成した面の局所座標系 ( chart ) の点が、すべて面の原子式 ( 面の領域を定める関数の符号条件 ) を満たすこと
- 面の曲面被覆 ( `face-atom-covering` ): 面の曲面被覆が、原子式の定める面を覆うこと
- 曲面のメッシュ表現 ( `finite-surface-representation` ): 公開 API が返すメッシュの各頂点が、宣言した残差の上界の範囲で曲面に載ること
- 弦誤差の上界 ( `certified-chord-bound` ): 局所座標系のパラメータ平面で線分または円弧として表した三角形の各辺について、その辺を曲面へ写した曲線の各点と、両端点を結ぶ弦との偏差が、公開 API の返す上界に収まること
- メッシュの検証結果 ( `mesh-judgment` ): 公開 API が返す、閉じた向き付きの多様体であるという結果と、棄却の分類が正しいこと

面、原子式、曲面被覆の対応は、公開 API の識別子で取ります。

- 面: `Face`
- 原子式: `atom`
- 曲面被覆: `covering`

判定値は、verified、refuted、undetermined、invalid の四つです。検査結果には、バッチ全体の判定値か、観測ごとの判定値の列を記録します。この列の各要素は、それぞれの観測に対応します。また、一つの生成要求と、それに対応する観測および判定値の組をケースと呼び、各ケースは空でない文字列のケース識別子で識別します。

## 再現手順

CAD と、検証が依存する四つのリポジトリを `git clone` で取得し、同じ親ディレクトリ `<root>` の下に並べます。

```text
<root>/neco-lean/neco-lean-main
<root>/neco-algebra/neco-algebra-main
<root>/neco-calculus/neco-calculus-main
<root>/neco-geometry/neco-geometry-main
<root>/neco-cad/neco-cad-main
```

検査器と観測生成器は、この配置を相対パスで参照します。そのため、コマンドは `<root>/neco-cad/neco-cad-main` で実行します。実行には、シェル、Cargo、Lake が必要です。

検査器の Lean プロジェクトを構築する前に、mathlib の構築済みの成果物を一度だけ取得します。

```sh
(cd verification/lean && lake exe cache get)
```

### 一括実行

観測の生成、検査器の構築、検査器による検査、照合器による照合を、次のスクリプトが順に実行します。

```sh
verification/scripts/run-cad-verification.sh [REQUEST]
```

引数 `REQUEST` を省いた場合の要求と、生成物の保存先は、スクリプトの中で決まっています。

- 既定の要求: `verification/vectors/requests/cad-request-batch.json`
- 生成物の保存先: `verification/artifacts/`

### 個別に実行

各段階は、次のコマンドで個別に実行できます。

```sh
cargo run --quiet --manifest-path verification/rust/Cargo.toml -p cad-observation-producer -- \
  generate --request verification/vectors/requests/cad-request-batch.json > batch.json
(cd verification/lean && lake build)
verification/lean/.lake/build/bin/neco-cad-checker check batch.json > checker-result.json
cargo run --quiet --manifest-path verification/observer/Cargo.toml -- \
  --batch batch.json --checker-result checker-result.json
```

段階ごとの検査に使うスクリプトもあります。

- `verification/scripts/verify-lean.sh`: 検査器の構築と既知観測の検査
- `verification/scripts/verify-rust-producer.sh`: 観測生成器のテスト
- `verification/scripts/verify-observer.sh`: 照合器のテスト

### 7 種類の検査系統

7 種類の検査系統 ( lane ) は、論理宣言とホスト設定を `run-lanes` に渡して実行します。
論理宣言 ( logical manifest ) は、ケースの集合、生成の規則、検査系統を識別子と版で宣言した JSON です。
実行環境のパスは、ホスト設定の側に書きます。
実行の前には、観測生成器、照合器とメッシュ計測器、検査器の構築が必要です。

```sh
cargo build --manifest-path verification/rust/Cargo.toml -p cad-observation-producer
cargo build --manifest-path verification/observer/Cargo.toml
(cd verification/lean && lake build)
```

ホスト設定は JSON のファイルで、照合器は正規の形のファイルだけを読み込みます。ファイルの中のパスにも、書き方の決まりがあります。

- 正規の JSON: キーを辞書順に並べ、空白と改行 ( 末尾の改行を含む ) を除いた形
- 作業ディレクトリ: 実在する絶対パスで書く
- 他のパス: 相対パスは作業ディレクトリを起点に解決する。この手順では絶対パスで書く

ホスト設定の例では、構築した観測生成器、検査器、メッシュ計測器のパスを指定しています。`<root>` は、実際の絶対パスに置き換えて使う部分です。

```json
{"controlProbePath":"<root>/neco-cad/neco-cad-main/verification/observer/target/debug/neco-cad-control-probe","domains":{"neco.cad":{"checkerPath":"<root>/neco-cad/neco-cad-main/verification/lean/.lake/build/bin/neco-cad-checker","producerPath":"<root>/neco-cad/neco-cad-main/verification/rust/target/debug/cad-observation-producer"}},"reportDirectory":"<root>/neco-cad/neco-cad-main/verification/artifacts/reports","temporaryDirectory":"<root>/neco-cad/neco-cad-main/verification/artifacts/temporary","workingDirectory":"<root>/neco-cad/neco-cad-main"}
```

検査系統の実行では、照合器に同梱された小規模の論理宣言 `smoke.json` を使い、`H` にはホスト設定のファイルのパスを指定します。

```sh
verification/observer/target/debug/neco-cad-observer run-lanes \
  --logical-manifest verification/observer/manifests/smoke.json \
  --host-execution-config H \
  --lanes observationVerification,soundnessTheorem,publicApiParity,mutationRejection,invalidProtocol,corpusExecution,repeatability \
  --require-shared-normal-corpus --require-structured-process-failures
```

`--lanes` に並べた七つの値は、それぞれ異なる性質を確かめる検査系統の名前です。

- `observationVerification`: 検査器の判定値と照合器の結果の一致
- `soundnessTheorem`: 検査器の `soundness` コマンドを起動し、その出力とダイジェストを記録すること
- `publicApiParity`: 公開 API の再実行との一致
- `mutationRejection`: 改変した観測の棄却
- `invalidProtocol`: 不正な入力の拒否
- `corpusExecution`: 宣言したケースの集合の生成と、その実行
- `repeatability`: 2 回の順次実行の結果の一致

報告は `reportDirectory/lanes.json` に保存され、検査に失敗した場合、コマンドは非ゼロの終了コードで終了します。ホスト設定と報告の形式は、[照合器の説明](observer/README.md) にあります。
