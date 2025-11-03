# OpenSpec インストラクション

OpenSpec を用いて仕様駆動開発を進める AI コーディングアシスタントのための指針です。

## TL;DR クイックチェックリスト

- 既存の作業を検索: `openspec spec list --long`, `openspec list`（全文検索が必要なときだけ `rg` を使用）
- スコープを判断: 新しいケイパビリティか、既存ケイパビリティの更新か
- 一意な `change-id` を決める: ケバブケースで動詞始まり（`add-`, `update-`, `remove-`, `refactor-` など）
- ひな形を準備: `proposal.md`、`tasks.md`、必要に応じて `design.md`、そして影響するケイパビリティごとの差分スペック
- 差分を書く: `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` を使い、各要件に最低 1 つ `#### Scenario:` を含める
- バリデーション: `openspec validate [change-id] --strict` を実行し、問題を修正する
- 承認の取得: 提案が承認されるまで実装を開始しない

## 3 段階ワークフロー

### ステージ 1: 変更の作成
以下の場合に提案を作成する:
- 機能や挙動を追加するとき
- 破壊的変更を行うとき（API やスキーマなど）
- アーキテクチャやパターンを変更するとき
- パフォーマンスを向上させる（挙動が変わる）とき
- セキュリティのパターンを更新するとき

トリガー例:
- 「変更提案を作りたい」
- 「変更計画を手伝ってほしい」
- 「提案をまとめたい」
- 「仕様提案を作りたい」
- 「仕様を作りたい」

ゆるい判定基準:
- `proposal`、`change`、`spec` のいずれかを含む
- かつ `create`、`plan`、`make`、`start`、`help` のいずれかを含む

提案をスキップできるケース:
- バグ修正（想定どおりの挙動に戻すだけ）
- タイポ、整形、コメントのみ
- 非破壊的な依存関係更新
- 設定ファイルの変更
- 既存挙動を検証するテストの追加

**ワークフロー**
1. 現在の状況を理解するために `openspec/project.md`、`openspec list`、`openspec list --specs` を確認する。
2. 動詞始まりの一意な `change-id` を選び、`openspec/changes/<id>/` に `proposal.md`、`tasks.md`、必要なら `design.md` とスペック差分を用意する。
3. 差分スペックでは各要件に最低 1 つの `#### Scenario:` を含めつつ `## ADDED|MODIFIED|REMOVED Requirements` を使って記述する。
4. 提案を共有する前に `openspec validate <id> --strict` を実行し、すべての問題を解消する。

### ステージ 2: 変更の実装
次の手順を TODO として管理し、順番に完了させる。
1. **proposal.md を読む** — 作業対象を理解する
2. **design.md を読む**（存在する場合）— 技術的判断を確認する
3. **tasks.md を読む** — 実装チェックリストを把握する
4. **タスクを順番に実装する** — 記載順に進める
5. **完了を確認する** — `tasks.md` の各項目が終わっていることを確認してからステータスを更新する
6. **チェックリストを更新する** — すべて完了したら該当項目を `- [x]` にして現状を反映する
7. **承認ゲート** — 提案がレビュー・承認されるまでは実装を開始しない

### ステージ 3: 変更のアーカイブ
リリース後は別 PR で次を行う:
- `changes/[name]/` を `changes/archive/YYYY-MM-DD-[name]/` へ移動
- ケイパビリティに変更がある場合は `specs/` を更新
- ツール類のみの変更は `openspec archive <change-id> --skip-specs --yes` を使用（必ず change ID を明示する）
- `openspec validate --strict` を走らせ、アーカイブした変更がチェックを通ることを確認

## タスク着手前チェック

**コンテキストチェックリスト:**
- [ ] `specs/[capability]/spec.md` 内の該当スペックを読む
- [ ] `changes/` に未完了の変更がないか確認する
- [ ] `openspec/project.md` で規約を確認する
- [ ] `openspec list` でアクティブな変更を確認する
- [ ] `openspec list --specs` で既存ケイパビリティを確認する

**スペック作成前:**
- 同じケイパビリティが既に存在しないか必ず確認する
- 重複を避けるため、既存スペックの修正を優先する
- `openspec show [spec]` で現状を確認する
- 依頼が曖昧な場合は 1〜2 件の質問で明確化してから作業する

