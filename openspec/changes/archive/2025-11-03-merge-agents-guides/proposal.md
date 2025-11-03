## Why
- エージェント向けガイドが英日で分離されており、更新の重複や同期漏れが発生しやすい。
- 日本語要約をメインガイドへ統合することで、参照先を一本化しメンテナンス負荷を軽減する。

## What Changes
- `openspec/AGENTS.md` に日本語クイックリファレンスを追加して既存の日本語要約を取り込む。
- `openspec/AGENTS.ja.md` を削除し、統合先として `openspec/AGENTS.md` のみを使用する。

## Impact
- 仕様や機能の挙動に変更はなく、ドキュメント構造が簡潔になる。
- 既存スペックへの差分は不要だが、他ドキュメントからの参照を確認する必要がある。

## Rollout / Adoption
- 統合後はエージェント向けアナウンスで参照先の変更を共有する。
- `openspec validate merge-agents-guides --strict` を実行し、構造チェックを行う。
