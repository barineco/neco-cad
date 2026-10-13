# CAD 観測の照合

照合器 ( observer ) を使うと、CAD の生成要求、公開 API の観測、Lean の検査器の判定値について、三者の件数と順序が一致するかを確かめることができます。
本書では、そのコマンド、入力の形式、7 種類の検査系統 ( lane ) 、報告の形式を説明します。
ここでは、一つの生成要求と、それに対応する観測および判定値の組をケースと呼び、ケースの集まり ( コーパス ) を「集合」と呼びます。
本文では日本語の呼び名を使い、コードブロックには対応する英字の識別子と値を示します。

## コマンド

照合器は、単独の照合、要求の生成、網羅の検査、7 種類の検査系統の実行、2 回の実行の比較を、それぞれ別の呼び出しで行います。その際、観測生成器、検査器、メッシュ計測器 ( control probe ) は、ホスト設定に書いたパスのバイナリから起動します。メッシュ計測器は、`mesh` コマンドを使えば単独でも呼び出すことができます。

```sh
neco-cad-observer --batch B --checker-result K
neco-cad-observer generate --logical-manifest L --host-execution-config H --shard-size 200 --out OUT
neco-cad-observer verify-coverage --logical-manifest L --host-execution-config H \
  --require-descriptor-buckets --require-selector-rules --require-shard-at-most-producer-max
neco-cad-observer run-lanes --logical-manifest L --host-execution-config H \
  --lanes observationVerification,soundnessTheorem,publicApiParity,mutationRejection,invalidProtocol,corpusExecution,repeatability \
  --require-shared-normal-corpus --require-structured-process-failures
neco-cad-observer compare-repeatability --logical-manifest L --host-execution-config H \
  --runs 2 --sequential --shard-size 200 --write-report REPORT
neco-cad-control-probe mesh --input M
```

`--require-` で始まるフラグは省略できます。照合器は、指定の有無にかかわらず、これらのフラグが表す要件を常に検査します。引数の不足、未知のフラグ、フラグの重複、および 2 回の順次実行以外の指定は、エラーになります。

## 正規のバイト列と判定値

入力の JSON は、UTF-8 で符号化され、オブジェクトのキーが辞書順に並び、空白と末尾の改行を除いた形式に限ります。重複したキーは復号時に拒否し、復号した値を再び符号化した結果が元のバイト列全体と一致することも検査します。検査器のコマンドの出力の末尾に改行が一つある場合は、その改行を出力の区切りとみなして取り除き、保存する結果を正規の JSON にします。

照合では、バッチ、ケース、観測、結果のそれぞれに条件を課します。

- バッチ: スキーマ ( schema ) と参照を検査し、ケースが一つ以上あることを確かめる
- ケース: すべての識別子が空でなく一意であることと、件数と順序の一致を確かめる
- 観測: 演算族と観測種別、型付き失敗の来歴を検査する
- 結果: 項目の集合が厳密に一致することを確かめ、入力のダイジェストと判定値を検査する

ケースを列挙できる正規のバッチに対して、検査器がバッチ全体を無効と主張した場合は、矛盾として拒否します。ケースの列挙に失敗した入力と、列挙したケースのうち無効なものには、別々の分類を付けます。復号器は、重複したキーと正規でないバイト列を別の失敗として区別します。子プロセスの起動、終了、資源に関する失敗についても、照合器が返す値は失敗の種類ごとに別々です。

```text
バッチのスキーマとケース: schema, cases
ケース識別子: caseId
観測種別: kind
判定値 ( verdict ): verified, refuted, undetermined, invalid
列挙可能なバッチへの矛盾した主張: batchVerdict=invalid
列挙不能な入力: batchInvalid
列挙された無効ケース: caseInvalid
重複キー: duplicateKey
非正規バイト列: noncanonicalBytes
実行の失敗: processFailure
ダイジェストの方式: SHA-256
```

型付き失敗は、判定値が未確定 ( undetermined ) であり、正常な観測の検証済みの件数とは別に数えます。メッシュの棄却は、検査可能な失敗の主張として、メッシュの棄却の集合 ( `cad-mesh-rejected` ) の検証済みの件数に記録します。

```text
型付き失敗の判定値: undetermined
正常観測の検証済み件数: verified
メッシュの棄却: rejected
検査可能な失敗の主張の判定値: verified
```

