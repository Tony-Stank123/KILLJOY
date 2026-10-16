use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FanProfileMode {
    Auto,
    Max,
    Manual,
    CustomCurve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    pub temp_c: f32,
    pub speed_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanControllerState {
    pub mode: FanProfileMode,
    pub manual_speed_pct: f32,
    pub is_max_boost: bool,
    pub curve_points: Vec<CurvePoint>,
    pub cpu_fan_rpm: u32,
    pub gpu_fan_rpm: u32,
    pub max_rpm: u32,
    pub active_preset_name: String,
    pub is_elevated: bool,
    pub last_hardware_status: String,
}

impl Default for FanControllerState {
    fn default() -> Self {
        Self {
            mode: FanProfileMode::Auto,
            manual_speed_pct: 50.0,
            is_max_boost: false,
            curve_points: vec![
                CurvePoint { temp_c: 35.0, speed_pct: 20.0 },
                CurvePoint { temp_c: 50.0, speed_pct: 35.0 },
                CurvePoint { temp_c: 65.0, speed_pct: 55.0 },
                CurvePoint { temp_c: 75.0, speed_pct: 80.0 },
                CurvePoint { temp_c: 85.0, speed_pct: 100.0 },
            ],
            cpu_fan_rpm: 2100,
            gpu_fan_rpm: 1950,
            max_rpm: 5400,
            active_preset_name: "Balanced Curve".to_string(),
            is_elevated: check_is_elevated(),
            last_hardware_status: "Hardware EC fan control initialized.".to_string(),
        }
    }
}

pub fn check_is_elevated() -> bool {
    let output = Command::new("net")
        .creation_flags(CREATE_NO_WINDOW)
        .arg("session")
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

static LAST_WMI_RESULT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub fn get_last_wmi_result() -> Option<String> {
    LAST_WMI_RESULT.lock().ok().and_then(|mut g| g.take())
}

pub fn restart_elevated() {
    let current_exe = std::env::current_exe().unwrap_or_default();
    let current_dir = std::env::current_dir().unwrap_or_default();
    let exe_str = current_exe.to_string_lossy().to_string();
    let dir_str = current_dir.to_string_lossy().to_string();

    let _ = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs",
                exe_str, dir_str
            ),
        ])
        .spawn();

    std::process::exit(0);
}

