# Installation-only administrator approval (0.2.1)

Ship the 0.2.1 **EXE/NSIS installer**. The older 0.2.0 MSI does not configure the sensor service and must not be used for this installation model.

The installer requests administrator privileges once, installs the UI, provisions the signed PawnIO 2.1.0 driver if absent/outdated, and registers `OverDriveSensors` (display name: OverDrive Hardware Sensors). The driver installer inherits the setup token; neither it nor the sensor process uses another `RunAs` request. Windows SmartScreen or organizational application-control policies are separate from UAC and cannot be suppressed by the app. Signing the distribution with the publisher's certificate is still required to establish a verified publisher.

The service runs automatically as LocalSystem because CPU hardware access needs a privileged token. Its executable and libraries live in the fixed, administrator-protected `C:\Program Files\OverDrive Sensors` directory (using the system's actual Program Files location). Ordinary users have read/execute permission only. The UI remains unelevated and reads loopback-only telemetry. The endpoint accepts only a read-only data request; it does not expose command execution, file operations, tuning, or service administration.

No UAC prompt is requested by normal OverDrive startup. The service persists after the UI closes, restarts after boot, and is removed by the EXE uninstaller. The uninstaller leaves PawnIO because other monitoring programs can share it. Installation/update/uninstallation can still require administrator approval.

On another supported Windows system, the single installer supplies the UI, hidden service, sensor libraries and PawnIO setup. CPU temperature availability still depends on hardware/driver support and machine policy. Native usage continues if temperature is unavailable. WebView2 is handled by Tauri's existing installer bootstrapper; a clean system may need network access for that prerequisite.

For the old per-user 0.2.0 installation, uninstall the old version before deploying the machine-wide installer to avoid duplicate shortcuts/uninstall entries. Preserve the user's application data and profiles. Do not redistribute the old MSI as an equivalent installer.

Sources: Microsoft ServiceBase documentation and LibreHardwareMonitor 0.9.6 PawnIO integration. PawnIO bundled installer SHA-256: `A3A46226C5E2824F4CDD42BE0EECBABFC672C86F7889710F5AB1E6AD385B47A0`; Authenticode publisher: namazso.eu. Official source/distribution: https://pawnio.eu/ and https://github.com/namazso/PawnIO . Existing LibreHardwareMonitor license and third-party notices ship with the service.

## Validation on 2026-10-29

The setup script installed the service on the development PC; read-only HTTP telemetry, service stop/restart, automatic startup configuration, PawnIO dependency and protected executable permissions were verified. A normal-user read returned a 61.38 C CPU package temperature. The existing UI was reopened using the service. The release build and NSIS bundle succeeded. Extracted installer copies of the service executable, setup script and PawnIO installer match their source SHA-256 hashes. A clean-machine installation and reboot were not performed in this session. The service remains installed and running on this PC.
