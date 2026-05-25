# GameManager仕様書

## 1. 概要

`GameManager`は、ルールベース・グリッドパズルゲームフレームワークの中核コンポーネントです。ゲーム全体の進行制御、GameStateの管理、プレイヤー入力の処理、そして各システム間の調整を担当します。

---

## 2. 基本責務

### 2.1. GameState管理
- GameStateの唯一の所有者として機能
- GameStateの読み込み、保存、初期化
- GameStateの整合性保証

### 2.2. ゲーム進行制御
- レベルの開始、終了、切り替え
- ターンベースの進行管理
- ゲームオーバー、クリア条件の判定

### 2.3. 入力処理
- プレイヤーの入力（キーボード、マウス等）を受信
- 入力をゲーム内アクションに変換
- 無効な入力の処理

### 2.4. システム調整
- RuleProcessorへのルール適用依頼
- HistoryManagerとの連携（undo/redo）
- UIシステムとの連携

---

## 3. 主要な機能

### 3.1. レベル管理

#### `load_level(level_id: String) -> bool`
- 指定されたレベルデータを読み込み
- GameStateを初期化
- レベル固有のルールやオブジェクト配置を設定
#### `next_level() -> bool`
- 次のレベルに進む
- レベル進行条件の確認

### 3.2. ターン処理

#### `restart_level() -> bool`
- 現在のレベルを最初の状態にリセット
- 履歴はそのまま保持 (restartをundoできる)

#### `undo() -> bool`
- 前の状態に戻す
- HistoryManagerと連携

#### `redo() -> bool`
- undoする前の状態に戻す
- HistoryManagerと連携
#### `process_player_input(input_type: String, direction: Vector2i = Vector2i.ZERO) -> bool`
- プレイヤーの入力を受け取り、1ターンを実行
- RuleProcessorを呼び出してGameStateを更新
- 変化を履歴に保存
### 3.3. 状態取得

#### `get_current_gamestate() -> Dictionary`
- 現在のGameStateの読み取り専用コピーを返す
- 外部からの直接変更を防ぐ

#### `get_global_variable(key: String) -> Variant`
- 指定されたグローバル変数の値を取得

#### `get_cell_objects(x: int, y: int) -> Array[String]`
- 指定座標のセルに含まれるオブジェクト一覧を取得

### 3.4. ゲーム状態判定

#### `is_level_complete() -> bool`
- 現在のレベルがクリア状態かを判定
- global_state内のクリア条件をチェック

---

## 4. イベント/シグナル

GameManagerは以下のシグナルを発信し、UIや他のシステムに状態変化を通知します。

### 4.1. ゲーム進行イベント
- `level_started(level_id: String)`: レベル開始時
- `level_completed(level_id: String, moves: int, time: int)`: レベルクリア時
- `game_reset()`: ゲームリセット時

### 4.2. ターンイベント
- `turn_processed(changes_made: bool)`: ターン処理完了時
- `gamestate_changed(new_state: Dictionary)`: GameState変更時
- `invalid_move_attempted(reason: String)`: 無効な移動試行時

### 4.3. システムイベント
- `rule_effect_triggered(effect_type: String, data: Dictionary)`: ルール効果発生時
  - サウンド再生要求 (`effect_type: "sound"`)
  - メッセージ表示要求 (`effect_type: "message"`)

---

## 5. 依存関係

### 5.1. 必須依存
| コンポーネント | 用途 | 関係 |
|:---|:---|:---|
| `RuleProcessor` | ルール適用、GameState更新 | GameManagerが呼び出し |
| `ObjectDB` | オブジェクト定義情報の取得 | 初期化時に参照設定 |
| `LevelLoader` | レベルデータの読み込み | レベル変更時に使用 |

### 5.2. オプション依存
| コンポーネント | 用途 | 関係 |
|:---|:---|:---|
| `HistoryManager` | undo/redo機能 | 存在する場合のみ使用 |
| `SaveManager` | セーブ/ロード機能 | 存在する場合のみ使用 |

---

## 6. 初期化と設定

### 6.1. 初期化フロー
1. ObjectDBの参照を取得
2. RuleProcessorを初期化
3. オプションシステム（History、Save等）の初期化
4. デフォルトレベルの読み込み
### 6.2. 設定項目
```gdscript
# GameManager設定例
var settings: Dictionary = {
	"auto_save": true,           # 自動セーブ有効
	"default_level": "level_01", # 初期レベル
	"debug_mode": false          # デバッグ出力有効
}
```

---

## 7. 状態遷移

GameManagerは以下の内部状態を持ちます：

| 状態        | 説明       | 可能な操作      |
| :-------- | :------- | :--------- |
| `LOADING` | 処理中      | なし         |
| `PLAYING` | 通常のプレイ状態 | 入力処理、undo等 |
| `PAUSED`  | 一時停止状態   | 再開、設定変更    |



---

## 8. エラーハンドリング

### 8.1. レベル読み込みエラー
- 存在しないレベルIDの処理
- 壊れたレベルデータの処理
- フォールバック動作の定義

### 8.2. ルール適用エラー
- RuleProcessorでのエラー処理
- GameState不整合時の復旧
- ログ出力とデバッグ情報

### 8.3. 入力エラー
- 無効な入力の適切な処理
- エラーメッセージの表示
- 状態の保持

---

## 9. パフォーマンス考慮事項

### 9.1. GameStateコピー
- 読み取り専用アクセス時の軽量コピー戦略
- 大きなpuzzle_stateのコピー最適化

### 9.2. 履歴管理
- 履歴サイズの制限
- 古い履歴の自動削除

### 9.3. イベント通知
- 不要な通知の抑制
- バッチング処理の検討

この仕様により、GameManagerはゲーム全体の中央制御塔として機能し、各システム間の調整と状態管理を効率的に行うことができます。