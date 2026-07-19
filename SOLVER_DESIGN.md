# Solver 設計

この文書は、このプロジェクトにおける solver の役割と設計方針を定義する。

これは一般的なグラフ解析計画ではない。また、あらゆるルールセットを賢く解く万能 solver を作る約束でもない。
非自明なパズルの状態グラフは大きすぎて、そのまま構築・解析する対象にはならない。solver は、ルール体系、ステージ、解経路、ステージ摂動を観測するための道具として設計する。

## 目的

solver の目的は、面白い挙動を見つけるために人間が手でプレイし続ける負荷を下げることにある。

抽象的には、パズルは状態遷移グラフとして理解できる。

- node は、goal と compiled transition に関係する object だけを持つ projected logical state
- edge は、1つの compiled logical input と、その rule evaluation 内で生じる `again` を完了する論理 transition
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

そのため、ゲーム固有の探索ロジックを手で書く方針は危険である。solver はコンパイル済み rule を真実の源泉とし、relevance analysis で閉じた projected model 上の logical transition を使う。restart、undo、level navigation などの session operation は探索 input に含めない。探索 transition が session semantics を要求する effect に遭遇した場合は、その境界で unsupported として失敗させる。

```txt
projected compiled game + projected logical state + input
  -> next projected logical state
```

solver transition は presentation timeline、debug trace、checkpoint、navigation wait、editor history を生成・保持しない。必要な証拠と real state は、選択された候補の入力列を authoritative player session で replay して観測する。wait や animation の再生時間は探索を進める入力にならない。

level completion は logical candidate の terminal metadata として扱う。navigation 後の session state は探索ノードに格納しない。候補を採用または表示するときは、同じ root と witness を `puzzle-play` で replay し、completion observation を含む real state を取得する。

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
- relevance-projected logical state の canonical key
- 高速な state hashing
- `projected logical key + input` に対する transition cache
- 繰り返し遷移に対する trace cache
- 可能であれば incremental / Zobrist-style hash update
- 早期の duplicate detection
- bounded search budget
- partial evidence を返せる anytime search
- small-level-first exploration
- 可能な範囲での parallel exploration

想定 workload は、巨大な1ステージを完璧に解くことではない。ルール編集、ステージ variation、生成された test arena に対して、小中規模の probe を大量に走らせることである。

## Solver State Slicing

solver の state key は、authoring syntax や object 名の prefix から直接決めない。
target contract は `STATE_VIEW_AND_SOLVER_SLICING_SPEC.md` に置く。

重要な方針:

- play/editor が扱う通常 state と、solver の duplicate detection / cache key に使う state は分ける。
- solver key から落とせる object は、見た目用だからではなく、future gameplay observation に影響しないと compiled rule analysis で分かるから落とす。
- object 名の表記を solver relevance の真実の源泉にしない。
- random は deterministic でなければならない。pruned rule が gameplay random stream を変える可能性があるなら、その依存は solver-visible として扱う。

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

## Manual Solver Workbench

### 定義と成功条件

manual solver は、探索を人間の手作業に戻す機能ではない。人間または AI が、現在地、
目標、次の入力、探索へ渡す仕事量、観察・採用する候補を明示的に選ぶ investigation
session である。logical search と real-state replay は同じ runtime が別の operation として所有する。

```txt
operator chooses question or action
  -> engine executes or searches
  -> engine returns state, provenance, diff, trace, candidates
  -> operator chooses the next question or action
```

人間と AI は表示方法や入力装置を共有する必要はない。共有すべきものは、同じ状態、
同じ command、同じ goal、同じ search frontier、同じ結果の意味である。Human UI と
agent JSONL が別々の solver implementation を持ち、たまたま似た答えを返す構成は
「共通」ではない。

manual solver が満たす成功条件は次のとおり。

- 人間は盤面を見ながら semantic input を1手ずつ実行し、履歴上の任意の状態から分岐できる。
- AI は同じ操作を opaque handle と typed response で実行できる。
- 人間と AI は同じ typed goal を評価し、同じ search を bounded allowance 単位で進められる。
- search candidate は projected logical state と witness だけを持ち、authoritative replay と projection equality check に成功したときだけ real state として観察・再利用できる。
- state、run、goal、search、candidate の由来が失われず、reachable、counterfactual、search-local を混同しない。
- source snapshot が変わった場合、古い session を新しい compile のものとして継続しない。
- UI は source text、object ID、raw slot、win 判定、lifecycle を再解釈しない。

### Runtime boundary

`puzzle-solver-runtime` は state、run、goal、search、candidate、provenance の owner である。
`puzzle-agent-runtime` は versioned JSONL envelope、request correlation、error serialization
だけを所有し、domain operation を実装しない。