## 論理宣言

論理宣言 ( logical manifest ) は、ケースの集合、生成の規則、検査系統を識別子と版で宣言した JSON です。実行環境のパスは、ホスト設定の側に書きます。論理宣言は、七つの必須項目と、CAD のケースの集合を参照する項目から成ります。

```text
schema: neco.verification.logical-manifest.v1
schema, protocolRevision, generator, domains, coverage, lanes, timeoutPolicy, sourceRefs
```

要求の生成器は、ケースごとの通し番号から九つの選択値を生成し、接頭辞と通し番号を連結してケース識別子を作ります。演算族は辞書順に並べ、分割 ( shard ) の大きさを変えても、全ケースの行と選択値は同じになります。

```text
生成方式: splitmix64-v1
選択値を引く順序の版: draw-schedule-v1
通し番号: globalCaseIndex
ケース識別子: caseId = case- + hex16(globalCaseIndex)
選択値: selector、各ケースに九つ
```

区分 ( bucket ) は、選択値を決まった数で割った余りの値ごとに分けた、選択値のまとまりです。
区分の名前と条件は、投影と区分の宣言 ( `verification/manifest/cad-projection-declaration.json` ) の `selectorBindings` に書かれています。正常なケースの既定の件数は、各演算族のすべての区分が 1 件以上現れる最小の件数です。
この件数は、固定した乱数の種 ( seed ) から得る選択値の列と、観測生成器が宣言する区分から導きます。
また、一つの分割に含めるケースは 200 件までです。
件数を増やす場合は、正常なケースの件数、分野ごとの正常なケースの件数 ( allocation ) 、`cad` 集合の演算族ごとの件数を併せて明示することが必要です。

- `generator.normalCaseCount`
- `generator.allocation`
- `cad` 集合の `familyCounts`

乱数の種には固定の値を使い、各集合は論理宣言の参照項目で宣言します。

```text
seed: 9e3779b97f4a7c15
集合の参照項目: sourceRefs
id, requestedObservationKind, expectedVerdict ( 件数を明示する場合は familyCounts を加える )
```

正常な集合と二つの失敗の集合では、要求、期待する判定値、適用する検査が異なります。

| 集合識別子 | 要求 | 期待する判定値 | 適用する検査 |
|---|---|---|---|
| `cad` | 五つの演算族の成功要求 | verified | 7 種類の検査系統 |
| `cad-typed-failure` | `exact-chart` の型付き失敗の要求 | undetermined | 生成、区分の網羅、公開 API、検査、照合、2 回の一致 |
| `cad-mesh-rejected` | `mesh-judgment` の棄却の要求 | verified | 上の行と同じ六つの検査を、検査可能な失敗の主張に適用 |

論理宣言の `sourceRefs` には、集合ごとに、要求する観測の種類 ( 成功または失敗 ) と、期待する判定値を記録します。
要求する観測の種類は `requestedObservationKind` に、期待する判定値は `expectedVerdict` に書きます。
演算族は、集合識別子から決まります。
下の矢印は、集合ごとの要求と判定値の対応を表す略記です。

```text
cad: success -> verified
cad-typed-failure: exact-chart / failure -> undetermined
cad-mesh-rejected: mesh-judgment / failure -> verified
```

メッシュの棄却の集合は、集合の参照項目に宣言した場合に実行します。要求の生成器は、全ケースの行と各分割の要求のダイジェストを、件数と保存先に対応付けて返します。要求は分割ごとのファイルに保存し、出力は、その保存先とダイジェストを集合ごとにまとめた JSON です。

```text
要求の保存先: OUT/id/shard-NNNN.json
出力形式: neco.cad.generated-corpus.v1
sourceRefs.<id>: directory, count, requestDigest, shards
shards[]: path, caseCount, requestDigest
```

各分野の版とダイジェストは、論理宣言と、観測生成器の `describe` コマンドの出力とで一致させます。アダプタの版 ( adapterRevision ) は、観測生成器が生成要求を公開 API の呼び出しへ変換する規則の版です。意味論の参照に記録するダイジェストは、宣言ファイルの実際のバイト列の SHA-256 です。論理宣言は、論理的な識別子と版だけで構成します。

