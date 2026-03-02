#Requires -Version 5.1

<#
.SYNOPSIS
    AdbDebloat - Android Package Removal Tool via ADB

.DESCRIPTION
    Safe and restorable Android debloater operating over ADB.
    Supports safe disable (pm uninstall -k --user 0, reversible) and
    permanent removal (pm uninstall, irreversible, blocked for DANGER_ZONE).
    Session-based JSON backups enable selective or full restore at any time.

.NOTES
    Name:    AdbDebloat
    Author:  Bugra
    Version: 1.0.0
    Created: 2026

    Safe Disable : adb shell pm uninstall -k --user 0 <package>
    Restore      : adb shell cmd package install-existing <package>
    Permanent    : adb shell pm uninstall <package>  [DANGER_ZONE blocked]

    ADB Location : Script directory (adb.exe) takes priority over PATH.
#>

$ErrorActionPreference = "Stop"

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

$script:Config = @{
    LogDirectory    = "$PSScriptRoot\AdbDebloat_Logs"
    BackupDirectory = "$PSScriptRoot\AdbDebloat_Backups"
    LogFile         = ""
    SessionID       = (Get-Date -Format "yyyyMMdd_HHmmss")
    AdbPath         = ""
    DeviceSerial    = $null
    DeviceCodename  = $null
    DeviceModel     = $null
    DeviceBrand     = $null
}

# ---------------------------------------------------------------------------
# Package Database
# ---------------------------------------------------------------------------
# CanPermanentDelete = $false  ->  brick protection, permanent remove BLOCKED.
# Brands = @()                 ->  empty means shown for ALL brands.
# Brands = @("xiaomi","redmi") ->  shown only when brand matches.
# Device-specific extras are merged at runtime via $script:DevicePackages.

$script:PackageDatabase = [ordered]@{

    GOOGLE_BLOAT = @{
        Label              = "Google Bloatware"
        Color              = "Yellow"
        CanPermanentDelete = $true
        Brands             = @()
        Packages           = [ordered]@{
            "com.google.android.apps.tachyon"                = "Google Meet (Duo)"
            "com.google.android.videos"                      = "Google Play Movies & TV"
            "com.google.android.apps.wellbeing"              = "Digital Wellbeing"
            "com.google.android.feedback"                    = "Google Feedback"
            "com.google.android.partnersetup"                = "Google Partner Setup"
            "com.google.android.onetimeinitializer"          = "Google One Time Initializer"
            "com.google.android.printservice.recommendation" = "Print Service Recommendation"
            "com.google.android.apps.docs"                   = "Google Drive"
            "com.google.android.apps.photos"                 = "Google Photos"
            "com.google.android.apps.maps"                   = "Google Maps"
            "com.google.android.youtube"                     = "YouTube"
            "com.google.android.apps.googleassistant"        = "Google Assistant"
            "com.google.android.marvin.talkback"             = "TalkBack"
            "com.google.android.storagemanager"              = "Google Storage Manager"
            "com.google.android.syncadapters.contacts"       = "Google Contacts Sync"
            "com.google.android.syncadapters.calendar"       = "Google Calendar Sync"
        }
    }

    MIUI_SAFE = @{
        Label              = "Xiaomi MIUI Safe Zone"
        Color              = "Green"
        CanPermanentDelete = $true
        Brands             = @("xiaomi", "redmi", "poco")
        Packages           = [ordered]@{
            "com.miui.msa.global"               = "MIUI Ad Service (MSA)"
            "com.miui.hybrid"                   = "MIUI MSA Hybrid"
            "com.miui.hybrid.accessory"         = "MIUI MSA Hybrid Accessory"
            "com.miui.analytics"                = "MIUI Analytics"
            "com.miui.daemon"                   = "MIUI Push Daemon"
            "com.miui.systemAdSolution"         = "MIUI System Ad Solution"
            "com.miui.bugreport"                = "MIUI Bug Report"
            "com.miui.getapps"                  = "GetApps (Mi Market)"
            "com.miui.yellowpage"               = "Mi Yellow Pages"
            "com.miui.videoplayer"              = "Mi Video Player"
            "com.miui.player"                   = "Mi Music"
            "com.mi.globalbrowser"              = "Mi Browser"
            "com.miui.weather2"                 = "Mi Weather"
            "com.miui.cleanmaster"              = "Phone Cleaner (Clean Master)"
            "com.miui.personalassistant"        = "MIUI Personal Assistant (Mint)"
            "com.miui.compass"                  = "Mi Compass"
            "com.miui.cloudservice"             = "MIUI Cloud Service"
            "com.miui.cloudbackup"              = "MIUI Cloud Backup"
            "com.xiaomi.gamecenter"             = "Mi Game Center"
            "com.xiaomi.gamecenter.sdk.service" = "Mi Game Center SDK"
            "com.miui.mishare.connectivity"     = "Mi Share"
            "com.miui.voiceassist"              = "Mi Voice Assistant"
            "com.miui.notes"                    = "Mi Notes"
            "com.miui.fm"                       = "Mi FM Radio"
            "com.miui.screenrecorder"           = "Mi Screen Recorder"
            "com.miui.scan"                     = "Mi Scanner"
            "com.miui.global.kitchenstory"      = "Kitchen Story"
            "com.miui.gallery"                  = "Mi Gallery"
            "com.miui.contentcatcher"           = "MIUI Content Catcher"
            "com.miui.phrase"                   = "MIUI Smart Input Phrases"
        }
    }

    MIUI_CAUTION = @{
        Label              = "Xiaomi MIUI Caution Zone"
        Color              = "DarkYellow"
        CanPermanentDelete = $true
        Brands             = @("xiaomi", "redmi", "poco")
        Packages           = [ordered]@{
            "com.miui.powerkeeper"     = "Mi Power Keeper"
            "com.miui.guard.monitor"   = "MIUI Guard Monitor"
            "com.miui.systemui.plugin" = "SystemUI Plugin"
            "com.android.browser"      = "AOSP Browser"
            "com.miui.securitycenter"  = "Mi Security Center"
        }
    }

    SAMSUNG_ONE_UI = @{
        Label              = "Samsung One UI Bloatware"
        Color              = "Cyan"
        CanPermanentDelete = $true
        Brands             = @("samsung")
        Packages           = [ordered]@{
            "com.samsung.android.bixby.agent"          = "Bixby Voice Assistant"
            "com.samsung.android.bixby.wakeup"         = "Bixby Wakeup"
            "com.samsung.android.app.spage"            = "Samsung Free (Bixby Home)"
            "com.samsung.android.app.tips"             = "Samsung Tips"
            "com.samsung.android.weather.nativeapp"    = "Samsung Weather"
            "com.samsung.android.game.gamehome"        = "Game Launcher"
            "com.samsung.android.game.gametools"       = "Game Booster"
            "com.sec.android.app.samsungapps"          = "Galaxy Store"
            "com.samsung.android.voc"                  = "Samsung Members (Diagnostics)"
            "com.samsung.android.app.soundassistant"   = "Sound Assistant"
            "com.samsung.android.smartswitchassistant" = "Smart Switch Assistant"
            "com.sec.android.app.chromecustomizations" = "Chrome Customizations"
            "com.samsung.android.kidsinstaller"        = "Kids Mode Installer"
            "com.samsung.android.arzone"               = "AR Zone"
            "com.samsung.android.aremoji"              = "AR Emoji"
            "com.sec.android.app.sbrowser"             = "Samsung Internet Browser"
            "com.samsung.android.email.provider"       = "Samsung Email"
            "com.samsung.android.calendar"             = "Samsung Calendar"
            "com.samsung.android.app.watchmanagerplugin" = "Galaxy Watch Manager Plugin"
            "com.samsung.android.dialer"               = "Samsung Dialer"
            "com.samsung.android.messaging"            = "Samsung Messages"
        }
    }

    OPPO_COLOROS = @{
        Label              = "OPPO / Realme / OnePlus Bloatware"
        Color              = "Magenta"
        CanPermanentDelete = $true
        Brands             = @("oppo", "realme", "oneplus")
        Packages           = [ordered]@{
            "com.heytap.market"             = "App Market (OPPO)"
            "com.heytap.themestore"         = "Theme Store"
            "com.nearme.gamecenter"         = "Game Center (OPPO)"
            "com.coloros.weather2"          = "ColorOS Weather"
            "com.coloros.filemanager"       = "ColorOS File Manager"
            "com.coloros.findmyphone"       = "Find My Phone (OPPO)"
            "com.oppo.market"               = "OPPO Market"
            "com.realme.ota"                = "Realme OTA Updates"
            "com.oplus.games"               = "Plus Games (OnePlus)"
            "com.oneplus.brickmode"         = "OnePlus Brick Mode"
            "com.oneplus.logkit"            = "OnePlus LogKit"
            "com.heytap.pictorial"          = "HeyTap Pictorial"
            "com.oppo.oauthsdk"             = "OPPO Auth SDK"
            "com.nearme.statistics.rom"     = "ROM Statistics (OPPO)"
        }
    }

    DANGER_ZONE = @{
        Label              = "!!! DANGER ZONE - Safe Disable ONLY - No Permanent Remove !!!"
        Color              = "Red"
        CanPermanentDelete = $false
        Brands             = @()
        Packages           = [ordered]@{
            "com.android.systemui"         = "System UI [BRICK RISK]"
            "com.miui.home"                = "MIUI Launcher [BRICK RISK]"
            "com.android.settings"         = "Android Settings [BRICK RISK]"
            "com.android.phone"            = "Phone App [BRICK RISK]"
            "com.android.packageinstaller" = "Package Installer [BRICK RISK]"
            "com.miui.aod"                 = "Always On Display / MIUI [BRICK RISK]"
            "com.samsung.android.lool"     = "Samsung Device Security [BRICK RISK]"
            "com.sec.android.app.launcher" = "Samsung Launcher [BRICK RISK]"
        }
    }
}