editor worker、browser player、WASM binding に残る orchestration は、この runtime contract を
呼ぶ adapter へ縮小する。特に editor は候補を選択するが、logical-to-real reconstruction や
goal matching を JavaScript で実装しない。

adapter に同じ domain operation が見つかった場合は runtime contract へ移し、adapter 側の
実装を削除する。旧実装へ委譲する compatibility path は持たない。

### 共通 domain model

#### Prepared artifact

prepared artifact は、1つの完全な source snapshot から compile された model を表す。
`artifact_id` は entry path、全 document 内容、選択 model、compiler contract version から
作る。level list、semantic input list、object/variable catalog、goal capability、model kind を
manifest として公開する。

artifact は source を再 parse する consumer のための JSON blob ではない。compiled model は
Rust/WASM service 内に留まり、consumer は opaque `artifact_id` だけを持つ。

#### Investigation session

investigation session は prepared artifact と選択 level に属し、次を所有する。

- immutable state node
- input sequence を実行した run edge
- goal
- resumable search と search-local candidate
- state、run、candidate に付けた label と note
- source fingerprint と contract version

session の各 handle は session-local で、別 artifact や別 process に持ち込めない。session を
close すると未保存の search frontier を含む全 handle を解放する。

#### Runtime state と search node

runtime state handle は editor が観察・再利用する real state を持てる。search node はこれと
異なり、relevance-projected logical state、goal completion metadata、parent/action linkage だけを
持つ。real state、run provenance、input history、checkpoint、navigation、UI state は frontier と
visited key に入れない。

provenance は少なくとも次を区別する。

- `authored`: compiled level start から materialize した状態
- `reachable`: authored または reachable state から semantic input を replay して得た状態
- `counterfactual`: base state から明示的な semantic edit で作った仮説状態
- `materialized_candidate`: search candidate を同じ root から replay して一致を検証した状態

`materialized_candidate` は search root の real state から witness を replay して作る。replay
結果を search と同じ slicer で projection し、logical candidate と一致した場合だけ handle を
作る。counterfactual root からの candidate を materialize しても provenance は reachable へ
変わらない。

#### Run edge と履歴

run edge は `from_state_id`、semantic input sequence、各 input 後の real observation、terminal
state、trace、result を持つ。これは search edge ではなく、`puzzle-play` が authoritative に
実行する materialized run である。

UI の Undo / Redo は state graph 上の cursor navigation である。ゲーム自身の undo command を
実行することではない。古い node から新しい input を実行すると branch が増え、既存 branch は
消えない。

#### Goal

共通 goal contract は少なくとも次を持つ。

- `level_completion`: game が定義した completion observation
- `semantic`: cell、object、variable に対する typed predicate
- `exact_state`: 指定 state node の logical observation と一致

2D semantic goal は `exact`、`contains`、`excludes`、`unknown` を canonical predicate とする。
ASCII は AI 向け codec、grid painter は人間向け editor であり、どちらも language-owned な
同じ semantic goal model へ lower する。JavaScript が legend や object name を解決しては
ならない。

3D の semantic goal editor は、同じ typed spatial predicate contract が定義されるまで
unsupported として明示的に失敗させる。level completion search や manual input まで無効に
する理由にはしない。

#### Search

search は root state、goal snapshot、algorithm、input set、heuristic、最大 depth、最大 stored
node を作成時に固定する。これらを変える場合は新しい search を作る。

`advance_search` は、既存 frontier に追加する `max_expanded_nodes` と `max_millis` を受け取る。
allowance 終了時は `paused` になり frontier を保持する。UI は hidden loop で無期限に再開せず、
operator が指定した allowance または明示的な run-until 条件だけを実行する。

candidate は search-local で、次を返す。

- stable candidate ID within the search
- score、depth、discovery order
- witness input sequence
- state hash
- goal diff
- root との差分要約

candidate の logical preview は read-only である。real preview または再利用可能な state が
必要な場合、`materialize_candidate` が witness を authoritative session で replay し、その
projection が candidate の logical state と一致することを確認してから state/run handle を作る。

### 共通 command surface

`puzzle-solver-runtime` は Rust の typed command と typed result を所有する。WASM method と
agent JSONL operation はその adapter であり、内部 model ではない。

