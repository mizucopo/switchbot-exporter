## Why
- `openspec archive` が `repo-guidelines` スペックのヘッダー不足で失敗しており、アーカイブフローが中断されている。
- 既存スペックが最新のエージェントガイド構造（`Agent Instructions Structure` 要件）を反映していないため、ツールが要求する差分を適用できない。

## What Changes
- `openspec/specs/repo-guidelines/spec.md` に `Agent Instructions Structure` 要件を追加し、マネージドブロックと日本語クイックリファレンスの構造を明示する。
- `openspec/AGENTS.md` に統合された日本語ガイドの位置付けをスペックで正式に定義する。

## Impact
- アーカイブワークフローが期待どおりに動作し、`repo-guidelines` の仕様が現状のドキュメント構成と一致する。
- プロジェクト機能への影響はなく、開発者向けドキュメントの整合性が向上する。

## Rollout / Adoption
- スペック更新後に `openspec validate fix-repo-guidelines-archive --strict` を実行して構造チェックを行う。
- アーカイブ作業を再試行し、完了を確認する。
