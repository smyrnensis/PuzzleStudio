# RuleProcessor 詳細仕様書

> 本書は RuleProcessor の内部ロジックを解説する補助ドキュメントである。
> API 定義は `RuleProcessor仕様書.md` を参照。
> アルゴリズムの疑似コードは `docs/フレームワーク計画.md` セクション 8 を参照。

---

## 1. 概要

`RuleProcessor` は、ルールベース・グリッドパズルゲームフレームワークの中核コンポーネントである。現在の `GameState` に対して `RuleGroup[]` で定義されたルールを適用し、新しい `GameState` を生成する。

---

## 2. 依存関係

### 2.1 外部依存関係

| コンポーネント | 用途 | 必須 |
|:---|:---|:---|
| `ObjectDB` | プロパティ解決（`@Movable` の展開等） | ○ |
| `GameState` | 盤面状態（`puzzle`）とグローバル変数（`globalState`）の読み取り | ○ |
| `GameData` | ルールグループ定義と `turnSequence` | ○ |

### 2.2 内部データ構造

```typescript
/** パターンマッチング結果 */
interface MatchResult {
  readonly blockIndex: number;
  readonly x: number;
  readonly y: number;
  readonly direction: Direction | null;
  readonly bindings: Readonly<Record<string, string>>;
  readonly matchedCells: readonly MatchedCellInfo[];
}

/** セルレベルのマッチ情報 */
interface MatchedCellInfo {
  readonly x: number;
  readonly y: number;
  readonly objectMatches: Readonly<Record<string, string>>;
}
```

---

## 3. 公開 API

### 3.1 メイン処理関数

```typescript
function processTurn(
  state: GameState,
  gameData: GameData,
  objectDB: ObjectDB
): RuleResult
```

**処理フロー:**
1. `turnSequence` の各グループ名について `applyRuleGroup` を実行
2. 累積された `RuleResult` を返却

### 3.2 補助関数

```typescript
function applyRule(state: GameState, rule: Rule, objectDB: ObjectDB): RuleResult;
function applyRuleGroup(state, group, allGroups, objectDB): RuleResult;
function evaluateCondition(condition: Condition, globalState: GlobalState): boolean;
function applyEffects(globalState, effects): { globalState; pendingEffects };
```

---

## 4. 内部関数の詳細設計

### 4.1 ルール適用制御

#### `applyRule(state, rule, objectDB) → RuleResult`

**処理ロジック:**

```
1. conditions を評価（不成立なら {state, changed: false, effects: []} を返却）
2. application に基づく制御:
   - "once": applyRuleOnce を1回呼び出し
   - "until_stable": applyRuleOnce を changed=false になるまでループ
3. 累積されたエフェクトと変化フラグを返却
```

#### `applyRuleOnce(state, rule, objectDB) → RuleResult`

**処理ロジック:**

```
1. bindings = {} で初期化
2. rule.patterns の各ブロックについて順に:
   a. findMatch(state.puzzle, block, rule.direction, objectDB, bindings) を実行
   b. マッチしなければ {state, changed: false, effects: []} を返却
   c. マッチしたら bindings をマージして次のブロックへ
3. 全ブロックがマッチ → 原子的に適用:
   a. 各マッチ結果について afterPattern を回転して applyMatch を実行
   b. effects を適用
4. {newState, changed: true, effects: pendingEffects} を返却
```

### 4.2 パターンマッチング

#### スキャン順序の重要性

パターンマッチングは **厳密な順序** で実行される：

1. **方向の優先順**: 各適用方向を順番に処理
   - `"any"` → `["up", "right", "down", "left"]`
   - `"vertical"` → `["up", "down"]`
   - `"horizontal"` → `["left", "right"]`
2. **位置順序**: 各方向について、y=0→height-1, x=0→width-1（左上→右下）
3. **即座に適用**: 最初にマッチした箇所で即座に置換を実行

この方式により、ルールの適用結果が **決定論的** かつ **予測可能** になる。

#### `findMatch(puzzle, block, ruleDirection, objectDB, existingBindings) → MatchResult | null`

```
1. directions = expandDirections(ruleDirection) で方向リストを生成
2. 各 direction について:
   a. rotatedBefore = rotatePattern(block.before, direction)
   b. patternHeight = rotatedBefore.length, patternWidth = rotatedBefore[0].length
   c. y=0..puzzle.height-patternHeight, x=0..puzzle.width-patternWidth で順次スキャン:
      result = tryMatch(puzzle, rotatedBefore, x, y, direction, objectDB, existingBindings)
      if result ≠ null: return result
3. return null
```

#### `tryMatch(puzzle, pattern, startX, startY, direction, objectDB, existingBindings) → MatchResult | null`

```
1. bindings = copy(existingBindings)
2. matchedCells = []
3. pattern の各セル (py, px) について:
   a. puzzleX = startX + px, puzzleY = startY + py
   b. cell = puzzle[puzzleY][puzzleX]

   c. objects チェック（置換対象）:
      for each patternObj in cellPattern.objects:
        candidates = objectDB.resolvePattern(patternObj の名前部分)
        matched = false
        for each candidateName in candidates:
          fullPattern = candidateName + patternObj のタグ部分
          for each actualObj in cell:
            newBindings = matchObjectPattern(actualObj, fullPattern, bindings)
            if newBindings ≠ null:
              bindings = newBindings; 記録: patternObj → actualObj; matched = true; break
          if matched: break
        if not matched: return null

   d. hasObjects チェック（存在確認）:
      for each requiredObj in cellPattern.hasObjects:
        candidates = objectDB.resolvePattern(requiredObj の名前部分)
        found = false
        for each candidateName in candidates:
          for each actualObj in cell:
            if actualObj の名前 == candidateName: found = true; break
          if found: break
        if not found: return null

   e. noObjects チェック（不存在確認）:
      for each forbiddenObj in cellPattern.noObjects:
        candidates = objectDB.resolvePattern(forbiddenObj の名前部分)
        for each candidateName in candidates:
          for each actualObj in cell:
            if actualObj の名前 == candidateName: return null

   f. matchedCells.push({ x: puzzleX, y: puzzleY, objectMatches })

4. return { blockIndex, x: startX, y: startY, direction, bindings, matchedCells }
```