| 操作 | 入力 | 主な結果 |
|---|---|---|
| `prepare` | workspace snapshot、entry、model selector | artifact handle、manifest |
| `create_session` | artifact、level | session、authored state |
| `inspect_state` | state handle、observation profile | symbolic state、summary、provenance |
| `apply_inputs` | from state、semantic input sequence、trace profile | run、points、terminal state、result |
| `compare_states` | two state handles | object/variable/session diff |
| `derive_counterfactual` | base state、typed semantic edit | counterfactual state、validated diff |
| `define_goal` | base state、typed goal spec | goal handle、normalized goal |
| `evaluate_goal` | goal、state | match、mismatch list |
| `create_search` | root、goal、algorithm、limits | search handle、`ready` status |
| `advance_search` | search、allowance | status、stats、best candidates |
| `inspect_search` | search、candidate limit | immutable search snapshot |
| `materialize_candidate` | search、candidate | replay-verified run/state |
| `close_search` | search | released search resources |
| `close_session` | session | released child resources |

`apply_inputs` は単手と複数手を同じ operation として扱う。人間の1手入力を特権的な別 contract
にしない。逆に、solve convenience operation は `create_search`、`advance_search`、
`materialize_candidate` の合成として adapter が勝手に再実装してはならない。必要なら runtime
が named orchestration として所有する。

### Ownership と data flow

```txt
Human Solver UI                      Agent / automation
  -> WASM adapter                       -> JSONL adapter
           \                           /
            puzzle-solver-runtime
              - handles and provenance
              - immutable run graph
              - goals and searches
                |        |        |
          puzzle-lang puzzle-play puzzle-solver
          compile/codec  replay     search machine
```

ownership は次のように分ける。

- `puzzle-lang`: workspace compile、semantic state/goal model と ASCII codec
- `puzzle-play`: real state の authoritative initialization、run、replay、completion observation
- `puzzle-solver`: logical transition、state slicing、search algorithm、frontier、candidate ordering、projected state key
- `puzzle-solver-runtime`: artifact/session/handle、provenance、run graph、goal/search orchestration、candidate materialization
- `puzzle-runtime-contract`: source-free な adapter transport type
- `puzzle-agent-runtime`: JSONL versioning、request correlation、error serialization
- `puzzle-wasm`: browser worker から呼ぶ thin WASM binding
- `html_editor`: DOM、keyboard、layout、render request、human-readable status

editor と agent adapter は source syntax、goal matching、state hash、candidate promotion、completion
判定を実装しない。

### Human UI

solver は独立した「答え再生パネル」ではなく、状態グラフを操作する workbench とする。
wide layout の基準形は次のとおり。

```txt
+ Solver: Level 4 / artifact 8f2a / current: reachable ----------------------+
| History / branches |                Board                 | Goal / Search  |
| Start              |                                      | Level complete |
|  R D                |     current logical observation      | [Evaluate]     |
|  + candidate-7     |                                      |                |
|  + manual branch   |  Inputs: [Up] [Down] [Left] [Right]  | Search paused  |
|                    |          [Action] [Restart]           | 820 visited    |
|                    |                                      | [Advance 100]  |
|                    |                                      | Candidates     |
|                    |                                      | #7 score 2     |
+--------------------+--------------------------------------+----------------+
| Diff / trace / notes                                                     |
+----------------------------------------------------------------------------+
```

#### Context bar

常に level、artifact fingerprint の短縮表示、current state provenance、source freshness を表示する。
source が変わったら `stale` を表示し、古い session は読み取り可能なまま凍結する。新 compile で
自動継続したり、同名 level へ暗黙に rebase しない。

#### Board と input

board は current state、selected history node、candidate preview のいずれを表示しているかを
明示する。candidate preview は border と label でも区別し、色だけに依存しない。

input control は compiled manifest の semantic input を列挙する。方向キーや game-specific key は
adapter が物理 key を semantic input ID へ変換するだけで、UI が名前から方向を推測しない。
複数手を入力欄へ queue して `Run` する操作も、単手 button も `apply_inputs` を使う。

#### History / branches

history は state node と run edge の tree として表示する。各 edge には入力列、result、state
provenance を表示し、node を選ぶと board、diff、trace が同時に切り替わる。

- Back / Forward: cursor navigation
- Branch: 選択 node を current root にする
- Pin: review 対象として保持する
- Compare: 2つの pin を比較する
- Label / Note: `interesting`、`deadend` などの判断を記録する

Restart は authored state へ cursor を戻す操作と、game の restart lifecycle を実行する操作を
別の名前で提示する。意味を混ぜない。

#### Goal editor

既定 goal は level completion である。Semantic goal mode では、base state から goal layer を
作り、cell ごとに `exact`、`contains`、`excludes`、`unknown` を選ぶ。object picker は manifest
由来で、保存時に Rust 側が collision layer、dimension、object identity を検証する。

goal card は checked cell 数、unknown cell 数、current state との差分を常時表示する。goal を
変更して既存 search を再利用せず、新 search 作成を要求する。

