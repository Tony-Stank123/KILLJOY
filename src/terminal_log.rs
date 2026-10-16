use chrono::Local;

#[derive(Clone, Debug)]
pub struct TerminalEntry {
    pub timestamp: String,
    pub tag: String,
    pub tag_color: egui::Color32,
    pub message: String,
}

pub struct CyberTerminal {
    pub entries: Vec<TerminalEntry>,
    pub input_buffer: String,
    pub command_history: Vec<String>,
    pub history_index: usize,
    pub auto_scroll: bool,
}

impl Default for CyberTerminal {
    fn default() -> Self {
        let mut term = Self {
            entries: Vec::new(),
            input_buffer: String::new(),
            command_history: Vec::new(),
            history_index: 0,
            auto_scroll: true,
        };

        term.log("SYS", egui::Color32::from_rgb(0, 240, 255), "KILJOY Cyber-Deck v1.0.4 initialized.");
        term.log("ACPI", egui::Color32::from_rgb(0, 255, 102), "HP WMI / ACPI interface linked at root\\wmi\\hpqBIntM.");
        term.log("GPU", egui::Color32::from_rgb(255, 0, 85), "NVIDIA RTX 4050 Laptop telemetry channel established (WDDM 616.64).");
        term.log("FAN", egui::Color32::from_rgb(100, 230, 255), "Dual fan controller active. Profile: AUTO [ACPI Dynamic Curve].");
        term.log("SYS", egui::Color32::from_rgb(0, 240, 255), "Ready. Type 'help' in terminal prompt for commands.");

        term
    }
}

impl CyberTerminal {
    pub fn push_history(&mut self, cmd: &str) {
        if !cmd.trim().is_empty() {
            self.command_history.push(cmd.to_string());
            self.history_index = self.command_history.len();
        }
    }

    pub fn history_prev(&mut self) -> Option<String> {
        if self.command_history.is_empty() {
            return None;
        }
        if self.history_index > 0 {
            self.history_index -= 1;
        }
        self.command_history.get(self.history_index).cloned()
    }

    pub fn history_next(&mut self) -> Option<String> {
        if self.command_history.is_empty() {
            return None;
        }
        if self.history_index + 1 < self.command_history.len() {
            self.history_index += 1;
            self.command_history.get(self.history_index).cloned()
        } else {
            self.history_index = self.command_history.len();
            Some(String::new())
        }
    }

    pub fn log(&mut self, tag: &str, tag_color: egui::Color32, msg: &str) {
        let now = Local::now().format("%H:%M:%S").to_string();
        self.entries.push(TerminalEntry {
            timestamp: now,
            tag: tag.to_string(),
            tag_color,
            message: msg.to_string(),
        });

        // Cap log size to 250 entries
        if self.entries.len() > 250 {
            self.entries.remove(0);
        }
    }

    pub fn log_info(&mut self, msg: &str) {
        self.log("INFO", egui::Color32::from_rgb(0, 240, 255), msg);
    }

    pub fn log_warn(&mut self, msg: &str) {
        self.log("WARN", egui::Color32::from_rgb(255, 170, 0), msg);
    }

    pub fn log_success(&mut self, msg: &str) {
        self.log("SUCCESS", egui::Color32::from_rgb(0, 255, 102), msg);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.log("SYS", egui::Color32::from_rgb(0, 240, 255), "Terminal cleared. KILJOY daemon listening.");
    }
}
