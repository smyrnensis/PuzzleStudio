# RuleProcessor 詳細仕様書

## 1. 概要

`RuleProcessor`は、ルールベース・グリッドパズルゲームフレームワークの中核コンポーネントです。現在の`GameState`に対して`RuleData`で定義されたルールを適用し、新しい`GameState`を生成します。

---

## 2. 依存関係

### 2.1 外部依存関係

| コンポーネント | 用途 | 必須 |
|:---|:---|:---|
| `ObjectDB` | オブジェクトの定義情報取得、プロパティ解決、インスタンス文字列の生成・解析 | ○ |
| `GameState` | 現在の盤面状態(`puzzleState`)とグローバル変数(`globalState`)の読み書き | ○ |
| `RuleData` | 適用すべきルールの定義リスト(JSON形式) | ○ |

### 2.2 内部データ構造

```gdscript
# 回転・反転された方向パターンのキャッシュ
var _direction_cache: Dictionary = {}

# パターンマッチング結果を格納する構造体
class MatchResult:
	var is_matched: bool
	var position: Vector2i
	var direction: String
	var matched_objects: Dictionary  # position -> matched_object_list
```

---

## 3. 公開API

### 3.1 メイン処理関数

```gdscript
func process_turn(current_gamestate: Dictionary, rules: Array) -> Dictionary
```

**役割**: 1ターン分のルール処理を実行し、新しいGameStateを返す
**処理フロー**:
1. GameStateの深いコピーを作成
2. ルールリストを順次処理
3. 変更されたGameStateを返却

### 3.2 初期化関数

```gdscript
func _init(object_db: ObjectDB)
```

**役割**: ObjectDBへの参照を設定し、内部キャッシュを初期化

---

## 4. 内部関数の詳細設計

### 4.1 ルール適用制御

#### `_apply_rule(gamestate: Dictionary, rule: Dictionary) -> bool`

**役割**: 単一ルールの適用制御
**戻り値**: 盤面に変化があったかどうか
**処理ロジック**:

```
1. ルールのenabled状態をチェック
2. applicationプロパティに基づく適用制御:
   - "once": 最大1回の適用（_apply_rule_onceを1回呼び出し）
   - "until_stable": 安定するまで繰り返し（_apply_rule_onceをfalseが返るまでループ）
3. 各適用で_apply_rule_onceを呼び出し
4. 変化の有無を追跡して返却
```

#### `_apply_rule_once(gamestate: Dictionary, rule: Dictionary) -> bool`

**役割**: ルールの1回分の適用処理
**戻り値**: この1回で盤面に変化があったかどうか
**処理ロジック**:

```
1. ルールのdirectionから適用方向リストを取得
2. 各方向について順次処理:
   a. 盤面を左上(0,0)から右下へ順次スキャン
   b. 最初にマッチした箇所で：
      - 条件判定をパス
      - 盤面更新
      - 即座にtrueを返却（変化あり）
3. 全方向・全位置を探索してもマッチしなかった場合、falseを返却
```

### 4.2 パターンマッチング

#### スキャン順序の重要性

PuzzleScriptとの互換性を保つため、パターンマッチングは**厳密な順序**で実行されます：

1. **方向優先**: 各適用方向を順番に処理（"up" → "down" → "left" → "right"）
2. **位置順序**: 各方向について、盤面を左上(0,0)から右下へ順次スキャン
3. **即座に適用**: 最初にマッチした箇所で即座に置換を実行

この方式により、ルールの適用結果が**決定論的**かつ**予測可能**になります。

#### `_find_first_pattern_match(puzzleState: Dictionary, rule: Dictionary) -> MatchResult`

**役割**: 盤面を左上から順次探索し、最初にマッチした箇所を返す
**戻り値**: 最初のマッチ結果（マッチしない場合はnull）
**処理ロジック**:

```
1. ルールのdirectionプロパティから適用方向リストを生成
2. 適用方向を1つ定める
3. 盤面を左上(0,0)から右下へ順次スキャン
3. 盤面全体を探索してもマッチしなかった場合はnullを返却
```

#### `_scan_puzzle_for_pattern_match(puzzleState: Dictionary, rule: Dictionary) -> MatchResult`

**役割**: 盤面を左上から順次スキャンし、パターンマッチングを実行
**戻り値**: 最初にマッチした結果（なければnull）
**処理ロジック**:

```
1. ルールのdirectionから適用方向リストを取得
2. 各方向について順次処理:
   a. y座標を0から最大値まで順次処理:
     i. x座標を0から最大値まで順次処理:
       - _match_pattern_at_positionを呼び出し
       - マッチした場合は即座にMatchResultを返却
3. 全方向・全位置を試してもマッチしない場合、nullを返却
```

#### `_get_direction_patterns(direction: String) -> Array[String]`

**役割**: direction指定から実際の適用方向リストを生成
**処理ロジック**:

```
- "any": ["up", "down", "left", "right"]
- "vertical": ["up", "down"]  
- "horizontal": ["left", "right"]
- "up"/"down"/"left"/"right": [指定方向のみ]
```

#### `_match_pattern_at_position(puzzleState: Dictionary, rule: Dictionary, position: Vector2i, direction: String) -> MatchResult`

**役割**: 特定位置・方向でのパターンマッチング実行
**戻り値**: マッチした場合はMatchResult、しなかった場合はnull
**処理ロジック**:

```
1. beforeパターンの各要素について:
   a. 相対座標を絶対座標に変換
   b. 座標が盤面範囲内かチェック
   c. セル内容を取得
   d. objects/has_objects/no_objectsの条件をチェック
2. いずれかの条件が不成立なら即座にnullを返却
3. すべての条件が満たされた場合、MatchResultを生成して返却
4. マッチしたオブジェクトの詳細を記録
```

### 4.3 座標・方向変換

#### `_transform_pattern_for_direction(pattern: Array, direction: String) -> Array`

**役割**: パターンを指定方向に合わせて回転・反転
**処理ロジック**:

```
1. 基準方向("right")からの回転角度を計算
2. パターン内の各要素について:
   a. position座標を回転変換
   b. パターンの左上が(0, 0)になるように相対座標を並進変換
   c. 相対方向タグ(>, <, ^, v)を絶対方向に変換
3. 変換されたパターンを返却
```

#### `_get_absolute_position(base_position: Vector2i, relative_position: Vector2i, direction: String) -> Vector2i`

**役割**: 相対座標を絶対座標に変換

#### `_transform_relative_direction_tag(tag: String, direction: String) -> String`

**役割**: 相対方向タグを絶対方向に変換
**処理ロジック**:

```
">" -> 適用方向に応じて "right", "down", "left", "up"
"<" -> 適用方向に応じて "left", "up", "right", "down"  
"^" -> 適用方向に応じて "up", "left", "down", "right"
"v" -> 適用方向に応じて "down", "right", "up", "left"
```

### 4.4 オブジェクト解決

#### `_resolve_object_specification(spec: String, cell_objects: Array) -> Array`

**役割**: ルール内のオブジェクト指定を実際のオブジェクトに解決
**処理ロジック**:

```
1. プロパティ指定(@Movable)の場合:
   a. セル内の各オブジェクトについてプロパティをチェック
   b. 条件に合うオブジェクトを収集
2. 否定指定(!Movable)の場合:
   a. 条件に合わないオブジェクトを収集
3. 複数条件(||)の場合:
   a. OR条件で評価
4. 具体的名前指定の場合:
   a. 名前とタグの完全一致または部分一致をチェック
```

#### `_match_object_against_spec(object_string: String, spec: String) -> bool`

**役割**: 単一オブジェクトが仕様に一致するかチェック
**処理ロジック**:

```
1. オブジェクト文字列を解析(name, tags)
2. spec種別を判定:
   - プロパティ指定: ObjectDBでプロパティをチェック
   - 具体名指定: 名前とタグの一致をチェック
   - 相対方向指定: タグを変換してチェック
```

### 4.5 条件判定

#### `_check_global_conditions(globalState: Dictionary, conditions: Dictionary) -> bool`

**役割**: ルールのconditionsブロックで指定された条件をチェック
**処理ロジック**:

```
1. conditions内の各キー・値ペアについて:
   a. globalStateから該当変数を取得
   b. 比較演算子に基づいて評価
   c. いずれかの条件が不成立なら false を返却
2. すべての条件が成立した場合 true を返却
```

#### `_evaluate_condition(actual_value: Variant, expected: Variant) -> bool`

**役割**: 単一条件の評価
**処理ロジック**:

```
expected が辞書の場合:
  - "eq": actual_value == expected["eq"]
  - "ne": actual_value != expected["ne"]  
  - "gt": actual_value > expected["gt"]
  - "gte": actual_value >= expected["gte"]
  - "lt": actual_value < expected["lt"]
  - "lte": actual_value <= expected["lte"]
expected が値の場合:
  - actual_value == expected (デフォルトはeq)
```