- CAD 自身: 意味論の参照の `declarationDigest` に、CAD の意味論の宣言のダイジェストを記録
- 上流の三分野 ( neco.algebra、neco.calculus、neco.geometry ) : 意味論の参照の `sharedSemanticsRefs` に、各分野の意味論の宣言のダイジェストと版を記録

```text
アダプタの版: adapterRevision
公開 API の版: producerApiRevision
投影の版: projectionRevision
意味論の参照: semanticsRef
観測生成器の記述子を出力するコマンド: describe
配布する小規模宣言: smoke
論理宣言に含めないソース識別子: source OID
```

## ホスト設定

ホスト設定には、作業ディレクトリ、一時ディレクトリ、報告の保存先、各分野の観測生成器と検査器のパス、メッシュ計測器のパスを必ず書きます。

```text
workingDirectory, temporaryDirectory, reportDirectory, domains, controlProbePath
domains.neco.cad: producerPath, checkerPath
```

1 回の実行の中で同時に処理する分割の数は、ホスト設定で指定できます。省略することも可能です。

```text
shardParallelism: 1 以上の整数 ( 省略時は実行環境が報告する並列度 )
```

結果は分割の順序で集約するため、並列度が異なってもダイジェストと報告のバイト列は一致します。再現性の確認では、2 回の実行を順に行います。

ホスト設定は、キーを辞書順に並べ、空白と末尾の改行を除いた正規の JSON で書きます。作業ディレクトリには実在する絶対パスを明示し、他の相対パスはこのディレクトリを起点に解決します。照合器には実行中の実行ファイル自身を使い、メッシュ計測器のパスは `controlProbePath` の値です。

## 7 種類の検査系統

正常な集合に対する 7 種類の検査系統は、いずれも同じ観測生成器と、同じ要求および観測を使います。生成した件数、宣言した選択値の区分、観測生成器の `describe` の出力にある分割の件数の上限を検査し、すべての正常なケースの判定値が verified であることと、公開 API の再実行による全ケースの行の一致も求めます。

```text
生成と実行: corpusExecution
観測の検証: observationVerification
公開 API の一致: publicApiParity
選択値の区分: selector bucket
分割件数の上限: maxBatchCases
```

検査系統 soundnessTheorem では、検査器の `soundness` コマンドを起動して得た出力と、そのダイジェストを記録します。依存する公理と定理名は、検査器の Lean のビルドが確かめます。

### 改変、不正な入力、繰り返しの検査系統

検査系統 mutationRejection では、五つの演算族のそれぞれについて、数学的に同値でない形へ改変した観測が棄却され、同値な形へ改変した観測の判定値が verified になることを求めます。`finite-surface-representation` の改変では、実際の多項式の残差が上界を超える頂点を厳密に選びます。`certified-chord-bound` の改変では、実際の線分の中点の偏差が正であることを確かめてから、二乗の上界を 0 に変えます。厳密演算が扱える整数の範囲を超える改変は、明示的なエラーになります。

```text
多項式の残差: atom residual
弦の改変: chord
厳密演算の整数範囲: i128
```

`mesh-judgment` の同値な改変では、すべての三角形の頂点の並びを巡回させた入力をメッシュ計測器で再実行し、得られた検査前の主張を同じ検査器で検査します。

検査系統 invalidProtocol では、五つの演算族に、不正なバイト列、重複したキーと識別子、空の識別子、誤ったスキーマと数値を与え、演算族と観測種別の不一致も検査します。誤った結果のダイジェスト、ケースの並べ替え、欠けたケース、未知の判定値、不正な結果のバイト列については、照合器を子プロセスとして実行した時に、照合器が入力を拒否することが合格の条件です。

検査系統 repeatability では、要求、観測、検査器の出力の全ケースの行を正規化した内容で比較し、集合ごとの判定値と失敗の分類を保存します。所要時間とファイルの保存先は、この比較の対象に含めません。

## 報告と終了状態

成功した実行の報告には、ケースの実行と各検査系統の結果を保存し、正常な観測と分類済みの失敗の件数を集合ごとに記録します。

