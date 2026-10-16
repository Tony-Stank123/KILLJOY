use crate::fans::{CurvePoint, FanProfileMode};
use crate::modes::DeviceMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_mode: DeviceMode,
    pub fan_mode: FanProfileMode,
    pub is_max_boost: bool,
    pub manual_fan_pct: f32,
    pub custom_curve: Vec<CurvePoint>,
    pub auto_purge_on_startup: bool,
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_mode: DeviceMode::Balanced,
            fan_mode: FanProfileMode::Auto,
            is_max_boost: false,
            manual_fan_pct: 50.0,
            custom_curve: vec![
                CurvePoint { temp_c: 35.0, speed_pct: 20.0 },
                CurvePoint { temp_c: 50.0, speed_pct: 35.0 },
                CurvePoint { temp_c: 65.0, speed_pct: 55.0 },
                CurvePoint { temp_c: 75.0, speed_pct: 80.0 },
                CurvePoint { temp_c: 85.0, speed_pct: 100.0 },
            ],
            auto_purge_on_startup: false,
            window_width: 1180.0,
            window_height: 780.0,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> &'static str {
        "kiljoy_config.json"
    }

    pub fn load() -> Self {
        if Path::new(Self::config_path()).exists() {
            if let Ok(content) = fs::read_to_string(Self::config_path()) {
                if let Ok(cfg) = serde_json::from_str(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::config_path(), json).map_err(|e| e.to_string())?;
        Ok(())
    }
}
