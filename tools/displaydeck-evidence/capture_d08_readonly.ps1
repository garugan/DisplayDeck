[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$OutputPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not ("DisplayDeckD08Native" -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;

public static class DisplayDeckD08Native
{
    [StructLayout(LayoutKind.Sequential)]
    public struct FileTime
    {
        public UInt32 Low;
        public UInt32 High;
    }

    [DllImport("kernel32.dll", ExactSpelling = true)]
    public static extern UInt64 GetTickCount64();

    [DllImport("kernel32.dll", ExactSpelling = true)]
    public static extern void GetSystemTimePreciseAsFileTime(out FileTime value);
}
'@
}

function Get-PreciseUtcFileTime {
    $value = New-Object DisplayDeckD08Native+FileTime
    [DisplayDeckD08Native]::GetSystemTimePreciseAsFileTime([ref]$value)
    return ([UInt64]$value.High * [UInt64]4294967296) + [UInt64]$value.Low
}

function Format-Hex16([UInt64]$Value) {
    return $Value.ToString("x16", [Globalization.CultureInfo]::InvariantCulture)
}

$fullOutputPath = [IO.Path]::GetFullPath($OutputPath)
$outputDirectory = [IO.Path]::GetDirectoryName($fullOutputPath)
if (-not [IO.Directory]::Exists($outputDirectory)) {
    throw "Output directory does not exist: $outputDirectory"
}
if ([IO.File]::Exists($fullOutputPath)) {
    throw "Refusing to overwrite existing evidence: $fullOutputPath"
}

[UInt64]$tickBefore = [DisplayDeckD08Native]::GetTickCount64()
[UInt64]$utcBefore = Get-PreciseUtcFileTime
$rows = @(Get-WmiObject -Class Win32_OperatingSystem -Property LastBootUpTime, Version, BuildNumber)
if ($rows.Count -ne 1) {
    throw "Expected exactly one Win32_OperatingSystem record; got $($rows.Count)"
}
$lastBoot = [string]$rows[0].LastBootUpTime
$version = [string]$rows[0].Version
$build = [string]$rows[0].BuildNumber
[UInt64]$utcAfter = Get-PreciseUtcFileTime
[UInt64]$tickAfter = [DisplayDeckD08Native]::GetTickCount64()

if ($lastBoot -notmatch '^[1-9][0-9]{13}\.[0-9]{6}[+-][0-9]{3}$') {
    throw "Win32_OperatingSystem.LastBootUpTime is not canonical DMTF"
}
if ($version -notmatch '^[1-9][0-9]{0,4}\.[0-9]{1,5}\.[0-9]{1,10}$') {
    throw "Win32_OperatingSystem.Version is not canonical"
}
if ($build -notmatch '^(0|[1-9][0-9]{0,9})$') {
    throw "Win32_OperatingSystem.BuildNumber is not canonical"
}
$versionParts = $version.Split('.')
$result = if ([UInt64]$versionParts[2] -eq [UInt64]$build) {
    "ACCEPTANCE_NOT_AUTHORIZED"
} else {
    "REJECT_CROSS_CHECK"
}

$capture = [ordered]@{
    schemaVersion = "D08-READONLY-CAPTURE-V1"
    captureStatus = "CAPTURED"
    probeAuthorization = "READ_ONLY_AUTHORIZED"
    lastBootUpTimeRaw = $lastBoot
    versionRaw = $version
    buildNumberRaw = $build
    tickBeforeMs = (Format-Hex16 $tickBefore)
    utcBeforeFileTime = (Format-Hex16 $utcBefore)
    tickAfterMs = (Format-Hex16 $tickAfter)
    utcAfterFileTime = (Format-Hex16 $utcAfter)
    maxBootIdentitySampleSpanMs = "UNSET"
    maxBootUtcDelta100ns = "UNSET"
    clockJumpRule = "UNSET"
    result = $result
}
$json = $capture | ConvertTo-Json -Compress
$utf8 = [Text.UTF8Encoding]::new($false)
$bytes = $utf8.GetBytes($json + "`n")
$stream = [IO.FileStream]::new(
    $fullOutputPath,
    [IO.FileMode]::CreateNew,
    [IO.FileAccess]::Write,
    [IO.FileShare]::None
)
try {
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Flush($true)
} finally {
    $stream.Dispose()
}

Write-Output "captured: $fullOutputPath"
