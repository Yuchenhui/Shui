use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub by_day: BTreeMap<String, u32>,
}

impl Stats {
    pub fn load() -> Self {
        std::fs::read_to_string(path().unwrap_or_default())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let p = path()?;
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn today_key() -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }

    pub fn add_today(&mut self) -> u32 {
        let k = Self::today_key();
        let v = self.by_day.entry(k).or_insert(0);
        *v += 1;
        *v
    }

    pub fn today(&self) -> u32 {
        self.by_day.get(&Self::today_key()).copied().unwrap_or(0)
    }
}

fn path() -> Result<PathBuf> {
    let dir = dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
    Ok(dir.join("shui-mini").join("stats.json"))
}