### 検索ガイド
- スペック一覧: `openspec spec list --long`（スクリプト用途は `--json`）
- 変更一覧: `openspec list`（非推奨だが `openspec change list --json` も利用可能）
- 詳細表示:
  - スペック: `openspec show <spec-id> --type spec`（フィルタ用に `--json`）
  - 変更: `openspec show <change-id> --json --deltas-only`
- 全文検索は ripgrep を使用: `rg -n "Requirement:|Scenario:" openspec/specs`

## クイックスタート

### CLI コマンド

```bash
# 基本コマンド
openspec list                  # 進行中の変更を一覧
openspec list --specs          # 仕様一覧
openspec show [item]           # 変更または仕様の内容を表示
openspec validate [item]       # 変更または仕様を検証
openspec archive <change-id> [--yes|-y]   # リリース後にアーカイブ（対話なしは --yes）

# プロジェクト管理
openspec init [path]           # OpenSpec を初期化
openspec update [path]         # 指示ファイルを更新

# 対話モード
openspec show                  # 対話的に対象を選択
openspec validate              # 一括バリデーション

# デバッグ
openspec show [change] --json --deltas-only
openspec validate [change] --strict
```

## プロジェクト固有メモ

- ローカル開発は `docker/` ディレクトリで uv 管理の仮想環境を作成する（`uv venv .venv` → `source .venv/bin/activate`）。
- 依存関係はコミット済みの `uv.lock` / `uv.dev.lock` を用いて `uv pip sync` で同期する。
- Lint・テスト・型チェックは uv 環境内から `pytest tests`、`mypy --pretty src`、`ruff check src`、`black src tests` を直接実行する。
- Docker イメージは配布フォーマットであり、`docker-compose.yml` には配布向けサービスのみが定義されている（リリース検証で活用する）。

### コマンドフラグ

- `--json` - 機械可読な出力
- `--type change|spec` - アイテムの種類を指定
- `--strict` - 厳密な検証
- `--no-interactive` - プロンプトなしで実行
- `--skip-specs` - スペック更新なしでアーカイブ
- `--yes`/`-y` - 確認プロンプトをスキップ（非対話アーカイブ向け）

## ディレクトリ構成

```
openspec/
├── project.md              # プロジェクト規約
├── specs/                  # すでに構築済みの真実
│   └── [capability]/       # 単一目的のケイパビリティ
│       ├── spec.md         # 要件とシナリオ
│       └── design.md       # 技術パターン
├── changes/                # 変更提案（これから変わるもの）
│   ├── [change-name]/
│   │   ├── proposal.md     # Why / What / Impact
│   │   ├── tasks.md        # 実装チェックリスト
│   │   ├── design.md       # 技術判断（必要なら）
│   │   └── specs/          # 差分スペック
│   │       └── [capability]/
│   │           └── spec.md # ADDED/MODIFIED/REMOVED
│   └── archive/            # 完了した変更
```

## 変更提案の作成

### 判断ツリー

```
新しい依頼？
├─ 仕様どおりの挙動へ戻すバグ修正 → 直接修正
├─ タイポ／整形／コメント → 直接修正
├─ 新機能／新ケイパビリティ → 提案を作成
├─ 破壊的変更 → 提案を作成
├─ アーキテクチャ変更 → 提案を作成
└─ 判断がつかない → 提案を作る方が安全
```

### 提案の構成

1. **ディレクトリを用意:** `changes/[change-id]/`（ケバブケースで動詞始まり、重複不可）

2. **proposal.md を記述:**
```markdown
## Why
[課題や機会を 1〜2 文で]

## What Changes
- [変更点の箇条書き]
- [破壊的変更は **BREAKING** を付記]

## Impact
- 影響するスペック: [ケイパビリティ一覧]
- 影響するコード: [主なファイル／システム]
```

3. **スペック差分を作成:** `specs/[capability]/spec.md`
```markdown
## ADDED Requirements
### Requirement: New Feature
The system SHALL provide...

#### Scenario: Success case
- **WHEN** user performs action
- **THEN** expected result

## MODIFIED Requirements
### Requirement: Existing Feature
[変更後の要件全文]

## REMOVED Requirements
### Requirement: Old Feature
**Reason**: [削除理由]
**Migration**: [移行方法]
```
複数のケイパビリティに影響する場合は `changes/[change-id]/specs/<capability>/spec.md` をそれぞれ作成する。

