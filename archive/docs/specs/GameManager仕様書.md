# GameManager 仕様書

> 詳細な API は `docs/フレームワーク計画.md` セクション 7.9 を参照。

---

## 1. 概要

`GameManager` は、ルールベース・グリッドパズルゲームフレームワークの中核コンポーネントである。ゲーム全体の進行制御、GameState の管理、プレイヤー入力の処理、そして各システム間の調整を担当する。

---

## 2. 基本責務

### 2.1 GameState 管理
- GameState の唯一の所有者として機能
- GameState の読み込み、初期化
- GameState の整合性保証

### 2.2 ゲーム進行制御
- レベルの開始、終了
- ターンベースの進行管理
- クリア条件の判定

### 2.3 入力処理
- `InputEvent` を受信し、ゲーム内アクションに変換
- `_input_direction` グローバル変数のセットアップ

### 2.4 システム調整
- RuleProcessor へのルール適用依頼
- HistoryManager との連携（undo/redo）

---

## 3. クラス定義

```typescript
export type GamePhase = "playing" | "won" | "paused";

export interface TurnResult {
  readonly state: GameState;
  readonly phase: GamePhase;
  readonly effects: readonly Effect[];
  readonly changed: boolean;
}

export class GameManager {
  private currentState: GameState;
  private readonly objectDB: ObjectDB;
  private readonly history: HistoryManager;
  private readonly gameData: GameData;
  private phase: GamePhase;
  private initialState: GameState;

  constructor(gameData: GameData);

  loadLevel(levelData: LevelData): GameState;
  processInput(input: InputEvent): TurnResult;
  getState(): GameState;
  getPhase(): GamePhase;

  private checkWinConditions(
    state: GameState,
    winConditions: readonly WinCondition[]
  ): boolean;
}
```

---

## 4. 主要な機能

### 4.1 レベル管理

#### `loadLevel(levelData: LevelData): GameState`

- レベルデータを読み込み、初期 GameState を構築
- 履歴をクリアし、初期状態をプッシュ
- `phase` を `"playing"` に設定
- `initialState` を保存（restart 用）

### 4.2 入力処理

#### `processInput(input: InputEvent): TurnResult`

プレイヤーの入力を受け取り、ゲーム状態を更新する。

**処理フロー:**

```
1. phase が "playing" でなければ無視（現在の状態をそのまま返す）
2. InputEvent の type に応じて分岐:
   - "undo": history.undo() を実行。状態があれば currentState を更新
   - "redo": history.redo() を実行。状態があれば currentState を更新
   - "restart": initialState を history.push して currentState を復帰
   - "move":
     a. globalState に _input_direction = input.direction をセット
     b. processTurn() でルール処理を実行
     c. _input_direction を "none" にリセット
     d. 結果の GameState を history.push
     e. winConditions を評価
   - "action":
     a. _input_direction = "none", _action = true をセット
     b. processTurn() を実行
     c. _input_direction, _action をリセット
     d. 結果を保存・評価
   - "wait":
     a. _input_direction = "none" をセット
     b. processTurn() を実行
     c. 結果を保存・評価
3. TurnResult を返す
```

**入力の設計（重要）:**

プレイヤーの移動は **ルールとして定義する**。GameManager は入力方向をグローバル変数 `_input_direction` に文字列としてセットし（`"up"`, `"down"`, `"left"`, `"right"`, `"none"`）、ルールが `_input_direction` を参照してプレイヤーを移動させる。これにより移動ロジックもデータ駆動になる。

### 4.3 状態取得

#### `getState(): GameState`

- 現在の GameState を返す（読み取り専用）

#### `getPhase(): GamePhase`

- 現在のゲームフェーズを返す

### 4.4 クリア条件判定

#### `checkWinConditions(state, winConditions): boolean`

`winConditions` の全条件を評価し、すべて満たされていれば `true` を返す。

| 条件タイプ | 判定ロジック |
|:---|:---|
| `"all"` | 指定オブジェクトが全て指定オブジェクトと同じセルにある |
| `"no"` | 指定オブジェクトが盤面に 1 つも存在しない |
| `"some"` | 指定オブジェクトが 1 つ以上指定オブジェクトと同じセルにある |
| `"global"` | グローバル変数の条件を `evaluateCondition` で評価 |

> アルゴリズムの詳細は `docs/フレームワーク計画.md` セクション 8.5 を参照。

---

## 5. 依存関係

| コンポーネント | 用途 | 関係 |
|:---|:---|:---|
| `RuleProcessor` | ルール適用、GameState 更新 | `processTurn()` を呼び出し |
| `ObjectDB` | オブジェクト定義情報の取得 | コンストラクタで初期化 |
| `HistoryManager` | undo/redo 機能 | コンストラクタで初期化 |
| `LevelLoader` | レベルデータの読み込み | `loadLevel()` で使用 |

---

## 6. 初期化フロー

```typescript
const gameData: GameData = /* JSON から読み込み */;
const manager = new GameManager(gameData);

// 内部で以下が実行される:
// 1. ObjectDB を gameData.objects から構築
// 2. HistoryManager を初期化
// 3. GameData を保持

// レベルロード
const levelData: LevelData = /* JSON から読み込み */;
const initialState = manager.loadLevel(levelData);
```

---

## 7. 状態遷移

| 状態 | 説明 | 可能な操作 |
|:---|:---|:---|
| `"playing"` | 通常のプレイ状態 | 入力処理、undo/redo、restart |
| `"won"` | クリア状態 | レベル変更のみ |
| `"paused"` | 一時停止状態 | 再開 |

---

## 8. TurnResult

`processInput` の戻り値 `TurnResult` は、レンダラーに渡す情報を含む。

```typescript
interface TurnResult {
  readonly state: GameState;     // 新しいゲーム状態
  readonly phase: GamePhase;     // 現在のフェーズ
  readonly effects: readonly Effect[];  // sound, message 等のエフェクト
  readonly changed: boolean;     // 状態が変化したか
}
```

レンダラーは `TurnResult` を受け取り：
- `state` を使って画面を再描画
- `effects` 内の `sound`, `message` を処理
- `phase` が `"won"` ならクリア演出を表示

---

## 9. エラーハンドリング

### 9.1 レベル読み込みエラー
- 不正な map（行長不揃い）→ 例外
- legend にないシンボルが map に含まれる → 例外

### 9.2 入力エラー
- `"playing"` 以外のフェーズでの入力 → 無視（現在状態をそのまま返す）
- undo/redo 不可能時 → 無視

### 9.3 ルール適用エラー
- RuleProcessor でのエラーは例外として伝播
