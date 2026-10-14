# バイナリの導入・設定・停止

GitHub Release の Downloads から利用するOS/CPUのarchiveと、同名の `.sha256` を取得します。すべて同じversion/tagと採番commitから生成されます。公開は全対象のnative検証・アップロードが完了した後です。

| OS / CPU | target | archive名（`VERSION` はRelease tag） |
| --- | --- | --- |
| Linux x64 | `x86_64-unknown-linux-gnu` | `switchbot-exporter-VERSION-linux-x86_64.tar.gz` |
| Windows x64 | `x86_64-pc-windows-msvc` | `switchbot-exporter-VERSION-windows-x86_64.zip` |
| Windows ARM64 | `aarch64-pc-windows-msvc` | `switchbot-exporter-VERSION-windows-arm64.zip` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `switchbot-exporter-VERSION-macos-arm64.tar.gz` |

Windows archiveには `switchbot-exporter.exe`、Linux/macOS archiveには `switchbot-exporter` が入ります。x86は64bitのx86_64を指し、32bit版はありません。Dockerの `linux/amd64` / `linux/arm64` 配布も維持します。

## 前提と検証

実行にRust・Python・uvは不要です。Windows 10/11は対応architectureの[Microsoft Visual C++ v14 Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)が必要です。LinuxはGNU/glibc環境向けです。Ubuntuのnative runnerでビルドするため、そのrunnerと互換のglibcを用意してください。Alpine/muslではDocker imageを使用します。macOSはApple Silicon向けです。署名・notarizationは提供していません。

WindowsはPowerShellでSHA-256を照合し、ZIPを展開します。

```powershell
$archive = 'switchbot-exporter-VERSION-windows-x86_64.zip'
$expected = ((Get-Content "$archive.sha256").Trim() -split '\s+')[0]
if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLower() -ne $expected) { throw 'Checksum mismatch' }
Expand-Archive $archive -DestinationPath '.\switchbot'
Set-Location '.\switchbot'
.\switchbot-exporter.exe --version
.\switchbot-exporter.exe --help
```

Linux/macOSでは `sha256sum -c ARCHIVE.sha256`（Linux）または `shasum -a 256 -c ARCHIVE.sha256`（macOS）で照合し、`tar -xzf ARCHIVE.tar.gz` で展開します。続けて `./switchbot-exporter --version` と `./switchbot-exporter --help` を実行します。arm64版では表のarchive名に置き換えます。

## 設定・起動

[env.example](../env.example)の設定を参考に、作業ディレクトリへUTF-8、BOMなしの `.env` を保存します。LF/CRLFの両方を読めます。PowerShell 5の既定のUTF-16やBOM付きUTF-8は使用しません。実行ファイルの場所ではなく、起動時の作業ディレクトリから親へ探索し、最初の `.env` 一つだけを読みます。環境変数が `.env` の値より優先されます。Windowsのプロセス環境変数名は大文字・小文字を区別しません。`.env` では表記どおりの大文字名を使用します。

PowerShellで現在のプロセスへ設定して起動する例:

```powershell
$env:SWITCHBOT_API_TOKEN = 'YOUR_TOKEN'
$env:SWITCHBOT_API_SECRET = 'YOUR_SECRET'
$env:SERVER_PORT = '9171'
.\switchbot-exporter.exe exporter
```

Linux/macOSで現在のshellへ設定して起動する例:

```sh
export SWITCHBOT_API_TOKEN='YOUR_TOKEN'
export SWITCHBOT_API_SECRET='YOUR_SECRET'
export SERVER_PORT=9171
./switchbot-exporter exporter
```

`http://localhost:9171/metrics` をPrometheusでscrapeします。`SERVER_PORT` は `0.0.0.0` でlistenするportです。設定一覧とHTTP 500の扱いは[README](../README.md#利用方法)を参照してください。`CACHE_DIR` は旧設定との互換用の有効化フラグで、Windowsでも `/tmp/switchbot` へファイルを書きません。

TLSはRustlsを使います。通常のHTTPS検証に加え、独自CAを使う場合は `REQUESTS_CA_BUNDLE`（空・未設定なら `CURL_CA_BUNDLE`）にPEMファイルのパスを指定します。指定時はそのbundleのみを信頼します。パスはプロセスの作業ディレクトリに対する相対パス、または絶対パスです。空白・日本語を含むパスも使え、`.env` は `REQUESTS_CA_BUNDLE='C:\certs with spaces\CA.pem'` のように引用します。shellの `~` や変数は `.env` 内では展開されません。

proxyは `.env` ではなくプロセス環境の `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` / `NO_PROXY` から読みます。PowerShellでは `$env:HTTPS_PROXY='http://proxy.example:8080'`、Linux/macOSでは `export HTTPS_PROXY='http://proxy.example:8080'` を設定します。proxy環境でも必要なCAを指定し、証明書検証を無効にしません。

## 停止と検証範囲

foregroundで起動したconsoleでCtrl+Cを押します。WindowsはCtrl+Breakにも対応し、Linux/macOSは `kill -TERM PID` でも停止します。新規接続の受付を止め、処理中のscrapeを完了して停止し、portを解放します。`API_TIMEOUT_SECOND` は各HTTPリクエストに適用されるため、複数デバイスの取得や `DELAY_SECOND` により停止までの時間は一つのtimeoutより長くなることがあります。強制終了とは区別します。

PR CIとReleaseはLinux x64・Windows x64/ARM64・macOS ARM64のnative runnerで展開済みバイナリを起動します。共通controllerのarchive/checksum検証と、外部転送しないloopback CONNECT/TLS mockによる署名・`.env`・環境優先順位・Unicodeパス・CLI・七つのメトリクス・cache・HTTP・エラー・timeout・停止を確認します。実トークンや実機操作は使いません。一般利用者の物理端末や実API認証はこの検証の対象外です。
