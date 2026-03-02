# adbDebloat

Android debloater via ADB. Safe, reversible package removal with session-based JSON backups and full restore capability.

> **WARNING**
> This tool is under active development and has been tested on Xiaomi Redmi 5 Plus (vince) only.
> Test with Safe Disable mode before using Permanent Remove. The author is not responsible for
> bricked devices, boot loops, or any damage resulting from misuse.

## Overview

Operates entirely over ADB without root. Two removal modes:

- **Safe Disable** - `pm uninstall -k --user 0` - package is disabled for user 0, data preserved, fully reversible via `cmd package install-existing`
- **Permanent Remove** - `pm uninstall` - irreversible, blocked for DANGER_ZONE packages (brick protection)

Package categories are filtered automatically based on detected device brand (Xiaomi, Samsung, OPPO, etc.).
Session JSON backups track every safe-disabled package for targeted restore.

## Requirements

- Windows 10+ / PowerShell 5.1+
- Android device with USB debugging enabled
- ADB (platform-tools)

## Installation

```powershell
git clone https://github.com/bugragungoz/adbDebloat.git
cd adbDebloat
.\AdbDebloat.ps1
```

### ADB Setup

The script searches for `adb.exe` in the following order:

1. `platform-tools\adb.exe` (next to script - portable bundle)
2. `adb.exe` (script root)
3. `%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe`
4. `C:\Program Files (x86)\Android\android-sdk\platform-tools\adb.exe`
5. System `PATH`

Place the `platform-tools\` folder from [Android SDK Platform Tools](https://developer.android.com/tools/releases/platform-tools)
next to the script for fully portable operation.

### Execution Policy

If PowerShell blocks execution:

```powershell
# Option 1 - Unblock file (recommended)
Unblock-File -Path .\AdbDebloat.ps1
.\AdbDebloat.ps1

# Option 2 - Bypass for current session
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\AdbDebloat.ps1

# Option 3 - One-liner
powershell -ExecutionPolicy Bypass -File .\AdbDebloat.ps1
```

## Usage

```
[1] Device Info
[2] Browse & Debloat  - select category, pick packages, choose removal mode
[3] Restore Packages  - restore from a previous backup session
[4] List Disabled     - live query of all packages disabled on device
[5] Refresh Device    - reconnect / redetect ADB device
[6] Exit
```

**Package selection input formats:**

| Format      | Example   | Meaning              |
|-------------|-----------|----------------------|
| all         | `all`     | Select all packages  |
| Single      | `3`       | Package #3           |
| List        | `1,3,5`   | Packages 1, 3, 5     |
| Range       | `1-10`    | Packages 1 through 10|
| Mixed       | `1-3,7,9` | Ranges and singles   |

## Package Categories

| Category                   | Brands              | Permanent Remove |
|----------------------------|---------------------|-----------------|
| Google Bloatware           | All                 | Yes             |
| Xiaomi MIUI Safe Zone      | Xiaomi / Redmi / POCO | Yes           |
| Xiaomi MIUI Caution Zone   | Xiaomi / Redmi / POCO | Yes           |
| Samsung One UI Bloatware   | Samsung             | Yes             |
| OPPO / Realme / OnePlus    | OPPO / Realme / OnePlus | Yes         |
| DANGER ZONE                | All                 | **Blocked**     |

Categories are shown based on `ro.product.brand` read from the connected device.

## Backup & Restore

Each Safe Disable session writes a JSON file to `AdbDebloat_Backups\`:

```json
{
  "SessionID": "20260301_143022",
  "Device": "vince",
  "Category": "MIUI_SAFE",
  "Mode": "SafeDisable",
  "Packages": [
    { "Package": "com.miui.msa.global", "FriendlyName": "MIUI Ad Service (MSA)" }
  ]
}
```

Restore via menu option 3 - select session, pick packages, confirm.

## Tested Devices

| Device               | Codename | Brand  | Status   |
|----------------------|----------|--------|----------|
| Xiaomi Redmi 5 Plus  | vince    | Xiaomi | Tested   |

Pull requests with tested device codenames and package lists are welcome.

## Adding a Device

In `AdbDebloat.ps1`, add an entry to `$script:DevicePackages`:

```powershell
"yourcodename" = @{
    MIUI_SAFE = [ordered]@{
        "com.example.package" = "Package Friendly Name"
    }
}
```

Device codename is read from `ro.product.device` at connection time.

## Project Structure

```
adbDebloat/
├── AdbDebloat.ps1          # Main executable
├── platform-tools/         # ADB binaries (optional, portable)
├── AdbDebloat_Logs/        # Session logs (auto-created)
└── AdbDebloat_Backups/     # JSON backup snapshots (auto-created)
```

## License

MIT License

## Development

**Author:** Bugra
**AI Architecture:** Claude Sonnet (Anthropic)
**ADB Platform:** Google Android SDK Platform Tools
