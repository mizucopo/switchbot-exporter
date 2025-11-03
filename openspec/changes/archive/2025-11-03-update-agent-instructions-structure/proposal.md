## Why
- 変更 `fix-repo-guidelines-archive` をアーカイブした結果、`Agent Instructions Structure` 要件がマネージドブロック維持と日本語クイックリファレンスの配置を強制するように更新された。
- 現行スペック `openspec/specs/repo-guidelines/spec.md` では `openspec/AGENTS.md` を単一の参照先として扱うだけで、マネージドブロック維持やクイックリファレンスの配置に関するシナリオが欠落している。
- この差分により、`openspec archive` がスペック未達としてエラーを報告し、コントリビュータへのガイダンスも不完全になっている。

## What Changes
- `openspec/specs/repo-guidelines/spec.md` の `Agent Instructions Structure` 要件に、「マネージドブロックを保持し、直後にローカライズ済みクイックリファレンスを配置する」シナリオを追加する。
- アーカイブされた変更に記載された `Preserve managed block` と `Highlight unified localization` の両シナリオをライブスペックへ反映する。

## Impact
- ライブスペックが現行ドキュメント構造と整合し、`openspec archive` の検証が通るようになる。
- コントリビュータがマネージドブロックを誤って変更・移動するリスクを減らし、クイックリファレンスが正しい位置にあることを保証する。

## Rollout / Adoption
- スペック更新後に `openspec validate update-agent-instructions-structure --strict` を実行し、形式チェックを通過させる。
- `openspec archive merge-agents-guides` を再実行してエラーが解消されたことを確認する。
