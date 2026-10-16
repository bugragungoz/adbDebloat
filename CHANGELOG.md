# Changelog

## 2.0.0

adbDebloat is now a Windows app with a window instead of a PowerShell menu.

- Connect by USB or over Wi-Fi (pairing code on Android 11 and later, or switch a cable connection
  to Wi-Fi), and download Google's platform-tools when the computer has no adb.
- Every package on the phone with a name, a description and a safety level, from adbDebloat's own
  list and, when allowed, the community list of Universal Android Debloater Next Generation.
- Remove, disable and restore many apps at once, always after a review of what will change.
- Safety rules checked in the backend: the core of Android, packages marked Unsafe, the home screen
  and the keyboard in use are never removed or disabled; Expert ones need expert mode.
- A history of every batch, with undo.
- English and Turkish, light and dark theme, on the Bugra design system.
- Removed: "Permanent Remove", which failed for system apps without root and deleted the data of
  installed apps; every removal is now reversible. Also removed: the "Danger Zone" that offered to
  disable System UI, Settings and launchers.
- Fixed package levels from version 1: the Samsung phone and messages apps, the Galaxy Store,
  Xiaomi Account and the Google sync adapters are no longer listed as plain bloat.

## 1.0.0

- First version: a PowerShell script with brand-aware package lists, safe disable, permanent
  remove and JSON backups for restore. Tested on a Xiaomi Redmi 5 Plus.
