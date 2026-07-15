use chrono::{DateTime, Local};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitWindow {
    pub label: String,
    pub remaining_percent: u8,
    pub reset_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageSnapshot {
    pub primary: Option<LimitWindow>,
    pub weekly: Option<LimitWindow>,
    pub status: Option<String>,
    pub sampled_at: Option<DateTime<Local>>,
}

impl UsageSnapshot {
    pub fn connecting() -> Self {
        Self {
            primary: None,
            weekly: None,
            status: Some("正在读取 Codex 用量…".to_string()),
            sampled_at: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            primary: None,
            weekly: None,
            status: Some(message.into()),
            sampled_at: None,
        }
    }
}

impl Default for UsageSnapshot {
    fn default() -> Self {
        Self::connecting()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connecting_has_no_fake_quota() {
        let snapshot = UsageSnapshot::connecting();
        assert!(snapshot.primary.is_none());
        assert!(snapshot.weekly.is_none());
        assert!(snapshot.status.is_some());
    }
}
