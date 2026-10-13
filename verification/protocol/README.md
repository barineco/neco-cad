# CAD 演算の生成要求と観測形式

本書は、CAD の検証で使う生成要求、観測、検査結果、照合結果の形式を定めます。これらの形式に従えば、要求から観測を生成し、その観測を Lean の検査器と公開 API の再実行で確かめることができます。観測バッチの内容は、検査器が判定値 ( verdict ) を付けるまでは未検証の主張として扱い、判定値は検査結果に記録します。本文では日本語の呼び名を使い、コードブロックと表には対応する英字の識別子と値を示します。

検証に使う三つのプログラムは、`verification/` の下のそれぞれのディレクトリにあります。観測は、版を付けた一つの形式に従います。

- 観測生成器 ( producer ): `verification/rust/`
- Lean で検査する検査器 ( checker ): `verification/lean/`
- 結果を照合する照合器 ( observer ): `verification/observer/`
- 観測の形式: `neco.cad.observation-batch.v1`

## コマンドライン

観測生成器には、記述子の出力、観測の生成、公開 API の再実行による照合の三つのコマンドがあります。ファイルのパスの位置に `-` を渡すと、標準入力から読み込みます。

```text
cad-observation-producer describe
cad-observation-producer generate --request <request-path|->
cad-observation-producer self-check --request <request-path|-> --batch <batch-path|->
```

出力は、オブジェクトのキーを辞書順に並べ、空白と末尾の改行を除いた UTF-8 の JSON です。

| 状態 | 出力 | 終了コード |
|---|---|---:|
| 記述子の出力、観測の生成の成功 | 標準出力の記述子 ( descriptor ) または観測バッチ | 0 |
| 公開 API の再実行と一致 | 標準出力の verified=true の報告 | 0 |
| 再実行との不一致 | 標準出力の verified=false の報告 | 1 |
| 要求、資源、呼び出し、エンジンの失敗 | 標準エラーの adapter-failure JSON | 2 |

照合器には五つの呼び出し方があり、メッシュ計測器は `mesh` コマンドで呼び出します。

```text
neco-cad-observer --batch B --checker-result K
neco-cad-observer generate --logical-manifest L --host-execution-config H --shard-size 200 --out OUT
neco-cad-observer verify-coverage --logical-manifest L --host-execution-config H --require-descriptor-buckets --require-selector-rules --require-shard-at-most-producer-max
neco-cad-observer run-lanes --logical-manifest L --host-execution-config H --lanes observationVerification,soundnessTheorem,publicApiParity,mutationRejection,invalidProtocol,corpusExecution,repeatability --require-shared-normal-corpus --require-structured-process-failures
neco-cad-observer compare-repeatability --logical-manifest L --host-execution-config H --runs 2 --sequential --shard-size 200 --write-report REPORT
neco-cad-control-probe mesh --input M
```

論理宣言、ホスト設定、検査系統 ( lane ) の一つである soundnessTheorem の記録、集合ごとの結果と終了状態は、[照合器の説明](../observer/README.md) にあります。7 種類の検査系統は正常なケースに適用し、型付き失敗とメッシュの検査可能な棄却は、別の集合として集計します。

## 生成要求

```text
schema: neco.verification.generated-request-batch.v1
domain: neco.cad
adapterRevision: cad-generated-request-v2
generator: { algorithm, drawScheduleRevision, seed }
cases: [ case, ... ]
case: { caseId, domainCaseIndex, family, globalCaseIndex, requestedObservationKind, selectors }
generator.algorithm: splitmix64-v1
generator.drawScheduleRevision: draw-schedule-v1
```

乱数の種、選択値、二つの番号は、u64 の値を小文字の 16 進数 16 桁で表した文字列です。二つの番号は、分野内の番号 domainCaseIndex と全体の通し番号 globalCaseIndex で、バッチ内でそれぞれ一意とします。観測生成器は、ケースが 1 件から 200 件までのバッチを受理し、余分な項目、重複したキー、欠けた選択値、識別子の不一致、正規でないバイト列、誤った生成メタデータを含む要求は拒否します。ケース識別子は、`case-` と通し番号を連結したものです。

選択値は、次に並べた名前の順に疑似乱数で求めます。要求を作る側は、乱数の種とこの共通の順序に従って選択値を求め、要求に書き込みます。観測生成器は、要求に明示された選択値を、公開 API に渡す要求 ( 以下、公開要求 ) の値へ変換します。本書では、この変換を投影と呼びます。