# Device-specific package additions, merged at runtime on top of generic database.
$script:DevicePackages = @{

    # Xiaomi Redmi 5 Plus (Snapdragon 625) - vince - tested
    "vince" = @{
        MIUI_SAFE = [ordered]@{
            "com.miui.rom.packager"  = "ROM Packager"
            "com.miui.virtualsim"    = "Virtual SIM"
            "com.xiaomi.account"     = "Xiaomi Account"
            "com.miui.calculator"    = "Mi Calculator"
        }
    }

    # Xiaomi Redmi Note 8 / Note 8T (Snapdragon 665) - ginkgo / willow
    "ginkgo" = @{
        MIUI_SAFE = [ordered]@{
            "com.miui.mms"           = "Mi Messages"
            "com.miui.mishare"       = "Mi Share"
            "com.miui.klo.bugreport" = "KLO Bug Report"
        }
    }

    # Xiaomi Redmi Note 7 (Snapdragon 660) - lavender
    "lavender" = @{
        MIUI_SAFE = [ordered]@{
            "com.miui.mms"          = "Mi Messages"
            "com.miui.virtualsim"   = "Virtual SIM"
        }
    }

    # Xiaomi Redmi Note 9 Pro / 9S (Snapdragon 720G) - miatoll
    "miatoll" = @{
        MIUI_SAFE = [ordered]@{
            "com.miui.mms"          = "Mi Messages"
            "com.miui.mishare"      = "Mi Share"
        }
    }

    # Xiaomi Redmi 9 (Helio G80) - lancelot
    "lancelot" = @{
        MIUI_SAFE = [ordered]@{
            "com.miui.mms"          = "Mi Messages"
            "com.miui.mishare"      = "Mi Share"
        }
    }

    # POCO X3 NFC / X3 Pro - surya / vayu
    "surya" = @{
        MIUI_SAFE = [ordered]@{
            "com.miui.mms"          = "Mi Messages"
            "com.miui.mishare"      = "Mi Share"
        }
    }
    "vayu" = @{
        MIUI_SAFE = [ordered]@{
            "com.miui.mms"          = "Mi Messages"
            "com.miui.mishare"      = "Mi Share"
        }
    }
}

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

function Write-Log {
    param(
        [string]$Message,
        [ValidateSet('INFO', 'WARNING', 'ERROR', 'SUCCESS', 'DEBUG')]
        [string]$Level = 'INFO'
    )
    if ($script:Config.LogFile) {
        $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        Add-Content -Path $script:Config.LogFile -Value "[$ts] [$Level] $Message" -ErrorAction SilentlyContinue
    }
}

# ---------------------------------------------------------------------------
# Banner
# ---------------------------------------------------------------------------

