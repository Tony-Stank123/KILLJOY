use crate::bloat_killer::BloatKiller;
use crate::config::AppConfig;
use crate::fans::{FanControllerState, FanProfileMode};
use crate::modes::DeviceMode;
use crate::terminal_log::CyberTerminal;
use crate::theme::CyberTheme;
use crate::vitals::{SystemVitals, VitalsMonitor};
use egui::{Color32, RichText, Stroke, Ui, Vec2};
use egui_plot::{Line, Plot, PlotPoints, Points};
use std::time::Instant;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ActiveTab {
    PerformanceControl,
    SystemVitals,
    FanCurveMatrix,
    BloatKiller,
    CyberTerminal,
}

pub struct KilJoyApp {
    pub monitor: VitalsMonitor,
    pub current_vitals: SystemVitals,
    pub active_mode: DeviceMode,
    pub fan_ctrl: FanControllerState,
    pub bloat_killer: BloatKiller,
    pub terminal: CyberTerminal,
    pub config: AppConfig,
    pub active_tab: ActiveTab,
    pub status_message: String,
    pub status_time: Instant,
    pub is_passive_mode: bool,
}

impl KilJoyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        CyberTheme::apply_to_ctx(&cc.egui_ctx);

        let config = AppConfig::load();
        let monitor = VitalsMonitor::new();
        let vitals = monitor.vitals.lock().map(|v| v.clone()).unwrap_or_default();

        let mut fan_ctrl = FanControllerState::default();
        fan_ctrl.mode = config.fan_mode;
        fan_ctrl.is_max_boost = config.is_max_boost;
        fan_ctrl.manual_speed_pct = config.manual_fan_pct;
        if !config.custom_curve.is_empty() {
            fan_ctrl.curve_points = config.custom_curve.clone();
        }

        let bloat = BloatKiller::default();
        let mut term = CyberTerminal::default();

        term.log(
            "SYS",
            CyberTheme::NEON_CYAN,
            "KilJoy native Win32 core online. Zero PowerShell overhead.",
        );

        Self {
            monitor,
            current_vitals: vitals,
            active_mode: config.active_mode,
            fan_ctrl,
            bloat_killer: bloat,
            terminal: term,
            config,
            active_tab: ActiveTab::PerformanceControl,
            status_message: "Hardware sensors synchronized. All systems ready.".to_string(),
            status_time: Instant::now(),
            is_passive_mode: false,
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.status_time = Instant::now();
        self.terminal.log_info(msg);
    }

    pub fn set_mode(&mut self, mode: DeviceMode) {
        if self.is_passive_mode {
            self.terminal.log_warn("KilJoy is in PASSIVE MONITOR mode. Stand-down active.");
            return;
        }

        self.active_mode = mode;
        self.config.active_mode = mode;
        let _ = self.config.save();

        match mode.apply_system_profile() {
            Ok(log) => {
                let msg = format!("[MODE] Set to {}: {}", mode.name(), log);
                self.terminal.log("ACPI", mode.accent_color(), &msg);
                self.status_message = format!("Engaged {} mode.", mode.name());
            }
            Err(e) => {
                let msg = format!("Failed to set {}: {}", mode.name(), e);
                self.terminal.log_warn(&msg);
            }
        }
    }

    pub fn toggle_max_fan(&mut self) {
        if self.is_passive_mode {
            self.terminal.log_warn("Cannot toggle fan: PASSIVE MONITOR mode is active.");
            return;
        }

        self.fan_ctrl.is_max_boost = !self.fan_ctrl.is_max_boost;
        if self.fan_ctrl.is_max_boost {
            self.fan_ctrl.mode = FanProfileMode::Max;
        } else {
            self.fan_ctrl.mode = FanProfileMode::Auto;
        }

        self.config.is_max_boost = self.fan_ctrl.is_max_boost;
        self.config.fan_mode = self.fan_ctrl.mode;
        let _ = self.config.save();

        // Direct hardware WMI dispatch
        self.fan_ctrl.apply_hardware_fan();

        if self.fan_ctrl.is_max_boost {
            self.terminal.log(
                "FAN",
                CyberTheme::NEON_PINK,
                "⚡ TURBO ENGAGED: Dispatched HP BIOS WMI Max Fan blast (100% ~5400 RPM).",
            );
            self.set_status("MAX FAN SPEED ACTIVE [100% TURBO].");
        } else {
            self.terminal.log(
                "FAN",
                CyberTheme::NEON_CYAN,
                "Dispatched HP BIOS WMI Auto Curve. Fans returned to automatic thermal curve.",
            );
            self.set_status("Fans returned to AUTO mode.");
        }
    }

    pub fn set_fan_max_explicit(&mut self, is_max: bool) {
        if self.fan_ctrl.is_max_boost != is_max {
            self.toggle_max_fan();
        }
    }

    /// Complete stand-down and exit: restores OEM services, resets fan overrides to BIOS auto,
    /// sets power scheme to Balanced, and closes KilJoy so OMEN Gaming Hub has 100% control.
    pub fn turn_off_and_restore_omen(&mut self, ctx: &egui::Context) {
        self.terminal.log(
            "DISENGAGE",
            CyberTheme::NEON_YELLOW,
            "Releasing hardware fan overrides back to OEM BIOS...",
        );

        // 1. Revert fans back to BIOS Auto
        self.fan_ctrl.is_max_boost = false;
        self.fan_ctrl.mode = FanProfileMode::Auto;
        crate::fans::send_hp_wmi_fan_command(0, false);

        // 2. Revert Windows power scheme to Balanced (OEM default)
        let _ = DeviceMode::Balanced.apply_system_profile();

        // 3. Restore all HP OEM background services for OMEN Gaming Hub
        let _ = self.bloat_killer.restore_all_services();

        self.set_status("KilJoy disengaged. OEM services restored. Exiting...");

        // 4. Clean exit
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Toggle passive monitor mode: keeps KilJoy open for monitoring without overriding fans or power
    pub fn toggle_passive_mode(&mut self) {
        self.is_passive_mode = !self.is_passive_mode;
        if self.is_passive_mode {
            // Relinquish fans to BIOS
            self.fan_ctrl.is_max_boost = false;
            self.fan_ctrl.mode = FanProfileMode::Auto;
            crate::fans::send_hp_wmi_fan_command(0, false);
            // Restore OEM services so OMEN Gaming Hub can run concurrently
            let _ = self.bloat_killer.restore_all_services();
            self.terminal.log(
                "STANDDOWN",
                CyberTheme::NEON_YELLOW,
                "Passive Monitor Mode engaged. Fan control relinquished to OMEN Gaming Hub.",
            );
            self.set_status("Passive Monitor Mode: Stand-down active.");
        } else {
            self.terminal.log(
                "STANDDOWN",
                CyberTheme::NEON_CYAN,
                "Active Control Mode resumed. KilJoy now managing hardware profiles.",
            );
            self.fan_ctrl.apply_hardware_fan();
            self.set_status("Active Control Mode resumed.");
        }
    }

    pub fn handle_terminal_command(&mut self) {
        let cmd = self.terminal.input_buffer.trim().to_string();
        if cmd.is_empty() {
            return;
        }

        self.terminal.push_history(&cmd);
        self.terminal
            .log("USER", CyberTheme::NEON_CYAN, &format!("> {}", cmd));
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let action = parts[0].to_lowercase();

        match action.as_str() {
            "help" => {
                self.terminal.log_info("Available commands:");
                self.terminal.log_info("  mode <eco|quiet|balanced|perf> - Switch laptop power profile");
                self.terminal.log_info("  fan <auto|max|<percent>>        - Set fan mode or speed (e.g. fan max)");
                self.terminal.log_info("  purge                           - Stop HP/Omen bloat services");
                self.terminal.log_info("  restore                         - Restart OEM services");
                self.terminal.log_info("  vitals                          - Print hardware telemetry");
                self.terminal.log_info("  clear / cls                     - Clear console buffer");
                self.terminal.log_info("  exit                            - Close KilJoy");
            }
            "mode" => {
                if parts.len() > 1 {
                    match parts[1].to_lowercase().as_str() {
                        "eco" => self.set_mode(DeviceMode::Eco),
                        "quiet" => self.set_mode(DeviceMode::Quiet),
                        "balanced" | "default" => self.set_mode(DeviceMode::Balanced),
                        "perf" | "performance" => self.set_mode(DeviceMode::Performance),
                        _ => self.terminal.log_warn("Unknown mode. Use: eco, quiet, balanced, perf"),
                    }
                } else {
                    self.terminal.log_warn("Usage: mode <eco|quiet|balanced|perf>");
                }
            }
            "fan" => {
                if parts.len() > 1 {
                    match parts[1].to_lowercase().as_str() {
                        "auto" => {
                            self.set_fan_max_explicit(false);
                            self.fan_ctrl.mode = FanProfileMode::Auto;
                            self.fan_ctrl.apply_hardware_fan();
                            self.terminal.log_success("Fans set to AUTO mode (HP WMI ACPI).");
                        }
                        "max" | "turbo" | "100" => {
                            self.set_fan_max_explicit(true);
                        }
                        s => {
                            if let Ok(pct) = s.parse::<f32>() {
                                self.fan_ctrl.is_max_boost = false;
                                self.fan_ctrl.mode = FanProfileMode::Manual;
                                self.fan_ctrl.manual_speed_pct = pct.clamp(20.0, 100.0);
                                self.fan_ctrl.apply_hardware_fan();
                                self.terminal.log_success(&format!("Fan speed manually dispatched: {:.0}%.", pct));
                            } else {
                                self.terminal.log_warn("Usage: fan <auto|max|<percent>>");
                            }
                        }
                    }
                } else {
                    self.terminal.log_warn("Usage: fan <auto|max|<percent>>");
                }
            }
            "passive" | "standdown" => {
                self.toggle_passive_mode();
            }
            "purge" => {
                let logs = self.bloat_killer.purge_all_bloat();
                for l in logs {
                    self.terminal.log("PURGE", CyberTheme::NEON_RED, &l);
                }
                self.terminal.log_success(&format!(
                    "Purged bloat. Reclaimed ~{} MB RAM.",
                    self.bloat_killer.total_ram_freed_mb
                ));
            }
            "restore" => {
                let logs = self.bloat_killer.restore_all_services();
                for l in logs {
                    self.terminal.log("RESTORE", CyberTheme::NEON_GREEN, &l);
                }
            }
            "vitals" => {
                let v = &self.current_vitals;
                self.terminal.log_info(&format!(
                    "CPU: {:.1}% | {:.1}°C | RAM: {:.1}/{:.1} GB",
                    v.cpu_usage, v.cpu_temperature_c, v.ram_used_gb, v.ram_total_gb
                ));
                self.terminal.log_info(&format!(
                    "GPU: {} | {}°C | {}% Util | {:.1}W",
                    v.gpu.name, v.gpu.temperature_c, v.gpu.utilization_gpu, v.gpu.power_watts
                ));
            }
            "clear" | "cls" => {
                self.terminal.clear();
            }
            "exit" => {
                std::process::exit(0);
            }
            _ => {
                self.terminal.log_warn(&format!("Unknown command: '{}'. Type 'help'.", action));
            }
        }

        self.terminal.input_buffer.clear();
    }
}