```text
value
familyParameter
observationClass
sign
multiplicity
intervalWidth
formulaVariant
mutation
reserved
```

各集合 ( 正常な集合と二つの失敗の集合 ) の要求は、乱数の種 `9e3779b97f4a7c15` から生成し、決まった件数ずつバッチに分けます。
ある集合の件数が 1 バッチの件数で割り切れない場合、その集合の最後のバッチの中身は、余りの件数の要求だけです。

1 バッチの件数は 1 から 200 までの値で、指定の方法は照合器のコマンドによって異なります。

- `generate`: オプション `--shard-size` の値
- `compare-repeatability`: オプション `--shard-size` の値
- `run-lanes`: 論理宣言の `generator.shardCaseCount`
- `verify-coverage`: 論理宣言の `generator.shardCaseCount`

区分 ( bucket ) は、選択値を決まった数で割った余りの値ごとに分けた、選択値のまとまりです。区分の名前と条件を定める項目は、投影と区分の宣言 ( `../manifest/cad-projection-declaration.json` ) の `selectorBindings` です。各集合の件数は、論理宣言で件数を明示しない場合、その集合の演算族ごとに、すべての区分が 1 件以上現れる最小の通し番号の範囲から導きます。各実行の報告には、実行した件数と判定値を記録します。

## 五つの演算族と選択値

- `exact-chart`: 球、または直方体の平面の面を、公開 API で実際に構成する
- `face-atom-covering`: 球の二つの局所座標系 ( chart ) と、被覆の証拠を取り出す
- `finite-surface-representation`: 公開 API が返すメッシュと、厳密な残差を取り出す
- `certified-chord-bound`: 局所座標系のパラメータ平面で線分または円弧として表した三角形の各辺と、その辺の全区間について公開 API が返す弦誤差の上界を取り出す
- `mesh-judgment`: 公開 API によるメッシュの検証結果と、分類済みの失敗を取り出す

正常な観測が記録する観測種別 ( kind ) は、演算族ごとに決まった値です。

| 演算族 | 正常な観測の種別 ( kind ) |
|---|---|
| exact-chart | exactChart |
| face-atom-covering | faceAtomCovering |
| finite-surface-representation | finiteSurfaceRepresentation |
| certified-chord-bound | certifiedChordBound |
| mesh-judgment | meshJudgment |

選択値から公開要求への投影は、選択値と演算族の組ごとに規則が異なります。投影の対象外の選択値と、予約した乱数の値 ( draw ) も、生成宣言に記録しています。

| 選択値 | 演算族 | 公開要求への投影 |
|---|---|---|
| value | 五つの演算族 | 元の 16 進数の値を版に使う |
| value | exact-chart | 後述の整数の中心を選ぶ |
| value | mesh-judgment | ( 4 で割った余り ) / 2 を x 方向の移動量の絶対値にする |
| familyParameter | exact-chart | 3 で割った余りで、平面と、球の二つの局所座標系 ( 南極からの立体射影、北極からの立体射影 ) のどれかを選ぶ |
| observationClass | exact-chart | 失敗要求では、2 で割った余りで、倍率が 0 の場合と、境界の種 ( 境界の候補を求める起点の点 ) が無い場合のどちらかを選ぶ |
| observationClass | mesh-judgment | 失敗要求では、3 で割った余りで三つの分類を選ぶ |
| sign | exact-chart | 整数の中心の符号。偶数が正、奇数が負 |
| sign | mesh-judgment | x 方向の移動量の符号。偶数が正、奇数が負 |
| multiplicity | exact-chart | 成功要求では、後述の四つの倍率の組から選ぶ |
| multiplicity | mesh-judgment | 倍率を 1 + ( 4 で割った余り ) / 4 とする |
| multiplicity | finite-surface-representation、certified-chord-bound | 曲面残差の上限を 1 + ( 2 で割った余り ) / 2 とする |
| intervalWidth | finite-surface-representation、certified-chord-bound | 弦誤差の上限を 0.5 + ( 2 で割った余り ) / 4 とする |
| formulaVariant | face-atom-covering | 被覆の最初のパッチを、2 で割った余りで選ぶ |
| formulaVariant | certified-chord-bound | 辺の順序を、2 で割った余りで選ぶ |
| mutation | 五つの演算族 | 照合器の改変による検査のために予約した選択値。投影の対象外 |
| reserved | 五つの演算族 | 予約した乱数の値。生成要求に記録するだけの値 |

