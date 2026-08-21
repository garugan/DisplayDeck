# D08 Windows read-only capture procedure

The integrated freeze-evidence authorization permits this bounded read-only
capture lane. `capture_d08_readonly.ps1` records one Windows sample without
changing a display setting. It writes one new evidence file and refuses to
overwrite an existing file. It does not create same-boot acceptance authority.

The first reported Windows 10 `10.0.19045` sample passed the capture validator.
Its derived, non-identifying timing summary was: sample tick span `109 ms`, UTC
span `98.326 ms`, predicted-boot spread `10.674 ms`, and absolute WMI-to-
predicted-boot deltas approximately `40.019..40.030 s`. Subsequent active and
sleep/resume batches passed capture and validation 10/10 with one boot tuple.
Active maxima were tick span `63 ms`, UTC span `57.072 ms`, and predicted-boot
spread `9.562 ms`; sleep/resume maxima were `63 ms`, `56.928 ms`, and
`10.276 ms`. Across the operator-controlled sleep interval, tick advanced
`589672 ms`, UTC advanced `589666.412 ms`, their difference was `5.588 ms`,
and BootTime remained unchanged. A subsequent full restart batch passed 5/5:
BootTime and the diagnostic BootId changed across restart, tick reset was
observed, version/build remained `10.0.19045`/`19045`, and the new boot tuple
was stable across all five samples. Restart maxima were tick span `110 ms`, UTC
span `96.023 ms`, and predicted-boot spread `13.977 ms`. All results remained
`ACCEPTANCE_NOT_AUTHORIZED`. The raw captures remain outside Git, so this is
not yet a formal bundled artifact or a tolerance freeze.

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
if (@($rows | Select-Object BootTime, Version, Build -Unique).Count -ne 1) { throw "D08 boot tuple changed within batch" }
if (@($rows | Where-Object Result -ne "ACCEPTANCE_NOT_AUTHORIZED").Count -ne 0) { throw "Unexpected D08 result" }
[PSCustomObject]@{
    Samples = @($rows).Count
    MaxTickSpanMs = ($rows | Measure-Object TickSpanMs -Maximum).Maximum
    MaxUtcSpanMs = ($rows | Measure-Object UtcSpanMs -Maximum).Maximum
    MaxPredictedBootSpreadMs = ($rows | Measure-Object PredictedBootSpreadMs -Maximum).Maximum
    BootTupleStable = $true
} | Format-List
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

Before rebooting, compare the last active observation with the first
sleep/resume observation. Per-query spans alone do not prove that
`GetTickCount64` advanced across sleep:

```powershell
$activeLast = Get-Content -Raw (Join-Path $activeBatch "sample-05.json") | ConvertFrom-Json
$resumeFirst = Get-Content -Raw (Join-Path $batch "sample-01.json") | ConvertFrom-Json
[Int64]$activeTick = [Convert]::ToUInt64($activeLast.tickAfterMs, 16)
[Int64]$resumeTick = [Convert]::ToUInt64($resumeFirst.tickBeforeMs, 16)
[Int64]$activeUtc = [Convert]::ToUInt64($activeLast.utcAfterFileTime, 16)
[Int64]$resumeUtc = [Convert]::ToUInt64($resumeFirst.utcBeforeFileTime, 16)
$tickAdvanceMs = $resumeTick - $activeTick
$utcAdvanceMs = ($resumeUtc - $activeUtc) / 10000.0
$advanceDifferenceMs = [Math]::Round([Math]::Abs($utcAdvanceMs - $tickAdvanceMs), 3)
if ($tickAdvanceMs -le 0 -or $utcAdvanceMs -le 0) { throw "Non-positive cross-sleep advance" }
if ($activeLast.lastBootUpTimeRaw -ne $resumeFirst.lastBootUpTimeRaw) { throw "Boot tuple changed across sleep/resume" }
[PSCustomObject]@{
    TickAdvanceMs = $tickAdvanceMs
    UtcAdvanceMs = [Math]::Round($utcAdvanceMs, 3)
    TickUtcDifferenceMs = $advanceDifferenceMs
    BootTimeUnchanged = $true
    ResultBefore = $activeLast.result
    ResultAfter = $resumeFirst.result
} | Format-List
```

After the cross-sleep comparison succeeds, the operator records the current
batch path and manually restarts Windows. The helper never initiates restart.
After restart, resolve the latest sleep/resume batch, capture five new samples,
and compare the candidate BootId diagnostics:

```powershell
git pull
$preRebootBatch = Get-ChildItem $env:TEMP -Directory -Filter "displaydeck-d08-sleep-resume-*" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if ([string]::IsNullOrWhiteSpace($preRebootBatch)) { throw "Pre-reboot D08 batch not found" }

$label = "restart"
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

$preCapture = Join-Path $preRebootBatch "sample-05.json"
$postCapture = Join-Path $batch "sample-01.json"
$pre = Get-Content -Raw $preCapture | ConvertFrom-Json
$post = Get-Content -Raw $postCapture | ConvertFrom-Json
$preBootId = (py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py --boot-id-only $preCapture).Trim()
if ($LASTEXITCODE -ne 0) { throw "Pre-reboot BootId diagnostic failed" }
$postBootId = (py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py --boot-id-only $postCapture).Trim()
if ($LASTEXITCODE -ne 0) { throw "Post-reboot BootId diagnostic failed" }
if ($pre.lastBootUpTimeRaw -eq $post.lastBootUpTimeRaw) { throw "BootTime did not change across restart" }
if ($preBootId -eq $postBootId) { throw "BootIdV1 did not change across restart" }
if ($pre.versionRaw -ne $post.versionRaw -or $pre.buildNumberRaw -ne $post.buildNumberRaw) { throw "Windows build changed; use a separate evidence cell" }
[Int64]$preTick = [Convert]::ToUInt64($pre.tickAfterMs, 16)
[Int64]$postTick = [Convert]::ToUInt64($post.tickBeforeMs, 16)
[PSCustomObject]@{
    BootTimeChanged = $true
    BootIdChanged = $true
    Version = $post.versionRaw
    Build = $post.buildNumberRaw
    TickResetObserved = ($postTick -lt $preTick)
    PreBootId = $preBootId
    PostBootId = $postBootId
    ResultBefore = $pre.result
    ResultAfter = $post.result
} | Format-List
```