function Show-Banner {
    Write-Host ""
    Write-Host "================================================================================" -ForegroundColor Cyan
    Write-Host "                                                                                " -ForegroundColor Cyan
    Write-Host "                                                                                " -ForegroundColor Cyan
    Write-Host "          ######  ########   #######  ##     ## ########                        " -ForegroundColor Cyan
    Write-Host "         ##    ## ##     ## ##     ##  ##   ##       ##                         " -ForegroundColor Cyan
    Write-Host "         ##       ##     ## ##     ##   ## ##       ##                          " -ForegroundColor Cyan
    Write-Host "         ##       ########  ##     ##    ###       ##                           " -ForegroundColor Cyan
    Write-Host "         ##       ##   ##   ##     ##   ## ##     ##                            " -ForegroundColor Cyan
    Write-Host "         ##    ## ##    ##  ##     ##  ##   ##   ##                             " -ForegroundColor Cyan
    Write-Host "          ######  ##     ##  #######  ##     ## ########                        " -ForegroundColor Cyan
    Write-Host "                                                                                " -ForegroundColor Cyan
    Write-Host "                 ADB Android Debloater v1.0.0                                   " -ForegroundColor Cyan
    Write-Host "                                                                                " -ForegroundColor Cyan
    Write-Host "================================================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  Author: Bugra  |  Session: $($script:Config.SessionID)" -ForegroundColor Gray
    Write-Host "  Safe Disable : pm uninstall -k --user 0  (reversible, recommended)" -ForegroundColor Green
    Write-Host "  Permanent    : pm uninstall  (irreversible - DANGER_ZONE always blocked)" -ForegroundColor Red
    Write-Host ""
}

# ---------------------------------------------------------------------------
# Environment
# ---------------------------------------------------------------------------

function Initialize-Environment {
    try {
        foreach ($dir in @($script:Config.LogDirectory, $script:Config.BackupDirectory)) {
            if (-not (Test-Path $dir)) {
                New-Item -ItemType Directory -Path $dir -Force | Out-Null
            }
        }
        $script:Config.LogFile = Join-Path $script:Config.LogDirectory "AdbDebloat_$($script:Config.SessionID).log"
        Write-Log "Session started" -Level INFO
        Write-Host "  [OK] Logs    : $($script:Config.LogDirectory)" -ForegroundColor Green
        Write-Host "  [OK] Backups : $($script:Config.BackupDirectory)" -ForegroundColor Green
        return $true
    }
    catch {
        Write-Host "  [ERROR] Environment init failed: $_" -ForegroundColor Red
        return $false
    }
}

# ---------------------------------------------------------------------------
# ADB
# ---------------------------------------------------------------------------

function Start-AdbServer {
    # $ErrorActionPreference = "Stop" (global) treats 2>&1 stderr ErrorRecord
    # objects as terminating errors. ADB daemon writes to stderr on startup,
    # so we must scope-isolate these calls with SilentlyContinue.
    $savedEA = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    try {
        # start-server is idempotent: no-op if daemon is already running.
        # kill-server is intentionally omitted - killing a healthy daemon and
        # racing to restart it causes connection-refused errors on Windows.
        & $script:Config.AdbPath start-server 2>&1 | Out-Null
        Start-Sleep -Milliseconds 800

        # Grab version string. Daemon startup info lines begin with '*' on stderr;
        # filter them to get the clean "Android Debug Bridge version X.Y.Z" line.
        $verLines = & $script:Config.AdbPath version 2>&1
        $ver = $verLines |
               Where-Object { $_.ToString() -match 'Android Debug Bridge' } |
               Select-Object -First 1

        if ($ver) {
            Write-Host "  [OK] $($ver.ToString().Trim())" -ForegroundColor Green
            Write-Log "ADB: $($ver.ToString().Trim())" -Level INFO
        }
        else {
            Write-Host "  [OK] ADB binary found and ready." -ForegroundColor Green
            Write-Log "ADB: binary found, version string unavailable" -Level INFO
        }
        return $true
    }
    finally {
        $ErrorActionPreference = $savedEA
    }
}

function Find-AdbExecutable {
    # Search order mirrors adbData project convention (portable bundle first, then SDK paths, then PATH)
    $searchPaths = @(
        (Join-Path $PSScriptRoot "platform-tools\adb.exe"),  # bundled subdirectory (preferred portable)
        (Join-Path $PSScriptRoot "adb.exe"),                 # script root flat placement
        "$env:LOCALAPPDATA\Android\Sdk\platform-tools\adb.exe",
        "C:\Program Files (x86)\Android\android-sdk\platform-tools\adb.exe",
        "C:\Android\platform-tools\adb.exe",
        "C:\adb\adb.exe"
    )

    foreach ($path in $searchPaths) {
        if (Test-Path $path) {
            $script:Config.AdbPath = $path
            Write-Host "  [OK] ADB found : $path" -ForegroundColor Green
            Write-Log "ADB: $path" -Level INFO
            return (Start-AdbServer)
        }
    }

    # Fallback: system PATH
    $pathAdb = Get-Command "adb" -ErrorAction SilentlyContinue
    if ($pathAdb) {
        $script:Config.AdbPath = $pathAdb.Source
        Write-Host "  [OK] ADB (PATH) : $($pathAdb.Source)" -ForegroundColor Green
        Write-Log "ADB: $($pathAdb.Source) (PATH)" -Level INFO
        return (Start-AdbServer)
    }

    Write-Host "  [ERROR] adb.exe not found." -ForegroundColor Red
    Write-Host "          Options:" -ForegroundColor Yellow
    Write-Host "            1) Place adb.exe next to this script" -ForegroundColor Yellow
    Write-Host "            2) Place platform-tools\ folder next to this script" -ForegroundColor Yellow
    Write-Host "            3) Add ADB to system PATH" -ForegroundColor Yellow
    Write-Host "            4) Install Android SDK (SDK Manager)" -ForegroundColor Yellow
    Write-Log "ADB not found" -Level ERROR
    return $false
}

