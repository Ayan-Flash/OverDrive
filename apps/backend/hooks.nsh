!define OVERDRIVE_BACKEND_SOURCE "${__FILEDIR__}"
!include "x64.nsh"

!macro RunSensorSetup MODE
  InitPluginsDir
  File /oname=$PLUGINSDIR\manage-sensor-service.ps1 "${OVERDRIVE_BACKEND_SOURCE}\tools\manage-sensor-service.ps1"
  ; Installation is already elevated. No RunAs is used for the service or driver.
  ${DisableX64FSRedirection}
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\manage-sensor-service.ps1" -Mode ${MODE}'
  ${EnableX64FSRedirection}
  Pop $0
  Pop $1
  ${If} $0 != 0
    DetailPrint "$1"
    MessageBox MB_OK|MB_ICONSTOP "OverDrive sensor setup failed. $1"
    Abort
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro RunSensorSetup Stop
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Fixed administrator-controlled location, independent of the selected UI path.
  SetOutPath "$PROGRAMFILES64\OverDrive Sensors"
  File /r /x LibreHardwareMonitor.exe /x LibreHardwareMonitor.exe.config "${OVERDRIVE_BACKEND_SOURCE}\resources\LibreHardwareMonitor\*"
  File "${OVERDRIVE_BACKEND_SOURCE}\resources\PawnIO\PawnIO_setup.exe"
  !insertmacro RunSensorSetup Install
  SetOutPath "$INSTDIR"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro RunSensorSetup Remove
  RMDir /r "$PROGRAMFILES64\OverDrive Sensors"
!macroend
