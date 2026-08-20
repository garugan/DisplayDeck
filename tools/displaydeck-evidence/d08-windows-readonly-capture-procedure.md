# D08 Windows read-only capture procedure

The integrated freeze-evidence authorization permits this bounded read-only
capture lane. `capture_d08_readonly.ps1` records one Windows sample without
changing a display setting. It writes one new evidence file and refuses to
overwrite an existing file. It does not create same-boot acceptance authority.

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