function Test-DeviceConnection {
    try {
        # Use -l (long format) to get model info at no extra cost.
        # Normalize to string and split on CRLF or LF (adbData pattern)
        # to handle mixed output types PowerShell returns for native commands.
        $savedEA = $ErrorActionPreference
        $ErrorActionPreference = "SilentlyContinue"
        $rawOutput = & $script:Config.AdbPath devices -l 2>&1
        $ErrorActionPreference = $savedEA

        if ($LASTEXITCODE -ne 0) {
            Write-Host "  [ERROR] ADB devices command failed (exit $LASTEXITCODE)." -ForegroundColor Red
            return $false
        }

        $rawStr = if ($rawOutput -is [System.Array]) { $rawOutput -join "`n" } else { [string]$rawOutput }
        $lines  = @(($rawStr -split '\r?\n') | Select-Object -Skip 1 | Where-Object { $_ -notmatch '^\s*$' })

        if ($lines.Count -eq 0) {
            Write-Host "  [ERROR] No device detected. Connect device and enable USB Debugging." -ForegroundColor Red
            Write-Log "No device connected" -Level ERROR
            return $false
        }

        $allDevices = @()
        foreach ($line in $lines) {
            $str   = $line.ToString().Trim()
            $parts = $str -split '\s+'
            if ($parts.Count -lt 2) { continue }

            $allDevices += [PSCustomObject]@{
                Serial    = $parts[0]
                State     = $parts[1]
                Model     = if ($str -match 'model:(\S+)') { $Matches[1] } else { "Unknown" }
                Transport = if ($parts[0] -match ':') { "WiFi/TCP" } else { "USB" }
            }
        }

        # Surface unauthorized devices with clear guidance
        $unauthorized = @($allDevices | Where-Object { $_.State -eq "unauthorized" })
        if ($unauthorized.Count -gt 0) {
            Write-Host ""
            foreach ($d in $unauthorized) {
                Write-Host "  [!] Device $($d.Serial) is UNAUTHORIZED." -ForegroundColor Yellow
                Write-Host "      -> On your device: accept the 'Allow USB Debugging' dialog." -ForegroundColor Yellow
            }
        }

        # Surface offline devices
        $offline = @($allDevices | Where-Object { $_.State -eq "offline" })
        if ($offline.Count -gt 0) {
            foreach ($d in $offline) {
                Write-Host "  [!] Device $($d.Serial) is OFFLINE. Reconnect USB cable." -ForegroundColor Yellow
            }
        }

        $ready = @($allDevices | Where-Object { $_.State -eq "device" })

        if ($ready.Count -eq 0) {
            Write-Host "  [ERROR] No authorized and ready device found." -ForegroundColor Red
            Write-Log "No ready device" -Level ERROR
            return $false
        }

        # Multi-device picker
        if ($ready.Count -gt 1) {
            Write-Host ""
            Write-Host "  Multiple devices detected:" -ForegroundColor Yellow
            for ($i = 0; $i -lt $ready.Count; $i++) {
                Write-Host ("  [{0}] {1}  ({2})  via {3}" -f `
                    ($i + 1), $ready[$i].Model, $ready[$i].Serial, $ready[$i].Transport) -ForegroundColor White
            }
            Write-Host ""
            $pick = Read-Host "  Select device [1-$($ready.Count)]"
            $pickIdx = 0
            if (-not [int]::TryParse($pick, [ref]$pickIdx) -or $pickIdx -lt 1 -or $pickIdx -gt $ready.Count) {
                Write-Host "  [ERROR] Invalid selection." -ForegroundColor Red
                return $false
            }
            $script:Config.DeviceSerial = $ready[$pickIdx - 1].Serial
        }
        else {
            $script:Config.DeviceSerial = $ready[0].Serial
        }

        $chosenDevice = $ready | Where-Object { $_.Serial -eq $script:Config.DeviceSerial } | Select-Object -First 1
        Write-Host "  [OK] Device ready : $($script:Config.DeviceSerial)  ($($chosenDevice.Model)  via $($chosenDevice.Transport))" -ForegroundColor Green
        Write-Log "Device: $($script:Config.DeviceSerial)" -Level INFO
        return $true
    }
    catch {
        Write-Host "  [ERROR] ADB communication failed: $_" -ForegroundColor Red
        Write-Log "ADB error: $_" -Level ERROR
        return $false
    }
}

function Get-DeviceInfo {
    try {
        $s = $script:Config.DeviceSerial
        $codename = (& $script:Config.AdbPath -s $s shell getprop ro.product.device 2>&1).Trim()
        $model    = (& $script:Config.AdbPath -s $s shell getprop ro.product.model 2>&1).Trim()
        $android  = (& $script:Config.AdbPath -s $s shell getprop ro.build.version.release 2>&1).Trim()
        $sdk      = (& $script:Config.AdbPath -s $s shell getprop ro.build.version.sdk 2>&1).Trim()
        $miui     = (& $script:Config.AdbPath -s $s shell getprop ro.miui.ui.version.name 2>&1).Trim()

        $brand    = (& $script:Config.AdbPath -s $s shell getprop ro.product.brand 2>&1).Trim()

        $script:Config.DeviceCodename = $codename
        $script:Config.DeviceModel    = $model
        $script:Config.DeviceBrand    = $brand.ToLower()

        Write-Host ""
        Write-Host "  Device Info" -ForegroundColor Cyan
        Write-Host "  -----------" -ForegroundColor Cyan
        Write-Host "  Serial   : $s" -ForegroundColor White
        Write-Host "  Brand    : $brand" -ForegroundColor White
        Write-Host "  Model    : $model  ($codename)" -ForegroundColor White
        Write-Host "  Android  : $android  (SDK $sdk)" -ForegroundColor White
        if ($miui) {
            Write-Host "  MIUI     : $miui" -ForegroundColor White
        }
        Write-Host ""

        if ($script:DevicePackages.ContainsKey($codename)) {
            Write-Host "  [OK] Device-specific package list loaded for: $codename" -ForegroundColor Green
        }
        else {
            Write-Host "  [INFO] No device-specific list for '$codename' - using generic $brand packages" -ForegroundColor Cyan
        }

        Write-Log "Device: $model ($codename) Android $android SDK $sdk" -Level INFO
        return $true
    }
    catch {
        Write-Host "  [ERROR] Failed to read device info: $_" -ForegroundColor Red
        Write-Log "Device info error: $_" -Level ERROR
        return $false
    }
}

# ---------------------------------------------------------------------------
# Package Helpers
# ---------------------------------------------------------------------------

function Get-MergedCategory {
    param([string]$CategoryKey)

    $category = $script:PackageDatabase[$CategoryKey]
    if (-not $category) { return $null }

    $merged = [ordered]@{}
    foreach ($kv in $category.Packages.GetEnumerator()) {
        $merged[$kv.Key] = $kv.Value
    }

    $codename = $script:Config.DeviceCodename
    if ($codename -and $script:DevicePackages.ContainsKey($codename)) {
        $deviceCat = $script:DevicePackages[$codename][$CategoryKey]
        if ($deviceCat) {
            foreach ($kv in $deviceCat.GetEnumerator()) {
                $merged[$kv.Key] = $kv.Value
            }
        }
    }

    return $merged
}

function Get-InstalledPackageSet {
    $raw = & $script:Config.AdbPath -s $script:Config.DeviceSerial shell pm list packages 2>&1
    $set = @{}
    foreach ($line in $raw) {
        if ($line -match "^package:(.+)$") {
            $set[$Matches[1].Trim()] = $true
        }
    }
    return $set
}

# ---------------------------------------------------------------------------
# Selection Parser
# ---------------------------------------------------------------------------

function ConvertTo-SelectedIndices {
    # $Input is a PowerShell reserved automatic variable (pipeline enumerator).
    # Using it as a param name breaks named argument binding - renamed to $RawInput.
    param([string]$RawInput, [int]$MaxIndex)

    if ($RawInput.Trim().ToLower() -eq "all") {
        return 1..$MaxIndex
    }

    $indices = @()
    foreach ($part in ($RawInput -split ',')) {
        $part = $part.Trim()
        if ($part -match '^(\d+)-(\d+)$') {
            $a = [int]$Matches[1]; $b = [int]$Matches[2]
            if ($a -ge 1 -and $b -le $MaxIndex -and $a -le $b) {
                $indices += $a..$b
            }
        }
        elseif ($part -match '^\d+$') {
            $n = [int]$part
            if ($n -ge 1 -and $n -le $MaxIndex) { $indices += $n }
        }
    }
    return ($indices | Sort-Object -Unique)
}

# ---------------------------------------------------------------------------
# Package Selection UI
# ---------------------------------------------------------------------------

function Invoke-CategoryBrowser {
    $brand = if ($script:Config.DeviceBrand) { $script:Config.DeviceBrand.ToLower() } else { "" }

    # Build list of visible categories filtered by detected brand.
    # Categories with empty Brands array are always shown (brand-agnostic).
    $visibleKeys = @($script:PackageDatabase.Keys | Where-Object {
        $cat = $script:PackageDatabase[$_]
        $cat.Brands.Count -eq 0 -or ($brand -and $brand -in $cat.Brands)
    })

    Write-Host ""
    Write-Host "  Package Categories  (brand: $($script:Config.DeviceBrand)):" -ForegroundColor Cyan
    Write-Host "  -------------------------------------------------------" -ForegroundColor Cyan

    for ($i = 0; $i -lt $visibleKeys.Count; $i++) {
        $key      = $visibleKeys[$i]
        $cat      = $script:PackageDatabase[$key]
        $pkgs     = Get-MergedCategory -CategoryKey $key
        $permNote = if (-not $cat.CanPermanentDelete) { " [SAFE DISABLE ONLY]" } else { "" }
        Write-Host ("  [{0}] {1} ({2} pkgs){3}" -f ($i + 1), $cat.Label, $pkgs.Count, $permNote) -ForegroundColor $cat.Color
    }

    Write-Host "  [0] Back" -ForegroundColor Gray
    Write-Host ""
    $choice = Read-Host "  Select category [0-$($visibleKeys.Count)]"
    if ($choice -eq "0" -or $choice -eq "") { return }

    $idx = 0
    if (-not [int]::TryParse($choice, [ref]$idx) -or $idx -lt 1 -or $idx -gt $visibleKeys.Count) {
        Write-Host "  [ERROR] Invalid selection." -ForegroundColor Red
        return
    }

    Invoke-PackageSelection -CategoryKey $visibleKeys[$idx - 1]
}

function Invoke-PackageSelection {
    param([string]$CategoryKey)

    $category    = $script:PackageDatabase[$CategoryKey]
    $packages    = Get-MergedCategory -CategoryKey $CategoryKey
    $packageKeys = @($packages.Keys)

    Write-Host ""
    Write-Host ("  === {0} ===" -f $category.Label) -ForegroundColor $category.Color
    Write-Host ""

    $installedSet = @{}
    try {
        $installedSet = Get-InstalledPackageSet
    }
    catch {
        Write-Host "  [WARNING] Could not check installed status - listing all." -ForegroundColor Yellow
    }

    Write-Host ("  {0,-4} {1,-48} {2,-32} {3}" -f "No.", "Package", "Friendly Name", "Status") -ForegroundColor Cyan
    Write-Host ("  {0}" -f ("-" * 104)) -ForegroundColor DarkGray

    for ($i = 0; $i -lt $packageKeys.Count; $i++) {
        $pkg    = $packageKeys[$i]
        $name   = $packages[$pkg]
        $active = $installedSet.ContainsKey($pkg)
        $status = if ($active) { "[active]  " } else { "[inactive]" }
        $sColor = if ($active) { $category.Color } else { "DarkGray" }

        Write-Host -NoNewline ("  {0,-4} {1,-48} {2,-32} " -f ($i + 1), $pkg, $name)
        Write-Host $status -ForegroundColor $sColor
    }

    Write-Host ""
    Write-Host "  Input: 'all'  |  single: 3  |  list: 1,3,5  |  range: 1-10  |  mixed: 1-3,7,9" -ForegroundColor DarkGray
    Write-Host "  [ENTER] Cancel" -ForegroundColor DarkGray
    Write-Host ""
    $selInput = Read-Host "  Select packages"

    if ([string]::IsNullOrWhiteSpace($selInput)) {
        Write-Host "  [INFO] Cancelled." -ForegroundColor Cyan
        return
    }

    $indices = ConvertTo-SelectedIndices -RawInput $selInput -MaxIndex $packageKeys.Count
    if ($indices.Count -eq 0) {
        Write-Host "  [ERROR] No valid packages in selection." -ForegroundColor Red
        return
    }

    $selectedPackages = [ordered]@{}
    foreach ($idx in $indices) {
        $pkg = $packageKeys[$idx - 1]
        $selectedPackages[$pkg] = $packages[$pkg]
    }

    Write-Host ""
    Write-Host "  Selected $($selectedPackages.Count) package(s):" -ForegroundColor Cyan
    foreach ($kv in $selectedPackages.GetEnumerator()) {
        Write-Host ("    - {0,-48} {1}" -f $kv.Key, $kv.Value) -ForegroundColor White
    }

    Write-Host ""
    Write-Host "  Operation Mode:" -ForegroundColor Cyan
    Write-Host "  [1] Safe Disable   - pm uninstall -k --user 0  (reversible)" -ForegroundColor Green
    if ($category.CanPermanentDelete) {
        Write-Host "  [2] Permanent Remove - pm uninstall  (IRREVERSIBLE, no restore)" -ForegroundColor Red
    }
    else {
        Write-Host "  [2] Permanent Remove - BLOCKED (brick protection for this category)" -ForegroundColor DarkGray
    }
    Write-Host "  [0] Cancel" -ForegroundColor Gray
    Write-Host ""
    $modeChoice = Read-Host "  Select mode [0-2]"

    switch ($modeChoice) {
        "1" { Invoke-SafeDisable -Packages $selectedPackages -CategoryKey $CategoryKey }
        "2" {
            if (-not $category.CanPermanentDelete) {
                Write-Host "  [BLOCKED] Permanent removal is not allowed for '$($category.Label)'." -ForegroundColor Red
                Write-Log "Permanent remove BLOCKED for $CategoryKey" -Level WARNING
                return
            }
            Invoke-PermanentRemove -Packages $selectedPackages -CategoryKey $CategoryKey
        }
        "0" { Write-Host "  [INFO] Cancelled." -ForegroundColor Cyan }
        default { Write-Host "  [ERROR] Invalid choice." -ForegroundColor Red }
    }
}

# ---------------------------------------------------------------------------
# ADB Operations
# ---------------------------------------------------------------------------

function Invoke-SafeDisable {
    param(
        [System.Collections.Specialized.OrderedDictionary]$Packages,
        [string]$CategoryKey
    )

    Write-Host ""
    Write-Host "  [SAFE DISABLE] pm uninstall -k --user 0" -ForegroundColor Green
    Write-Host "  Packages remain on device. Full restore available at any time." -ForegroundColor DarkGray
    Write-Host ""
    Write-Host "  Type 'CONFIRM' to proceed: " -ForegroundColor Yellow -NoNewline
    if ((Read-Host) -ne "CONFIRM") {
        Write-Host "  [CANCELLED]" -ForegroundColor Cyan
        return
    }

    $successPkgs = [ordered]@{}

    foreach ($kv in $Packages.GetEnumerator()) {
        $pkg  = $kv.Key
        $name = $kv.Value

        Write-Host -NoNewline ("  {0,-50} " -f $pkg) -ForegroundColor Gray
        Write-Log "Safe disable: $pkg" -Level INFO

        try {
            $out = (& $script:Config.AdbPath -s $script:Config.DeviceSerial shell pm uninstall -k --user 0 $pkg 2>&1).Trim()
            if ($out -eq "Success") {
                Write-Host "[OK] $name" -ForegroundColor Green
                Write-Log "SUCCESS: $pkg" -Level SUCCESS
                $successPkgs[$pkg] = $name
            }
            else {
                Write-Host "[FAILED] $out" -ForegroundColor Red
                Write-Log "FAILED: $pkg - $out" -Level WARNING
            }
        }
        catch {
            Write-Host "[ERROR] $_" -ForegroundColor Red
            Write-Log "ERROR: $pkg - $_" -Level ERROR
        }
    }

    if ($successPkgs.Count -gt 0) {
        Save-BackupSnapshot -Packages $successPkgs -CategoryKey $CategoryKey -Mode "SafeDisable"
    }

    Write-Host ""
    Write-Host ("  Completed: {0} disabled, {1} failed." -f $successPkgs.Count, ($Packages.Count - $successPkgs.Count)) -ForegroundColor Cyan
    Write-Log "Safe disable done: $($successPkgs.Count)/$($Packages.Count)" -Level INFO
}

function Invoke-PermanentRemove {
    param(
        [System.Collections.Specialized.OrderedDictionary]$Packages,
        [string]$CategoryKey
    )

    Write-Host ""
    Write-Host "  ================================================================================" -ForegroundColor Red
    Write-Host "  PERMANENT REMOVE - IRREVERSIBLE OPERATION" -ForegroundColor Red
    Write-Host "  ================================================================================" -ForegroundColor Red
    Write-Host ""
    Write-Host "  These packages will be PERMANENTLY deleted from the device." -ForegroundColor Red
    Write-Host "  They cannot be restored via this tool or ADB. Factory reset is required." -ForegroundColor Red
    Write-Host ""

    foreach ($kv in $Packages.GetEnumerator()) {
        Write-Host ("    - {0}  ({1})" -f $kv.Key, $kv.Value) -ForegroundColor Red
    }

    Write-Host ""
    Write-Host "  Type 'DELETE PERMANENTLY' to confirm: " -ForegroundColor Red -NoNewline
    if ((Read-Host) -ne "DELETE PERMANENTLY") {
        Write-Host "  [CANCELLED] Permanent removal cancelled." -ForegroundColor Cyan
        return
    }

    $successCount = 0

    foreach ($kv in $Packages.GetEnumerator()) {
        $pkg  = $kv.Key
        $name = $kv.Value

        Write-Host -NoNewline ("  {0,-50} " -f $pkg) -ForegroundColor Gray
        Write-Log "Permanent remove: $pkg" -Level WARNING

        try {
            $out = (& $script:Config.AdbPath -s $script:Config.DeviceSerial shell pm uninstall $pkg 2>&1).Trim()
            if ($out -eq "Success") {
                Write-Host "[DELETED] $name" -ForegroundColor Red
                Write-Log "PERMANENTLY DELETED: $pkg" -Level WARNING
                $successCount++
            }
            else {
                Write-Host "[FAILED] $out" -ForegroundColor Yellow
                Write-Log "FAILED permanent: $pkg - $out" -Level ERROR
            }
        }
        catch {
            Write-Host "[ERROR] $_" -ForegroundColor Red
            Write-Log "ERROR permanent: $pkg - $_" -Level ERROR
        }
    }

    Write-Host ""
    Write-Host ("  Completed: {0} permanently removed." -f $successCount) -ForegroundColor Red
    Write-Log "Permanent remove done: $successCount/$($Packages.Count)" -Level WARNING
}

# ---------------------------------------------------------------------------
# Backup & Restore
# ---------------------------------------------------------------------------

function Save-BackupSnapshot {
    param(
        [System.Collections.Specialized.OrderedDictionary]$Packages,
        [string]$CategoryKey,
        [string]$Mode
    )

    $snapshot = [ordered]@{
        SessionID = $script:Config.SessionID
        Timestamp = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
        Device    = $script:Config.DeviceCodename
        Model     = $script:Config.DeviceModel
        Serial    = $script:Config.DeviceSerial
        Category  = $CategoryKey
        Mode      = $Mode
        Packages  = @()
    }

    foreach ($kv in $Packages.GetEnumerator()) {
        $snapshot.Packages += [ordered]@{
            Package      = $kv.Key
            FriendlyName = $kv.Value
            Category     = $CategoryKey
        }
    }

    $file = Join-Path $script:Config.BackupDirectory "backup_$($script:Config.SessionID)_${CategoryKey}.json"
    $snapshot | ConvertTo-Json -Depth 5 | Out-File -FilePath $file -Encoding UTF8

    Write-Host "  [OK] Backup : $(Split-Path $file -Leaf)" -ForegroundColor Green
    Write-Log "Backup saved: $file" -Level INFO
}

function Invoke-RestoreMode {
    Write-Host ""
    Write-Host "  ================================================================================" -ForegroundColor Magenta
    Write-Host "  RESTORE MODE" -ForegroundColor Magenta
    Write-Host "  ================================================================================" -ForegroundColor Magenta
    Write-Host ""

    $backupFiles = Get-ChildItem -Path $script:Config.BackupDirectory -Filter "backup_*.json" `
                    -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending

    if (-not $backupFiles -or $backupFiles.Count -eq 0) {
        Write-Host "  [INFO] No backup files found. Run a safe disable first." -ForegroundColor Yellow
        Write-Log "Restore: no backup files" -Level INFO
        return
    }

    $sessions = @()
    Write-Host "  Backup sessions:" -ForegroundColor Cyan
    Write-Host ""

    for ($i = 0; $i -lt $backupFiles.Count; $i++) {
        try {
            $data     = Get-Content $backupFiles[$i].FullName -Raw | ConvertFrom-Json
            $pkgCount = ($data.Packages | Measure-Object).Count
            Write-Host ("  [{0}] {1}  Device:{2}  Category:{3}  Mode:{4}  ({5} pkgs)" -f `
                ($i + 1), $data.Timestamp, $data.Device, $data.Category, $data.Mode, $pkgCount) -ForegroundColor White
            $sessions += $data
        }
        catch {
            Write-Host ("  [{0}] {1} [UNREADABLE]" -f ($i + 1), $backupFiles[$i].Name) -ForegroundColor DarkGray
            $sessions += $null
        }
    }

    Write-Host "  [0] Back" -ForegroundColor Gray
    Write-Host ""
    $choice = Read-Host "  Select session [0-$($backupFiles.Count)]"
    if ($choice -eq "0" -or $choice -eq "") { return }

    $idx = 0
    if (-not [int]::TryParse($choice, [ref]$idx) -or $idx -lt 1 -or $idx -gt $sessions.Count) {
        Write-Host "  [ERROR] Invalid selection." -ForegroundColor Red
        return
    }

    $session = $sessions[$idx - 1]
    if (-not $session) {
        Write-Host "  [ERROR] Backup unreadable." -ForegroundColor Red
        return
    }

    $pkgs = $session.Packages
    Write-Host ""
    Write-Host "  Packages in session ($($pkgs.Count) total):" -ForegroundColor Cyan
    Write-Host ""
    for ($i = 0; $i -lt $pkgs.Count; $i++) {
        Write-Host ("  [{0,-3}] {1,-48} {2}" -f ($i + 1), $pkgs[$i].Package, $pkgs[$i].FriendlyName) -ForegroundColor White
    }

    Write-Host ""
    Write-Host "  Input: 'all'  |  single: 3  |  list: 1,3,5  |  range: 1-10" -ForegroundColor DarkGray
    $selInput = Read-Host "  Select packages to restore"
    if ([string]::IsNullOrWhiteSpace($selInput)) { return }

    $indices = ConvertTo-SelectedIndices -RawInput $selInput -MaxIndex $pkgs.Count
    if ($indices.Count -eq 0) {
        Write-Host "  [ERROR] No valid packages selected." -ForegroundColor Red
        return
    }

    Write-Host ""
    Write-Host "  Type 'CONFIRM' to restore: " -ForegroundColor Yellow -NoNewline
    if ((Read-Host) -ne "CONFIRM") {
        Write-Host "  [CANCELLED]" -ForegroundColor Cyan
        return
    }

    $successCount = 0
    foreach ($i in $indices) {
        $pkg  = $pkgs[$i - 1].Package
        $name = $pkgs[$i - 1].FriendlyName

        Write-Host -NoNewline ("  {0,-50} " -f $pkg) -ForegroundColor Gray
        Write-Log "Restore: $pkg" -Level INFO

        try {
            $out = (& $script:Config.AdbPath -s $script:Config.DeviceSerial shell cmd package install-existing $pkg 2>&1).Trim()
            # MIUI and stock Android both indicate success with "installed for user" substring.
            # Treat absence of error keywords as success to handle firmware-specific output variations.
            $isSuccess = ($out -match '(?i)installed') -and ($out -notmatch '(?i)(error|exception|failed|not found)')
            if ($isSuccess) {
                Write-Host "[OK] $name" -ForegroundColor Green
                Write-Log "RESTORED: $pkg - $out" -Level SUCCESS
                $successCount++
            }
            else {
                Write-Host "[FAILED] $out" -ForegroundColor Red
                Write-Log "RESTORE FAILED: $pkg - $out" -Level ERROR
            }
        }
        catch {
            Write-Host "[ERROR] $_" -ForegroundColor Red
            Write-Log "RESTORE ERROR: $pkg - $_" -Level ERROR
        }
    }

    Write-Host ""
    Write-Host ("  Restore complete: {0}/{1} packages restored." -f $successCount, $indices.Count) -ForegroundColor Cyan
    Write-Log "Restore done: $successCount/$($indices.Count)" -Level INFO
}

# ---------------------------------------------------------------------------
# List Disabled Packages
# ---------------------------------------------------------------------------

function Show-DisabledPackages {
    Write-Host ""
    Write-Host "  Querying device for disabled packages..." -ForegroundColor Cyan

    try {
        # pm list packages        -> active packages only
        # pm list packages -u     -> all packages (active + uninstalled-for-user)
        # Difference = uninstalled-for-user (i.e. what we disabled via pm uninstall -k --user 0)
        $allRaw    = & $script:Config.AdbPath -s $script:Config.DeviceSerial shell pm list packages -u 2>&1
        $activeRaw = & $script:Config.AdbPath -s $script:Config.DeviceSerial shell pm list packages 2>&1

        $allPkgs    = @{}
        $activePkgs = @{}

        foreach ($line in $allRaw) {
            $lineStr = $line.ToString().Trim()
            if ($lineStr -match "^package:(.+)$") { $allPkgs[$Matches[1].Trim()] = $true }
        }
        foreach ($line in $activeRaw) {
            $lineStr = $line.ToString().Trim()
            if ($lineStr -match "^package:(.+)$") { $activePkgs[$Matches[1].Trim()] = $true }
        }

        if ($allPkgs.Count -eq 0) {
            Write-Host "  [WARNING] Could not retrieve package list from device." -ForegroundColor Yellow
            return
        }

        # Disabled-for-user = in all but not in active
        $disabled = @($allPkgs.Keys | Where-Object { -not $activePkgs.ContainsKey($_) } | Sort-Object)

        if (-not $disabled -or @($disabled).Count -eq 0) {
            Write-Host "  [INFO] No disabled packages found on device." -ForegroundColor Cyan
            return
        }

        Write-Host ("  {0} disabled package(s):" -f @($disabled).Count) -ForegroundColor Yellow
        Write-Host ""
        Write-Host ("  {0,-50} {1}" -f "Package", "Friendly Name") -ForegroundColor Cyan
        Write-Host ("  {0}" -f ("-" * 80)) -ForegroundColor DarkGray

        foreach ($pkg in $disabled) {
            $friendlyName = ""
            foreach ($catKey in $script:PackageDatabase.Keys) {
                $merged = Get-MergedCategory -CategoryKey $catKey
                if ($merged.Contains($pkg)) {
                    $friendlyName = $merged[$pkg]
                    break
                }
            }
            $nameDisplay = if ($friendlyName) { $friendlyName } else { "(unknown)" }
            $color       = if ($friendlyName) { "White" } else { "DarkGray" }
            Write-Host ("  {0,-50} {1}" -f $pkg, $nameDisplay) -ForegroundColor $color
        }

        Write-Log "Listed $(@($disabled).Count) disabled packages" -Level INFO
    }
    catch {
        Write-Host "  [ERROR] Failed to query packages: $_" -ForegroundColor Red
        Write-Log "List disabled error: $_" -Level ERROR
    }
}

# ---------------------------------------------------------------------------
# Main Menu
# ---------------------------------------------------------------------------

function Show-MainMenu {
    Write-Host ""
    Write-Host "  ================================================================================" -ForegroundColor Cyan
    $deviceDisplay = if ($script:Config.DeviceCodename) {
        "$($script:Config.DeviceModel) ($($script:Config.DeviceCodename))  |  Serial: $($script:Config.DeviceSerial)"
    } else {
        "No device"
    }
    Write-Host "  $deviceDisplay" -ForegroundColor Gray
    Write-Host "  ================================================================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  [1] Device Info" -ForegroundColor White
    Write-Host "  [2] Browse & Debloat  (by category)" -ForegroundColor Green
    Write-Host "  [3] Restore Packages  (from backup session)" -ForegroundColor Magenta
    Write-Host "  [4] List Disabled Packages  (live from device)" -ForegroundColor Yellow
    Write-Host "  [5] Refresh Device Connection" -ForegroundColor Cyan
    Write-Host "  [6] Exit" -ForegroundColor Gray
    Write-Host ""
    Write-Host "  ================================================================================" -ForegroundColor Cyan
    Write-Host ""
    return (Read-Host "  Select [1-6]")
}

# ---------------------------------------------------------------------------
# Entry Point
# ---------------------------------------------------------------------------

try {
    Show-Banner

    Write-Host "[Step 1] Initializing environment..." -ForegroundColor Cyan
    if (-not (Initialize-Environment)) { exit 1 }

    Write-Host ""
    Write-Host "[Step 2] Locating ADB..." -ForegroundColor Cyan
    if (-not (Find-AdbExecutable)) { exit 1 }

    Write-Host ""
    Write-Host "[Step 3] Connecting to device..." -ForegroundColor Cyan
    if (-not (Test-DeviceConnection)) { exit 1 }

    Get-DeviceInfo | Out-Null

    Write-Host ""
    Write-Host "  Press any key to continue to main menu..." -ForegroundColor Gray
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

    $running = $true
    while ($running) {
        Clear-Host
        Show-Banner

        $choice = Show-MainMenu

        switch ($choice) {
            "1" {
                Get-DeviceInfo | Out-Null
                Write-Host ""
                Write-Host "  Press any key..." -ForegroundColor Gray
                $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
            }
            "2" {
                Invoke-CategoryBrowser
                Write-Host ""
                Write-Host "  Press any key..." -ForegroundColor Gray
                $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
            }
            "3" {
                Invoke-RestoreMode
                Write-Host ""
                Write-Host "  Press any key..." -ForegroundColor Gray
                $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
            }
            "4" {
                Show-DisabledPackages
                Write-Host ""
                Write-Host "  Press any key..." -ForegroundColor Gray
                $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
            }
            "5" {
                Write-Host ""
                Write-Host "  Reconnecting..." -ForegroundColor Cyan
                $script:Config.DeviceSerial    = $null
                $script:Config.DeviceCodename  = $null
                $script:Config.DeviceModel     = $null
                if (Test-DeviceConnection) { Get-DeviceInfo | Out-Null }
                Write-Host ""
                Write-Host "  Press any key..." -ForegroundColor Gray
                $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
            }
            "6" {
                Write-Host ""
                Write-Host "  Exiting." -ForegroundColor Cyan
                Write-Log "Session ended by user" -Level INFO
                $running = $false
            }
            default {
                Write-Host "  [ERROR] Invalid selection." -ForegroundColor Red
                Start-Sleep -Milliseconds 700
            }
        }
    }
}
catch {
    Write-Host ""
    Write-Host "  ================================================================================" -ForegroundColor Red
    Write-Host "  CRITICAL ERROR" -ForegroundColor Red
    Write-Host "  ================================================================================" -ForegroundColor Red
    Write-Host "  $_" -ForegroundColor Yellow
    Write-Host ""
    Write-Log "Critical error: $_" -Level ERROR
    exit 1
}
finally {
    Write-Host ""
    Write-Host ("  Session: {0}  |  Ended: {1}" -f $script:Config.SessionID, (Get-Date -Format "yyyy-MM-dd HH:mm:ss")) -ForegroundColor Gray
    if ($script:Config.LogFile) {
        Write-Host "  Log: $($script:Config.LogFile)" -ForegroundColor Gray
    }
    Write-Host ""
}