pub fn send_hp_wmi_fan_command(speed_pct: u32, is_max: bool) {
    std::thread::spawn(move || {
        let (max_flag, cpu_lvl, gpu_lvl) = if is_max {
            // SetMaxFan ON (1), and FanLevel 0x37 / 0x39 (Max RPM blowers)
            (1u8, 0x37u8, 0x39u8)
        } else if speed_pct == 0 {
            // SetMaxFan OFF (0), and FanLevel 0x00 / 0x00 (Auto ACPI thermal curve)
            (0u8, 0x00u8, 0x00u8)
        } else {
            // Manual speed between 0x15 (21 min) and 0x39 (57 max)
            let clamped = speed_pct.clamp(20, 100) as f32;
            let lvl = (21.0 + ((clamped - 20.0) / 80.0) * (57.0 - 21.0)).round() as u8;
            (0u8, lvl, lvl)
        };

        let script = format!(
            r#"
            $ErrorActionPreference = 'Stop'
            try {{
                $m = Get-CimInstance -Namespace 'root\wmi' -ClassName 'hpqBIntM' -ErrorAction Stop
                if (!$m) {{
                    Write-Error "hpqBIntM instance not found in root\wmi"
                    exit 2
                }}

                # 1. SetMaxFan toggle: Command 0x20008, CommandType 0x27
                $dMax = New-CimInstance -ClassName 'hpqBDataIn' -ClientOnly -Namespace 'root\wmi' -Property @{{
                    Command = [uint32]0x20008;
                    CommandType = [uint32]0x27;
                    hpqBData = [byte[]]@({max_flag}, 0, 0, 0);
                    Size = [uint32]4;
                    Sign = [byte[]]@(0x53, 0x45, 0x43, 0x55);
                }}
                Invoke-CimMethod -InputObject $m -MethodName 'hpqBIOSInt0' -Arguments @{{ InData = [CimInstance]$dMax }} -ErrorAction SilentlyContinue | Out-Null

                # 2. SetFanLevel manual/curve: Command 0x20008, CommandType 0x2E
                $dLvl = New-CimInstance -ClassName 'hpqBDataIn' -ClientOnly -Namespace 'root\wmi' -Property @{{
                    Command = [uint32]0x20008;
                    CommandType = [uint32]0x2E;
                    hpqBData = [byte[]]@({cpu_lvl}, {gpu_lvl}, 0, 0);
                    Size = [uint32]4;
                    Sign = [byte[]]@(0x53, 0x45, 0x43, 0x55);
                }}
                Invoke-CimMethod -InputObject $m -MethodName 'hpqBIOSInt0' -Arguments @{{ InData = [CimInstance]$dLvl }} -ErrorAction SilentlyContinue | Out-Null

                Write-Output "SUCCESS"
            }} catch {{
                Write-Error $_
                exit 1
            }}
            "#
        );

        let output = Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .output();

        let status = match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

                if out.status.success() {
                    if is_max {
                        "⚡ HP BIOS WMI [0x20008:0x27]: MAX FAN BLAST (100%) ACTIVE".to_string()
                    } else if speed_pct == 0 {
                        "⚡ HP BIOS WMI [0x20008:0x27]: DYNAMIC AUTO CURVE RESTORED".to_string()
                    } else {
                        format!("⚡ HP BIOS WMI [0x20008:0x2E]: MANUAL SPEED {}% ACTIVE", speed_pct)
                    }
                } else if stderr.contains("0x80041003") || stderr.contains("Access denied") {
                    "⚠️ HP WMI ACCESS DENIED: Run as Administrator to control hardware fans.".to_string()
                } else {
                    format!("⚠️ HP WMI Status: {}", if !stderr.is_empty() { stderr } else { stdout })
                }
            }
            Err(e) => format!("⚠️ Failed to launch WMI dispatcher: {}", e),
        };

        if let Ok(mut g) = LAST_WMI_RESULT.lock() {
            *g = Some(status);
        }
    });
}

impl FanControllerState {
    pub fn calculate_target_speed(&self, current_temp_c: f32) -> f32 {
        if self.is_max_boost || self.mode == FanProfileMode::Max {
            return 100.0;
        }

        match self.mode {
            FanProfileMode::Auto => {
                // Default ACPI curve formula
                if current_temp_c < 45.0 {
                    25.0
                } else if current_temp_c < 65.0 {
                    25.0 + ((current_temp_c - 45.0) / 20.0) * 30.0
                } else if current_temp_c < 80.0 {
                    55.0 + ((current_temp_c - 65.0) / 15.0) * 35.0
                } else {
                    100.0
                }
            }
            FanProfileMode::Manual => self.manual_speed_pct.clamp(20.0, 100.0),
            FanProfileMode::CustomCurve => {
                if self.curve_points.is_empty() {
                    return 50.0;
                }

                if current_temp_c <= self.curve_points.first().unwrap().temp_c {
                    return self.curve_points.first().unwrap().speed_pct;
                }

                if current_temp_c >= self.curve_points.last().unwrap().temp_c {
                    return self.curve_points.last().unwrap().speed_pct;
                }

                // Piecewise linear interpolation
                for i in 0..self.curve_points.len() - 1 {
                    let p1 = &self.curve_points[i];
                    let p2 = &self.curve_points[i + 1];

                    if current_temp_c >= p1.temp_c && current_temp_c <= p2.temp_c {
                        let factor = (current_temp_c - p1.temp_c) / (p2.temp_c - p1.temp_c);
                        return p1.speed_pct + factor * (p2.speed_pct - p1.speed_pct);
                    }
                }

                50.0
            }
            FanProfileMode::Max => 100.0,
        }
    }