```text
schema: neco.cad.completion-report.v1
ルート項目: logicalManifestDigest, corpusResults, laneResults
corpusExecution / observationVerification / repeatability: status=passed
soundnessTheorem.neco.cad: status=passed, outputDigest, text
publicApiParity.CORPUS-SHARD: verified=true, resultMatched=true, report
mutationRejection.FAMILY: classification observed/expected=refuted,
  originalDigest, mutatedDigest, equivalentDigest, equivalentVerified=true
invalidProtocol: 実際の入力と結果のダイジェスト、分類、拒否の観測を持つ配列
```

`run-lanes` を実行すると、7 種類の検査系統の報告を `reportDirectory/lanes.json` に保存します。
また、2 回の実行それぞれの要求、観測、検査器の出力は、実行とケースの範囲ごとのファイルに保存します。
保存先は、`compare-repeatability` の報告を除いて `reportDirectory` の下です。

```text
reportDirectory/lanes.json
reportDirectory/first-ID-NNNN-request.json
reportDirectory/first-ID-NNNN-observation.json
reportDirectory/first-ID-NNNN-checker.json
reportDirectory/second-ID-NNNN-request.json
reportDirectory/second-ID-NNNN-observation.json
reportDirectory/second-ID-NNNN-checker.json
compare-repeatability: --write-report で指定したファイル
```

照合器は、成功、検査の失敗、呼び出しや設定の不備を、終了コードで区別します。単独の照合での拒否には、不備と同じ終了コードを返します。

- 0: 成功
- 1: 実際の入力の不一致、または必須の検査の失敗
- 2: コマンド、論理宣言、ホスト設定の不備、または単独の照合での拒否

エラーは、次のスキーマに従って構造化し、標準エラーへ出力します。

```text
error schema: neco.cad.observer-failure.v1
```

## 公開メッシュ API の計測入力

メッシュ計測器は、頂点と三角形から成るメッシュと、その出所 ( source ) を JSON で受け取り、公開 API の結果を JSON で返します。

```text
入力のスキーマ: neco.cad.mesh-api-probe-request.v1
入力の項目: schema, mesh, source
mesh: vertices [{x:dyadic,y:dyadic,z:dyadic}], triangles [[i,j,k]]
dyadic: integer (decimal string), exponent (u64)
source: identity, revision (nonempty strings)
出力のスキーマ: neco.cad.mesh-api-probe-result.v1
出力の項目: schema, inputDigest, mesh, outcome, api
```

メッシュ計測器は、座標として、二進有理数のうち倍精度の浮動小数点数 ( binary64 ) で正確に表せる値を受理します。そのうえで公開メッシュ API を呼び、実際の証拠か分類済みの失敗を返します。返した証拠や失敗の主張が数学的に成り立つかは、Lean の検査器が確かめます。

```text
二進有理数: dyadic
浮動小数点数の表現: binary64
```

## 公開スクリプト

観測の生成から照合までを一度に実行するスクリプトを用意しています。

```sh
verification/scripts/run-cad-verification.sh [REQUEST]
```

- 必要なツール: シェル、Cargo、Lake
- 既定の要求 ( `REQUEST` を省略した場合 ): `verification/vectors/requests/cad-request-batch.json`
- 生成物の保存先: `verification/artifacts/`

スクリプトは観測生成器で観測を生成し、Lean の検査器を構築し、検査器の判定値を照合器に渡して照合します。7 種類の検査系統を実行するには、構築済みの観測生成器、検査器、メッシュ計測器のパスを書いたホスト設定を `run-lanes` に渡します。

## スキーマ

| 対象 | スキーマのファイル |
|---|---|
| 単独の照合 | [cad-reconciliation-result](../protocol/schema/cad-reconciliation-result.schema.json) |
| 7 種類の検査系統の報告と 2 回の一致 | [cad-completion-report](../protocol/schema/cad-completion-report.schema.json) |
| 明示したホスト設定 | [cad-host-execution-config](../protocol/schema/cad-host-execution-config.schema.json) |
| 公開メッシュ API の計測入力 | [cad-mesh-api-probe-request](../protocol/schema/cad-mesh-api-probe-request.schema.json) |
| 公開メッシュ API の観測 | [cad-mesh-api-probe-result](../protocol/schema/cad-mesh-api-probe-result.schema.json) |
| 照合器の構造化エラー | [cad-observer-failure](../protocol/schema/cad-observer-failure.schema.json) |