impl eframe::App for KilJoyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous repaint for smooth animations and 1s telemetry updates
        ctx.request_repaint_after(std::time::Duration::from_millis(250));

        // Sync vitals from background thread
        if let Ok(v) = self.monitor.vitals.lock() {
            self.current_vitals = v.clone();
            self.fan_ctrl
                .update_telemetry(v.cpu_temperature_c, v.gpu.temperature_c as f32);
        }

        // Left Navigation Sidebar (Armoury Crate / OMEN Hub styling)
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .exact_width(220.0)
            .frame(
                egui::Frame::none()
                    .fill(CyberTheme::BG_PANEL)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .inner_margin(egui::Margin::symmetric(14.0, 16.0)),
            )
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        // Bottom Status Ticker
        egui::TopBottomPanel::bottom("footer_panel")
            .frame(
                egui::Frame::none()
                    .fill(CyberTheme::BG_PANEL)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .inner_margin(egui::Margin::symmetric(16.0, 8.0)),
            )
            .show(ctx, |ui| {
                self.render_footer(ui);
            });

        // Central Content Area
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(CyberTheme::BG_VOID)
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .show(ctx, |ui| match self.active_tab {
                ActiveTab::PerformanceControl => self.render_performance_tab(ui),
                ActiveTab::SystemVitals => self.render_vitals_tab(ui),
                ActiveTab::FanCurveMatrix => self.render_fan_curve_tab(ui),
                ActiveTab::BloatKiller => self.render_bloat_killer_tab(ui),
                ActiveTab::CyberTerminal => self.render_terminal_tab(ui),
            });
    }
}

