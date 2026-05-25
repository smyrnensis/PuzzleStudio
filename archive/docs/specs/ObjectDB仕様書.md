# ObjectDB 仕様書

> 詳細な API は `docs/フレームワーク計画.md` セクション 7.4 を参照。
> オブジェクト文字列ユーティリティは `docs/フレームワーク計画.md` セクション 7.1 を参照。

---

## 1. 概要

`ObjectDB` は、ゲームのオブジェクト定義（`ObjectDefinition[]`）を一元管理するクラスである。コンストラクタでオブジェクト定義の配列を受け取り、名前によるルックアップやプロパティの逆引きを効率的に行う。

---

## 2. 責務

- **定義の管理**: `ObjectDefinition[]` をコンストラクタで受け取り、内部 Map に格納
- **情報提供**: オブジェクト名から定義情報（プロパティ、レイヤーID、タグ等）を返す
- **プロパティ逆引き**: プロパティ名から該当する全オブジェクト名を返す（例: `"Movable"` → `["Player", "Box"]`）
- **パターン解決**: `@Movable` のようなプロパティ指定を実際のオブジェクト名リストに展開

---

## 3. クラス定義

```typescript
import type { ObjectDefinition } from "../types/index.js";

export class ObjectDB {
  constructor(definitions: readonly ObjectDefinition[]);

  /** 名前でオブジェクト定義を取得。未定義名で例外 */
  getDefinition(name: string): ObjectDefinition;

  /** プロパティ名に該当する全オブジェクト名を返す */
  getObjectsByProperty(propertyName: string): readonly string[];

  /** 指定オブジェクトが指定プロパティを持つか判定 */
  hasProperty(objectName: string, propertyName: string): boolean;

  /** パターン文字列を解決。"@Movable" → ["Player", "Box"]、通常名はそのまま */
  resolvePattern(pattern: string): readonly string[];

  /** 全オブジェクト定義を返す */
  getAllDefinitions(): readonly ObjectDefinition[];
}
```

---

## 4. 内部実装の方針

- コンストラクタで以下の 2 つの Map を構築：
  - `Map<string, ObjectDefinition>`: オブジェクト名 → 定義（O(1) ルックアップ）
  - `Map<string, string[]>`: プロパティ名 → オブジェクト名リスト（逆引き用）
- `resolvePattern`: `@` プレフィックスならプロパティ逆引き Map を使用、なければ `[pattern]` をそのまま返す
- ファイルI/Oは行わない。データはコンストラクタの引数として外部から渡される（engine パッケージは外部依存ゼロ）

---

## 5. 使用例

```typescript
import { ObjectDB } from "@puzzlemaker/engine";

// GameData.objects を渡してインスタンス化
const objectDB = new ObjectDB(gameData.objects);

// 定義の取得
const playerDef = objectDB.getDefinition("Player");
// → { name: "Player", layerId: 1, properties: ["Movable", "PlayerControlled"], ... }

// プロパティ逆引き
objectDB.getObjectsByProperty("Movable");
// → ["Player", "Box"]

// プロパティ判定
objectDB.hasProperty("Player", "Movable");
// → true

// パターン解決
objectDB.resolvePattern("@Solid");
// → ["Wall", "Box"]

objectDB.resolvePattern("Player");
// → ["Player"]
```

---

## 6. オブジェクト文字列ユーティリティ

オブジェクトインスタンス文字列（`"Name:tag1:tag2"`）の操作は、ObjectDB とは別モジュール `utils/object-string.ts` が担当する。

| 関数 | 説明 | 例 |
|:---|:---|:---|
| `parseObjectString` | 文字列を名前 + タグに分解 | `"Player:right"` → `{ name: "Player", tags: ["right"] }` |
| `buildObjectString` | 名前 + タグから文字列を生成 | `("Player", ["right"])` → `"Player:right"` |
| `getObjectName` | 名前のみ取得 | `"Player:right"` → `"Player"` |
| `getObjectTags` | タグのみ取得 | `"Player:right"` → `["right"]` |
| `matchObjectPattern` | パターンマッチ（タグバインディング対応） | `("Player:red", "Player:$color", {})` → `{ "$color": "red" }` |
| `applyBindings` | バインディング適用 | `("Box:$color", { "$color": "red" })` → `"Box:red"` |

> 詳細は `docs/フレームワーク計画.md` セクション 7.1 を参照。
