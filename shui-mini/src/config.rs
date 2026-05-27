use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub interval_minutes: u32,
    pub work_start_hour: u32,
    pub work_end_hour: u32,
    pub workdays_only: bool,
    pub pause_on_lock: bool,
    pub autostart: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_minutes: 45,
            work_start_hour: 9,
            work_end_hour: 18,
            workdays_only: true,
            pause_on_lock: true,
            autostart: false,
        }
    }
}

impl Config {
    pub fn in_work_window(&self, now: chrono::DateTime<chrono::Local>) -> bool {
        use chrono::{Datelike, Timelike, Weekday};
        if self.workdays_only {
            let wd = now.weekday();
            if matches!(wd, Weekday::Sat | Weekday::Sun) {
                return false;
            }
        }
        let h = now.hour();
        h >= self.work_start_hour && h < self.work_end_hour
    }

    pub fn load() -> Self {
        match Self::try_load() {
            Ok(c) => c,
            Err(_) => {
                let c = Self::default();
                let _ = c.save();
                c
            }
        }
    }

    fn try_load() -> Result<Self> {
        let txt = std::fs::read_to_string(path()?)?;
        Ok(toml::from_str(&txt)?)
    }

    pub fn save(&self) -> Result<()> {
        let p = path()?;
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(p, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

pub fn path() -> Result<PathBuf> {
    let dir = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(dir.join("shui-mini").join("config.toml"))
}