#### Search inspector

search inspector は次を同時に見せる。

- root state と goal
- algorithm と immutable limits
- status: `ready` / `paused` / `solved` / `exhausted` / `resource_limit` / `failed`
- visited、expanded、frontier、max depth、elapsed
- allowance control: expanded nodes と wall time
- ranked candidate list

`Advance` は指定 allowance だけ進める。`Run until solved` を置く場合も、明示した total limit と
Pause を持ち、内部では同じ advance を使う。`paused` と `resource_limit` を「解なし」と表示しない。

candidate を選ぶと board と goal diff を preview する。`Materialize as branch` で replay verification
を通過した場合だけ history に追加する。solution candidate も同じ経路を通し、特別な未検証の
solution object を作らない。

#### Inspector

選択 state について、前 state との差分、発火 rule、variable/session change、completion
observation を tabs で表示する。人間向け文言は symbolic contract から生成し、raw slot 配列を
primary UI にしない。board を読めない利用者向けに、座標と object 名の textual diff を提供する。

#### Responsive behavior

狭い preview pane では board を先頭に置き、History、Goal/Search、Inspector を tabs または
drawer にする。情報を削除した簡易 solver へ切り替えず、同じ state selection と command を
異なる layout で表示する。

### Human / AI handoff

live handle や frontier は process-local なので、そのまま clipboard や file へ保存しない。
handoff 用には replayable な `SolverNotebook` artifact を別途定義する。

保存対象:

- contract version と exact source fingerprint
- model / level identity
- root provenance
- semantic goal spec
- pinned state への authoritative input sequence、または base 付き counterfactual spec
- materialize 済み candidate の witness
- labels、notes、比較対象
- search configuration、final status、stats

保存しない対象:

- compiled model blob
- raw object IDs だけの board snapshot
- search frontier、visited table、process-local handle
- renderer state

Notebook を開くときは exact source fingerprint を検証し、authoritative replay で state を再構築する。
一致しなければ drift を報告して停止する。類似する level や古い source へ自動変換しない。

### Contract acceptance

最低限、次を cross-adapter test と UI test で保証する。

1. 同じ artifact、level、from state、input sequence は、Human UI 経由と agent 経由で同じ result、
   logical state hash、trace summary を返す。
2. 同じ typed goal は同じ match と mismatch を返す。
3. node-count allowance が同じ search は同じ stats と candidate order を返す。
4. candidate materialization は replay state が一致しない限り handle を作らない。
5. UI history navigation は game input や lifecycle を実行しない。
6. source fingerprint change は active session を stale にし、新 artifact として継続しない。
7. unsupported な 3D semantic goal は明示 error になり、3D manual input と level-completion search は
   それ自体の contract が成立する限り動く。
8. JavaScript が `.puzzle` source、legend、object ID、win condition を解釈しないことを source-contract
   test で守る。

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

## 実装順序

各段階は最終 ownership に直接移し、旧 owner の同じ実装を同じ変更で削除する。一時的な
新旧 solver route は作らない。

1. editor の manual input を共通 `apply_inputs` / inspect contract へ接続し、WASM を adapter のみにする。
2. semantic state/goal model と ASCII codec を `puzzle-lang` の owned contract にし、agent artifact と 2D Goal UI を
   同じ normalized spec へ lower する。
3. `html_play` と editor worker に残る search orchestration を共通 runtime operation に置き換える。
4. editor の現在の one-button solve/replay panel を、board、history、goal、search、inspector を持つ Solver Workbench に置き換える。
5. cross-adapter conformance test を追加し、node allowance、logical state hash、goal diff、candidate order、materialized replay result を比較する。
6. replayable `SolverNotebook` を追加して human / AI handoff を可能にする。
7. その後に perturbation、naive model、sampling、macro link、human label corpus、motif matching を追加する。

最初の有用な solver は、賢い必要はない。速く、再現可能で、証拠を明確に見せられる必要がある。

## 境界

避けるべき罠:

- 完全な状態グラフを解析しようとする
- rule set ごとに game-specific solver を作る
- shortest solution だけを重要 signal とみなす
- solver が見つけた経路を自動的に良いパズルとみなす
- 非自明さの完全自動判定を要求する
- perfect solver ができるまで有用な tooling を遅らせる
- Human UI と agent protocol に別々の state、goal、search implementation を持たせる
- real state を search frontier や duplicate key に格納する
- materialized real state の projection を logical candidate と照合しない
- search を hidden loop で自動再開し、operator が探索量を決められなくする
- candidate preview を replay verification なしで reachable state に昇格する

solver は、パズル設計を理論的に解決するものではなく、発見のための実用的な観測器であり続けるべきである。
