# Solver 設計

この文書は、このプロジェクトにおける solver の役割と設計方針を定義する。

これは一般的なグラフ解析計画ではない。また、あらゆるルールセットを賢く解く万能 solver を作る約束でもない。
非自明なパズルの状態グラフは大きすぎて、そのまま構築・解析する対象にはならない。solver は、ルール体系、ステージ、解経路、ステージ摂動を観測するための道具として設計する。

## 目的

solver の目的は、面白い挙動を見つけるために人間が手でプレイし続ける負荷を下げることにある。

抽象的には、パズルは状態遷移グラフとして理解できる。

- node はゲーム状態
- edge は入力による状態遷移
- start と goal は関心のある状態領域
- 面白さは、プレイヤーが非自明な状態間のつながりを発見するところに生まれる

ただし、完全なグラフは通常大きすぎて構築も観察もできない。solver はグラフ全体を理解しようとしない。代わりに、局所的な探索、サンプリング、比較、報告を行い、人間が有望な macro link や創発現象に気づくための証拠を出す。

solver の主な出力は「答え」ではない。主な出力は証拠である。

- witness path
- 失敗例
- bottleneck
- 状態差分
- rule firing trace
- 摂動への応答
- 非自明そうな link 候補

その結果が面白いか、美しいか、面倒なだけか、不公平か、レベル列に育てる価値があるかは、最後は人間が判断する。

## 中核原則

solver は oracle ではなく observatory である。

authoring 中に何度も走らせられるだけ速く、変化するルールセットに追従できるだけ柔軟で、なぜその経路や失敗を flag したのか説明できるだけ明示的であるべき。

目的は人間の審美判断を置き換えることではない。人間の仕事を「すべての経路を手で歩くこと」から、「選ばれた trace、witness path、応答パターンをレビューすること」に移すことである。

## 非自明さ

非自明さは、グラフそのものの絶対的な性質ではない。観測者モデルに対する性質である。

同じ link でも、次のように見え方が変わる。

- 作者には自明
- 初見プレイヤーには見えない
- 熟練者には定石
- exhaustive solver にはただの経路
- naive player model には到達不能

したがって、システムは「この link は非自明である」と最終判定しようとしない。弱い proxy signal によって候補を出し、人間に label させる。

有用な proxy signal には次のようなものがある。

- 強い solver は経路を見つけるが、naive solver は失敗する
- 経路が一時的に見かけの goal から遠ざかる
- irreversible に見える手が必要になる
- 経路が狭い bottleneck state を通る
- 小さなステージ変更で solution が消える
- 小さなステージ変更で solvability は残るが solution structure が変わる
- unusual な phase や順序で rule が使われる
- 既知 motif で説明できない
- 1つの局所変更が reachability や solution length に大きな差を作る

これらは review のための flag であり、品質の定義ではない。

## Solver の種類

1つの solver にすべてを背負わせない。

異なる強みを持つ複数の solver mode を用意する。それぞれの成功と失敗を比較すること自体が、単独の結果より有用なことが多い。

### Exact Solver

exact solver は、境界付きの状態空間を探索し、goal に到達可能かを報告する。

用途:

- 小さな実験ステージ
- regression example
- shortest witness path
- 単純な test case がまだ機能することの確認

BFS、A*、IDA*、その他の決定論的探索を使ってよい。完全性が有用なのは現実的な境界の内側だけである。大きなケースでは exact search が拒否してよい。

### Sampling Solver

sampling solver は、完全性を捨てる代わりに速度と幅を取る。

用途:

- 生成された多数のステージを素早く試す
- unusual な経路を拾う
- approximate な solution family を見つける
- 人間レビュー用の candidate trace を出す

戦略としては、random walk、beam search、weighted random search、MCTS 風探索、heuristic-guided search などが考えられる。

### Naive Player Model

naive player model は、普通の局所的・goal-directed なプレイヤー挙動を近似する。

これは意図的に弱い solver である。その失敗が有用である。

用途:

- counterintuitive な手が必要な経路を見つける
- 人間に自明そうな挙動と solver に自明な挙動を比較する
- 見かけの goal から遠ざかる必要がある level を flag する

### Known-Motif Solver

known-motif solver は、すでに記録された macro link、motif、再利用可能な変換だけを使って探索する。

用途:

- 新しい level が既知の考え方だけで解けるかを確認する
- 新現象を含む可能性のある候補を flag する
- prerequisite motif を推定し、curriculum 構成を支援する

exact または sampling search が成功する一方で known-motif search が失敗する場合、その結果は新しい phenomenon の候補になる。

## 可変ルールセット

solver は、変化するルールセットに対して動かなければならない。

そのため、ゲーム固有の探索ロジックを手で書く方針は危険である。solver は rule engine を真実の源泉として扱い、コンパイル済みの transition function に依存するべきである。

```txt
compiled rules + state + input -> next state + trace
```

