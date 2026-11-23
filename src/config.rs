//! Settings and target list, persisted as JSON in the platform config dir.
//!
//! The key names match the 1.x config (`timeoutMs`, `ttl`, `historyLength`,
//! `targets[].host`, `targets[].displayName`), so an existing file on Windows
//! carries over.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub timeout_ms: u32,
    pub interval_ms: u32,
    pub ttl: u8,
    pub history_length: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { timeout_ms: 1000, interval_ms: 1000, ttl: 128, history_length: 60 }
    }
}

impl Settings {
    pub fn clamped(mut self) -> Self {
        self.timeout_ms = self.timeout_ms.clamp(100, 10_000);
        self.interval_ms = self.interval_ms.clamp(250, 10_000);
        self.ttl = self.ttl.max(1);
        self.history_length = self.history_length.clamp(10, 600);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub host: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default = "yes")]
    pub active: bool,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(flatten)]
    pub settings: Settings,
    pub targets: Vec<Entry>,
}

impl Default for Config {
    fn default() -> Self {
        let t = |host: &str, name: &str| Entry {
            host: host.into(),
            display_name: name.into(),
            active: true,
        };
        Self {
            settings: Settings::default(),
            targets: vec![
                t("8.8.8.8", "Google DNS Primary"),
                t("8.8.4.4", "Google DNS Secondary"),
                t("1.1.1.1", "Cloudflare Primary"),
                t("1.0.0.1", "Cloudflare Secondary"),
                t("9.9.9.9", "Quad9"),
                t("208.67.222.222", "OpenDNS"),
            ],
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Option<Self> {
        let mut c: Config = serde_json::from_str(text).ok()?;
        c.settings = c.settings.clamped();
        c.targets.retain(|e| !e.host.trim().is_empty());
        if c.targets.is_empty() {
            return None;
        }
        Some(c)
    }
}

/// `--config <path>` wins, then the platform default. On Windows that is
/// `%APPDATA%\Pingy\pingy_config.json`, where 1.x kept it.
pub fn path() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--config"
            && let Some(p) = args.next()
        {
            return p.into();
        }
    }
    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("Pingy").join("pingy_config.json");
    }
    directories::ProjectDirs::from("com", "Locke Werks", "Pingy")
        .map(|d| d.config_dir().join("pingy_config.json"))
        .unwrap_or_else(|| PathBuf::from("pingy_config.json"))
}

pub fn load() -> Config {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|t| Config::parse(&t))
        .unwrap_or_default()
}

pub fn save(c: &Config) {
    let p = path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Ok(text) = serde_json::to_string_pretty(c) else { return };
    // Write then rename, so a crash mid-write cannot leave a truncated config
    // that silently resets the target list on the next start.
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_v1_config() {
        let v1 = r#"{
          "timeoutMs": 800, "ttl": 64, "historyLength": 120,
          "targets": [ { "host": "1.1.1.1", "displayName": "CF" } ]
        }"#;
        let c = Config::parse(v1).unwrap();
        assert_eq!(c.settings.timeout_ms, 800);
        assert_eq!(c.settings.ttl, 64);
        assert_eq!(c.settings.history_length, 120);
        assert_eq!(c.settings.interval_ms, 1000);
        assert_eq!(c.targets[0].display_name, "CF");
        assert!(c.targets[0].active);
    }

    #[test]
    fn empty_target_list_falls_back_to_defaults() {
        assert!(Config::parse(r#"{"targets": []}"#).is_none());
        assert!(Config::parse("not json").is_none());
    }

    #[test]
    fn out_of_range_settings_are_clamped() {
        let c = Config::parse(r#"{"timeoutMs": 5, "historyLength": 99999, "targets": [{"host": "a"}]}"#).unwrap();
        assert_eq!(c.settings.timeout_ms, 100);
        assert_eq!(c.settings.history_length, 600);
    }

    #[test]
    fn round_trips() {
        let c = Config::default();
        let back = Config::parse(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(c, back);
    }
}
