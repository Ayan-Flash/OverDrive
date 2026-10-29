$ErrorActionPreference = 'Stop'
$backendDirectory = Split-Path $PSScriptRoot -Parent
$resourceDirectory = Join-Path $backendDirectory 'resources\LibreHardwareMonitor'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if (!(Test-Path -LiteralPath $compiler)) { throw 'The .NET Framework 4 compiler is required.' }
& $compiler /nologo /target:winexe /platform:x64 "/reference:$resourceDirectory\LibreHardwareMonitorLib.dll" /reference:System.Web.Extensions.dll /reference:System.ServiceProcess.dll "/out:$resourceDirectory\CpuSensorHost.exe" "$PSScriptRoot\CpuSensorHost.cs"
if ($LASTEXITCODE -ne 0) { throw 'Sensor host compilation failed.' }
# GUI subsystem is intentional: even an elevated launch must not open a console.