4. **tasks.md を作成:**
```markdown
## 1. Implementation
- [ ] 1.1 データベーススキーマを作成
- [ ] 1.2 API エンドポイントを実装
- [ ] 1.3 フロントエンドコンポーネントを実装
- [ ] 1.4 テストを書く
```

5. **必要なときだけ design.md を作成:**
以下のいずれかに該当する場合は `design.md` を追加し、それ以外は省略する:
- サービス／モジュールをまたぐ横断的な変更、または新しいアーキテクチャパターン
- 新しい外部依存や大きなデータモデル変更
- セキュリティ、パフォーマンス、移行の複雑性が高い変更
- 実装前に技術判断を整理した方がよい曖昧さが残っている場合

最小の `design.md` ひな形:
```markdown
## Context
[背景、制約、ステークホルダー]

## Goals / Non-Goals
- Goals: [...]
- Non-Goals: [...]

## Decisions
- Decision: [内容と理由]
- Alternatives considered: [選択肢と判断理由]

## Risks / Trade-offs
- [リスク] → 対応策

## Migration Plan
[手順、ロールバック方法]

## Open Questions
- [...]
```

## スペックファイルのフォーマット

### 重要: シナリオの書式

**正しい例**（`####` ヘッダーを使用）:
```markdown
#### Scenario: User login success
- **WHEN** valid credentials provided
- **THEN** return JWT token
```

**誤った例**（箇条書きや太字は不可）:
```markdown
- **Scenario: User login**  ❌
**Scenario**: User login     ❌
### Scenario: User login      ❌
```

すべての要件には少なくとも 1 つのシナリオが必要。

### 要件の表現
- 規範的な要件には SHALL/MUST を使用（意図的に非拘束とする場合以外は should/may を避ける）

### 差分操作

- `## ADDED Requirements` - 新しいケイパビリティ
- `## MODIFIED Requirements` - 既存挙動の変更
- `## REMOVED Requirements` - 廃止する機能
- `## RENAMED Requirements` - 名称変更

ヘッダーは `trim(header)` で比較されるため、前後の空白は無視される。

#### ADDED と MODIFIED の使い分け
- ADDED: 独立した要件として成立する新しい機能や副次機能を追加するとき。既存の要件に影響しない場合は ADDED を優先。
- MODIFIED: 既存要件の挙動・範囲・受け入れ条件が変わるとき。ヘッダーとシナリオを含む要件全文を貼り付け、更新後の姿をそのまま記述する。部分更新だけでは旧情報が欠落する。
- RENAMED: 名称だけ変える場合に使用。挙動も変えるなら RENAMED（名称）と MODIFIED（内容）の両方を記述する。

ありがちな落とし穴: MODIFIED で新しい論点を追加したのに既存要件の全文を含めないケース。アーカイブ時に旧情報が失われるので、既存要件を丸ごと貼ってから編集すること。

MODIFIED 要件の正しい手順:
1) `openspec/specs/<capability>/spec.md` から該当要件を見つける。
2) `### Requirement: ...` からシナリオまでのブロックをコピーする。
3) `## MODIFIED Requirements` 配下に貼り付け、変更内容を反映させる。
4) ヘッダーの文言を完全一致させ（空白は無視）、少なくとも 1 つ `#### Scenario:` を残す。

RENAMED の例:
```markdown
## RENAMED Requirements
- FROM: `### Requirement: Login`
- TO: `### Requirement: User Authentication`
```

## トラブルシューティング

### よくあるエラー

**「Change must have at least one delta」**
- `changes/[name]/specs/` に .md ファイルが存在することを確認
- ファイルに操作種別ヘッダー（## ADDED Requirements など）があるか確認

**「Requirement must have at least one scenario」**
- `#### Scenario:` 形式で書かれているか確認
- 箇条書きや太字で書いていないか確認

**シナリオ解析が黙って失敗する場合**
- 正しい書式は `#### Scenario: 名前`。これ以外は認識されない。
- `openspec show [change] --json --deltas-only` でデバッグする。

### バリデーションのコツ

```bash
# 厳密なチェックを常に利用
openspec validate [change] --strict

# 差分解析をデバッグ
openspec show [change] --json | jq '.deltas'

# 特定の要件を確認
openspec show [spec] --json -r 1
```

## ハッピーパスの例

