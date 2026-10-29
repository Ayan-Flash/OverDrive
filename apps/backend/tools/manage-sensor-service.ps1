param([Parameter(Mandatory=$true)][ValidateSet('Stop','Install','Remove')][string]$Mode)
$ErrorActionPreference = 'Stop'
$serviceName = 'OverDriveSensors'
$serviceDirectory = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'OverDrive Sensors'
$serviceBinary = Join-Path $serviceDirectory 'CpuSensorHost.exe'
$binaryPath = '"' + $serviceBinary + '" --service'
$existing = Get-CimInstance Win32_Service -Filter "Name='OverDriveSensors'"
if ($existing -and $existing.PathName -ne $binaryPath) {
    throw 'An unrelated service uses the OverDriveSensors name. Installation stopped.'
}

function Stop-SensorService {
    $service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    if ($service -and $service.Status -ne 'Stopped') {
        Stop-Service -Name $serviceName
        $service.WaitForStatus('Stopped', [TimeSpan]::FromSeconds(20))
    }
}

if ($Mode -eq 'Stop') { Stop-SensorService; exit 0 }
if ($Mode -eq 'Remove') {
    Stop-SensorService
    if ($existing) {
        & "$env:WINDIR\System32\sc.exe" delete $serviceName
        if ($LASTEXITCODE -ne 0) { throw 'Could not remove OverDrive sensor service.' }
    }
    # PawnIO may be shared with other programs: leave the driver installed.
    exit 0
}

if (!(Test-Path -LiteralPath $serviceBinary)) { throw 'Bundled sensor executable is missing.' }
# The service never loads binaries from a per-user application-data directory.
& "$env:WINDIR\System32\icacls.exe" $serviceDirectory /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-32-545:(OI)(CI)RX' /Q
if ($LASTEXITCODE -ne 0) { throw 'Could not protect the sensor service directory.' }
& "$env:WINDIR\System32\icacls.exe" (Join-Path $serviceDirectory '*') /reset /T /Q
if ($LASTEXITCODE -ne 0) { throw 'Could not apply inherited sensor file permissions.' }

$driverKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO'
$driver = Get-ItemProperty $driverKey -ErrorAction SilentlyContinue
if (!$driver -or [version]$driver.DisplayVersion -lt [version]'2.1.0.0') {
    $installer = Join-Path $serviceDirectory 'PawnIO_setup.exe'
    $signature = Get-AuthenticodeSignature -LiteralPath $installer
    $hash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=namazso.eu' -or
        $hash -ne 'A3A46226C5E2824F4CDD42BE0EECBABFC672C86F7889710F5AB1E6AD385B47A0') {
        throw 'Bundled PawnIO installer verification failed.'
    }
    # Inherits the installer token: no second RunAs/UAC prompt.
    $process = Start-Process -FilePath $installer -ArgumentList '-install' -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "PawnIO installation failed: $($process.ExitCode)" }
    $driver = Get-ItemProperty $driverKey -ErrorAction Stop
    if ([version]$driver.DisplayVersion -lt [version]'2.1.0.0') { throw 'PawnIO installation did not complete.' }
}
if (!$existing) {
    New-Service -Name $serviceName -BinaryPathName $binaryPath -DisplayName 'OverDrive Hardware Sensors' -StartupType Automatic | Out-Null
}
& "$env:WINDIR\System32\sc.exe" config $serviceName start= auto depend= PawnIO
if ($LASTEXITCODE -ne 0) { throw 'Could not configure the PawnIO startup dependency.' }
& "$env:WINDIR\System32\sc.exe" description $serviceName 'Read-only local CPU telemetry for OverDrive. Installed and removed with OverDrive.'
& "$env:WINDIR\System32\sc.exe" failure $serviceName reset= 86400 actions= restart/5000/restart/15000/restart/60000
if ($LASTEXITCODE -ne 0) { throw 'Could not configure sensor recovery.' }
Start-Service -Name $serviceName
(Get-Service $serviceName).WaitForStatus('Running', [TimeSpan]::FromSeconds(20))
for ($attempt = 0; $attempt -lt 30; $attempt++) {
    try {
        $feed = Invoke-RestMethod 'http://127.0.0.1:8085/data.json' -TimeoutSec 1
        if ($feed.ServiceMode -eq $true -and $feed.Provider -eq 'OverDriveSensors' -and $feed.DriverInstalled -eq $true) { exit 0 }
    } catch { }
    Start-Sleep -Milliseconds 500
}
throw 'Sensor service did not become ready. Close any application using port 8085 and run the installer again.'