### 4.3 パターン回転

#### `rotatePattern<T>(pattern, direction) → rotated pattern`

基準方向は **right**（パターンは右向きに記述されている）。

| direction | 変換 |
|:---|:---|
| `right` (または `null`) | そのまま返す |
| `down` | 90° 時計回り: `new[c][rows-1-r] = pattern[r][c]` |
| `left` | 180° 回転: `new[rows-1-r][cols-1-c] = pattern[r][c]` |
| `up` | 90° 反時計回り: `new[cols-1-c][r] = pattern[r][c]` |

**相対方向タグの変換:**

回転時にパターン内の相対方向タグ（`>`, `<`, `^`, `v`）も同じ回転を適用する。

| 元のタグ | right (0°) | down (90°) | left (180°) | up (270°) |
|:---|:---|:---|:---|:---|
| `>` (右) | `>` | `v` | `<` | `^` |
| `<` (左) | `<` | `^` | `>` | `v` |
| `^` (上) | `^` | `>` | `v` | `<` |
| `v` (下) | `v` | `<` | `^` | `>` |

### 4.4 オブジェクト解決

#### `resolveObjectSpecification(spec, cell, objectDB)`

パターン内のオブジェクト指定を実際のオブジェクトに解決する：

1. `@` プレフィックス: `objectDB.resolvePattern(spec)` でプロパティ逆引き
2. 通常の名前: そのまま名前マッチ
3. `$variable` タグ: バインディングとして処理（`matchObjectPattern` が担当）

### 4.5 条件判定

#### `evaluateCondition(condition, globalState) → boolean`

```typescript
// condition = { variable: string, op: ComparisonOp, value: GlobalValue }
const actual = globalState[condition.variable];
switch (condition.op) {
  case "==": return actual === condition.value;
  case "!=": return actual !== condition.value;
  case ">":  return actual > condition.value;
  case ">=": return actual >= condition.value;
  case "<":  return actual < condition.value;
  case "<=": return actual <= condition.value;
}
```

### 4.6 状態更新

#### `applyMatch(puzzle, match, afterPattern, direction, bindings) → Puzzle`

1. after パターンの各セルについて：
   - 対応する before の `objects` でマッチしたオブジェクトをセルから削除
   - after で指定された新しいオブジェクトを追加（`applyBindings` で `$variable` を展開）
2. 相対方向タグがあれば現在の方向に変換
3. 新しい Puzzle を返す（元の Puzzle は変更しない）

#### `applyEffects(globalState, effects) → { globalState, pendingEffects }`

- `"set"`: `globalState[variable] = value` で新しい globalState を生成
- `"change"`: `globalState[variable] += amount` で新しい globalState を生成
- `"sound"`, `"message"`, `"call"`: `pendingEffects` に追加（レンダラー/呼び出し元に委譲）

### 4.7 グリッド操作ユーティリティ

グリッド操作は `utils/puzzle.ts` モジュールの関数を使用する：

| 関数 | 説明 |
|:---|:---|
| `getCell(puzzle, x, y)` | 指定座標のセルを取得（範囲外は `undefined`） |
| `setCell(puzzle, x, y, cell)` | セルを置き換えた新しい Puzzle を返す |
| `findObjectsInCell(cell, name)` | 名前でセル内オブジェクトを検索 |
| `replaceObjectInCell(cell, old, new)` | オブジェクトを置換した新しいセルを返す |
| `removeObjectFromCell(cell, obj)` | オブジェクトを除去した新しいセルを返す |
| `addObjectToCell(cell, obj)` | オブジェクトを追加した新しいセルを返す |
| `findAllObjects(puzzle, name)` | グリッド全体からオブジェクト位置を検索 |
| `directionToOffset(direction)` | 方向 → `{dx, dy}` オフセット変換 |

> 詳細は `docs/フレームワーク計画.md` セクション 7.2 を参照。

---

## 5. エラーハンドリング

### 5.1 入力検証
- ルールデータの必須フィールド存在チェック
- 座標の範囲チェック
- オブジェクト名の定義存在チェック（`objectDB.getDefinition` が未定義時に例外）

### 5.2 実行時エラー
- 無効なプロパティ指定に対する `console.warn` 出力
- `until_stable` の最大繰り返し回数制限（無限ループ防止）

---

## 6. パフォーマンス考慮事項

### 6.1 最適化戦略
- パターン回転結果のキャッシュ
- 早期リターンによる不要な計算の回避

### 6.2 計算量
- パターンマッチング: O(盤面サイズ × パターンサイズ × 方向数) ※最悪ケース
- 実際の動作: 最初のマッチで即座に処理が終わるため、平均的には O(1)〜O(盤面サイズ)
- 全体的な複雑度: O(ルール数 × 平均マッチ位置 × 最大適用回数)

**注意**: `"until_stable"` ルールの場合、最悪ケースでは非常に多くの繰り返しが発生する可能性があるため、適用回数の上限設定を推奨。
