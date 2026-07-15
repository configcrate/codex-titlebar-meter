use chrono::{DateTime, Local};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitWindow {
    pub duration_minutes: u64,
    pub remaining_percent: u8,
    pub resets_at: Option<DateTime<Local>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageStatus {
    Connecting,
    Retrying,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageSnapshot {
    pub primary: Option<LimitWindow>,
    pub weekly: Option<LimitWindow>,
    pub status: Option<UsageStatus>,
    pub sampled_at: Option<DateTime<Local>>,
}

impl UsageSnapshot {
    pub fn connecting() -> Self {
        Self {
            primary: None,
            weekly: None,
            status: Some(UsageStatus::Connecting),
            sampled_at: None,
        }
    }

    pub fn retrying() -> Self {
        Self {
            primary: None,
            weekly: None,
            status: Some(UsageStatus::Retrying),
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