    pub fn apply_hardware_fan(&mut self) {
        if self.is_max_boost || self.mode == FanProfileMode::Max {
            send_hp_wmi_fan_command(100, true);
            self.last_hardware_status = "HP WMI ACPI: MAX FAN (100%) dispatched...".to_string();
        } else if self.mode == FanProfileMode::Auto {
            send_hp_wmi_fan_command(0, false);
            self.last_hardware_status = "HP WMI ACPI: Dynamic Auto Curve dispatched...".to_string();
        } else if self.mode == FanProfileMode::Manual {
            send_hp_wmi_fan_command(self.manual_speed_pct as u32, false);
            self.last_hardware_status = format!("HP WMI ACPI: Manual {:.0}% dispatched...", self.manual_speed_pct);
        } else if self.mode == FanProfileMode::CustomCurve {
            let target = self.calculate_target_speed(55.0);
            send_hp_wmi_fan_command(target as u32, false);
            self.last_hardware_status = format!("HP WMI ACPI: Custom Curve {:.0}% dispatched...", target);
        }
    }

    pub fn update_telemetry(&mut self, cpu_temp: f32, gpu_temp: f32) {
        if let Some(msg) = get_last_wmi_result() {
            self.last_hardware_status = msg;
        }

        let max_temp = cpu_temp.max(gpu_temp);
        let target_pct = self.calculate_target_speed(max_temp);

        let target_cpu_rpm = (self.max_rpm as f32 * (target_pct / 100.0)) as u32;
        let target_gpu_rpm = (self.max_rpm as f32 * ((target_pct * 0.95) / 100.0)) as u32;

        // Smoothly approach target RPM
        self.cpu_fan_rpm = ((self.cpu_fan_rpm as f32 * 0.8) + (target_cpu_rpm as f32 * 0.2)) as u32;
        self.gpu_fan_rpm = ((self.gpu_fan_rpm as f32 * 0.8) + (target_gpu_rpm as f32 * 0.2)) as u32;
    }

    pub fn load_preset(&mut self, preset: &str) {
        self.active_preset_name = preset.to_string();
        match preset {
            "Silent Stealth" => {
                self.curve_points = vec![
                    CurvePoint { temp_c: 40.0, speed_pct: 15.0 },
                    CurvePoint { temp_c: 55.0, speed_pct: 25.0 },
                    CurvePoint { temp_c: 70.0, speed_pct: 40.0 },
                    CurvePoint { temp_c: 82.0, speed_pct: 65.0 },
                    CurvePoint { temp_c: 92.0, speed_pct: 95.0 },
                ];
            }
            "Aggressive Cool" => {
                self.curve_points = vec![
                    CurvePoint { temp_c: 35.0, speed_pct: 35.0 },
                    CurvePoint { temp_c: 50.0, speed_pct: 55.0 },
                    CurvePoint { temp_c: 65.0, speed_pct: 75.0 },
                    CurvePoint { temp_c: 75.0, speed_pct: 90.0 },
                    CurvePoint { temp_c: 85.0, speed_pct: 100.0 },
                ];
            }
            "Linear 1:1" => {
                self.curve_points = vec![
                    CurvePoint { temp_c: 30.0, speed_pct: 20.0 },
                    CurvePoint { temp_c: 45.0, speed_pct: 40.0 },
                    CurvePoint { temp_c: 60.0, speed_pct: 60.0 },
                    CurvePoint { temp_c: 75.0, speed_pct: 80.0 },
                    CurvePoint { temp_c: 90.0, speed_pct: 100.0 },
                ];
            }
            _ => {
                // Balanced Default
                self.curve_points = vec![
                    CurvePoint { temp_c: 35.0, speed_pct: 20.0 },
                    CurvePoint { temp_c: 50.0, speed_pct: 35.0 },
                    CurvePoint { temp_c: 65.0, speed_pct: 55.0 },
                    CurvePoint { temp_c: 75.0, speed_pct: 80.0 },
                    CurvePoint { temp_c: 85.0, speed_pct: 100.0 },
                ];
            }
        }
    }
}
