このディレクトリの証明書と秘密鍵は、localhost / 127.0.0.1 の隔離された HTTPS 回帰テストと配布mock proxy専用です。実サービスや製品の認証には使用しません。

- `ca.pem`: テストサーバを署名した CA。
- `other-ca.pem`: テストサーバを信頼しない別の CA。
- `server.pem` / `server-key.pem`: テストサーバの証明書と対応する公開済みテスト鍵。
- `distribution-ca.pem` / `distribution.pem` / `distribution-key.pem`: 配布バイナリの専用loopback CONNECT proxyがTLSを終端するためのCA、server証明書と公開済みテスト鍵。SANは `DNS:api.switch-bot.com`。proxyは外部へ転送せず、製品の固定API URLを変更しない。2036年10月までの検証専用であり、製品の信頼ストアやarchiveには含めない。

証明書は 2026 年 10 月 8 日から 2036 年 10 月 5 日まで有効です。更新時には同じ SAN (`IP:127.0.0.1`, `DNS:localhost`) と serverAuth を保持してください。
