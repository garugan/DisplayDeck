# D08 Windows read-only capture procedure — pending

The integrated freeze-evidence authorization permits this bounded read-only
capture lane. It has not been executed because the current workspace is macOS;
the template therefore remains `PENDING`. This permission does not allow a
display-setting change, storage mutation, process/fault harness, or creation of
a same-boot acceptance authority.

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

The capture document is validated offline with
`validate_d08_readonly_capture.py`. Thresholds remain `UNSET` in every
document under this authorization. A completed Windows sample can only state
`REJECT_NO_AUTHORITY`, `REJECT_CROSS_CHECK`, or
`ACCEPTANCE_NOT_AUTHORIZED`; it never grants same-boot acceptance.