高速性を保つため、rule system は探索前に正規化された中間表現へ compile する。

重要な compile step:

- direction variant を展開する
- pattern cell を正規化する
- object / property lookup を事前計算する
- repeat rule ID を割り当てる
- phase order を事前計算する
- event と state change を分離する
- rule application を決定論的にする

solver は探索中に authoring syntax を再 parse してはいけない。

## 高速化方針

solver は、反復的な authoring loop に最適化する。

重要な技術:

- canonical state encoding
- 高速な state hashing
- `state + input` に対する transition cache
- 繰り返し遷移に対する trace cache
- 可能であれば incremental / Zobrist-style hash update
- 早期の duplicate detection
- bounded search budget
- partial evidence を返せる anytime search
- small-level-first exploration
- 可能な範囲での parallel exploration

想定 workload は、巨大な1ステージを完璧に解くことではない。ルール編集、ステージ variation、生成された test arena に対して、小中規模の probe を大量に走らせることである。

## Level Perturbation Analysis

level perturbation は solver の一級の仕事である。

面白い挙動は、単一の solution ではなく、ステージを少し変えたときの solution の応答に現れることが多い。

システムは、ステージの小さな mutation を生成し、それぞれの solver result を比較できるべきである。

有用な perturbation:

- 壁を1つ足す / 消す
- goal を1マス動かす
- object を1つ足す / 消す
- object を1マスずらす
- corridor を狭める / 広げる
- side path を塞ぐ / 開く
- initial tag / property を変える
- available resource を変える

各 variant について、solver は次を比較する。

- solvability
- shortest solution length
- 見つかった solution 数
- rule firing sequence
- bottleneck state
- irreversible move
- repeated motif
- 同じ macro link が残るか

目的は response pattern を検出することである。

- 多くの variant で残る robust motif
- 小さな変更で消える fragile link
- tiny edit が reachability を大きく変える phase transition
- 局所変更で切り替わる alternative solution family
- 見た目は似ているが solution structure が違う level 群

良い創発現象は、完全に robust でも random でもないことが多い。圧力を外すと消えるが、関連する圧力のもとでは再発する、という再現条件を持つ。

## Macro Link Candidate

solver は、full-level solution だけではなく macro link candidate を報告するべきである。

macro link は、状態領域間の意味ある multi-step connection である。必ずしも1入力の edge ではない。

記録例:

```txt
macro_link_candidate:
  from: state or state-region summary
  to: state or state-region summary
  witness: input sequence and replay
  level: source level ID
  flags:
    - naive solver failed
    - heuristic distance increased for 5 steps
    - solution crosses bottleneck state
    - one-wall perturbation destroys the path
  trace:
    - fired rules
    - changed cells
    - emitted events
  human_label: unknown
```

人間の label は保存する。

- interesting
- obvious
- tedious
- unfair
- known
- unclear

これらの label は、将来の ranking や filtering のためのデータになる。ただし普遍的な真実として扱ってはいけない。

## Solver Output

solver は、人間と AI の両方が検査できる artifact を出すべきである。

期待される output:

- solution witness
- replay
- trace
- state diff sequence
- failed search summary
- perturbation comparison table
- macro link candidate list
- bottleneck state list
- motif match / motif failure report

これらの output は、design note、regression test、AI との会話から参照できる程度に安定しているべきである。

## Authoring Flow との関係

制作フローは、solver が全知であることに依存してはいけない。

solver は、authoring loop の中で境界付きの問いに答える。

- この局所 setup は解けるか
- witness path は1つあるか
- この壁を動かすと solution はどう変わるか
- naive model はここで失敗するか
- これは既知 motif に似ているか
- どの trace がこの挙動を作ったか

authoring loop は人間主導のままである。

```txt
rule system
  -> experimental level
  -> solver observation
  -> human judgment
  -> rule or level revision
```

solver は observation を出す。何を重視するかは人間が決める。

## 初期実装方針

段階的に作る。

1. rule を deterministic transition function に compile する。
2. canonical state hashing と transition cache を実装する。
3. 小さな level 用の exact BFS を作る。
4. replay と rule firing trace output を追加する。
5. level perturbation generation と比較を追加する。
6. 単純な naive player model を追加する。
7. 大きめの実験用に sampling search を追加する。
8. macro link candidate extraction を追加する。
9. human-labeled example の corpus ができてから motif matching を追加する。

最初の有用な solver は、賢い必要はない。速く、再現可能で、証拠を明確に見せられる必要がある。

## 境界

避けるべき罠:

- 完全な状態グラフを解析しようとする
- rule set ごとに game-specific solver を作る
- shortest solution だけを重要 signal とみなす
- solver が見つけた経路を自動的に良いパズルとみなす
- 非自明さの完全自動判定を要求する
- perfect solver ができるまで有用な tooling を遅らせる

solver は、パズル設計を理論的に解決するものではなく、発見のための実用的な観測器であり続けるべきである。
