# D08 Windows read-only capture procedure

The integrated freeze-evidence authorization permits this bounded read-only
capture lane. `capture_d08_readonly.ps1` records one Windows sample without
changing a display setting. It writes one new evidence file and refuses to
overwrite an existing file. It does not create same-boot acceptance authority.

The first reported Windows 10 `10.0.19045` sample passed the capture validator.
Its derived, non-identifying timing summary was: sample tick span `109 ms`, UTC
span `98.326 ms`, predicted-boot spread `10.674 ms`, and absolute WMI-to-
predicted-boot deltas approximately `40.019..40.030 s`. A subsequent active-
session batch passed capture and validation 5/5 with one boot tuple; its maxima
were tick span `63 ms`, UTC span `57.072 ms`, and predicted-boot spread
`9.562 ms`. The raw captures remain outside Git, so this is not yet a formal
bundled artifact or a tolerance freeze.

On an authorized Windows evidence host, the capture operator records the
following order in the D08 capture schema:

1. Capture `t0 = GetTickCount64`.
2. Capture `u0 = precise UTC FILETIME`.
3. Read `Win32_OperatingSystem.LastBootUpTime`, `Version`, and `BuildNumber`.
4. Strictly parse the approved DMTF and version grammars; reject on any parse
   or build-number disagreement.
5. Capture `u1 = precise UTC FILETIME`.
6. Capture `t1 = GetTickCount64`.
7. Compute the static BootId preimage only from canonical boot UTC and parsed
   major/minor/build. Do not hash time-varying samples.
8. Leave `maxBootIdentitySampleSpanMs`, `maxBootUtcDelta100ns`, and
   `clockJumpRule` as `UNSET` until approved Windows evidence freezes them.

From the repository root in Windows PowerShell 5.1, write the raw sample
outside the repository and validate it:

```powershell
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$capture = Join-Path $env:TEMP "displaydeck-d08-$stamp.json"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\displaydeck-evidence\capture_d08_readonly.ps1 -OutputPath $capture
if ($LASTEXITCODE -ne 0) { throw "D08 capture failed: $LASTEXITCODE" }
py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py $capture
if ($LASTEXITCODE -ne 0) { throw "D08 validation failed: $LASTEXITCODE" }
Get-Content -Raw $capture
```

Before selecting any tolerance, collect one bounded five-sample batch in the
same active session. This reuses the single-sample helper and creates no new
probe surface:

```powershell
$label = "active"
$batch = Join-Path $env:TEMP ("displaydeck-d08-{0}-{1}" -f $label, (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $batch -ErrorAction Stop | Out-Null
1..5 | ForEach-Object {
    $capture = Join-Path $batch ("sample-{0:d2}.json" -f $_)
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\displaydeck-evidence\capture_d08_readonly.ps1 -OutputPath $capture
    if ($LASTEXITCODE -ne 0) { throw "D08 capture failed: $LASTEXITCODE" }
    py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py $capture
    if ($LASTEXITCODE -ne 0) { throw "D08 validation failed: $LASTEXITCODE" }
    Start-Sleep -Milliseconds 500
}
Write-Output "batch: $batch"
```

Later batches use the same command after an operator-controlled sleep/resume
and full restart. Hibernate/resume and Fast Startup are collected only for a
support cell that intends to admit them. The helper never triggers those power
transitions. Wall-clock changes and WMI failure injection remain outside this
read-only lane and require separate authorization. No sample count or observed
maximum is itself an approved production threshold.

After the batch, list the five boot tuples and bounded timing differences. The
underscore variable is `$_`; `$*` is not the current pipeline item.

```powershell
$rows = Get-ChildItem "$batch\sample-*.json" -File | Sort-Object Name | ForEach-Object {
    $j = Get-Content -Raw $_.FullName | ConvertFrom-Json
    [Int64]$t0 = [Convert]::ToUInt64($j.tickBeforeMs, 16)
    [Int64]$t1 = [Convert]::ToUInt64($j.tickAfterMs, 16)
    [Int64]$u0 = [Convert]::ToUInt64($j.utcBeforeFileTime, 16)
    [Int64]$u1 = [Convert]::ToUInt64($j.utcAfterFileTime, 16)
    [PSCustomObject]@{
        Sample = $_.Name
        BootTime = $j.lastBootUpTimeRaw
        Version = $j.versionRaw
        Build = $j.buildNumberRaw
        TickSpanMs = $t1 - $t0
        UtcSpanMs = [Math]::Round(($u1 - $u0) / 10000.0, 3)
        PredictedBootSpreadMs = [Math]::Round([Math]::Abs((($u1 - ($t1 * 10000)) - ($u0 - ($t0 * 10000))) / 10000.0), 3)
        Result = $j.result
    }
}
$rows | Format-Table -AutoSize
if (@($rows).Count -ne 5) { throw "Expected 5 D08 samples" }
if (@($rows | Select-Object BootTime, Version, Build -Unique).Count -ne 1) { throw "D08 boot tuple changed within active batch" }
if (@($rows | Where-Object Result -ne "ACCEPTANCE_NOT_AUTHORIZED").Count -ne 0) { throw "Unexpected D08 result" }
```

The next bounded scenario is a five-sample batch after an operator-controlled
sleep/resume. Preserve the active batch path before sleeping; the helper does
not trigger the power transition:

```powershell
$activeBatch = $batch

# The operator manually sleeps and resumes Windows before continuing.
$label = "sleep-resume"
$batch = Join-Path $env:TEMP ("displaydeck-d08-{0}-{1}" -f $label, (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $batch -ErrorAction Stop | Out-Null
1..5 | ForEach-Object {
    $capture = Join-Path $batch ("sample-{0:d2}.json" -f $_)
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\displaydeck-evidence\capture_d08_readonly.ps1 -OutputPath $capture
    if ($LASTEXITCODE -ne 0) { throw "D08 capture failed: $LASTEXITCODE" }
    py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py $capture
    if ($LASTEXITCODE -ne 0) { throw "D08 validation failed: $LASTEXITCODE" }
    Start-Sleep -Milliseconds 500
}
Write-Output "active batch: $activeBatch"
Write-Output "sleep/resume batch: $batch"
```

Run the same aggregation block against the new `$batch`. Keep restart, Fast
Startup, and hibernate evidence separate; no observed maximum authorizes a
production threshold.

The capture document is validated offline with
`validate_d08_readonly_capture.py`. Thresholds remain `UNSET` in every
document under this authorization. A completed Windows sample can only state
`REJECT_NO_AUTHORITY`, `REJECT_CROSS_CHECK`, or
`ACCEPTANCE_NOT_AUTHORIZED`; it never grants same-boot acceptance. Keep the raw
file outside Git until its evidence owner, redaction, retention, and bundle
location have been approved.

Primary API references:

- [`GetTickCount64`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64)
- [`GetSystemTimePreciseAsFileTime`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemtimepreciseasfiletime)
- [`Win32_OperatingSystem`](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-operatingsystem)