`exact-chart` の演算族では、球または直方体を構築して変換し、解析的な境界表現を導出します。
また、`face-atom-covering` と `finite-surface-representation` の演算族では、単位球について公開 API を呼び、それぞれ被覆の証拠と、実際の頂点の厳密な残差を取り出します。

選択値 formulaVariant で辺を逆順にする場合は、三角形が参照する辺の番号も、逆順にした辺の並びに合わせてすべて付け替えます。要求値と、公開 API が返す証明付きの上界は、別々に保持する値です。

版の書式、局所座標系の選び方、区間の分割の深さは、固定の設定で決まります。

```text
sourceRevision: cad-<domainCaseIndex hex16>-<value hex16>
exact-chart familyParameter % 3:
  0: plane
  1: sphere-south
  2: sphere-north
API pipeline: construct -> derive_complete_boundary -> derive_analytic_brep
exact-chart transform API: OperationRequest::Transform
subdivisionDepth.line (maximum depth): 4
subdivisionDepth.arc (maximum depth): 8
adaptive enclosureBits: 48
```

`sphere-south` は、南極からの立体射影による球の局所座標系で、パラメータの単位円板は北半球に対応します。`sphere-north` は、北極からの立体射影による局所座標系で、単位円板は南半球に対応します。

弦の偏差の上界は、辺のパラメータ区間から始める適応的な二分法で検査します。`subdivisionDepth` は、この二分法の深さを次のように制限する値です。

- 値: 二分の最大の深さ
- 受理: 三軸の偏差の平方和の上界が、辺の上界以下になった区間
- 細分: 受理の条件を満たさない区間を二分する
- 判定値: 最大の深さに達しても、受理の条件を満たさない区間が残れば refuted

生成の規則、投影と区分、資源と上限、CAD の意味論宣言は、機械で読み取る宣言ファイルとして用意しています。

- 生成の規則: `../manifest/cad-generated-request-declaration.json`
- 投影と区分: `../manifest/cad-projection-declaration.json`
- 資源と上限: `../manifest/cad-resource-declaration.json`
- CAD の意味論宣言: `../manifest/cad-semantics.json`

## 正常な要求を公開 API に適用する条件

厳密な局所座標系の成功要求は、公開 API の境界導出が行う整数格子の探索に合わせて選んだ、要求の集合です。倍率の選択値 multiplicity から、倍率 scale と、それを打ち消す形状の大きさ primitiveExtent の組 ( 以下、倍率の組 ) を下の表から選び、指定した値を公開コンストラクタで厳密な二進のスカラーに変換します。

| multiplicity mod 4 | scale | primitiveExtent |
|---:|---:|---:|
| 0 | 1 | 1 |
| 1 | 2 | 0.5 |
| 2 | 4 | 0.25 |
| 3 | 8 | 0.125 |

球の半径には primitiveExtent の値を、直方体の辺の長さにはその 2 倍を渡します。その形状に倍率 scale の変換を公開 API で適用するため、変換後の半径または辺の長さの半分は 1 になります。観測値は、構築、変換、境界導出の公開 API から得た面の結果から取り出します。

中心は選択値 value を 4 で割った余りで選び、形状は公開 API の構築と変換で作ります。符号は 0 でないすべての座標に同じ値を使い、中心の座標の絶対値は最大で 2 です。

```text
value % 4:
  0: (0,0,0)
  1: (+/-1,0,0)
  2: (+/-2,0,0)
  3: (+/-1,+/-1,0)
primitiveExtent: JSON number in {1,0.5,0.25,0.125}
scalar API: ExactScalar::new(f64)
sphere radius: primitiveExtent
box lengths: [2*primitiveExtent; 3]
transform API: OperationRequest::Transform
observed Face fields: atom, chart, domain
```

中心を $c$、座標軸の単位ベクトルを $e$ とすると、球の極と直方体の各面の中心 $c \pm e$ は、探索する格子の中の整数点です。境界点から軸の正負の向きへ 1 ずつ動かした 2 点は、一方が内部、他方が外部になります。検査器は、入力の primitiveExtent を有理数として読み、表の四つの値のどれかであることを確かめます。

観測生成器の記述子 ( descriptor ) は、厳密な局所座標系について、正常な集合の適用条件と、選択値ごとに要求する観測の種類を宣言します。

