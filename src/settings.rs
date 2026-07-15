use std::{env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub palette_index: usize,
}

impl Settings {
    pub fn load() -> Self {
        fs::read(settings_path())
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = settings_path();
        let Some(directory) = path.parent() else {
            return;
        };
        if fs::create_dir_all(directory).is_err() {
            return;
        }
        let Ok(bytes) = serde_json::to_vec_pretty(self) else {
            return;
        };
        let temporary = path.with_extension("json.tmp");
        if fs::write(&temporary, bytes).is_ok() {
            let _ = fs::remove_file(&path);
            let _ = fs::rename(temporary, path);
        }
    }
}

pub fn data_directory() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("ConfigCrate")
        .join("CodexTitlebarMeter")
}

fn settings_path() -> PathBuf {
    data_directory().join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_is_first() {
        assert_eq!(Settings::default().palette_index, 0);
    }
}