```bash
# 1) 現状を調べる
openspec spec list --long
openspec list
# 任意の全文検索:
# rg -n "Requirement:|Scenario:" openspec/specs
# rg -n "^#|Requirement:" openspec/changes

# 2) change-id を決めてひな形を作成
CHANGE=add-two-factor-auth
mkdir -p openspec/changes/$CHANGE/{specs/auth}
printf "## Why\n...\n\n## What Changes\n- ...\n\n## Impact\n- ...\n" > openspec/changes/$CHANGE/proposal.md
printf "## 1. Implementation\n- [ ] 1.1 ...\n" > openspec/changes/$CHANGE/tasks.md

# 3) 差分スペックを追加（例）
cat > openspec/changes/$CHANGE/specs/auth/spec.md << 'EOF'
## ADDED Requirements
### Requirement: Two-Factor Authentication
Users MUST provide a second factor during login.

#### Scenario: OTP required
- **WHEN** valid credentials are provided
- **THEN** an OTP challenge is required
EOF

# 4) バリデーション
openspec validate $CHANGE --strict
```

## 複数ケイパビリティの例

```
openspec/changes/add-2fa-notify/
├── proposal.md
├── tasks.md
└── specs/
    ├── auth/
    │   └── spec.md   # ADDED: Two-Factor Authentication
    └── notifications/
        └── spec.md   # ADDED: OTP Email Notification
```

auth/spec.md
```markdown
## ADDED Requirements
### Requirement: Two-Factor Authentication
...
```

notifications/spec.md
```markdown
## ADDED Requirements
### Requirement: OTP Email Notification
...
```

## ベストプラクティス

### シンプル第一
- 新規コードは基本 100 行未満に抑える
- 単一ファイル実装から始め、必要になるまで分割しない
- 明確な理由がなければ新たなフレームワーク導入を避ける
- 地に足のついた実績あるパターンを選ぶ

### 複雑化のトリガー
以下の状況でのみ複雑さを追加する:
- 現状では性能が足りないと示すデータがある
- 具体的なスケール要求（1000 ユーザー超、100MB 超など）がある
- 複数の確かなユースケースが抽象化を求めている

### 明快な参照
- コード位置は `file.ts:42` 形式
- スペック参照は `specs/auth/spec.md`
- 関連する変更や PR もリンクする

### ケイパビリティ命名
- 動詞 + 名詞の組み合わせ（例: `user-auth`, `payment-capture`）
- ケイパビリティは単一目的に絞る
- 「10 分で理解できる」ことを目安にする
- 説明に AND が必要になるなら分割を検討

### change-id の命名
- ケバブケースで短く説明的に: `add-two-factor-auth`
- 動詞始まりを推奨: `add-`、`update-`、`remove-`、`refactor-` など
- 重複していたら `-2`, `-3` を付けて調整

## ツール選択ガイド

| タスク | ツール | 理由 |
|------|------|-----|
| パターンでファイル検索 | Glob | パターン検索が高速 |
| コード内容を検索 | Grep | 正規表現検索に最適化 |
| 特定ファイルを読む | Read | ファイルに直接アクセス |
| スコープ調査 | Task | 複数ステップの調査に適する |

## エラー復旧

### 変更の衝突
1. `openspec list` を実行してアクティブな変更を確認
2. 重複するスペックがないか調べる
3. 変更の担当者と調整する
4. 必要なら提案を統合する

### バリデーション失敗
1. `--strict` フラグで再実行
2. JSON 出力で詳細を確認
3. スペックファイルのフォーマットを確認
4. シナリオが正しい形式かチェック

### コンテキスト不足
1. まず project.md を読む
2. 関連スペックを確認
3. 最近アーカイブされた変更を読む
4. 不明点があれば質問する

## クイックリファレンス

### ステージの目安
- `changes/` - 提案中で未実装
- `specs/` - 実装済み・デプロイ済み
- `archive/` - 完了してアーカイブ済み

### 各ファイルの用途
- `proposal.md` - Why / What
- `tasks.md` - 実装ステップ
- `design.md` - 技術判断
- `spec.md` - 要件と挙動

### CLI の要点
```bash
openspec list              # 進行中の変更は？
openspec show [item]       # 詳細を確認
openspec validate --strict # 検証は通る？
openspec archive <change-id> [--yes|-y]  # 完了したらアーカイブ（自動化は --yes）
```

覚えておくべきこと: スペックが唯一の真実。変更は提案。両者を常に同期させること。
