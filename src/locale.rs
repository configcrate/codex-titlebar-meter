use std::{env, fs, path::PathBuf};

use chrono::{Datelike, Timelike};
use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;

use crate::model::{LimitWindow, UsageStatus};

const LOCALE_NAME_CAPACITY: usize = 85;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppLocale {
    Chinese,
    English,
}

impl AppLocale {
    pub fn detect() -> Self {
        codex_locale_override()
            .as_deref()
            .map(Self::from_language_tag)
            .unwrap_or_else(system_locale)
    }

    pub fn metric_text(self, window: &LimitWindow) -> String {
        format!(
            "{}  {}%  {}",
            self.duration_label(window.duration_minutes),
            window.remaining_percent,
            self.reset_label(window)
        )
    }

    pub fn status_text(self, status: UsageStatus) -> &'static str {
        match (self, status) {
            (Self::Chinese, UsageStatus::Connecting) => "正在读取 Codex 用量…",
            (Self::Chinese, UsageStatus::Retrying) => "暂时无法读取用量，正在重试…",
            (Self::English, UsageStatus::Connecting) => "Reading Codex usage…",
            (Self::English, UsageStatus::Retrying) => "Usage unavailable. Retrying…",
        }
    }

    fn from_language_tag(tag: &str) -> Self {
        if tag.trim().to_ascii_lowercase().starts_with("zh") {
            Self::Chinese
        } else {
            Self::English
        }
    }

    fn duration_label(self, minutes: u64) -> String {
        match self {
            Self::Chinese if minutes == 10_080 => "1周".to_string(),
            Self::English if minutes == 10_080 => "1 week".to_string(),
            Self::Chinese if minutes >= 1_440 && minutes.is_multiple_of(1_440) => {
                format!("{}天", minutes / 1_440)
            }
            Self::English if minutes >= 1_440 && minutes.is_multiple_of(1_440) => {
                plural(minutes / 1_440, "day")
            }
            Self::Chinese if minutes >= 60 && minutes.is_multiple_of(60) => {
                format!("{}小时", minutes / 60)
            }
            Self::English if minutes >= 60 && minutes.is_multiple_of(60) => {
                plural(minutes / 60, "hour")
            }
            Self::Chinese => "当前".to_string(),
            Self::English => "Current".to_string(),
        }
    }

    fn reset_label(self, window: &LimitWindow) -> String {
        match (self, window.resets_at.as_ref()) {
            (_, None) => "--".to_string(),
            (Self::Chinese, Some(reset)) if window.duration_minutes >= 1_440 => {
                format!("{}月{}日", reset.month(), reset.day())
            }
            (Self::English, Some(reset)) if window.duration_minutes >= 1_440 => {
                reset.format("%b %-d").to_string()
            }
            (_, Some(reset)) => format!("{:02}:{:02}", reset.hour(), reset.minute()),
        }
    }
}

fn plural(value: u64, unit: &str) -> String {
    if value == 1 {
        format!("{value} {unit}")
    } else {
        format!("{value} {unit}s")
    }
}

fn codex_locale_override() -> Option<String> {
    let path = codex_config_path()?;
    let contents = fs::read_to_string(path).ok()?;
    contents.lines().find_map(parse_locale_override)
}

fn codex_config_path() -> Option<PathBuf> {
    if let Some(home) = env::var_os("CODEX_HOME") {
        return Some(PathBuf::from(home).join("config.toml"));
    }
    env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".codex/config.toml"))
}

fn parse_locale_override(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    if key.trim() != "localeOverride" {
        return None;
    }
    let value = value.trim();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let remainder = &value[quote.len_utf8()..];
    let end = remainder.find(quote)?;
    let locale = remainder[..end].trim();
    (!locale.is_empty()).then(|| locale.to_string())
}

fn system_locale() -> AppLocale {
    let mut buffer = [0_u16; LOCALE_NAME_CAPACITY];
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    if length > 1 {
        let tag = String::from_utf16_lossy(&buffer[..length as usize - 1]);
        AppLocale::from_language_tag(&tag)
    } else {
        AppLocale::English
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    fn window(minutes: u64) -> LimitWindow {
        LimitWindow {
            duration_minutes: minutes,
            remaining_percent: 94,
            resets_at: Local.timestamp_opt(1_800_500_000, 0).single(),
        }
    }

    #[test]
    fn parses_codex_locale_override() {
        assert_eq!(
            parse_locale_override("localeOverride = \"zh-CN\""),
            Some("zh-CN".to_string())
        );
        assert_eq!(parse_locale_override("# localeOverride = \"en\""), None);
        assert_eq!(parse_locale_override("theme = \"dark\""), None);
    }

    #[test]
    fn formats_metrics_in_chinese_and_english() {
        let weekly = window(10_080);
        assert!(
            AppLocale::Chinese
                .metric_text(&weekly)
                .starts_with("1周  94%")
        );
        assert!(
            AppLocale::English
                .metric_text(&weekly)
                .starts_with("1 week  94%")
        );
    }
}