```text
projectionClaim.projectionDeclarations[].normalApplicability:
  boundarySeedLattice: { lower: -3, upper: 3 }
  maximumAbsoluteCenterCoordinate: 2
  primitiveExtentByMultiplicityRemainder: [1,0.5,0.25,0.125]
  scaleByMultiplicityRemainder: [1,2,4,8]
  seedCondition: string
  sideSampleOffsets: [-1,1]
  worldExtent: 1
selectorBindings[].requestedObservationKinds: optional ["success"|"failure", ...]
```

要求する観測の種類を宣言した選択値の網羅 ( coverage ) は、その種類の実際の要求で確かめます。選択値ごとの宣言を省略した場合に使う種類は、演算族全体に宣言した種類です。

## 正常な観測と分類済みの失敗

要求する観測の種類 ( requestedObservationKind ) は、success と failure の二つです。
failure の要求は、`exact-chart` と `mesh-judgment` の二つの演算族だけが受理します。
生成した観測は、正常な集合、未確定の失敗、検査可能なメッシュの棄却の三つの集合に分けて集計します。
未確定の失敗は、判定値が未確定 ( undetermined ) になる型付き失敗です。

| 集合 | 要求 | 観測と数学的な対象 |
|---|---|---|
| normal | 五つの演算族の success | 正常な公開観測の主張 |
| 未確定の失敗 | exact-chart の failure | 倍率 0 による公開 API の構築の拒否、または整数格子に境界の種を持たない球に対する境界導出の拒否。判定値は undetermined |
| 検査可能なメッシュの棄却 | mesh-judgment の failure | 公開 API による棄却の分類が正しいという主張 |

公開 API の演算が返した分類が、宣言した条件と一致する場合に限り、型付き失敗を記録し、実際の要求の識別子と入力を保存します。未確定の失敗の記録には、失敗の種類と理由に加えて、失敗した操作、入力、破った条件を来歴として残します。

```text
publicObservation.kind: typedFailure
publicObservation.value: { failureKind, provenance, reason }
provenance: { caseId, failingOperation, input, violatedCondition }
failureKind: engineRejection
failingOperation: construct
violatedCondition: transform.scale must be positive

exact-chart observationClass % 2:
  0: primitiveExtent=1, scale=0
     failureKind=engineRejection
     failingOperation=construct
     violatedCondition=transform.scale must be positive
  1: sphere-south, primitiveExtent=1, scale=1.25, translation=(-0.5,0,0)
     failureKind=boundaryRejection
     failingOperation=derive_complete_boundary
     violatedCondition=no exact boundary seed in integer lattice [-3,3]^3
reason: actual API failure value, string
```

メッシュについては、公開 API が返す三つの分類を記録します。分類の選択値 observationClass の適用先は失敗要求だけで、厳密な局所座標系の倍率の選択値 multiplicity の適用先は成功要求です。

| observationClass mod 3 | 入力の変化 | 公開 API の分類 |
|---:|---|---|
| 0 | 最後の三角形の除去 | openEdge |
| 1 | 最初の三角形の向きの反転 | orientationMismatch |
| 2 | 頂点 0 と 1 を同じ位置にする | degenerateTriangle |

検査可能な棄却の判定値が verified であることと、メッシュが正しい閉じた立体であることは、別の主張です。たとえば開いた辺の検査では、宣言した無向辺に接する三角形の数が 1 であることを確かめます。棄却と未確定の失敗は、正常な観測の件数には加えません。

## 観測生成器の記述子と資源

`describe` コマンドは、意味論、投影、資源の宣言と、実際のビルドの識別子を、記述子として出力します。記述子のフィールドは、いずれも必須です。

```text
schema: neco.verification.producer-description.v1
domain: neco.cad
semanticsRef: { declarationDigest, revision }
producerApiRevision: cad-producer-api-v1
adapterRevision: cad-generated-request-v2
rustcIdentity: <actual build compiler rustc -vV>
targetTriple: <Cargo TARGET>
projectionClaim: { projectionRevision, projectionDeclarationDigest, projectionDeclarations }
resourceClaim: { resourceRevision, resourceDeclarationDigest, maxBatchCases }
semanticsRef.revision: cad-semantics-v2
resourceClaim.resourceRevision: cad-resource-v2
```

意味論宣言と資源宣言のダイジェストは、宣言全体の正規のバイト列の SHA-256 です。投影の版とダイジェストは、どちらも、五つの演算族を列挙した配列だけの正規のバイト列から求めた SHA-256 です。宣言とバイナリを照合する側は、各演算族が一意であることも確かめます。

四つの分野に共通する意味論宣言のフィールドは、neco-algebra で定義しています。CAD の宣言 `verification/manifest/cad-semantics.json` は、CAD が持つモジュールと、直接の上流の意味論を列挙します。

