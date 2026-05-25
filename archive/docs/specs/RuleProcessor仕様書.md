# RuleProcessor 仕様書

> 詳細な API は `docs/フレームワーク計画.md` セクション 7.6, 7.7 を参照。
> アルゴリズムの詳細は `docs/フレームワーク計画.md` セクション 8 を参照。
> 内部ロジックの解説は `RuleProcessor詳細.md` を参照。

---

## 1. 概要

`RuleProcessor` は、ルールベース・グリッドパズルゲームフレームワークの中核をなすデータ駆動型のモジュールである。

主な役割は、現在のゲーム状態（`GameState`）とルール定義（`RuleGroup[]`）を受け取り、ルールを適用してゲーム状態を 1 ターン進め、新しい `GameState` を生成すること。ロジックはすべて外部のルールデータによって定義される。

---

## 2. 責務

- **ルールの逐次処理**: `turnSequence` に従ってルールグループを定義された順序で処理する
- **パターンマッチング**: 現在の `puzzle` から、各ルールの `before` パターンに一致する箇所を検索する
- **条件判定**: パターンがマッチした場合、ルールの `conditions` で指定された `globalState` の条件を検証する
- **状態更新**: マッチと条件判定が成功した場合、`after` パターンで `puzzle` を、`effects` で `globalState` を更新する
- **適用方式のハンドリング**: `"once"` / `"until_stable"` に従ってルールの適用を制御する
- **方向の展開**: `direction` に基づきパターンを回転させてマッチングを行う
- **複数パターンブロックの処理**: 複数 `before`/`after` ブロックを原子的に同時処理する

---

## 3. 依存関係

| コンポーネント/データ | 目的 |
|:---|:---|
| `GameState` | 現在の盤面（`puzzle`）とグローバル変数（`globalState`）のデータソース |
| `GameData` | ルールグループ定義と `turnSequence` |
| `ObjectDB` | プロパティ解決（`@Movable` → `["Player", "Box"]` 等） |

---

## 4. 処理フロー

### 4.1 ターン全体の流れ（`processTurn`）

`turnSequence` に従ってルールグループを順に実行する。

```
processTurn(state, gameData, objectDB) → RuleResult
  for each groupName in gameData.turnSequence:
    group = gameData.ruleGroups.find(g => g.name === groupName)
    result = applyRuleGroup(state, group, gameData.ruleGroups, objectDB)
    state = result.state
```

### 4.2 ルールグループの実行（`applyRuleGroup`）

- `"once"`: グループ内のルールを 1 回ずつ順次実行
- `"until_stable"`: グループ全体で変化がなくなるまで繰り返し

### 4.3 個別ルールの適用（`applyRule`）

1. `conditions` を評価（すべて真でなければ不適用）
2. 全 `patterns` ブロックについて `findMatch` を実行
   - 1 つでもマッチしなければ不適用
   - タグバインディングは前のブロックから後のブロックに引き継がれる
3. すべてのマッチ結果を使って after パターンを原子的に適用
4. `effects` を適用
5. `application` が `"until_stable"` の場合、変化がなくなるまで 1-4 を繰り返す

---

## 5. 中核ロジック詳細

### 5.1 パターンマッチング

ルールの `before` ブロックの各セル条件（`objects`, `hasObjects`, `noObjects`）を盤面の対応するセルと比較する。

- **オブジェクトの解決**: `ObjectDB.resolvePattern()` で `@Movable` → `["Player", "Box"]` に展開
- **タグバインディング**: `$variable` 構文で before のタグ値を捕捉し、after で再利用
- **方向の回転**: `direction` 設定に基づきパターンの 2D 配列を回転

#### スキャン順序

パターンマッチングは **決定論的な順序** で実行される：

1. **方向**: `"any"` → `["up", "right", "down", "left"]` の順で試行
2. **位置**: y=0→height-1, x=0→width-1（左上→右下）
3. **即座に適用**: 最初にマッチした箇所で置換を実行

### 5.2 オブジェクト単位の置換

セル全体を上書きするのではなく、**マッチしたオブジェクトのみを置換** する精密な方法で行う。

#### 更新アルゴリズム

1. **マッチング**: before 条件でマッチしたオブジェクトを特定
2. **削除**: マッチ対象オブジェクトのみをセルから削除
3. **追加**: after で指定された新しいオブジェクトを追加

**例:**
- ルール `[A] → [C]`, セル `["A", "B"]` → 結果 `["C", "B"]`（B は影響を受けない）
- ルール `[A] → []`, セル `["A", "B"]` → 結果 `["B"]`（A だけ消える）
- ルール `[A, B] → [D]`, セル `["A", "B", "C"]` → 結果 `["D", "C"]`

### 5.3 複数パターンブロックの処理

1. **ブロック対応**: `patterns` 配列の各ブロックの before/after がペア
2. **独立したマッチング**: 各 before ブロックは独立した位置でマッチング
3. **全マッチ必須**: すべてがマッチした場合のみ、全 after ブロックが原子的に適用

---

## 6. API

### 6.1 モジュール関数

```typescript
/** 1つのルールを適用 */
function applyRule(
  state: GameState,
  rule: Rule,
  objectDB: ObjectDB
): RuleResult;

/** 1つのルールグループを実行 */
function applyRuleGroup(
  state: GameState,
  group: RuleGroup,
  allGroups: readonly RuleGroup[],
  objectDB: ObjectDB
): RuleResult;

/** 1ターン分のルール処理を実行 */
function processTurn(
  state: GameState,
  gameData: GameData,
  objectDB: ObjectDB
): RuleResult;

/** 条件式を評価 */
function evaluateCondition(
  condition: Condition,
  globalState: GlobalState
): boolean;

/** エフェクトを適用 */
function applyEffects(
  globalState: GlobalState,
  effects: readonly Effect[]
): { globalState: GlobalState; pendingEffects: readonly Effect[] };
```

### 6.2 戻り値

```typescript
interface RuleResult {
  readonly state: GameState;
  readonly changed: boolean;
  readonly effects: readonly Effect[];
}
```

`effects` には `sound`, `message`, `call` 等のレンダラーや呼び出し元に委譲すべきエフェクトが含まれる。`set` と `change` は `globalState` に直接反映されるため、ここには含まれない。

---

## 7. パフォーマンス考慮事項

- **パターン回転のキャッシュ**: 同じ方向のパターンは再計算しない
- **早期リターン**: 条件不成立やマッチ失敗時に即座に処理を中断
- **計算量**: パターンマッチング O(盤面サイズ × パターンサイズ × 方向数)。最初のマッチで即座に適用されるため、平均的には O(盤面サイズ) 程度
- **`until_stable` の安全策**: 最大繰り返し回数の上限設定を推奨

> アルゴリズムの疑似コードは `docs/フレームワーク計画.md` セクション 8.1-8.4 を参照。
