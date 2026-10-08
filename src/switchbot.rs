use std::{collections::HashMap, fs, time::SystemTime};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use reqwest::{Certificate, Client, Url, header::HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use tokio::time::Instant;

use crate::{config::Config, metrics::Metrics};

pub const BASE_URL: &str = "https://api.switch-bot.com";
pub const DEVICE_TYPES: [&str; 12] = [
    "Bot",
    "Ceiling Light",
    "Color Bulb",
    "Contact Sensor",
    "Curtain",
    "Hub Mini",
    "Indoor Cam",
    "Meter",
    "MeterPro(CO2)",
    "Motion Sensor",
    "Plug Mini (JP)",
    "Remote",
];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub device_id: String,
    #[serde(default = "unknown_device_type")]
    pub device_type: String,
    pub device_name: String,
}

fn unknown_device_type() -> String {
    "-".to_owned()
}

struct CacheEntry {
    value: Value,
    fetched_at: Instant,
}

pub struct Switchbot {
    config: Config,
    http: Client,
    base_url: Url,
    cache: HashMap<String, CacheEntry>,
}

impl Switchbot {
    pub fn new(config: Config) -> Result<Self> {
        Self::with_base_url(config, Url::parse(BASE_URL)?)
    }

    /// テストではローカルHTTP境界を注入する。CLIから接続先は変更しない。
    pub fn with_base_url(config: Config, base_url: Url) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(config.api_timeout)
            .redirect(reqwest::redirect::Policy::none());
        if let Some(path) = &config.ca_bundle {
            let pem = fs::read(path).context("Could not read configured CA bundle")?;
            let certificates =
                Certificate::from_pem_bundle(&pem).context("Invalid configured PEM CA bundle")?;
            if certificates.is_empty() {
                bail!("Configured CA bundle contains no certificates");
            }
            // RequestsのCA overrideと同じく、指定したbundleのみを信頼する。
            builder = builder.tls_certs_only(certificates);
        }
        let http = builder
            .build()
            .context("Could not initialize HTTP client")?;
        Ok(Self {
            config,
            http,
            base_url,
            cache: HashMap::new(),
        })
    }

    pub async fn fetch_devices(&mut self) -> Result<Vec<Device>> {
        let url = self.base_url.join("/v1.1/devices")?;
        let response = self.fetch(url).await?;
        let devices = response
            .get("body")
            .and_then(|body| body.get("deviceList"))
            .context("SwitchBot response is missing body.deviceList")?;
        serde_json::from_value(devices.clone()).context("Invalid SwitchBot device list")
    }

    pub async fn fetch_device_status(&mut self, device_id: &str) -> Result<Value> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| anyhow::anyhow!("Invalid SwitchBot base URL"))?
            .clear()
            .extend(["v1.1", "devices", device_id, "status"]);
        self.fetch(url).await
    }

    pub async fn fetch_metrics(&mut self) -> Result<Metrics> {
        let devices = self.fetch_devices().await?;
        let mut metrics = Metrics::default();
        for device in &devices {
            if DEVICE_TYPES.contains(&device.device_type.as_str()) {
                let response = self.fetch_device_status(&device.device_id).await?;
                let status = response
                    .get("body")
                    .and_then(Value::as_object)
                    .context("SwitchBot status response is missing body")?;
                metrics.add(device, status)?;
            }
        }
        Ok(metrics)
    }

    async fn fetch(&mut self, url: Url) -> Result<Value> {
        if self.config.cache_enabled {
            // 期限切れのデバイスを保持し続けない。
            self.cache
                .retain(|_, entry| entry.fetched_at.elapsed() <= self.config.cache_ttl);
            if let Some(entry) = self.cache.get(url.as_str()) {
                return Ok(entry.value.clone());
            }
        }
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis()
            .to_string();
        let sign = signature(&self.config.api_token, &self.config.api_secret, &timestamp);
        let mut authorization = HeaderValue::from_str(&self.config.api_token)
            .map_err(|_| anyhow::anyhow!("Invalid SwitchBot API token header"))?;
        authorization.set_sensitive(true);
        let response = self
            .http
            .get(url.clone())
            .header("Authorization", authorization)
            .header("sign", sign)
            .header("t", timestamp)
            .header("nonce", "")
            .send()
            .await
            .context("SwitchBot API request failed")?
            .error_for_status()
            .context("SwitchBot API returned an HTTP error")?;
        let body = response
            .bytes()
            .await
            .context("Could not read SwitchBot response")?;
        // キャッシュhitには待機せず、成功した実リクエストの後に待機する。
        let fetched_at = Instant::now();
        if !self.config.delay.is_zero() {
            tokio::time::sleep(self.config.delay).await;
        }
        let value: Value =
            serde_json::from_slice(&body).context("Invalid SwitchBot JSON response")?;
        if let Some(code) = value.get("statusCode")
            && code.as_i64() != Some(100)
        {
            bail!("SwitchBot API returned an unsuccessful statusCode");
        }
        if self.config.cache_enabled {
            self.cache.insert(
                url.to_string(),
                CacheEntry {
                    value: value.clone(),
                    fetched_at,
                },
            );
        }
        Ok(value)
    }
}

pub fn signature(token: &str, secret: &str, timestamp: &str) -> String {
    let mut hmac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    hmac.update(token.as_bytes());
    hmac.update(timestamp.as_bytes());
    // 現行の空nonceを維持する。
    STANDARD.encode(hmac.finalize().into_bytes())
}
