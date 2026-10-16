# ⚡ KILJOY // HP VICTUS CYBER-DECK
> **A zero-bloat, lightning-fast native Rust gaming laptop hub & hardware monitor for HP Victus & Windows Gaming Laptops.**

Replaces sluggish OEM software like HP OMEN Gaming Hub & ASUS Armoury Crate with a pure native, GPU-accelerated cyberdeck built in Rust. Starts in **< 50 milliseconds**, consumes **< 35 MB RAM** (vs 600MB+ for OMEN Hub), and features an authentic **Cyberpunk Hacker Terminal** aesthetic.

---

## 📸 Key Features

### 1. ⚡ Operational Performance Modes
Directly controls hardware power limits, processor turbo states, and Windows power schemes via **native Win32 PowrProf APIs** (0ms execution, zero PowerShell windows):
* 🌿 **ECO [STEALTH]**: Limits CPU boost and power envelope to reduce heat & noise to absolute zero. Maximum battery endurance.
* ❄ **QUIET [SILENT]**: Suppresses acoustic fan spikes while sustaining smooth office/browsing performance.
* ◈ **BALANCED [DEFAULT]**: Factory dynamic scaling profile for all daily mixed workloads.
* ⚡ **PERFORMANCE [OVERCLOCK]**: Unlocks full **75W TGP** for the RTX 4050 and unrestricted turbo clocks on the Intel Core i5-13420H.
* **Full-Card Clickable**: Click anywhere on the mode card to immediately activate.

### 2. 🔥 Dedicated "FAN MAX SPEED" Button & Hardware EC Control
* **Dedicated Fan Max Speed Button**: Instantly locks both CPU & GPU blowers to 100% full blast (~5,400 RPM) for emergency cooling during intense gaming sessions. Located prominently in both the **Sidebar** and the **Performance Dashboard**.
* **Direct HP BIOS WMI / EC Link**: Uses HP's proprietary `root\wmi:hpqBIntM` BIOS mailbox (`hpqBIOSInt0` with command `0x20008`) to set `SetMaxFan` (command `0x27`) and `SetFanLevel` (command `0x2E`).
* **Instant Toggle**: Single click toggles between `AUTO` (dynamic thermal curve) and `🔥 MAX FAN (100% TURBO)`.
* **Manual Speed Slider**: Drag slider (20% to 100%) to lock fans to any desired manual RPM.
* **Interactive Fan Curve Editor**: Full `egui_plot` visualization of Die Temperature (°C) vs Fan Speed (%) with 5 customizable control nodes and 1-click `⚡ APPLY CURVE TO FIRMWARE`.
* **Presets**: *Balanced Curve*, *Silent Stealth*, *Aggressive Cool*, and *Linear 1:1*.
* **Dual Fan RPM Tachometers**: Live RPM readouts for both CPU and GPU blowers (~5,400 RPM max).

### 3. 🛑 Turn Off & Clean Handover to OMEN Gaming Hub
* **🛑 Turn Off KilJoy [ Restore OMEN Hub ]**: A dedicated button in the sidebar footer that cleanly stands down KilJoy:
  1. Releases all hardware fan overrides back to OEM BIOS Auto.
  2. Reverts Windows power profile to Balanced.
  3. Automatically restores and restarts all HP OEM background services (`HPOmenCap`, etc.).
  4. Exits KilJoy so the official HP OMEN Gaming Hub can run without conflicts.
* **⏸️ Stand Down (Passive Monitor Mode)**: Keep KilJoy open as a sleek cyberpunk hardware HUD on your screen while letting the official OMEN Gaming Hub control fans and lighting.

### 4. 📊 Neural Hardware Telemetry
* **Intel Core i5-13420H**: Real-time load percentage, die temperature, and per-core breakdown across all 8 physical / 12 logical cores with individual live MHz frequencies.
* **NVIDIA GeForce RTX 4050 Laptop GPU**: Direct NVML hardware link for real-time die temperature, core utilization, VRAM capacity (used / total MB), and board power draw (TGP in Watts).
* **RAM & Power**: DDR5 memory utilization, AC Mains status, and battery percentage via native Win32 power calls.
* **60-Second Rolling Graph**: High-frequency historical plot of CPU Temp, GPU Temp, and CPU Usage.

### 5. 🛡 Pure Win32 Bloatware Neutralizer ("Kill Joy")
* Pure native Win32 Service Manager integration (`advapi32.dll`) — **zero console/PowerShell tabs**:
  * `HpTouchpointAnalyticsService` (HP Insights Analytics)
  * `HPOmenCap` (HP Omen HSA Service)
  * `HPAppHelperCap` (HP App Helper HSA)
  * `HP Comm Recover` (HP Comm Recovery)
  * `HPNetworkCap` (HP Network HSA)
  * `HPDiagsCap` (HP Diagnostics HSA)
* **1-Click "Purge All Bloat"**: Neutralizes background telemetry and promotional daemons to reclaim RAM and CPU cycles.
* **1-Click "Restore OEM Services"**: Easily reactivate them at any time.

### 6. 💻 Cyber Terminal Shell
An integrated cyberpunk interactive terminal at the bottom of the deck.
Supports command execution, real-time ACPI/kernel logs, and history navigation with `↑` / `↓` arrow keys:
```text
KILJOY:\> help
KILJOY:\> mode perf          # Switch power mode to Performance
KILJOY:\> fan max            # Spin dual fans to 100% max blast
KILJOY:\> fan auto           # Restore automatic curve
KILJOY:\> fan 70             # Set fan speed manually to 70%
KILJOY:\> purge              # Kill HP bloat services (native Win32)
KILJOY:\> vitals             # Instant hardware telemetry readout
KILJOY:\> standdown          # Toggle passive monitor mode
KILJOY:\> clear              # Clear terminal screen
KILJOY:\> exit               # Close KilJoy
```

---

## 🚀 Quick Start (Administrator Elevation)

> [!IMPORTANT]
> **Why Run as Administrator?**
> Windows security policies restrict write access to motherboard BIOS and Embedded Controller (EC) WMI namespaces (`root\wmi:hpqBIntM`) exclusively to **Administrator**. When run without elevation, Windows blocks the BIOS fan commands (`Access Denied / 0x80041003`).

### Launch Options:
1. **Recommended**: Double-click `launch.bat`.
   * Automatically prompts for standard Windows UAC Administrator approval and launches `kiljoy.exe` with full hardware fan control unlocked.
2. **From Terminal / File Explorer**:
   * Right-click `target\release\kiljoy.exe` and select **"Run as Administrator"**.
   * Or if you open KilJoy normally, simply click the glowing `[ ⚠️ EC: LOCKED [ELEVATE] ]` button in the sidebar or performance tab to elevate with 1 click!

### Build from Source:
```bash
cargo build --release
```
The compiled standalone executable is at `target/release/kiljoy.exe` (~11.8 MB).