```text
schema: neco.verification.semantics-declaration.v2
domain: neco.cad
revision, cadCheckerApiRevision, cadModuleRevision
ownedModules[]: { module, sourcePath }
upstreamSemantics[]: { domain, revision, sourcePath }
CAD direct upstream: neco.algebra, neco.calculus, neco.geometry
```

上流の意味は、Lean の依存関係と定理が表します。これに対し、CAD の検査が使うモジュールの一覧の原典は、宣言の `ownedModules` だけです。

宣言ファイルの実際のバイト列の SHA-256 は、観測バッチの参照と、論理宣言の意味論の参照に記録します。宣言を読み取る Lean のモジュールは、コンパイル時にファイルから実際の宣言を読み取り、そこから派生する参照を計算します。

資源宣言は、五つの演算族の構成の上限と、次の実行予算を定めます。観測生成器はバッチの件数の上限を検査し、時間、メモリ、出力量についてのプロセスの制限は、観測生成器を起動する実行環境が設定します。

```text
maxBatchCases: 200
cpuTimeSeconds: 3600
memoryBytes: 8589934592
stdoutPayloadBytes: 536870912
timeoutSeconds: 7200
```

これらの値は要求する予算であり、測定した資源の使用量とは別の値です。

## CAD の観測バッチと参照

```text
schema: neco.cad.observation-batch.v1
root: { cases, proofSubjectRef, schema, testSetRef }
case: { caseId, input, operationFamily, publicObservation }
publicObservation: { kind, value }
versionedRef: { contentDigest, versionIdentity }
```

演算族は、観測 1 件につき 1 回だけ記録し、入力には、選択値を投影した公開要求を記録します。参照のダイジェストは、SHA-256 を小文字の 16 進数 64 桁で表したもので、対応する宣言の正規のバイト列から求めた値と一致させます。

証明対象は、証明すべき五つの命題 ( obligations ) を定め、CAD の意味論宣言を参照します。検査集合は、既知観測の 6 件のケースを、各ケースに期待する判定値と証明すべき命題とともに宣言したファイルです。検査集合は、証明対象、遷移分類、生成要求の宣言を参照します。遷移分類は、ケースが通りうる処理の種類と、その結果の分類の組を列挙した宣言です。検査集合の各ケースは、この組の一つを指定します。参照の向きは、検査集合から証明対象へ、証明対象から意味論宣言へという一方向です。

実行したソースの OID とバイナリのダイジェストは、実行報告に記録します。一方、CAD が持つモジュールの一覧と、直接の上流の意味論の版とパスは、意味論宣言に記録します。

## 公開 API との一致

再実行による照合のスキーマは `neco.verification.public-api-parity-result.v1` で、表のフィールドはいずれも必須です。

| 位置 | フィールド | 型 |
|---|---|---|
| root | schema | 上記の定数の文字列 |
| root | verified | boolean |
| root | requestDigest/observationDigest | SHA-256 hex64 |
| root | cases | 件数と順序が要求と同じ配列 |
| case | caseId | 要求の caseId |
| case | verified | boolean |
| case | replayedObservationDigest/suppliedObservationDigest | 観測 1 件全体の正規のバイト列の SHA-256 hex64 |

同じ要求を再び公開 API に渡し、得られた観測を、渡されたバッチと比較します。比較では、ケース識別子、投影した公開要求、演算族、観測値の全体を照らし合わせます。バッチ全体の一致は、観測ごとの一致の論理積です。

## 検査器の結果

検査器は、バッチ全体の判定値か、観測ごとの判定値の配列を返します。

```text
{"batchVerdict":"invalid","digest":"<sha256>"}
{"cases":[{"caseId":"<id>","verdict":"<verified|refuted|undetermined|invalid>"}],"digest":"<sha256>"}
```

入力を正規のバッチとして復号できない場合と、識別子が一意でない場合は、バッチ全体の判定値 `batchVerdict` を返します。観測ごとの判定値を導ける場合は、その配列 cases を返し、空の識別子と予約値 `__digest__` は受理しません。

照合では、正規のバイト列、形式、全観測の構成、識別子の一意性、検査結果のダイジェスト、ケース識別子の件数と順序の一致、四つの判定値を検査します。ケースを列挙できた正規のバッチに対して、検査器がバッチ全体を invalid と主張した場合は、照合器が矛盾として拒否します。数学的な反証とプロセスの失敗は、別々に記録します。