### 4.6 状態更新

#### `_apply_pattern_update(puzzleState: Dictionary, rule: Dictionary, match_result: MatchResult)`

**役割**: マッチしたパターンに対してafterブロックによる更新を適用
**処理ロジック**:

```
1. afterパターンの各要素について:
   a. 対応するbeforeのobjectsで指定されたセルを特定
   b. マッチしたオブジェクトをセルから削除
   c. afterで指定された新しいオブジェクトを追加
2. 相対方向タグを絶対方向に変換
3. オブジェクトインスタンス文字列を正規化
```

**重要**: この関数はマッチ1件につき1回だけ呼び出される。複数マッチを同時処理することはない。

#### `_apply_effects(globalState: Dictionary, effects: Dictionary) -> Array`

**役割**: ルールのeffectsブロックを適用
**戻り値**: 発生したイベント(sound, message)のリスト
**処理ロジック**:

```
1. "set": globalStateの変数を指定値に設定
2. "change": globalStateの変数を指定値だけ増減
3. "sound": サウンドイベントをイベントリストに追加
4. "message": メッセージイベントをイベントリストに追加
```

### 4.7 セル操作

#### `_get_cell_objects(puzzleState: Dictionary, position: Vector2i, layer_id: int = -1) -> Array`

**役割**: 指定位置のセルからオブジェクトリストを取得
**処理ロジック**:

```
1. 座標が盤面範囲内かチェック
2. layer_id指定がある場合は該当レイヤーのみ
3. layer_id指定がない場合はすべてのレイヤーを統合
```

#### `_set_cell_objects(puzzleState: Dictionary, position: Vector2i, layer_id: int, objects: Array)`

**役割**: 指定位置のセルにオブジェクトリストを設定

#### `_remove_objects_from_cell(puzzleState: Dictionary, position: Vector2i, layer_id: int, objects_to_remove: Array)`

**役割**: セルから特定のオブジェクトのみを削除
**処理ロジック**:

```
1. 現在のセル内容を取得
2. 削除対象オブジェクトを除外した新しいリストを作成
3. セルを更新
```

#### `_add_objects_to_cell(puzzleState: Dictionary, position: Vector2i, layer_id: int, objects_to_add: Array)`

**役割**: セルに新しいオブジェクトを追加

### 4.8 ユーティリティ

#### `_normalize_object_string(object_string: String, direction: String) -> String`

**役割**: オブジェクト文字列内の相対方向タグを絶対方向に正規化

#### `_is_position_valid(puzzleState: Dictionary, position: Vector2i) -> bool`

**役割**: 座標が盤面範囲内かチェック

#### `_deep_copy_gamestate(gamestate: Dictionary) -> Dictionary`

**役割**: GameStateの深いコピーを作成（参照を完全に分離）

---

## 5. エラーハンドリング

### 5.1 入力検証

- ルールデータの必須フィールド存在チェック
- 座標の範囲チェック
- オブジェクト名の定義存在チェック

### 5.2 実行時エラー

- 無効なプロパティ指定に対する警告出力
- 存在しないオブジェクトへの参照に対するエラー処理
- メモリ不足時の適切なフォールバック

### 5.3 デバッグサポート

- 詳細なログ出力オプション
- パフォーマンス計測機能
- ルール適用ステップの可視化機能

---

## 6. パフォーマンス考慮事項

### 6.1 最適化戦略

- パターン変換結果のキャッシュ
- 頻繁にアクセスされるオブジェクト情報のキャッシュ
- 早期リターンによる不要な計算の回避

### 6.2 メモリ管理

- 大きなGameStateのコピー最小化
- 一時的なデータ構造の適切な解放
- キャッシュサイズの制限

### 6.3 計算量

- パターンマッチング: O(盤面サイズ × パターンサイズ × 方向数) ※最悪ケース
- 実際の動作: 最初のマッチで即座に処理が終わるため、平均的にはO(1)からO(盤面サイズ)
- オブジェクト解決: O(セル内オブジェクト数 × プロパティ数)
- 全体的な複雑度: O(ルール数 × 平均マッチ位置 × 最大適用回数)

**注意**: "until_stable"ルールの場合、最悪ケースでは指数的な時間がかかる可能性があるため、適用回数の制限やタイムアウト機構が必要。