Run the five-sample aggregation block against the restart `$batch`. If an OS
update changes Version/Build, stop and retain it as a separate evidence cell.
The printed BootId is diagnostic only and does not grant boot authority.

The current Windows 10 evidence cell next admits an operator-controlled
hibernate/resume observation. Capture five pre-hibernate samples first. The
helper never starts hibernate. If Hibernate is not available in the Windows UI,
stop; do not enable it with `powercfg` in this lane.

```powershell
git pull
$label = "hibernate-pre"
$preHibernateBatch = Join-Path $env:TEMP ("displaydeck-d08-{0}-{1}" -f $label, (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $preHibernateBatch -ErrorAction Stop | Out-Null
1..5 | ForEach-Object {
    $capture = Join-Path $preHibernateBatch ("sample-{0:d2}.json" -f $_)
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\displaydeck-evidence\capture_d08_readonly.ps1 -OutputPath $capture
    if ($LASTEXITCODE -ne 0) { throw "D08 capture failed: $LASTEXITCODE" }
    py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py $capture
    if ($LASTEXITCODE -ne 0) { throw "D08 validation failed: $LASTEXITCODE" }
    Start-Sleep -Milliseconds 500
}
Write-Output "pre-hibernate batch: $preHibernateBatch"

# The operator now manually selects Hibernate in the Windows UI.
```

After resume, open PowerShell at the DisplayDeck repository root. This block
also works from a new session:

```powershell
$preHibernateBatch = Get-ChildItem $env:TEMP -Directory -Filter "displaydeck-d08-hibernate-pre-*" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if ([string]::IsNullOrWhiteSpace($preHibernateBatch)) { throw "Pre-hibernate D08 batch not found" }

$label = "hibernate-resume"
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
Write-Output "pre-hibernate batch: $preHibernateBatch"
Write-Output "hibernate/resume batch: $batch"
```

Compare the cross-hibernate interval without inventing a tolerance:

```powershell
$preCapture = Join-Path $preHibernateBatch "sample-05.json"
$postCapture = Join-Path $batch "sample-01.json"
$pre = Get-Content -Raw $preCapture | ConvertFrom-Json
$post = Get-Content -Raw $postCapture | ConvertFrom-Json
$preBootId = (py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py --boot-id-only $preCapture).Trim()
if ($LASTEXITCODE -ne 0) { throw "Pre-hibernate BootId diagnostic failed" }
$postBootId = (py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py --boot-id-only $postCapture).Trim()
if ($LASTEXITCODE -ne 0) { throw "Post-hibernate BootId diagnostic failed" }
[Int64]$preTick = [Convert]::ToUInt64($pre.tickAfterMs, 16)
[Int64]$postTick = [Convert]::ToUInt64($post.tickBeforeMs, 16)
[Int64]$preUtc = [Convert]::ToUInt64($pre.utcAfterFileTime, 16)
[Int64]$postUtc = [Convert]::ToUInt64($post.utcBeforeFileTime, 16)
$tickAdvanceMs = $postTick - $preTick
$utcAdvanceMs = ($postUtc - $preUtc) / 10000.0
$comparison = [PSCustomObject]@{
    TickAdvanceMs = $tickAdvanceMs
    UtcAdvanceMs = [Math]::Round($utcAdvanceMs, 3)
    TickUtcDifferenceMs = [Math]::Round([Math]::Abs($utcAdvanceMs - $tickAdvanceMs), 3)
    BootTimeUnchanged = ($pre.lastBootUpTimeRaw -eq $post.lastBootUpTimeRaw)
    BootIdUnchanged = ($preBootId -eq $postBootId)
    VersionBuildUnchanged = ($pre.versionRaw -eq $post.versionRaw -and $pre.buildNumberRaw -eq $post.buildNumberRaw)
    ResultBefore = $pre.result
    ResultAfter = $post.result
}
$comparison | Format-List
if ($tickAdvanceMs -le 0 -or $utcAdvanceMs -le 0) { throw "Non-positive cross-hibernate advance" }
if (-not $comparison.VersionBuildUnchanged) { throw "Windows build changed; use a separate evidence cell" }
if (-not $comparison.BootTimeUnchanged -or -not $comparison.BootIdUnchanged) { throw "Hibernate changed boot identity; cell not qualified" }
if ($pre.result -ne "ACCEPTANCE_NOT_AUTHORIZED" -or $post.result -ne "ACCEPTANCE_NOT_AUTHORIZED") { throw "Unexpected D08 result" }
```

Run the existing five-sample aggregation block against both
`$batch = $preHibernateBatch` and the resume `$batch`. Keep the raw captures
outside Git. Fast Startup remains a separate cell.

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
