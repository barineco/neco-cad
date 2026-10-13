# CAD 演算の検証ベクター

検証ベクターを使うと、観測生成器と検査器の動作を、決まった入力で確かめることができます。要求ベクターは公開 API に渡す要求の例で、既知観測は Lean の検査器に渡す観測の例です。正常な要求と既知観測は正規の JSON で書き、不正入力の例は、拒否されるべき条件を含みます。

- `requests/`: 公開 API へ渡す要求のバッチ
- `known/`: 成り立つべき性質を述べた既知観測
- `invalid/`: 復号と識別子の検査で拒否されるべき入力の例

## 生成要求の例

要求の形式は `neco.verification.generated-request-batch.v1` です。アダプタの版は、観測生成器が生成要求を公開 API の呼び出しへ変換する規則の版で、二つのファイルで同じ値です。

```text
adapterRevision: cad-generated-request-v2
```

| ファイル | 要求の件数 | 対象 |
|---|---:|---|
| requests/cad-request-batch.json | 7 | 厳密な局所座標系の 3 件 ( 平面、南極からの立体射影による球、北極からの立体射影による球 ) と、残る四つの演算族の正常な要求 4 件 |
| requests/cad-failure-request-batch.json | 5 | 判定値 ( verdict ) が未確定になる構築の拒否と境界導出の拒否を各 1 件、検査可能なメッシュの棄却を 3 件 |

これらのファイルは要求の例です。正常な観測の集合を生成する規則は、`verification/manifest/` の生成宣言に書かれています。その件数は、すべての区分 ( bucket ) が現れる件数から導きます。区分は、選択値を決まった数で割った余りの値ごとに分けた、選択値のまとまりです。実行の結果は、各実行の報告に記録されます。厳密な局所座標系の正常な要求は、形状の大きさ ( primitiveExtent ) が倍率 ( scale ) の逆数になる 4 通りの組のいずれか一つと、整数の中心を、公開 API の構築と変換に渡すものに限ります。

観測生成器で実際の観測を生成し、公開 API の再実行と照合するには、リポジトリの最上位のディレクトリで次のコマンドを実行します。

```sh
cad-observation-producer generate --request verification/vectors/requests/cad-request-batch.json > normal-batch.json
cad-observation-producer self-check --request verification/vectors/requests/cad-request-batch.json --batch normal-batch.json
cad-observation-producer generate --request verification/vectors/requests/cad-failure-request-batch.json > failure-batch.json
cad-observation-producer self-check --request verification/vectors/requests/cad-failure-request-batch.json --batch failure-batch.json
```

## 既知の 6 件と失敗の主張

既知観測の形式は `neco.cad.observation-batch.v1` です。既知の 6 件は、五つの正常観測と、開いたメッシュを棄却するという主張 1 件から成ります。生成要求の 7 件と既知の 6 件は別の集合であり、既知のファイルを、実際に生成した観測のバイト列と照合する期待値としては使いません。

開いた辺を棄却する主張は、ケース `cad-mesh-known-broken-001` の観測です。

```text
caseId: cad-mesh-known-broken-001
expectedVerdict: verified
role: checkableRejectedCase
publicObservation.kind: meshJudgment
outcome.kind: rejected
outcome.failureKind: openEdge
outcome.failure: { firstVertex: 1, secondVertex: 2 }
```

この観測は、無向辺 ( 1, 2 ) に接する三角形の数が 1 であることと、棄却の分類が正しいことを主張します。この主張は、メッシュが妥当な閉じた立体であるという主張とは別の主張です。そのため、正常なメッシュの成功件数には加えません。また、数学的に同値でない形へ改変した観測が反証されることは、この既知観測とは別に明示した改変で検査します。

生成要求の失敗の集合では、観測の種類ごとに、検査する対象と期待する判定値が決まっています。

| 観測 | 検査する対象 | 期待する判定値 |
|---|---|---|
| typedFailure/engineRejection | 倍率 0 の変換を公開 API の構築が拒否した観測と来歴 | undetermined |
| typedFailure/boundaryRejection | 半径 1、倍率 1.25、中心 (-0.5, 0, 0) の球が整数格子に境界の種 ( 境界の候補を求める起点の点 ) を持たず、公開 API の境界導出が拒否した観測と来歴 | undetermined |
| meshJudgment/rejected | 開いた辺、向きの不一致、退化した三角形という、公開 API による分類の主張 | verified |

判定値が未確定 ( undetermined ) になる失敗と、検査可能な棄却は、正常な集合とは別に集計します。判定値の根拠は、Lean の検査器の実行結果です。

## 不正入力と参照

不正入力の例には、重複したケース識別子と、空のケース識別子が含まれます。正規の形でない入力も、正規の観測としては受理しません。拒否の理由を一つずつ検査する場合は、正規の形でないことで拒否される例と、識別子の不正で拒否される例を使い分けます。

観測が持つ参照のダイジェストは、次の宣言の正規のバイト列から求めた値です。

- 証明対象: `../manifest/cad-proof-subject.json`
- 検査集合: `../manifest/cad-finite-test-set.json`。既知の 6 件のケースを、各ケースに期待する判定値と証明すべき命題とともに宣言したファイル

参照の値は、SHA-256 を小文字の 16 進数 64 桁で表したものです。宣言を変更した場合は、その宣言を参照する既知観測と不正入力の参照も更新します。要求、宣言、観測、照合結果のフィールドとコマンドラインの形式は、[観測形式の説明](../protocol/README.md) にあります。
