# Python 補助処理の用途と保持範囲

exporter 本体は Rust です。配布バイナリの利用に Python・uv は不要で、Docker の runtime image にも含みません。Python は開発・CI の分類、採番、配布とその検証に使用します。旧 Python アプリを実行するための環境ではありません。

## 残存ファイルと判断

Issue #84 の整理では、次の現役ファイルを保持します。旧アプリ source・`.pyi`・requirements ファイル、Flask/Gunicorn のアプリ依存は残っていません。代替実装がない状態でテストや品質ゲートを削除して Python をなくした扱いにはしません。

| ファイル・設定 | 用途・呼び出し元 | 保持する理由 |
| --- | --- | --- |
| `.github/scripts/release.py` | `release-classification.yml` の分類、`docker-release.yml` の採番・計画・配布・image 確認・Release 公開・Latest 判定。PR の配布対象 matrix もこの module を読み込む | repo-template 由来の共通 controller。同一 commit/tag、所有権、部分公開からの再実行、完成後の公開と Latest の保護を維持する |
| `tests/release_smoke.py` | `pr-quality-checks.yml` の `native-distribution` と `docker-release.yml` の `rust-builds` | archive/checksum、CLI、設定、TLS/proxy、mock API、HTTP、cache、エラー、timeout と処理中 scrape の正常終了を、配布バイナリで検証する。Python 標準ライブラリで動き、pytest/uv は要求しない |
| `tests/test_release.py` | 下記 runner から pytest で実行 | Cargo manifest/lock の version 一致、採番時の変更範囲、Docker Hub の匿名照会と確認不能時の停止を mock で検証する |
| `tests/run_pytest.py` | `pyproject.toml` の `task check` | pytest の収集結果を扱う共通 runner。既存のテスト失敗・収集エラーを維持する |
| `pyproject.toml`、`uv.lock`、`.python-version` | ローカル、PR の `quality-checks`、Release の `release` で開発環境を構築 | Python 3.14 と開発専用 dependency group を固定する。pytest/pytest-mock は回帰テスト、Ruff は lint/format、mypy は型検査、taskipy は品質コマンドを担当する。uv は環境・lock を管理する |
| `.github/dependabot.yml` の `uv` 設定 | 開発依存の更新 | 本体が Rust でも、現役の Python 開発依存を更新する必要がある |
| `.gitignore` の Python cache・仮想環境・pytest 記述 | 補助処理のローカル生成物を除外 | 開発時には生成される。共通テンプレートの他言語向け除外も、runtime 依存の宣言ではない |
| `.codex/project.md`、`.codex/languages/python.md`、`AGENTS.md` | 人と agent の作業手順 | 補助処理の用途と必要な品質ゲートを示す |

`pr-quality-checks.yml` の `distribution-targets` は runner の Python で共通 controller と `.github/release.json` を読み、対象 matrix を生成します。各 native job は Python 3.14 を用意し、Rust の品質検証と配布 smoke を実行します。PR の `quality-checks` と Release の `release` は `.python-version` と uv を使って以下の gate を実行します。

```sh
uv sync --locked
uv run task check
```

この gate は `tests/` の Ruff lint/format、mypy と controller の pytest 回帰テストです。配布 smoke の実行は含まれず、各対象 runner の別 step で行います。Linux x64 でローカル確認する場合は、Rust toolchain を用意して以下を実行できます。出力先は作業ツリー外を指定します。

```sh
python -I tests/release_smoke.py \
  --target x86_64-unknown-linux-gnu --output /tmp/switchbot-distribution-check
```

他の対象は対応する native runner で検証します。smoke は固定の公開 API 宛て通信を loopback TLS/proxy mock で受け、外部へ転送しません。実 SwitchBot の認証情報や制御は使用しません。

### 削除したファイル

空の `tests/__init__.py` を削除しました。`tests.*` import・相対 import・package entrypoint はなく、整理時の削除前後で pytest の同じ14件が収集され、回帰テストと Ruff・mypy の品質ゲートが成功しました。共通 runner の空 initializer を除外する処理は、ファイルの存在を要求しないため変更しません。

## Python の記述と配布物の境界

- `.copier-answers.yml` の `use_python: false` は Python アプリの生成を無効にします。共通 controller や製品固有の補助検証を削除する指定ではありません。
- Docker の root context は `.dockerignore` に従い、Rust manifest/source、Rust テストと公開 TLS fixture を許可します。Python helper、pyproject/uv.lock/仮想環境は build 入力に含めません。runtime stage にコピーする製品成果物は Rust binary です。
- `docs/docker-build-context.md` の Python application/package の説明、`docs/release.md` の Python stage hook は共通テンプレートの他構成向けの例です。この exporter の Python アプリ起動方法や依存追加の指示ではありません。現在の `stage_command` は空で、配布処理に製品固有の Python hook を追加していません。
- `docs/rust-migration.md` と ADR の Python/Flask/Gunicorn 記述、Rust source/tests の Python dict・python-decouple 互換説明は、旧版との挙動や移行の説明です。実行依存とは区別して保持します。

## 削除・移行の条件

共通 controller の言語・公開設計を変更する場合は、まず repo-template で設計と生成/更新テストを揃え、その後 Copier で反映します。consumer に独自 controller を作ったり、公開保護を外したりしません。[repo-template #204](https://github.com/mizucopo/repo-template/issues/204) の公開後確認再試行は別課題であり、この整理では変更しません。

補助 Python の削減を行う将来の変更では、移行対象の呼び出し元、同等の検証と品質設定を同時に更新します。回帰テストを Rust 等へ移しても、共通 controller や配布 smoke が Python を使う間はその実行環境を保持します。すべての依存先が移行してから pyproject/uv.lock/.python-version、runner/package marker、Dependabot と guidance の削除可否を判断します。

最終配布物が Python/uv を要求しない契約、4対象の archive/checksum・起動/HTTP/終了検証、通常品質ゲート、採番・再実行・Release/Latest の保護を移行後も維持します。検証のための実際の Release 公開は行いません。
