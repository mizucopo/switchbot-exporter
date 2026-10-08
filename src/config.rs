use std::{collections::HashMap, fs, path::Path, time::Duration};

use anyhow::{Context, Result, bail};

pub type Values = HashMap<String, String>;

#[derive(Clone)]
pub struct Config {
    pub api_token: String,
    pub api_secret: String,
    pub cache_enabled: bool,
    pub cache_ttl: Duration,
    pub delay: Duration,
    pub api_timeout: Duration,
}

impl Config {
    pub fn load(directory: &Path) -> Result<Self> {
        Self::from_values(&load_values(directory)?)
    }

    pub fn from_values(values: &Values) -> Result<Self> {
        let required = |name: &str| {
            values
                .get(name)
                .cloned()
                .with_context(|| format!("必須の環境変数 '{name}' が設定されていません。"))
        };
        let ttl: i64 = value(values, "CACHE_EXPIRE_SECOND", "600")
            .trim()
            .parse()
            .context("CACHE_EXPIRE_SECOND must be an integer")?;
        let delay = duration(values, "DELAY_SECOND", "1", true)?;
        let api_timeout = duration(values, "API_TIMEOUT_SECOND", "30", false)?;
        Ok(Self {
            api_token: required("SWITCHBOT_API_TOKEN")?,
            api_secret: required("SWITCHBOT_API_SECRET")?,
            cache_enabled: !value(values, "CACHE_DIR", "/tmp/switchbot").is_empty() && ttl > 0,
            cache_ttl: Duration::from_secs(ttl.max(0) as u64),
            delay,
            api_timeout,
        })
    }
}

fn value<'a>(values: &'a Values, name: &str, default: &'a str) -> &'a str {
    values.get(name).map(String::as_str).unwrap_or(default)
}

fn duration(values: &Values, name: &str, default: &str, allow_zero: bool) -> Result<Duration> {
    let seconds: f64 = value(values, name, default)
        .trim()
        .parse()
        .with_context(|| format!("{name} must be a number"))?;
    let duration = Duration::try_from_secs_f64(seconds)
        .with_context(|| format!("{name} must be finite and non-negative"))?;
    if !allow_zero && duration.is_zero() {
        bail!("{name} must be greater than zero");
    }
    Ok(duration)
}

pub fn server_port(directory: &Path) -> Result<u16> {
    let values = load_values(directory)?;
    value(&values, "SERVER_PORT", "9171")
        .trim()
        .parse()
        .context("SERVER_PORT must be an integer between 0 and 65535")
}

pub fn load_values(directory: &Path) -> Result<Values> {
    let mut values = Values::new();
    // python-decouple と同じく、最初に見つかった .env のみを読み込む。
    for ancestor in directory.ancestors() {
        let path = ancestor.join(".env");
        match fs::read_to_string(&path) {
            Ok(contents) => {
                values = parse_env(&contents);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Could not read .env"),
        }
    }
    values.extend(std::env::vars());
    Ok(values)
}

pub fn parse_env(contents: &str) -> Values {
    let mut values = Values::new();
    for line in contents.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, raw)) = line.split_once('=') {
            let raw = raw.trim();
            let quoted = raw.len() >= 2
                && ((raw.starts_with('"') && raw.ends_with('"'))
                    || (raw.starts_with('\'') && raw.ends_with('\'')));
            let value = if quoted { &raw[1..raw.len() - 1] } else { raw };
            values.insert(key.trim().to_owned(), value.to_owned());
        }
    }
    values
}
