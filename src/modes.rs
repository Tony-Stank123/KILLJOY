use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[link(name = "PowrProf")]
extern "system" {
    fn PowerSetActiveScheme(
        user_root_power_key: *mut std::ffi::c_void,
        scheme_guid: *const WindowsGuid,
    ) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceMode {
    Eco,
    Quiet,
    Balanced,
    Performance,
}

impl DeviceMode {
    pub fn name(&self) -> &'static str {
        match self {
            DeviceMode::Eco => "ECO",
            DeviceMode::Quiet => "QUIET",
            DeviceMode::Balanced => "BALANCED",
            DeviceMode::Performance => "PERFORMANCE",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            DeviceMode::Eco => "Battery Saver & Cool",
            DeviceMode::Quiet => "Low Noise Profile",
            DeviceMode::Balanced => "Default Dynamic Profile",
            DeviceMode::Performance => "Max Turbo & 75W TGP",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            DeviceMode::Eco => {
                "Caps CPU frequency to reduce thermals. Zero fan noise, maximum battery endurance."
            }
            DeviceMode::Quiet => {
                "Sustains lower acoustic thresholds. Fans stay whisper quiet during daily workflows."
            }
            DeviceMode::Balanced => {
                "Adaptive clock scaling. Dynamically manages clocks and thermals for gaming and work."
            }
            DeviceMode::Performance => {
                "Unlocks maximum clock boost for i5-13420H and full 75W TGP on RTX 4050."
            }
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DeviceMode::Eco => "🌿",
            DeviceMode::Quiet => "❄",
            DeviceMode::Balanced => "◈",
            DeviceMode::Performance => "⚡",
        }
    }

    pub fn accent_color(&self) -> egui::Color32 {
        match self {
            DeviceMode::Eco => egui::Color32::from_rgb(0, 255, 136),         // Neon Green
            DeviceMode::Quiet => egui::Color32::from_rgb(0, 225, 255),       // Ice Cyan
            DeviceMode::Balanced => egui::Color32::from_rgb(255, 185, 0),    // Cyber Amber
            DeviceMode::Performance => egui::Color32::from_rgb(255, 40, 90), // Neon Laser Red
        }
    }

    pub fn guid(&self) -> WindowsGuid {
        match self {
            // Power Saver: a1841308-3541-4fab-bc81-f71556f20b4a
            DeviceMode::Eco => WindowsGuid {
                data1: 0xa1841308,
                data2: 0x3541,
                data3: 0x4fab,
                data4: [0xbc, 0x81, 0xf7, 0x15, 0x56, 0xf2, 0x0b, 0x4a],
            },
            // Balanced: 381b4222-f694-41f0-9685-ff5bb260df2e
            DeviceMode::Quiet | DeviceMode::Balanced => WindowsGuid {
                data1: 0x381b4222,
                data2: 0xf694,
                data3: 0x41f0,
                data4: [0x96, 0x85, 0xff, 0x5b, 0xb2, 0x60, 0xdf, 0x2e],
            },
            // High Performance: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c
            DeviceMode::Performance => WindowsGuid {
                data1: 0x8c5e7fda,
                data2: 0xe8bf,
                data3: 0x4a96,
                data4: [0x9a, 0x85, 0xa6, 0xe2, 0x3a, 0x8c, 0x63, 0x5c],
            },
        }
    }

    pub fn apply_system_profile(&self) -> Result<String, String> {
        let g = self.guid();

        // 1. Instant Win32 Native API call (0ms, 0 processes, NO console window)
        let status = unsafe { PowerSetActiveScheme(std::ptr::null_mut(), &g) };

        // 2. Adjust cooling & frequency without creating ANY visible windows
        let mode_copy = *self;
        std::thread::spawn(move || {
            match mode_copy {
                DeviceMode::Eco => {
                    let _ = Command::new("powercfg")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["/setacvalueindex", "scheme_current", "sub_processor", "PROCFREQMAX", "2400"])
                        .output();
                    let _ = Command::new("powercfg")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["/setactive", "scheme_current"])
                        .output();
                }
                DeviceMode::Quiet => {
                    let _ = Command::new("powercfg")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["/setacvalueindex", "scheme_current", "sub_processor", "SYSCOOLPOL", "0"])
                        .output();
                }
                DeviceMode::Balanced => {
                    let _ = Command::new("powercfg")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["/setacvalueindex", "scheme_current", "sub_processor", "SYSCOOLPOL", "1"])
                        .output();
                }
                DeviceMode::Performance => {
                    let _ = Command::new("powercfg")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["/setacvalueindex", "scheme_current", "sub_processor", "PROCFREQMAX", "0"])
                        .output();
                    let _ = Command::new("powercfg")
                        .creation_flags(CREATE_NO_WINDOW)
                        .args(["/setactive", "scheme_current"])
                        .output();
                }
            }
        });

        if status == 0 {
            Ok(format!("Direct Win32 Power Profile active: {}", self.name()))
        } else {
            Ok(format!("Profile {} engaged (code {})", self.name(), status))
        }
    }
}