impl KilJoyApp {
    fn render_sidebar(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // App Title Banner
            ui.label(
                RichText::new("⚡ KILJOY")
                    .font(egui::FontId::monospace(22.0))
                    .strong()
                    .color(CyberTheme::NEON_CYAN),
            );
            ui.label(
                RichText::new("HP VICTUS CYBER-DECK")
                    .font(egui::FontId::monospace(10.0))
                    .color(CyberTheme::TEXT_MUTED),
            );

            ui.add_space(4.0);

            // Administrator Elevation Status
            if self.fan_ctrl.is_elevated {
                ui.label(
                    RichText::new("🛡 EC LINK: ACTIVE (ADMIN)")
                        .font(egui::FontId::monospace(9.5))
                        .strong()
                        .color(CyberTheme::NEON_GREEN),
                );
            } else {
                let elev_btn = egui::Button::new(
                    RichText::new("⚠️ EC: LOCKED [ELEVATE]")
                        .font(egui::FontId::monospace(9.5))
                        .strong()
                        .color(CyberTheme::NEON_YELLOW),
                )
                .fill(Color32::from_rgb(38, 22, 10))
                .stroke(Stroke::new(1.0, CyberTheme::NEON_YELLOW));

                if ui.add(elev_btn).on_hover_text("Click to restart KilJoy with Administrator privileges to unlock direct HP BIOS fan control.").clicked() {
                    crate::fans::restart_elevated();
                }
            }

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(12.0);

            // Navigation Tabs
            let tabs = [
                (ActiveTab::PerformanceControl, "⚡ PERFORMANCE"),
                (ActiveTab::SystemVitals, "📊 SYSTEM VITALS"),
                (ActiveTab::FanCurveMatrix, "🌀 FAN MATRIX"),
                (ActiveTab::BloatKiller, "🛡 DE-BLOATER"),
                (ActiveTab::CyberTerminal, "💻 TERMINAL"),
            ];

            for (tab, label) in tabs {
                let is_active = self.active_tab == tab;
                let btn_bg = if is_active {
                    Color32::from_rgb(20, 28, 42)
                } else {
                    CyberTheme::BG_CARD
                };
                let border_stroke = if is_active {
                    Stroke::new(1.5, CyberTheme::NEON_CYAN)
                } else {
                    Stroke::new(1.0, CyberTheme::BORDER_DIM)
                };
                let text_color = if is_active {
                    CyberTheme::NEON_CYAN
                } else {
                    CyberTheme::TEXT_MAIN
                };

                let btn = egui::Button::new(
                    RichText::new(label)
                        .font(egui::FontId::monospace(12.0))
                        .strong()
                        .color(text_color),
                )
                .fill(btn_bg)
                .stroke(border_stroke)
                .min_size(Vec2::new(192.0, 36.0));

                if ui.add(btn).clicked() {
                    self.active_tab = tab;
                }
                ui.add_space(6.0);
            }

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(12.0);

            // DEDICATED SIDEBAR "FAN MAX SPEED" BUTTON
            ui.label(
                RichText::new("// EMERGENCY COOLING")
                    .font(egui::FontId::monospace(10.0))
                    .color(CyberTheme::TEXT_MUTED),
            );
            ui.add_space(4.0);

            let is_max = self.fan_ctrl.is_max_boost;
            let (max_btn_text, max_btn_color, max_btn_bg, max_stroke) = if is_max {
                (
                    "🔥 FAN MAX: ACTIVE (100%)",
                    CyberTheme::NEON_RED,
                    Color32::from_rgb(45, 12, 22),
                    Stroke::new(2.0, CyberTheme::NEON_RED),
                )
            } else {
                (
                    "❄ FAN MAX SPEED",
                    CyberTheme::TEXT_MAIN,
                    CyberTheme::BG_CARD,
                    Stroke::new(1.0, CyberTheme::NEON_CYAN),
                )
            };

            let max_fan_btn = egui::Button::new(
                RichText::new(max_btn_text)
                    .font(egui::FontId::monospace(11.0))
                    .strong()
                    .color(max_btn_color),
            )
            .fill(max_btn_bg)
            .stroke(max_stroke)
            .min_size(Vec2::new(192.0, 38.0));

            if ui.add(max_fan_btn).clicked() {
                self.toggle_max_fan();
            }

            ui.add_space(14.0);

            // Telemetry Snapshot in Sidebar
            egui::Frame::none()
                .fill(Color32::from_rgb(10, 13, 19))
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                .rounding(4.0)
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("SYSTEM VITALS")
                            .font(egui::FontId::monospace(10.0))
                            .color(CyberTheme::TEXT_MUTED),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("CPU:").font(egui::FontId::monospace(11.0)).color(CyberTheme::TEXT_MUTED));
                        ui.label(
                            RichText::new(format!("{:.0}°C | {:.0}%", self.current_vitals.cpu_temperature_c, self.current_vitals.cpu_usage))
                                .font(egui::FontId::monospace(11.0))
                                .color(CyberTheme::NEON_GREEN)
                                .strong(),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("GPU:").font(egui::FontId::monospace(11.0)).color(CyberTheme::TEXT_MUTED));
                        ui.label(
                            RichText::new(format!("{}°C | {}%", self.current_vitals.gpu.temperature_c, self.current_vitals.gpu.utilization_gpu))
                                .font(egui::FontId::monospace(11.0))
                                .color(CyberTheme::NEON_PINK)
                                .strong(),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("FAN:").font(egui::FontId::monospace(11.0)).color(CyberTheme::TEXT_MUTED));
                        let fan_str = if self.fan_ctrl.is_max_boost {
                            "MAX 5400 RPM".to_string()
                        } else {
                            format!("{} RPM", self.fan_ctrl.cpu_fan_rpm)
                        };
                        ui.label(
                            RichText::new(fan_str)
                                .font(egui::FontId::monospace(11.0))
                                .color(if self.fan_ctrl.is_max_boost { CyberTheme::NEON_RED } else { CyberTheme::NEON_CYAN })
                                .strong(),
                        );
                    });
                });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // STAND DOWN / PASSIVE MONITOR TOGGLE
            let (standdown_txt, standdown_col, standdown_bg) = if self.is_passive_mode {
                ("🟢 RESUME ACTIVE HUD", CyberTheme::NEON_GREEN, Color32::from_rgb(15, 30, 20))
            } else {
                ("⏸️ STAND DOWN (OMEN ON)", CyberTheme::TEXT_MUTED, CyberTheme::BG_CARD)
            };

            let standdown_btn = egui::Button::new(
                RichText::new(standdown_txt)
                    .font(egui::FontId::monospace(10.5))
                    .strong()
                    .color(standdown_col),
            )
            .fill(standdown_bg)
            .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
            .min_size(Vec2::new(192.0, 30.0));

            if ui.add(standdown_btn).on_hover_text("Toggles Passive Mode: relinquishes fan and power overrides to OEM BIOS & OMEN Gaming Hub while keeping KilJoy open as a live telemetry monitor.").clicked() {
                self.toggle_passive_mode();
            }

            ui.add_space(8.0);

            // DEDICATED TURN OFF BUTTON (Releases fan override, restores HP services, and exits)
            let exit_btn = egui::Button::new(
                RichText::new("🛑 TURN OFF KILJOY\n[ RESTORE OMEN HUB ]")
                    .font(egui::FontId::monospace(10.5))
                    .strong()
                    .color(Color32::from_rgb(255, 110, 110)),
            )
            .fill(Color32::from_rgb(38, 14, 18))
            .stroke(Stroke::new(1.2, Color32::from_rgb(180, 45, 50)))
            .min_size(Vec2::new(192.0, 38.0));

            if ui.add(exit_btn).on_hover_text("Completely turns off KilJoy: returns hardware fans to BIOS Auto, sets power scheme to Balanced, restores HP background services, and exits so OMEN Gaming Hub operates without any interference.").clicked() {
                self.turn_off_and_restore_omen(ui.ctx());
            }
        });
    }

    fn render_performance_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Header
            ui.label(
                RichText::new("// PERFORMANCE CONTROL MATRIX")
                    .font(egui::FontId::monospace(18.0))
                    .color(CyberTheme::NEON_CYAN)
                    .strong(),
            );
            ui.label(
                RichText::new("Select operational power mode for Intel Core i5-13420H & NVIDIA RTX 4050.")
                    .font(egui::FontId::monospace(11.0))
                    .color(CyberTheme::TEXT_MUTED),
            );

            ui.add_space(16.0);

            // 4 Big Clickable Cyberpunk Mode Cards (Grid 4 Columns)
            let modes = [
                DeviceMode::Eco,
                DeviceMode::Quiet,
                DeviceMode::Balanced,
                DeviceMode::Performance,
            ];

            ui.columns(4, |cols| {
                for (idx, mode) in modes.iter().enumerate() {
                    let col = &mut cols[idx];
                    let is_active = self.active_mode == *mode;
                    let color = mode.accent_color();

                    let border_stroke = if is_active {
                        Stroke::new(2.0, color)
                    } else {
                        Stroke::new(1.0, CyberTheme::BORDER_DIM)
                    };

                    let bg_color = if is_active {
                        Color32::from_rgb(18, 25, 36)
                    } else {
                        CyberTheme::BG_CARD
                    };

                    // Whole Card Frame
                    let card_response = egui::Frame::none()
                        .fill(bg_color)
                        .stroke(border_stroke)
                        .rounding(6.0)
                        .inner_margin(egui::Margin::same(16.0))
                        .show(col, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new(mode.icon())
                                        .font(egui::FontId::monospace(28.0)),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(mode.name())
                                        .font(egui::FontId::monospace(17.0))
                                        .strong()
                                        .color(color),
                                );
                                ui.label(
                                    RichText::new(mode.subtitle())
                                        .font(egui::FontId::monospace(10.0))
                                        .color(CyberTheme::TEXT_MUTED),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(mode.description())
                                        .font(egui::FontId::monospace(11.0))
                                        .color(CyberTheme::TEXT_MAIN),
                                );
                                ui.add_space(14.0);

                                let (btn_text, btn_fill) = if is_active {
                                    ("● ACTIVE", color)
                                } else {
                                    ("[ ENGAGE ]", CyberTheme::BORDER_DIM)
                                };

                                let btn = egui::Button::new(
                                    RichText::new(btn_text)
                                        .font(egui::FontId::monospace(12.0))
                                        .strong()
                                        .color(if is_active { Color32::BLACK } else { CyberTheme::TEXT_MAIN }),
                                )
                                .fill(btn_fill)
                                .min_size(Vec2::new(110.0, 30.0));

                                if ui.add(btn).clicked() {
                                    self.set_mode(*mode);
                                }
                            });
                        });

                    // Clicking ANYWHERE on the card selects the mode
                    if card_response.response.interact(egui::Sense::click()).clicked() {
                        self.set_mode(*mode);
                    }
                }
            });

            ui.add_space(20.0);

            // DEDICATED FAN SPEED & MAX FAN CONTROL BAR
            egui::Frame::none()
                .fill(CyberTheme::BG_CARD)
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(20.0, 14.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("FAN CONTROLLER:")
                                .font(egui::FontId::monospace(13.0))
                                .strong()
                                .color(CyberTheme::TEXT_MAIN),
                        );

                        ui.add_space(14.0);

                        // Mode: Auto Button
                        let is_auto = !self.fan_ctrl.is_max_boost && self.fan_ctrl.mode == FanProfileMode::Auto;
                        let auto_btn = egui::Button::new(
                            RichText::new(if is_auto { "● AUTO [DYNAMIC]" } else { "AUTO" })
                                .font(egui::FontId::monospace(12.0))
                                .strong()
                                .color(if is_auto { CyberTheme::NEON_CYAN } else { CyberTheme::TEXT_MUTED }),
                        )
                        .fill(if is_auto { Color32::from_rgb(18, 30, 42) } else { CyberTheme::BG_PANEL })
                        .stroke(Stroke::new(1.0, if is_auto { CyberTheme::NEON_CYAN } else { CyberTheme::BORDER_DIM }))
                        .min_size(Vec2::new(120.0, 32.0));

                        if ui.add(auto_btn).clicked() {
                            self.fan_ctrl.is_max_boost = false;
                            self.fan_ctrl.mode = FanProfileMode::Auto;
                            self.config.is_max_boost = false;
                            self.config.fan_mode = FanProfileMode::Auto;
                            let _ = self.config.save();
                            self.fan_ctrl.apply_hardware_fan();
                            self.set_status("Fans set to dynamic AUTO curve (HP WMI ACPI).");
                        }

                        ui.add_space(8.0);

                        // Mode: MAX FAN SPEED BUTTON (The dedicated max button requested by user)
                        let is_max = self.fan_ctrl.is_max_boost;
                        let max_btn = egui::Button::new(
                            RichText::new(if is_max { "🔥 MAX FAN (100% TURBO) ●" } else { "🔥 MAX FAN (100%)" })
                                .font(egui::FontId::monospace(12.0))
                                .strong()
                                .color(if is_max { CyberTheme::NEON_RED } else { CyberTheme::TEXT_MAIN }),
                        )
                        .fill(if is_max { Color32::from_rgb(50, 14, 25) } else { CyberTheme::BG_PANEL })
                        .stroke(Stroke::new(1.5, if is_max { CyberTheme::NEON_RED } else { CyberTheme::NEON_CYAN }))
                        .min_size(Vec2::new(170.0, 32.0));

                        if ui.add(max_btn).clicked() {
                            self.toggle_max_fan();
                        }

                        ui.add_space(14.0);

                        // Manual Fan Speed Slider
                        let is_manual = !self.fan_ctrl.is_max_boost && self.fan_ctrl.mode == FanProfileMode::Manual;
                        let mut slider_val = self.fan_ctrl.manual_speed_pct;
                        ui.label(RichText::new("MANUAL:").font(egui::FontId::monospace(11.0)).color(if is_manual { CyberTheme::NEON_CYAN } else { CyberTheme::TEXT_MUTED }));
                        let slider_res = ui.add(
                            egui::Slider::new(&mut slider_val, 20.0..=100.0)
                                .text("%")
                                .fixed_decimals(0)
                        );
                        if slider_res.changed() {
                            self.fan_ctrl.manual_speed_pct = slider_val;
                            self.fan_ctrl.is_max_boost = false;
                            self.fan_ctrl.mode = FanProfileMode::Manual;
                            self.config.manual_fan_pct = slider_val;
                            self.config.fan_mode = FanProfileMode::Manual;
                            self.config.is_max_boost = false;
                            let _ = self.config.save();
                            self.fan_ctrl.apply_hardware_fan();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let curve_btn = egui::Button::new(
                                RichText::new("[ EDIT FAN CURVE ]")
                                    .font(egui::FontId::monospace(11.0))
                                    .color(CyberTheme::NEON_CYAN),
                            )
                            .fill(CyberTheme::BG_PANEL)
                            .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM));

                            if ui.add(curve_btn).clicked() {
                                self.active_tab = ActiveTab::FanCurveMatrix;
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Status and RPM readout row
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "BLOWERS: CPU {} RPM | GPU {} RPM",
                                self.fan_ctrl.cpu_fan_rpm, self.fan_ctrl.gpu_fan_rpm
                            ))
                            .font(egui::FontId::monospace(11.0))
                            .strong()
                            .color(CyberTheme::NEON_GREEN),
                        );

                        ui.add_space(14.0);

                        ui.label(
                            RichText::new(&self.fan_ctrl.last_hardware_status)
                                .font(egui::FontId::monospace(10.5))
                                .strong()
                                .color(if self.fan_ctrl.last_hardware_status.contains("ACCESS DENIED") {
                                    CyberTheme::NEON_RED
                                } else if self.fan_ctrl.last_hardware_status.contains("MAX FAN") {
                                    CyberTheme::NEON_PINK
                                } else {
                                    CyberTheme::NEON_CYAN
                                }),
                        );

                        if !self.fan_ctrl.is_elevated {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let elev_btn = egui::Button::new(
                                    RichText::new("⚠️ RUN AS ADMIN TO UNLOCK FANS")
                                        .font(egui::FontId::monospace(10.0))
                                        .strong()
                                        .color(Color32::BLACK),
                                )
                                .fill(CyberTheme::NEON_YELLOW);

                                if ui.add(elev_btn).on_hover_text("HP BIOS WMI commands require Windows Administrator privileges. Click to elevate now.").clicked() {
                                    crate::fans::restart_elevated();
                                }
                            });
                        }
                    });
                });

            ui.add_space(20.0);

            // Bottom Live Vitals Ticker Cards (Matching the Omen Hub Screenshot!)
            ui.label(
                RichText::new("// LIVE SYSTEM VITALS")
                    .font(egui::FontId::monospace(13.0))
                    .color(CyberTheme::TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.columns(4, |cols| {
                // Card 1: CPU Temperature
                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(16.0))
                    .show(&mut cols[0], |ui| {
                        ui.vertical_centered(|ui| {
                            let temp = self.current_vitals.cpu_temperature_c;
                            let color = if temp < 50.0 {
                                CyberTheme::NEON_GREEN
                            } else if temp < 75.0 {
                                CyberTheme::NEON_CYAN
                            } else if temp < 85.0 {
                                CyberTheme::NEON_AMBER
                            } else {
                                CyberTheme::NEON_RED
                            };

                            ui.label(
                                RichText::new(format!("{:.0}°C", temp))
                                    .font(egui::FontId::monospace(36.0))
                                    .strong()
                                    .color(color),
                            );
                            ui.label(
                                RichText::new("CPU TEMPERATURE")
                                    .font(egui::FontId::monospace(11.0))
                                    .color(CyberTheme::TEXT_MUTED),
                            );
                        });
                    });

                // Card 2: GPU Temperature
                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(16.0))
                    .show(&mut cols[1], |ui| {
                        ui.vertical_centered(|ui| {
                            let gtemp = self.current_vitals.gpu.temperature_c;
                            let color = if gtemp < 50 {
                                CyberTheme::NEON_GREEN
                            } else if gtemp < 72 {
                                CyberTheme::NEON_CYAN
                            } else {
                                CyberTheme::NEON_PINK
                            };

                            ui.label(
                                RichText::new(format!("{}°C", gtemp))
                                    .font(egui::FontId::monospace(36.0))
                                    .strong()
                                    .color(color),
                            );
                            ui.label(
                                RichText::new("GPU TEMPERATURE")
                                    .font(egui::FontId::monospace(11.0))
                                    .color(CyberTheme::TEXT_MUTED),
                            );
                        });
                    });

                // Card 3: CPU Utilization
                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(16.0))
                    .show(&mut cols[2], |ui| {
                        ui.vertical_centered(|ui| {
                            let usage = self.current_vitals.cpu_usage;
                            ui.label(
                                RichText::new(format!("{:.0}%", usage))
                                    .font(egui::FontId::monospace(36.0))
                                    .strong()
                                    .color(CyberTheme::NEON_CYAN),
                            );
                            ui.label(
                                RichText::new("CPU UTILIZATION")
                                    .font(egui::FontId::monospace(11.0))
                                    .color(CyberTheme::TEXT_MUTED),
                            );
                        });
                    });

                // Card 4: GPU Power & Util
                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(16.0))
                    .show(&mut cols[3], |ui| {
                        ui.vertical_centered(|ui| {
                            let power = self.current_vitals.gpu.power_watts;
                            ui.label(
                                RichText::new(format!("{:.0} W", power))
                                    .font(egui::FontId::monospace(36.0))
                                    .strong()
                                    .color(CyberTheme::NEON_PINK),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "GPU TGP ({}% UTIL)",
                                    self.current_vitals.gpu.utilization_gpu
                                ))
                                .font(egui::FontId::monospace(11.0))
                                .color(CyberTheme::TEXT_MUTED),
                            );
                        });
                    });
            });
        });
    }

    fn render_vitals_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("// HARDWARE TELEMETRY & NEURAL GAUGES")
                    .font(egui::FontId::monospace(18.0))
                    .color(CyberTheme::NEON_CYAN)
                    .strong(),
            );
            ui.label(
                RichText::new("Real-time telemetry stream from CPU register rings, NVML, and memory buses.")
                    .font(egui::FontId::monospace(11.0))
                    .color(CyberTheme::TEXT_MUTED),
            );

            ui.add_space(14.0);

            // Columns: Left = CPU Cores, Right = GPU & RAM & Chart
            ui.columns(2, |cols| {
                // Left Column: CPU Core Breakdown
                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(14.0))
                    .show(&mut cols[0], |ui| {
                        ui.label(
                            RichText::new(&format!("INTEL CORE I5-13420H ({:.1}%)", self.current_vitals.cpu_usage))
                                .font(egui::FontId::monospace(14.0))
                                .strong()
                                .color(CyberTheme::NEON_GREEN),
                        );
                        ui.label(
                            RichText::new(format!("Temperature: {:.1}°C | 8 Cores / 12 Logical Processors", self.current_vitals.cpu_temperature_c))
                                .font(egui::FontId::monospace(11.0))
                                .color(CyberTheme::TEXT_MUTED),
                        );
                        ui.add_space(10.0);

                        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                            for core in &self.current_vitals.cpu_cores {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&core.name)
                                            .font(egui::FontId::monospace(11.0))
                                            .color(CyberTheme::TEXT_MUTED),
                                    );
                                    let progress = core.usage / 100.0;
                                    let bar = egui::ProgressBar::new(progress)
                                        .desired_width(170.0)
                                        .text(format!("{:.0}%", core.usage));
                                    ui.add(bar);

                                    ui.label(
                                        RichText::new(format!("{} MHz", core.frequency_mhz))
                                            .font(egui::FontId::monospace(11.0))
                                            .color(CyberTheme::TEXT_MAIN),
                                    );
                                });
                            }
                        });
                    });

                // Right Column: GPU & Memory Stats
                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(14.0))
                    .show(&mut cols[1], |ui| {
                        ui.label(
                            RichText::new("NVIDIA GEFORCE RTX 4050 LAPTOP")
                                .font(egui::FontId::monospace(14.0))
                                .strong()
                                .color(CyberTheme::NEON_PINK),
                        );
                        ui.label(
                            RichText::new(format!("Temp: {}°C | Clock Power: {:.1}W / {:.1}W", self.current_vitals.gpu.temperature_c, self.current_vitals.gpu.power_watts, self.current_vitals.gpu.power_limit_watts))
                                .font(egui::FontId::monospace(11.0))
                                .color(CyberTheme::TEXT_MUTED),
                        );
                        ui.add_space(10.0);

                        // GPU Util Bar
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("GPU CORE:").font(egui::FontId::monospace(11.0)).color(CyberTheme::TEXT_MUTED));
                            let g_bar = egui::ProgressBar::new(self.current_vitals.gpu.utilization_gpu as f32 / 100.0)
                                .text(format!("{}%", self.current_vitals.gpu.utilization_gpu));
                            ui.add(g_bar);
                        });

                        // VRAM Bar
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("VRAM:    ").font(egui::FontId::monospace(11.0)).color(CyberTheme::TEXT_MUTED));
                            let total = self.current_vitals.gpu.memory_total_mb.max(1) as f32;
                            let used = self.current_vitals.gpu.memory_used_mb as f32;
                            let vram_bar = egui::ProgressBar::new(used / total)
                                .text(format!("{:.0}/{:.0} MB", used, total));
                            ui.add(vram_bar);
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // System RAM & Battery
                        ui.label(
                            RichText::new("SYSTEM RAM & POWER")
                                .font(egui::FontId::monospace(12.0))
                                .strong()
                                .color(CyberTheme::NEON_CYAN),
                        );

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("DDR5 RAM:").font(egui::FontId::monospace(11.0)).color(CyberTheme::TEXT_MUTED));
                            let ram_bar = egui::ProgressBar::new(self.current_vitals.ram_usage_percent / 100.0)
                                .text(format!("{:.1} / {:.1} GB ({:.0}%)", self.current_vitals.ram_used_gb, self.current_vitals.ram_total_gb, self.current_vitals.ram_usage_percent));
                            ui.add(ram_bar);
                        });

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("POWER:   ").font(egui::FontId::monospace(11.0)).color(CyberTheme::TEXT_MUTED));
                            let bat = self.current_vitals.battery_percent.unwrap_or(100);
                            let pwr_str = if self.current_vitals.is_on_ac {
                                format!("{}% [AC MAINS CONNECTED]", bat)
                            } else {
                                format!("{}% [ON BATTERY]", bat)
                            };
                            ui.label(
                                RichText::new(pwr_str)
                                    .font(egui::FontId::monospace(11.0))
                                    .color(if self.current_vitals.is_on_ac { CyberTheme::NEON_GREEN } else { CyberTheme::NEON_AMBER }),
                            );
                        });
                    });
            });

            ui.add_space(16.0);

            // Real-Time Plot
            egui::Frame::none()
                .fill(CyberTheme::BG_CARD)
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                .rounding(6.0)
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("// TELEMETRY TIME-SERIES (LAST 60 SECONDS)")
                            .font(egui::FontId::monospace(12.0))
                            .strong()
                            .color(CyberTheme::TEXT_MAIN),
                    );

                    let cpu_temp_line = Line::new(PlotPoints::new(self.current_vitals.cpu_temp_history.clone()))
                        .color(CyberTheme::NEON_GREEN)
                        .name("CPU Temp (°C)");

                    let gpu_temp_line = Line::new(PlotPoints::new(self.current_vitals.gpu_temp_history.clone()))
                        .color(CyberTheme::NEON_PINK)
                        .name("GPU Temp (°C)");

                    let cpu_usage_line = Line::new(PlotPoints::new(self.current_vitals.cpu_usage_history.clone()))
                        .color(CyberTheme::NEON_CYAN)
                        .name("CPU Usage (%)");

                    Plot::new("vitals_plot")
                        .height(180.0)
                        .legend(egui_plot::Legend::default())
                        .show(ui, |plot_ui| {
                            plot_ui.line(cpu_temp_line);
                            plot_ui.line(gpu_temp_line);
                            plot_ui.line(cpu_usage_line);
                        });
                });
        });
    }

    fn render_fan_curve_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("// FAN CURVE MATRIX & THERMAL INTERPOLATOR")
                    .font(egui::FontId::monospace(18.0))
                    .color(CyberTheme::NEON_CYAN)
                    .strong(),
            );
            ui.label(
                RichText::new("Configure cooling fans RPM profile relative to component die thermals.")
                    .font(egui::FontId::monospace(11.0))
                    .color(CyberTheme::TEXT_MUTED),
            );

            ui.add_space(14.0);

            // Controls Bar
            ui.horizontal(|ui| {
                ui.label(RichText::new("PRESETS:").font(egui::FontId::monospace(12.0)).color(CyberTheme::TEXT_MUTED));

                let presets = ["Balanced Curve", "Silent Stealth", "Aggressive Cool", "Linear 1:1"];
                for p in presets {
                    let is_active = self.fan_ctrl.active_preset_name == p;
                    let btn = egui::Button::new(
                        RichText::new(if is_active { format!("● {}", p) } else { p.to_string() })
                            .font(egui::FontId::monospace(11.0))
                            .strong()
                            .color(if is_active { CyberTheme::NEON_CYAN } else { CyberTheme::TEXT_MAIN }),
                    )
                    .fill(if is_active { Color32::from_rgb(18, 28, 40) } else { CyberTheme::BG_CARD })
                    .stroke(Stroke::new(1.0, if is_active { CyberTheme::NEON_CYAN } else { CyberTheme::BORDER_DIM }));

                    if ui.add(btn).clicked() {
                        self.fan_ctrl.load_preset(p);
                        self.fan_ctrl.mode = FanProfileMode::CustomCurve;
                        self.config.custom_curve = self.fan_ctrl.curve_points.clone();
                        let _ = self.config.save();
                        self.fan_ctrl.apply_hardware_fan();
                        self.terminal.log_info(&format!("Loaded fan curve preset: {}", p));
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_max = self.fan_ctrl.is_max_boost;
                    let max_toggle_btn = egui::Button::new(
                        RichText::new(if is_max { "🔥 DISENGAGE MAX" } else { "🔥 FAN MAX SPEED" })
                            .font(egui::FontId::monospace(11.0))
                            .strong()
                            .color(if is_max { CyberTheme::NEON_RED } else { CyberTheme::NEON_CYAN }),
                    )
                    .fill(if is_max { Color32::from_rgb(50, 14, 25) } else { CyberTheme::BG_CARD })
                    .stroke(Stroke::new(1.5, if is_max { CyberTheme::NEON_RED } else { CyberTheme::NEON_CYAN }));

                    if ui.add(max_toggle_btn).clicked() {
                        self.toggle_max_fan();
                    }
                });
            });

            ui.add_space(12.0);

            // Fan Curve Plot
            egui::Frame::none()
                .fill(CyberTheme::BG_CARD)
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                .rounding(6.0)
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    let mut plot_points = Vec::new();
                    // Sample curve from 30 to 95 C
                    for temp in 30..=95 {
                        let t = temp as f32;
                        let speed = self.fan_ctrl.calculate_target_speed(t);
                        plot_points.push([t as f64, speed as f64]);
                    }

                    let curve_line = Line::new(PlotPoints::new(plot_points))
                        .color(CyberTheme::NEON_CYAN)
                        .width(2.5)
                        .name("Target Fan %");

                    // Node markers
                    let nodes: Vec<[f64; 2]> = self
                        .fan_ctrl
                        .curve_points
                        .iter()
                        .map(|p| [p.temp_c as f64, p.speed_pct as f64])
                        .collect();
                    let node_points = Points::new(PlotPoints::new(nodes))
                        .color(CyberTheme::NEON_PINK)
                        .radius(5.0)
                        .name("Control Nodes");

                    // Current temp marker
                    let cur_temp = self.current_vitals.cpu_temperature_c as f64;
                    let cur_speed = self.fan_ctrl.calculate_target_speed(cur_temp as f32) as f64;
                    let live_point = Points::new(PlotPoints::new(vec![[cur_temp, cur_speed]]))
                        .color(CyberTheme::NEON_GREEN)
                        .radius(7.0)
                        .name("Live Operating Point");

                    Plot::new("fan_curve_plot")
                        .height(230.0)
                        .include_x(30.0)
                        .include_x(100.0)
                        .include_y(0.0)
                        .include_y(100.0)
                        .x_axis_label("Die Temperature (°C)")
                        .y_axis_label("Fan Speed (%)")
                        .legend(egui_plot::Legend::default())
                        .show(ui, |plot_ui| {
                            plot_ui.line(curve_line);
                            plot_ui.points(node_points);
                            plot_ui.points(live_point);
                        });
                });

            ui.add_space(14.0);

            // Node Slider Adjusters with Apply Button
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("// ADJUST CURVE CONTROL NODES")
                        .font(egui::FontId::monospace(12.0))
                        .color(CyberTheme::TEXT_MUTED),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let apply_btn = egui::Button::new(
                        RichText::new("⚡ APPLY CURVE TO FIRMWARE")
                            .font(egui::FontId::monospace(11.0))
                            .strong()
                            .color(CyberTheme::NEON_CYAN),
                    )
                    .fill(Color32::from_rgb(18, 30, 45))
                    .stroke(Stroke::new(1.0, CyberTheme::NEON_CYAN));

                    if ui.add(apply_btn).clicked() {
                        self.fan_ctrl.mode = FanProfileMode::CustomCurve;
                        self.fan_ctrl.is_max_boost = false;
                        self.config.custom_curve = self.fan_ctrl.curve_points.clone();
                        let _ = self.config.save();
                        self.fan_ctrl.apply_hardware_fan();
                        self.set_status("Custom fan curve dispatched to HP BIOS firmware.");
                    }
                });
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                for (idx, point) in self.fan_ctrl.curve_points.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(format!("Node {}", idx + 1))
                                .font(egui::FontId::monospace(11.0))
                                .color(CyberTheme::NEON_PINK),
                        );
                        ui.label(
                            RichText::new(format!("{:.0}°C", point.temp_c))
                                .font(egui::FontId::monospace(10.0))
                                .color(CyberTheme::TEXT_MUTED),
                        );
                        ui.add(egui::Slider::new(&mut point.speed_pct, 10.0..=100.0).text("%"));
                    });
                    ui.add_space(16.0);
                }
            });

            ui.add_space(8.0);

            // Hardware EC Status bar
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("HARDWARE STATUS:")
                        .font(egui::FontId::monospace(10.5))
                        .color(CyberTheme::TEXT_MUTED),
                );
                ui.label(
                    RichText::new(&self.fan_ctrl.last_hardware_status)
                        .font(egui::FontId::monospace(10.5))
                        .strong()
                        .color(if self.fan_ctrl.last_hardware_status.contains("ACCESS DENIED") {
                            CyberTheme::NEON_RED
                        } else if self.fan_ctrl.last_hardware_status.contains("MAX FAN") {
                            CyberTheme::NEON_PINK
                        } else {
                            CyberTheme::NEON_CYAN
                        }),
                );
            });

            ui.add_space(14.0);

            // Live RPM Meters
            ui.columns(2, |cols| {
                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(12.0))
                    .show(&mut cols[0], |ui| {
                        ui.label(
                            RichText::new(format!("CPU BLOWER: {} RPM", self.fan_ctrl.cpu_fan_rpm))
                                .font(egui::FontId::monospace(13.0))
                                .strong()
                                .color(CyberTheme::NEON_GREEN),
                        );
                        let pct = (self.fan_ctrl.cpu_fan_rpm as f32 / self.fan_ctrl.max_rpm as f32).clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(pct).text(format!("{:.0}%", pct * 100.0)));
                    });

                egui::Frame::none()
                    .fill(CyberTheme::BG_CARD)
                    .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::same(12.0))
                    .show(&mut cols[1], |ui| {
                        ui.label(
                            RichText::new(format!("GPU BLOWER: {} RPM", self.fan_ctrl.gpu_fan_rpm))
                                .font(egui::FontId::monospace(13.0))
                                .strong()
                                .color(CyberTheme::NEON_PINK),
                        );
                        let pct = (self.fan_ctrl.gpu_fan_rpm as f32 / self.fan_ctrl.max_rpm as f32).clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(pct).text(format!("{:.0}%", pct * 100.0)));
                    });
            });
        });
    }

    fn render_bloat_killer_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("// OEM BLOATWARE NEUTRALIZER & MEMORY LIBERATOR")
                    .font(egui::FontId::monospace(18.0))
                    .color(CyberTheme::NEON_RED)
                    .strong(),
            );
            ui.label(
                RichText::new("HP background services run promotional telemetry and background diagnostic scanners. Neutralize them in pure native Win32 (0ms).")
                    .font(egui::FontId::monospace(11.0))
                    .color(CyberTheme::TEXT_MUTED),
            );

            ui.add_space(14.0);

            // Action Buttons Bar
            ui.horizontal(|ui| {
                let purge_btn = egui::Button::new(
                    RichText::new("☠ PURGE ALL BLOAT SERVICES")
                        .font(egui::FontId::monospace(12.0))
                        .strong()
                        .color(CyberTheme::NEON_RED),
                )
                .fill(Color32::from_rgb(45, 12, 20))
                .stroke(Stroke::new(1.5, CyberTheme::NEON_RED))
                .min_size(Vec2::new(210.0, 36.0));

                if ui.add(purge_btn).clicked() {
                    let logs = self.bloat_killer.purge_all_bloat();
                    for l in logs {
                        self.terminal.log("PURGE", CyberTheme::NEON_RED, &l);
                    }
                    self.set_status("HP telemetry & background services neutralized.");
                }

                ui.add_space(10.0);

                let restore_btn = egui::Button::new(
                    RichText::new("↺ RESTORE OEM SERVICES")
                        .font(egui::FontId::monospace(12.0))
                        .color(CyberTheme::NEON_GREEN),
                )
                .fill(CyberTheme::BG_CARD)
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                .min_size(Vec2::new(180.0, 36.0));

                if ui.add(restore_btn).clicked() {
                    let logs = self.bloat_killer.restore_all_services();
                    for l in logs {
                        self.terminal.log("RESTORE", CyberTheme::NEON_GREEN, &l);
                    }
                    self.set_status("HP services restored.");
                }

                ui.add_space(10.0);

                let rescan_btn = egui::Button::new(
                    RichText::new("⟳ RESCAN")
                        .font(egui::FontId::monospace(12.0))
                        .color(CyberTheme::TEXT_MAIN),
                )
                .fill(CyberTheme::BG_CARD)
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                .min_size(Vec2::new(90.0, 36.0));

                if ui.add(rescan_btn).clicked() {
                    self.bloat_killer.refresh_service_states();
                    self.set_status("Service states updated via Win32.");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let saved_badge = format!("[ +{} MB RAM RECLAIMED ]", self.bloat_killer.total_ram_freed_mb);
                    ui.label(
                        RichText::new(saved_badge)
                            .font(egui::FontId::monospace(13.0))
                            .strong()
                            .color(CyberTheme::NEON_GREEN),
                    );
                });
            });

            ui.add_space(16.0);

            // Service Table
            egui::Frame::none()
                .fill(CyberTheme::BG_CARD)
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_DIM))
                .rounding(6.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for svc in &self.bloat_killer.services {
                            ui.horizontal(|ui| {
                                let (status_str, status_color) = if svc.is_running {
                                    ("RUNNING (DRAINING)", CyberTheme::NEON_AMBER)
                                } else {
                                    ("NEUTRALIZED (SILENT)", CyberTheme::NEON_GREEN)
                                };

                                ui.label(
                                    RichText::new(format!("[ {:<20} ]", status_str))
                                        .font(egui::FontId::monospace(11.0))
                                        .color(status_color),
                                );

                                ui.label(
                                    RichText::new(&svc.display_name)
                                        .font(egui::FontId::monospace(13.0))
                                        .strong()
                                        .color(CyberTheme::TEXT_MAIN),
                                );

                                ui.label(
                                    RichText::new(format!("({})", svc.name))
                                        .font(egui::FontId::monospace(10.0))
                                        .color(CyberTheme::TEXT_MUTED),
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("~{} MB", svc.ram_saving_mb))
                                            .font(egui::FontId::monospace(11.0))
                                            .color(CyberTheme::NEON_CYAN),
                                    );
                                });
                            });

                            ui.label(
                                RichText::new(format!("    └─ {}", svc.description))
                                    .font(egui::FontId::monospace(10.0))
                                    .color(CyberTheme::TEXT_MUTED),
                            );

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);
                        }
                    });
                });
        });
    }

    fn render_terminal_tab(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("// CYBER-DECK INTERACTIVE SHELL")
                    .font(egui::FontId::monospace(18.0))
                    .color(CyberTheme::NEON_CYAN)
                    .strong(),
            );
            ui.label(
                RichText::new("Direct kernel telemetry interface and ACPI controller. Type 'help' for instructions.")
                    .font(egui::FontId::monospace(11.0))
                    .color(CyberTheme::TEXT_MUTED),
            );

            ui.add_space(10.0);

            // Log output viewport
            let available_height = ui.available_height() - 44.0;
            egui::Frame::none()
                .fill(Color32::from_rgb(6, 8, 12))
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_CYAN))
                .rounding(4.0)
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(available_height)
                        .stick_to_bottom(self.terminal.auto_scroll)
                        .show(ui, |ui| {
                            for entry in &self.terminal.entries {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&entry.timestamp)
                                            .font(egui::FontId::monospace(10.0))
                                            .color(CyberTheme::TEXT_MUTED),
                                    );

                                    ui.label(
                                        RichText::new(format!("[{}]", entry.tag))
                                            .font(egui::FontId::monospace(11.0))
                                            .color(entry.tag_color)
                                            .strong(),
                                    );

                                    ui.label(
                                        RichText::new(&entry.message)
                                            .font(egui::FontId::monospace(11.0))
                                            .color(CyberTheme::TEXT_MAIN),
                                    );
                                });
                            }
                        });
                });

            ui.add_space(8.0);

            // Terminal Input Prompt
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("KILJOY:\\>")
                        .font(egui::FontId::monospace(13.0))
                        .strong()
                        .color(CyberTheme::NEON_GREEN),
                );

                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    if let Some(cmd) = self.terminal.history_prev() {
                        self.terminal.input_buffer = cmd;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    if let Some(cmd) = self.terminal.history_next() {
                        self.terminal.input_buffer = cmd;
                    }
                }

                let text_edit = egui::TextEdit::singleline(&mut self.terminal.input_buffer)
                    .font(egui::FontId::monospace(12.0))
                    .desired_width(ui.available_width() - 80.0);

                let response = ui.add(text_edit);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.handle_terminal_command();
                    response.request_focus();
                }

                let send_btn = egui::Button::new(
                    RichText::new("[ SEND ]")
                        .font(egui::FontId::monospace(11.0))
                        .color(CyberTheme::NEON_CYAN),
                )
                .fill(CyberTheme::BG_CARD)
                .stroke(Stroke::new(1.0, CyberTheme::BORDER_CYAN));

                if ui.add(send_btn).clicked() {
                    self.handle_terminal_command();
                }
            });
        });
    }

    fn render_footer(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("● KERNEL STATUS:")
                    .font(egui::FontId::monospace(10.0))
                    .color(CyberTheme::NEON_GREEN),
            );

            ui.label(
                RichText::new(&self.status_message)
                    .font(egui::FontId::monospace(10.0))
                    .color(CyberTheme::TEXT_MAIN),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("KILJOY v1.1.0 // PURE NATIVE WIN32 // ZERO BLOAT")
                        .font(egui::FontId::monospace(10.0))
                        .color(CyberTheme::TEXT_MUTED),
                );
            });
        });
    }
}
