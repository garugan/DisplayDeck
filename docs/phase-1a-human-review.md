# DisplayDeck Phase 1A Human Review Packet

作成日: 2026-08-08  
対象: Phase 1A Technical Preflight の人間レビュー  
文書種別: 技術レビュー資料。承認記録・実行許可ではない  
Human approval: `PENDING`  
Phase 1A execution: `NOT EXECUTABLE`

## 1. 結論

**最終判定: `NEEDS_TECHNICAL_REVISION`**

Rust 1.97.1、`x86_64-pc-windows-msvc`、minimal profile、`windows` 0.62.2、および初回候補で残る7 Cargo featureには、version変更を要求する技術的理由は見つからなかった。一方、allowlist、forbidden policy、static audit plan、evidence/redaction planには、人間が承認可否を選ぶ前に直すべき技術事項がある。

| 項目 | 技術判定 | Human decision |
| --- | --- | --- |
| Rust 1.97.1 / stable / x64 MSVC / minimal | `RECOMMENDED` | `PENDING` |
| `windows = 0.62.2` | `RECOMMENDED` | `PENDING` |
| Cargo feature 7件 | `RECOMMENDED`。初回Required候補でも7件すべて残る | `PENDING` |
| 37-row allowlist | 22 CORE / 5 SUPPORTING / 7 OPTIONAL_EXPLORATORY / 3 REMOVE_FROM_FIRST_CELL | 全row `PENDING` |
| Forbidden policy | `MISSING`。限定的な追記が必要 | `PENDING` |
| Static audit plan | `CHANGE_REQUIRED` | `PENDING` |
| Evidence plan | `CHANGE_RECOMMENDED`。初回セルに対して一部過剰 | `PENDING` |
| Redaction policy | `CHANGE_RECOMMENDED`。35 field中4 field | `PENDING` |

### 1.1 人間判断前の技術修正

1. **SID validity rowがない。** Microsoftは、`GetLengthSid`へ渡すSIDを事前に`IsValidSid`で検証するよう要求し、invalid SIDに対する`GetLengthSid`のreturnはundefinedとしている。現行`P1A-SEC-010`の「zero/error terminal」は公式contractと一致しない。`TokenGroups`と`TokenIntegrityLevel`のSIDを読むなら、`IsValidSid`のexact rowを追加し、`SEC-010/011/012/013`のpreconditionにする必要がある。[Microsoft: GetLengthSid](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getlengthsid)
2. **Dependency audit commandが最小feature検証と矛盾する。** `cargo tree --locked --all-features`は、承認した7 featureだけでなく全featureを有効にする。approved buildと同じexact feature/default-feature指定で`cargo tree -e features --locked`相当を採取し、`--all-features`は使うとしても非承認surfaceを調べる参考監査へ分離する。
3. **PE import判定にprovenance ruleが足りない。** sourceで直接呼んでいない`GetLastError`、stdout用`WriteFile`等がgenerated wrapper/Rust runtime由来で現れる可能性を、即allowlist違反にも黙認にもせず、artifact・symbol・dependencyへ帰属させる規則が必要である。display mutation importは帰属にかかわらずblockする。
4. **Forbidden policyのbypass表現が不足する。** manual `extern "system"`/`#[link]`、`windows-sys`等の代替binding、`GetModuleHandle*`、`LdrGetProcedureAddress`、build/link scriptによるunreviewed importを明示的に扱う必要がある。
5. **Evidence/redactionを縮小allowlistへ合わせる必要がある。** registry/raw/database/preferred/advanced-colorを初回必須結果として扱わず、PIDとraw absolute timeを共有bundleの必須値にしない。

このreviewは上記変更を承認していない。Execution Recordへ反映後、改めて人間がrow/policy/versionを判断する。

## 2. レビュー対象と根拠

正本はworking tree上の`docs/phase-1a-execution-record.md`である。レビュー時raw-byte SHA-256は`bccf3368640a2e8c16d06e96be23cb8ea3107b2a2a965692bc6ecf45df9a39ec`。このrevisionはHEADに固定されていない。

最低限、次を照合した。

- `docs/phase-1a-execution-record.md`
- `docs/windows-display-research.md`
- `docs/requirements.md`
- `docs/architecture.md`
- `docs/security.md`
- `docs/testing-strategy.md`
- `docs/implementation-plan.md`
- `docs/tauri-design-closure-review.md`

主な外部根拠:

- Rust 1.97.1はRust Release Teamのstable point releaseである。[Rust 1.97.1](https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/)
- `x86_64-pc-windows-msvc`はTier 1 with host toolsである。[rustc platform support](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)
- minimal profileは`rustc`、`rust-std`、Cargoを含む。`rustfmt`とClippyはdefault profileには含まれるがminimalには含まれず、個別追加できる。[rustup profiles](https://rust-lang.github.io/rustup/concepts/profiles.html)
- `windows` 0.62.2はMSRV `1.82`を宣言し、defaultは`std`である。Rust 1.97.1とのversion条件上の矛盾はない。[windows 0.62.2 Cargo.toml](https://docs.rs/crate/windows/0.62.2/source/Cargo.toml.orig)
- GDI/CCDのflag、buffer、return、console-session errorはMicrosoft公式contractを基礎にした。[EnumDisplayDevicesW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw)、[EnumDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsexw)、[GetDisplayConfigBufferSizes](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdisplayconfigbuffersizes)、[QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)、[DisplayConfigGetDeviceInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-displayconfiggetdeviceinfo)
- `windows` 0.62.2 generated shapeはraw pointer/typed flag/`WIN32_ERROR`/`Result`の混在で、small `unsafe` wrapperと明示return変換が必要だが採用阻害ではない。[generated EnumDisplayDevicesW](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/Gdi/fn.EnumDisplayDevicesW.html)、[generated QueryDisplayConfig](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Devices/Display/fn.QueryDisplayConfig.html)、[generated GetTokenInformation](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/fn.GetTokenInformation.html)

## 3. 分類基準と件数

| 分類 | 判定基準 | 件数 |
| --- | --- | ---: |
| `CORE` | 初回セルのdisplay列挙、current/available mode、GDI/CCD identity、active topology、minimum privilege/session profile、bounded timingに必須 | 22 |
| `SUPPORTING` | 必須ではないがmachine/privilege/logon/adapter identityのcross-checkにかなり有用 | 5 |
| `OPTIONAL_EXPLORATORY` | Phase 1A全体には価値があるが、最初の最小セルへ同時に入れる必要はない | 7 |
| `REMOVE_FROM_FIRST_CELL` | Required rowで同じ目的を満たせるか、direct callとして不要 | 3 |
| 合計 |  | 37 |

## 4. API allowlist 37-row review

全rowのHuman decisionは意図的に`PENDING`のままとする。

### 4.1 GDI rows

| Row ID | API | 現在の目的 | 分類 | 初回セルに必要な理由 | 削除した場合に失うevidence | 安全性への影響 | complexity | 推奨 | Human decision |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| P1A-GDI-001 | `EnumDisplayDevicesW(NULL, i, 0)` | adapter列挙 | `CORE` | display起点とGDI `DeviceName`を得る | adapter/primary/desktop state | target列挙不能 | Low | Required | `PENDING` |
| P1A-GDI-002 | `EnumDisplayDevicesW(adapter, i, 0)` | monitor列挙 | `CORE` | adapter配下monitorをboundedに列挙する | monitor flags/name/IDs | GDI monitor relationが欠落 | Low | Required | `PENDING` |
| P1A-GDI-003 | `EnumDisplayDevicesW(..., EDD_GET_DEVICE_INTERFACE_NAME)` | monitor interface identity | `CORE` | MicrosoftがGDI monitorからSetupAPI monitorへのlinkとして定義し、CCD target pathとのexact mapping試験に必要 | interface-path hashとexact/unmapped判定 | label推測を避けるidentity evidenceが弱くなる | Medium | Required。first cellで重点確認 | `PENDING` |
| P1A-GDI-004 | `EnumDisplaySettingsExW(ENUM_CURRENT_SETTINGS)` | current mode | `CORE` | initial objectiveのC0/current mode | current DEVMODE tuple | currentと候補の比較不能 | Medium | Required | `PENDING` |
| P1A-GDI-005 | `EnumDisplaySettingsExW(ENUM_REGISTRY_SETTINGS)` | registry/persisted mode | `OPTIONAL_EXPLORATORY` | 初回最小目的には不要。P0研究には有用 | C0/P0差 | read-onlyなので削除による安全低下なし。future recovery研究が遅れる | Medium | Deferred to second read-only cell | `PENDING` |
| P1A-GDI-006 | `EnumDisplaySettingsExW(index, 0)` | compatible available modes | `CORE` | initial objectiveのavailable modes | normal mode tuple/duplicate/current-not-listed | candidate研究不能 | High | Required | `PENDING` |
| P1A-GDI-007 | `EnumDisplaySettingsExW(index, EDS_RAWMODE)` | raw driver modes | `OPTIONAL_EXPLORATORY` | normal compatible listで初回目的を満たせる | raw-vs-normal、DLDSR-like診断 | raw modeをproduct候補へ誤用する面を削減 | High | Deferred | `PENDING` |

`EDD_GET_DEVICE_INTERFACE_NAME`はCOREだが、返値の文字列一致を成功と仮定してはいけない。exact match、unmapped、ambiguousを観測するためのrowである。[Microsoft: EDD flag](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw)

### 4.2 CCD rows

| Row ID | API | 現在の目的 | 分類 | 初回セルに必要な理由 | 削除した場合に失うevidence | 安全性への影響 | complexity | 推奨 | Human decision |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| P1A-CCD-001 | `GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS \| QDC_VIRTUAL_MODE_AWARE)` | active query sizing | `CORE` | CCD active topology取得の必須pair | counts/race/cap | unbounded allocation回避とtopology取得を失う | Medium | Required | `PENDING` |
| P1A-CCD-002 | `QueryDisplayConfig(active flags)` | active topology/modes | `CORE` | active path/source/target/rational refreshの正本候補 | active topology、source/target modes | GDIだけの曖昧なmappingになる | High | Required | `PENDING` |
| P1A-CCD-003 | `GetDisplayConfigBufferSizes(QDC_DATABASE_CURRENT \| QDC_VIRTUAL_MODE_AWARE)` | database query sizing | `OPTIONAL_EXPLORATORY` | active topologyだけで初回目的を満たす | persisted database counts | initial safety低下なし | Medium | Deferred with CCD-004 | `PENDING` |
| P1A-CCD-004 | `QueryDisplayConfig(database flags)` | database topology | `OPTIONAL_EXPLORATORY` | persistence研究用で初回active identityには不要 | database topology/topology ID | incomplete database recordの複雑性を初回から除ける | High | Deferred | `PENDING` |
| P1A-CCD-005 | `DisplayConfigGetDeviceInfo(GET_SOURCE_NAME)` | CCD sourceのGDI名 | `CORE` | returned `viewGdiDeviceName`はEnumDisplaySettingsへ渡せるdocumented bridge | exact source↔GDI name | GDI/CCD source mappingが弱くなる | Medium | Required | `PENDING` |
| P1A-CCD-006 | `DisplayConfigGetDeviceInfo(GET_TARGET_NAME)` | target monitor name/path | `CORE` | target identity、monitor path、connector evidenceに必要 | target path/friendly name/output facts | monitor label推測のrisk | Medium | Required | `PENDING` |
| P1A-CCD-007 | `DisplayConfigGetDeviceInfo(GET_TARGET_PREFERRED_MODE)` | preferred mode | `OPTIONAL_EXPLORATORY` | preferred/native boundaryは有用だがcurrent/available/mappingに必須でない | preferred tuple/boundary | initial read-only safety低下なし | Medium | Deferred | `PENDING` |
| P1A-CCD-008 | `DisplayConfigGetDeviceInfo(GET_ADAPTER_NAME)` | adapter path | `SUPPORTING` | LUID-to-adapter pathの追加identity cross-check | adapter path hash | source/target bridgeで最小mapping可能。削除時はadapter診断力低下 | Low | Optional | `PENDING` |
| P1A-CCD-009 | `DisplayConfigGetDeviceInfo(GET_ADVANCED_COLOR_INFO)` | advanced color | `OPTIONAL_EXPLORATORY` | mutationしない初回セルでは必須でない | HDR/WCG/encoding/bpc | future non-regression baselineを失うが初回安全性は不変 | Medium/OS-sensitive | Deferred to approved HDR cell | `PENDING` |

`QDC_DATABASE_CURRENT`はpersistence database上のactive pathsを返し、database entryがなければmode detailが完全でない場合がある。active queryと同じ初回必須証跡として扱わない。[Microsoft: QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)

### 4.3 Security rows

| Row ID | API | 現在の目的 | 分類 | 初回セルに必要な理由 | 削除した場合に失うevidence | 安全性への影響 | complexity | 推奨 | Human decision |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| P1A-SEC-001 | `GetCurrentProcess` | current process pseudo-handle | `CORE` | token queryをforeign processへ広げない | current-process provenance | wrong-process token risk | Low | Required | `PENDING` |
| P1A-SEC-002 | `OpenProcessToken(TOKEN_QUERY)` | current process token | `CORE` | minimum privilege profileの入口 | token-open/access result | profile取得不能 | Medium | Required | `PENDING` |
| P1A-SEC-003 | `GetTokenInformation(TokenUser)` | user SID | `SUPPORTING` | bundle-local same-user identityに有用だがprivilege判定には不要 | user SID scoped hash | first-cell privilege safetyは不変 | Medium | Optional | `PENDING` |
| P1A-SEC-004 | `GetTokenInformation(TokenGroups)` | Administrators membership/deny-only | `CORE` | allowed execution profileを区別する | admin membership/attributes | elevated=falseだけではadmin-member non-elevatedを識別不能 | High | Required。`IsValidSid`追加条件付き | `PENDING` |
| P1A-SEC-005 | `GetTokenInformation(TokenElevation)` | elevated boolean | `CORE` | elevated runを拒否するminimum profile | elevation boolean | unauthorized elevated cellを見逃す | Low | Required | `PENDING` |
| P1A-SEC-006 | `GetTokenInformation(TokenElevationType)` | UAC elevation type | `SUPPORTING` | Default/Full/Limitedのcross-checkに有用 | elevation type | `SEC-004/005/008`でminimum profileは作れる | Low | Optional。global UAC policyを推測しない | `PENDING` |
| P1A-SEC-007 | `GetTokenInformation(TokenSessionId)` | token session ID | `CORE` | current token sessionとphysical consoleを1 rowで比較できる | token session | local-console判定が弱くなる | Low | Required | `PENDING` |
| P1A-SEC-008 | `GetTokenInformation(TokenIntegrityLevel)` | integrity SID | `CORE` | unknown/high-integrityをprofile化する | integrity level | privilege profile incomplete | High | Required。`IsValidSid`追加条件付き | `PENDING` |
| P1A-SEC-009 | `GetTokenInformation(TokenStatistics)` | authentication LUID | `SUPPORTING` | same-logon equalityに有用だがfirst-cell mutation fenceではない | logon LUID hash | initial display evidenceは不変 | Medium | Optional | `PENDING` |
| P1A-SEC-010 | `GetLengthSid` | SID長の取得 | `CORE` | SID hash/属性読取りのbuffer boundに必要 | SID length evidence | **現contractのままではunsafe**。invalid SID returnはundefined | High | Required only after `IsValidSid` row and contract correction | `PENDING` |
| P1A-SEC-011 | `IsWellKnownSid` | Administrators SID比較 | `CORE` | group listからadmin resultを得る | well-known match | profile分類不能 | Medium | Required after validity check | `PENDING` |
| P1A-SEC-012 | `GetSidSubAuthorityCount` | integrity SID count | `CORE` | final integrity RID indexをboundする | subauthority count | invalid index/pointer risk | High | Required after validity/length check | `PENDING` |
| P1A-SEC-013 | `GetSidSubAuthority` | integrity RID | `CORE` | normalized integrity levelを得る | integrity RID/label | profile分類不能 | High | Required after validity/count check | `PENDING` |
| P1A-SEC-014 | `CloseHandle` | token handle close | `CORE` | `SEC-002`のowned handleを必ず閉じる | cleanup result | handle leak/double-close risk | Medium | Required with RAII provenance | `PENDING` |

Microsoftはspecific group membershipには`CheckTokenMembership`も案内しているが、このrecordはAdministrators SIDの存在に加えenabled/deny-only attributesを証跡化するため`TokenGroups`を使う設計である。その選択は妥当だが、全SID pointerにvalidity、containing-buffer、count arithmeticの検証が必要である。[GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation)、[TOKEN_INFORMATION_CLASS](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ne-winnt-token_information_class)

### 4.4 Session/System rows

| Row ID | API | 現在の目的 | 分類 | 初回セルに必要な理由 | 削除した場合に失うevidence | 安全性への影響 | complexity | 推奨 | Human decision |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| P1A-SES-001 | `GetCurrentProcessId` | current PID | `REMOVE_FROM_FIRST_CELL` | `TokenSessionId`でcurrent token sessionを取得できる | PID | PID共有を避けられ、目的損失なし | Low | Remove | `PENDING` |
| P1A-SES-002 | `ProcessIdToSessionId` | process session ID | `REMOVE_FROM_FIRST_CELL` | `SEC-007`と重複し、追加2-row chainになる | independent process↔session cross-check | minimum local-console判定はSEC-007/SES-003で可能 | Low | Remove | `PENDING` |
| P1A-SES-003 | `WTSGetActiveConsoleSessionId` | physical-console session | `CORE` | token sessionとphysical consoleを比較する | console session/NO_CONSOLE | RDP/non-consoleを見逃す | Low | Required | `PENDING` |
| P1A-SYS-001 | `GetNativeSystemInfo` | native CPU architecture | `SUPPORTING` | approved x64 cellのruntime cross-checkに有用 | runtime native arch/level | machine manifest/manual toolchainでも確認可能 | Low | Optional | `PENDING` |
| P1A-SYS-002 | `GetTickCount64` | monotonic elapsed/timing | `CORE` | per-call/family timeboxのsame-boot elapsedを直接測る | elapsed/latency budget | bounded evidenceとhang傾向の判定が弱くなる | Low | Required。公開bundleはraw tickよりelapsedを優先 | `PENDING` |
| P1A-SYS-003 | `GetSystemTimePreciseAsFileTime` | precise UTC | `OPTIONAL_EXPLORATORY` | external capture timestampで初回相関は可能 | sub-ms UTC pairing | live timeboxには使えず安全性不変 | Low | Deferred | `PENDING` |
| P1A-SYS-004 | direct `GetLastError` | extended error | `REMOVE_FROM_FIRST_CELL` | 0.62.2の多くのBOOL APIsはgenerated `Result`が直後にerrorを取得し、CCDはcodeを直接返す。GDI enumeration contractはendとfailureをGLEで区別しない | direct GLE sequence | stale error misuse面を減らす | Medium | Remove direct row。generated-wrapper importはprovenance audit | `PENDING` |

`WTSGetActiveConsoleSessionId`はRemote Desktop Servicesが動いていなくてもphysical console session IDを返し、no-consoleは`0xFFFFFFFF`である。[Microsoft: WTSGetActiveConsoleSessionId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-wtsgetactiveconsolesessionid) `GetTickCount64`はsystem startからのmsを返し、wall-clock調整の影響を受けない。[Microsoft: GetTickCount64](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64)

## 5. `PHASE1A-ALLOWLIST-V1-CANDIDATE`

Status: `PROPOSED`  
Human approval: `PENDING`  
Executable: **No**。`IsValidSid` row追加とpolicy修正が先。

### Required rows（22）

```text
P1A-GDI-001
P1A-GDI-002
P1A-GDI-003
P1A-GDI-004
P1A-GDI-006
P1A-CCD-001
P1A-CCD-002
P1A-CCD-005
P1A-CCD-006
P1A-SEC-001
P1A-SEC-002
P1A-SEC-004
P1A-SEC-005
P1A-SEC-007
P1A-SEC-008
P1A-SEC-010
P1A-SEC-011
P1A-SEC-012
P1A-SEC-013
P1A-SEC-014
P1A-SES-003
P1A-SYS-002
```

Required technical addition（37-row count外、仮IDであり未承認）:

```text
P1A-SEC-015-PROVISIONAL — IsValidSid
purpose: SEC-003/004/008から得たSIDをSEC-010/011/012/013より前に検証
status: TECHNICAL_REVISION_REQUIRED / PENDING_HUMAN_REVIEW
```

### Optional rows（5）

```text
P1A-CCD-008
P1A-SEC-003
P1A-SEC-006
P1A-SEC-009
P1A-SYS-001
```

### Deferred rows（10）

```text
P1A-GDI-005
P1A-GDI-007
P1A-CCD-003
P1A-CCD-004
P1A-CCD-007
P1A-CCD-009
P1A-SES-001
P1A-SES-002
P1A-SYS-003
P1A-SYS-004
```

Deferredのうち`SES-001/002`とdirect `SYS-004`はfirst cellからremove、残る7件は別のread-only exploratory cell候補である。既存`PHASE1A-ALLOWLIST-V1`はこの文書で変更していない。

## 6. Rust toolchain review

判定: **`RECOMMENDED`**  
Human approval: `PENDING`

| 項目 | 評価 | 推奨 |
| --- | --- | --- |
| stable 1.97.1 | official stable point release。nightly-only機能は不要 | 採用候補維持 |
| nightly | binding、FFI、JSON、static auditに不要 | 禁止/除外維持。必要判明時はreviewへ戻す |
| `x86_64-pc-windows-msvc` | first x64 Windows cellに適切なTier 1 host target | 維持。target machine CPUを人間入力でfreeze |
| minimal profile | compiler/Cargoだけの最小installとして適切 | 維持 |
| `rustfmt` | code review差分とformat再現性に有用 | `SHOULD`、`cargo fmt --check`。Phase execution authorization後のみ追加/実行 |
| Clippy | unsafe周辺の一般lintに有用だがsecurity proofではない | `SHOULD`、exact command/lint policyをfreeze。false positive waiverを記録 |
| Phase 1A evidence上の必須性 | rustc/Cargo/MSVC/SDKとbuild commandはMUST。rustfmt/Clippyは高価でなくreview qualityを上げる | static auditの`SHOULD`。未実行だけでWin32 semanticsを証明済みとはしない |

minimal profileに`rustfmt`/Clippyを個別追加してもprofile選択自体をdefaultへ広げる必要はない。現時点ではinstall/add/runを行わない。

## 7. `windows` crate review

判定: **`RECOMMENDED`**  
Human approval: `PENDING`

- 0.62.2のdeclared MSRVは1.82であり、Rust 1.97.1はversion条件を満たす。
- 37 rowが使う既存20 functionのmodule surfaceは0.62.2に存在する。追加が必要な`IsValidSid`も`Win32::Security` surfaceであり、新featureは不要。
- GDI callsはraw `BOOL`、CCDは`WIN32_ERROR`または`i32`、token queryは`Result<()>`などshapeが異なる。各wrapperはreturn semanticsをAPIごとに固定し、汎用success converterを作らない。
- `DISPLAYCONFIG_*` union/index、`DEVMODEW`、token variable buffer、SID pointerはsmall `unsafe` boundaryを要するが、重大なgenerated-shape blockerではない。
- 0.62.2からversionを下げる根拠はない。採用理由は「最新だから」ではなく、必要surface、declared MSRV、target docs、exact pin/review可能性である。
- Execution Record 8.1は9 functionしか個別列挙していないため、「37 rowに必要なbinding確認」の記録としては不完全。20 existing functions + proposed `IsValidSid`のmodule/signature tableへ改訂すべきである。
- `default-features`は未決定。std binaryであるPhase 1Aではdefault `std`を維持する候補が単純だが、exact valueをauthorization recordへfreezeする。

## 8. Cargo feature review

判定: **`RECOMMENDED`（7/7維持）**  
Human approval: `PENDING`

| Feature | Required API / row | Required/Optional | 外せるか | 外すと失うrow | dangerous sibling exposure | allowlist抑制 |
| --- | --- | --- | --- | --- | --- | --- |
| `Win32_Devices_Display` | CCD-001/002/005/006 | Required | No | CCD core全体 | `SetDisplayConfig`, setters | exact callsite + flag + PE import auditで抑制 |
| `Win32_Foundation` | HANDLE/BOOL/error/LUID、SEC-014 | Required | No | SEC-014と共通types | broad handle/error surface | handle provenance、direct-call listで抑制 |
| `Win32_Graphics_Gdi` | GDI-001/002/003/004/006、CCD generated types | Required | No | GDI core | `ChangeDisplaySettings*`, graphics mutation | exact mutation name/flag/import blockが必須 |
| `Win32_Security` | SEC core + proposed `IsValidSid` | Required | No | privilege profile | setters、ACL、impersonation等 | query-class/access-mask allowlistで抑制 |
| `Win32_System_RemoteDesktop` | SES-003 | Required | No | physical-console comparison | WTS logoff/disconnect/control | one getterのみdirect/import allow |
| `Win32_System_SystemInformation` | SYS-002 | Required | No | elapsed/timebox | system/time setters | exact getter allowlist |
| `Win32_System_Threading` | SEC-001 | Required | No | token query provenance | process/thread/job/named object | spawn/termination/object/direct import block |

Featureはnamespaceを生成する単位でありsecurity boundaryではない。dangerous siblingが見えることは即違反ではないが、source allowlistだけでも十分ではない。source/AST、resolved feature graph、build script、PE importの組合せが必要である。

## 9. `PHASE1A-FORBIDDEN-V1` review

判定: **`MISSING`**  
Human decision: `PENDING`

| 観点 | 判定 | コメント |
| --- | --- | --- |
| display mutation / mutation flags | `APPROPRIATE` | GDI/CCD/DDC/HDR/scale/profileを広くblock |
| registry write | `APPROPRIATE` | direct/crate wrapperを含む |
| filesystem persistence | `APPROPRIATE` | stdout例外と`WriteFile` attribution方針あり |
| process spawn/termination/job | `APPROPRIATE` | watchdog/workerもblock |
| named objects/DACL/locks | `APPROPRIATE` | Phase 2A境界と一致 |
| PowerShell/cmd/script host | `APPROPRIATE` | shell executeもblock |
| Tauri/UI/watchdog/worker | `APPROPRIATE` | Phase 1A isolationと一致 |
| elevation/token mutation/impersonation | `APPROPRIATE` | broad categoryはある |
| dynamic loading | `MISSING` | `GetModuleHandle*`、`LdrGetProcedureAddress`、delay/manual resolutionを追加 |
| binding bypass | `MISSING` | manual `extern "system"`/`#[link]`、`windows-sys`/libloading/alternate raw bindingを追加 |
| build/link-time injection | `MISSING` | unapproved `build.rs`、`.cargo/config` rustflags/link-arg/native lib、proc-macro codegenを明示review |
| generated/runtime import handling | `TOO_BROAD` risk | 現文言は全binary importをdirect source authorizationと同一視し得る。provenance exceptionをsilent waiverではなくversioned rule化 |

推奨追記は既存禁止範囲を弱めない。display mutation、registry/file write、spawn/kill、named object、shell、Tauri、elevation/token mutationはそのままMUSTである。

## 10. `PHASE1A-STATIC-AUDIT-V1` review

判定: **`CHANGE_REQUIRED`**  
Human decision: `PENDING`

| Audit | 分類 | 推奨pass condition |
| --- | --- | --- |
| Win32 callsites | `MUST` | AST + manual inventory = approved rows。provisional SID row未解決ならfail |
| Forbidden API names | `MUST` | source/config/build/generated-use pathを区別し、unauthorized callable reference 0 |
| Mutation flags | `MUST` | source call constructionで`CDS_*`/`SDC_*` 0、rejected QDC/EDS flag 0 |
| `unsafe` blocks | `MUST` | file/line/row/invariant/owner/test一覧100% |
| FFI boundaries | `MUST` | pointer/handle/unionがsafe domain/evidence層へescapeしない |
| Windows imports/source uses | `MUST` | approved function/type/constant importとmanual FFI 0 |
| Cargo features | `MUST` | exact 7 + exact `default-features`。resolved graphをapproved build commandと同条件で照合 |
| Filesystem writes | `MUST` | source direct write/persist 0。stdout pathだけdataflow証明 |
| Registry writes | `MUST` | API/access mask/crate/string scanで0 |
| Process spawn | `MUST` | Rust/Win32/shell/process crate 0 |
| Process termination | `MUST` | terminate/job/APC/remote-thread 0 |
| Named objects | `MUST` | create/open/wait/global/local object path 0 |
| Tauri/React/WebView | `MUST` | dependency/source/config 0 |
| PowerShell/cmd | `MUST` | source/config/script/artifact string 0 |
| Dynamic loading | `MUST` | LoadLibrary/GetProcAddress/GetModuleHandle/Ldr*/libloading/manual function pointer 0 |
| Raw GetProcAddress path | `MUST` | Dynamic loading auditの明示sub-checkとして0 |
| Inline/global assembly | `MUST` | source/build/rustflags 0 |
| `build.rs` | `MUST` | root/transitive build scripts inventory。rootはabsent、transitiveはhash/purpose/link output review |
| Dependency tree | `MUST` | `--locked` + exact approved features。`--all-features`をpass commandにしない |
| Source hashes | `MUST` | approved commitのsource/manifest/config per-file + aggregate |
| Cargo.lock hash | `MUST` | raw hash、source/checksum、unexpected package 0 |
| Binary import table | `MUST` | execution対象PEそのもの。mutation import 0、他importはtoolchain/dependency provenanceとsource reachabilityを説明 |
| Binary/source reproducibility | `OPTIONAL` | initial first cellでrepeat artifact byte equalityは不要。command/toolchain/hash再現性はMUST |
| `cargo fmt --check` | `SHOULD` | approved rustfmt version、diff 0 |
| Clippy | `SHOULD` | exact target/features/lint policy、unreviewed warning 0 |
| License/advisory snapshot | `SHOULD` | exact dependency graphに対する記録。advisory absenceだけでsafeとしない |

**source scanとbinary import reviewは両方MUST。** source scanはdynamic/build/config bypassやdead codeも見つける。PE importはgenerated wrapper、linker、transitive/runtime dependencyを確認する。ただしPE importだけでは呼出dataflowやdynamic resolutionを証明できず、source scanだけでは最終artifactを証明できない。

## 11. `PHASE1A-EVIDENCE-V1` review

判定: **`CHANGE_RECOMMENDED`**  
Human decision: `PENDING`

| Evidence field/group | 分類 | 初回セル向け判断 |
| --- | --- | --- |
| Evidence ID、policy/version、approval ID | `MUST_CAPTURE` | runとauthorization bindingの正本 |
| Machine alias、Windows edition/version/build、KB coverage、ESU | `MUST_CAPTURE` | exact cell qualification。collection methodを記録 |
| CPU architecture | `MUST_CAPTURE` | x64 target binding。SYS-001を使わない場合はtrusted manual/toolchain evidence |
| GPU model/identity、driver exact version/date/provider | `MUST_CAPTURE` | hardware cell |
| Display model/firmware、connection/port、dock/adapter | `MUST_CAPTURE` | hardware cell。serialは不要 |
| local/RDP、console session、multi-user/FUS | `MUST_CAPTURE` | fail-closed environment profile |
| admin membership、elevated、integrity、effective profile | `MUST_CAPTURE` | minimum execution privilege profile |
| UAC global policy | `DEFER` | TokenElevationTypeからglobal policyを推測しない。token-derived factだけなら`USEFUL` |
| current resolution、rational refresh、orientation、bpp | `MUST_CAPTURE` | current modeとCCD mapping |
| scale | `USEFUL` | source/methodを明示。今回のallowlistにscale API rowはない |
| HDR/advanced color/bits per channel | `DEFER` | CCD-009をapproved optional cellへ移す。manual environment noteは`USEFUL` |
| branch/full HEAD/worktree state | `MUST_CAPTURE` | source authorization binding |
| Design Baseline aggregate SHA | `MUST_CAPTURE` | design binding |
| Design Baseline全14 per-file hashのrun bundle複製 | `USEFUL` | immutable recordへの参照 + aggregateで足りる。毎run必須にしない |
| Phase 1A source/manifest/config hashes | `MUST_CAPTURE` | source baseline |
| Cargo.lock hash/source/checksum/resolved graph | `MUST_CAPTURE` | dependency baseline |
| rustup/rustc/Cargo、host/target/components | `MUST_CAPTURE` | toolchain identity |
| MSVC/linker/Windows SDK exact version | `MUST_CAPTURE` | native build identity |
| exact Cargo feature/default-feature set | `MUST_CAPTURE` | generated surface identity |
| API row ID、sequence、flags/query class、input row IDs | `MUST_CAPTURE` | call-to-policy traceability |
| return/code/error、attempt/retry/count/cap decision | `MUST_CAPTURE` | result semanticsとboundedness |
| per-call elapsed/timebox result | `MUST_CAPTURE` | latency/stop rule |
| raw `GetTickCount64` absolute values | `USEFUL` | restricted bundleのみ。共有bundleはelapsedを優先 |
| start/end UTC | `USEFUL` | capture correlation。precise API rowをfirst-cell requiredにしない |
| exit code、stdout sanitized content/hash、truncation=false | `MUST_CAPTURE` | bounded source output |
| raw stderr content | `USEFUL` | hash/sizeはMUST、contentはredaction review後 |
| GDI adapters/monitors/current/normal modes | `MUST_CAPTURE` | initial objectives |
| GDI↔CCD source/target mapping、AMBIGUOUS/UNMAPPED | `MUST_CAPTURE` | primary spike result |
| CCD active topology/source/target/rational fields | `MUST_CAPTURE` | primary spike result |
| registry mode | `DEFER` | GDI-005 deferred |
| raw modes | `DEFER` | GDI-007 deferred |
| database topology/topology ID | `DEFER` | CCD-003/004 deferred |
| preferred mode | `DEFER` | CCD-007 deferred |
| adapter path hash | `USEFUL` | CCD-008 optional |
| user SID/logon LUID hashes | `USEFUL` | SEC-003/009 optional。raw values禁止 |
| PID | `DEFER` | SES-001/002 removal候補。`CURRENT_PROCESS` provenanceで代替 |
| all exclusions/cap/hotplug/access-denied/most-severe result | `MUST_CAPTURE` | failed/partial runも保持 |

Evidence planの8 MiB、enumeration/count/allocation capは適切であり維持する。過剰なのはvolume capではなく、deferred rowの結果を初回bundleの必須結果にしている点である。

## 12. `PHASE1A-REDACTION-V1` field review

判定: **31 `CURRENT_RULE_OK` / 4 `CHANGE_RECOMMENDED`**  
Human decision: `PENDING`

| Field | Current rule | 判定 | 推奨/理由 |
| --- | --- | --- | --- |
| User SID | `HASH_SHA256` | `CURRENT_RULE_OK` | bundle-local equalityを維持。raw禁止 |
| Username/domain | `DROP` | `CURRENT_RULE_OK` | 不要 |
| Computer name | `HASH_SHA256` | `CHANGE_RECOMMENDED` | Machine ID aliasでbundle内相関可能。原則`DROP`、必要時だけrestricted hash |
| Machine ID | `MASK` | `CURRENT_RULE_OK` | human-issued alias |
| Repository absolute path | `MASK` | `CURRENT_RULE_OK` | `$REPO` + relative path |
| Exact command | `MASK` | `CURRENT_RULE_OK` | path/user名を除きflags維持 |
| Windows device path | `HASH_SHA256` | `CURRENT_RULE_OK` | equalityを壊さない |
| GDI `DeviceName` | `HASH_SHA256` | `CURRENT_RULE_OK` | session cross-map key |
| GDI `DeviceString` | `KEEP` | `CHANGE_RECOMMENDED` | rationale内のfriendly-name reviewとclassificationが不一致。`HUMAN_REVIEW`へ変更しmodel-onlyならKEEP |
| GDI `DeviceID` | `HASH_SHA256` | `CURRENT_RULE_OK` | hardware fingerprint保護 |
| GDI `DeviceKey` | `HASH_SHA256` | `CURRENT_RULE_OK` | raw registry identity禁止 |
| Monitor friendly name | `HUMAN_REVIEW` | `CURRENT_RULE_OK` | custom/serial-like labelをdrop/mask |
| Monitor device path | `HASH_SHA256` | `CURRENT_RULE_OK` | GDI/CCD equality維持 |
| Monitor serial | `DROP` | `CURRENT_RULE_OK` | mappingに不要 |
| Adapter LUID | `HASH_SHA256` | `CURRENT_RULE_OK` | source/target equality維持 |
| Source ID | `KEEP` | `CURRENT_RULE_OK` | session-only、hashed LUIDとpair |
| Target ID | `KEEP` | `CURRENT_RULE_OK` | session-only、hashed LUIDとpair |
| EDID/raw descriptor | `HUMAN_REVIEW` | `CURRENT_RULE_OK` | default DROP |
| Logon/authentication LUID | `HASH_SHA256` | `CURRENT_RULE_OK` | same-logon equalityのみ |
| Process ID | `KEEP` | `CHANGE_RECOMMENDED` | first cellでexport不要。`DROP`しprovenanceを`CURRENT_PROCESS`に正規化 |
| Raw handle/pointer/address | `DROP` | `CURRENT_RULE_OK` | 不要/危険 |
| OS edition/version/build | `KEEP` | `CURRENT_RULE_OK` | qualification必須 |
| Installed KB IDs | `KEEP` | `CURRENT_RULE_OK` | coverageとcollection method付き |
| ESU status | `KEEP` | `CURRENT_RULE_OK` | human cell decisionに必要 |
| GPU model | `KEEP` | `CURRENT_RULE_OK` | support cell必須 |
| GPU driver version/provider/date | `KEEP` | `CURRENT_RULE_OK` | support cell必須 |
| Display model/firmware | `HUMAN_REVIEW` | `CURRENT_RULE_OK` | model/firmwareだけ保持、serial/custom削除 |
| Connection/physical port | `KEEP` | `CURRENT_RULE_OK` | `NOT EXPOSED`許容 |
| Resolution | `KEEP` | `CURRENT_RULE_OK` | primary result |
| Rational refresh/Hz | `KEEP` | `CURRENT_RULE_OK` | equalityを壊さない |
| Orientation/scaling/bpp/advanced-color | `KEEP` | `CURRENT_RULE_OK` | approved rowで得たtechnical valueのみ |
| API function/row/flags | `KEEP` | `CURRENT_RULE_OK` | audit必須 |
| API/Win32 error | `KEEP` | `CURRENT_RULE_OK` | troubleshooting必須 |
| UTC/tick/elapsed/exit | `KEEP` | `CHANGE_RECOMMENDED` | fieldを分割。elapsed/exit KEEP、UTCはrestricted/coarsening review、raw absolute tickは共有bundle DROP |
| Raw stdout/stderr | `HUMAN_REVIEW` | `CURRENT_RULE_OK` | serializer後も再scan |

Identity equalityに必要なGDI/CCD device path、LUID、SID/logon LUIDはbundle-local domain-separated hashを維持する。source/target IDはそれ単独ではsmall session indexであり、hashed adapter LUIDとのpairでのみ意味を持たせる。

## 13. 人間入力チェックリスト

**Primary checklist entries: 52。** API allowlist binding entryには37 existing rowの個別`APPROVE/REJECT`と、技術修正後の`IsValidSid` row判断が内包される。これらを一括承認やwildcardへ置換してはいけない。

| ID | 分類 | 人間が入力/確認する項目 | Completion condition |
| --- | --- | --- | --- |
| M-01 | MACHINE | Target Machine ID | evidence aliasをfreeze |
| M-02 | MACHINE | Windows edition | exact value |
| M-03 | MACHINE | Windows version | exact value |
| M-04 | MACHINE | OS build | exact value |
| M-05 | MACHINE | Installed KB list + collection evidence/coverage | 「complete」と誤称しない |
| M-06 | MACHINE | ESU status/evidence | current `NOT CONFIRMED`を確認または更新 |
| M-07 | MACHINE | non-ESU-confirmed cellをPhase 1Aに採用するdecision | explicit yes/no |
| M-08 | MACHINE | CPU architecture | exact x64確認 |
| M-09 | MACHINE | GPU exact model/ID | exact value |
| M-10 | MACHINE | GPU driver version/date/provider | exact value |
| M-11 | MACHINE | display exact model | serialを分離 |
| M-12 | MACHINE | display firmware | unavailableも明示 |
| M-13 | MACHINE | connection type | exact value |
| M-14 | MACHINE | physical port | exactまたは`NOT EXPOSED` |
| M-15 | MACHINE | dock/adapter/cable path | noneも明示 |
| M-16 | MACHINE | current resolution | execution直前観測 |
| M-17 | MACHINE | current rational refresh | numerator/denominator + label |
| M-18 | MACHINE | HDR/advanced-color state + method | unknownを許容 |
| M-19 | MACHINE | scale + method | unknownを許容 |
| M-20 | MACHINE | local console/RDP state | local予定をfreeze |
| M-21 | MACHINE | multi-user/FUS/second interactive state | none/unknownを明示 |
| P-01 | PRIVILEGE | Administrators group membership/deny-only | token-derived result |
| P-02 | PRIVILEGE | process elevated | boolean |
| P-03 | PRIVILEGE | integrity | RID + normalized label |
| P-04 | PRIVILEGE | UAC evidence | token-derived factsとmanual factを区別 |
| P-05 | PRIVILEGE | effective profile | allowed enum。elevated/unknownはstop |
| H-01 | PEOPLE | Operator | identified person/role |
| H-02 | PEOPLE | Evidence Owner | identified person/role |
| H-03 | PEOPLE | Reviewer | identified person/role |
| H-04 | PEOPLE | Approver | identified person/role |
| E-01 | EVIDENCE | Evidence ID | immutable scheme/value |
| E-02 | EVIDENCE | Evidence location | approved fixed location |
| E-03 | EVIDENCE | retention period | explicit duration/terminal rule |
| E-04 | EVIDENCE | access principals | exact people/groups |
| X-01 | EXECUTION | execution location | physical location/lab |
| X-02 | EXECUTION | planned execution date/timezone | exact date/timezone |
| A-01 | AUTHORIZATION | Approval ID | immutable ID |
| A-02 | AUTHORIZATION | Reviewer decision | explicit result |
| A-03 | AUTHORIZATION | Approver decision / authorization state | explicit result。record completionとexecution permissionを区別 |
| A-04 | AUTHORIZATION | approved Target Machine ID | M-01へexact bind |
| A-05 | AUTHORIZATION | approved repository/Execution Record commit SHA | final committed SHA |
| A-06 | AUTHORIZATION | Rust toolchain/target/profile/components decision | exact versions/components |
| A-07 | AUTHORIZATION | `windows` crate decision | exact 0.62.2/source |
| A-08 | AUTHORIZATION | Cargo feature/default-feature decision | exact set |
| A-09 | AUTHORIZATION | approved allowlist version + exact row IDs | 37個別decision + fixed SID row。wildcard禁止 |
| A-10 | AUTHORIZATION | forbidden policy version/hash | revised versionへbind |
| A-11 | AUTHORIZATION | static audit plan version/hash | revised versionへbind |
| A-12 | AUTHORIZATION | evidence plan version/hash | minimized versionへbind |
| A-13 | AUTHORIZATION | redaction policy version/hash | revised versionへbind |
| A-14 | AUTHORIZATION | authorized Phase 1A scope | source/build/audit/executionを個別明示 |
| A-15 | AUTHORIZATION | exclusions/stop conditions/never-return incident procedure | no process kill/spawn inside source |
| A-16 | AUTHORIZATION | signature / signed record reference | approver identityとtimestamp |

## 14. Repository record note

- Frozen repository baselineは`main` at `09be6a3e05651b9587d526c2d57e542823ec9297`、ID `P1A-REPO-09be6a3`であり、このreview時点でそのcommit自体は変わっていない。
- Design Baseline SHA V1は`d3764f3e0cafa9c7b0a89468e59b7452f627885483de90e90f68dedafabb015e`であり、対象14文書にworking-tree差分はない。
- `docs/phase-1a-execution-record.md`にはbaseline確認後の未commit変更があり、そのlatest working revisionはまだHEADへ固定されていない。本review自身も新規working-tree artifactである。
- Human inputs、技術修正、全row/policy decision、phase-specific authorizationを完成した後、**Phase 1A sourceを1 byteでも作成する前**にExecution Recordをcommitし、immutable authorization record commit SHAを取得する。
- そのcommitによりDesign Baseline SHA V1を再計算する必要はない。ただしDesign Baseline対象14文書が1 byteでも変わった場合は別である。
- Phase 1A source/Cargo manifestを承認後に作成し、static auditを通した後、実行前にsource commit SHA、per-file/aggregate hashes、`Cargo.lock` raw hash、resolved dependency graph、Cargo feature/default-feature、toolchain/build command、PE hash/import tableを別のsource baselineとしてfreezeする。
- Execution Record commitとPhase 1A source baseline commitを同じauthorityとして扱わない。前者は何を作成・監査してよいか、後者は何を実行してよいかをbindする。

## 15. Reviewer/Approver decision sheet

この表は人間が後続recordへ転記するためのdecision sheetであり、本書上はすべて`PENDING`である。

| Decision | Technical recommendation | Human decision |
| --- | --- | --- |
| Rust 1.97.1 | `RECOMMENDED` | `PENDING` |
| x86_64 MSVC/minimal | `RECOMMENDED` | `PENDING` |
| rustfmt/Clippy | `SHOULD` before execution | `PENDING` |
| windows 0.62.2 | `RECOMMENDED` | `PENDING` |
| seven Cargo features | `RECOMMENDED` | `PENDING` |
| default `std` feature | enable candidate; exact decision required | `PENDING` |
| 22 Required existing rows | candidate only | `PENDING` |
| 5 Optional rows | reviewer choice per row | `PENDING` |
| 10 Deferred rows | defer/remove candidate | `PENDING` |
| proposed `IsValidSid` row | technical addition required | `PENDING` |
| Forbidden policy revision | required before approval | `PENDING` |
| Static audit revision | required before approval | `PENDING` |
| Evidence plan minimization | recommended before approval | `PENDING` |
| Redaction 4-field revision | recommended before approval | `PENDING` |
| Phase 1A authorization | cannot issue from this document | `PENDING` |

## 16. 完了報告

1. **API 37 rows分類件数:** CORE 22、SUPPORTING 5、OPTIONAL_EXPLORATORY 7、REMOVE_FROM_FIRST_CELL 3。
2. **Required候補row:** 22 existing rows。Section 5参照。加えて`IsValidSid`の新規exact rowが技術的に必要。
3. **Optional候補row:** 5 rows（CCD-008、SEC-003、SEC-006、SEC-009、SYS-001）。
4. **Deferred候補row:** 10 rows。うち7 exploratory、3 remove。
5. **Rust判定:** `RECOMMENDED`、Human `PENDING`。
6. **windows crate判定:** 0.62.2 `RECOMMENDED`、Human `PENDING`。
7. **Cargo features判定:** 7件維持を`RECOMMENDED`。default-featureはfreeze必要。
8. **Forbidden policy判定:** `MISSING`。binding/dynamic/build bypassとruntime import provenanceを追加。
9. **Static audit判定:** `CHANGE_REQUIRED`。exact feature graphとsource+PE二重監査へ修正。
10. **Evidence plan判定:** `CHANGE_RECOMMENDED`。initial core/resultへ縮小しdeferred fieldを分離。
11. **Redaction policy判定:** 31 OK、4 CHANGE_RECOMMENDED。
12. **人間入力項目数:** primary checklist 52。別途37 existing API rowの個別decisionと追加SID row decision。
13. **技術修正が必要な箇所:** SID validity、allowlist binding inventory、forbidden bypass、dependency command、PE provenance、evidence/redaction alignment。
14. **Execution Record最終commit timing:** 人間入力・技術修正・approval scope完成後、Phase 1A source作成前。その後source/Cargo/lock/binaryを別freeze。
15. **最終判定:** `NEEDS_TECHNICAL_REVISION`。修正後は`READY_FOR_HUMAN_DECISIONS`へ再レビュー可能。

Stop point: `docs/phase-1a-human-review.md`作成まで。人間承認、Execution Record更新、commit、Phase 1A source/Cargo作成、build/test、Windows API実行、display setting変更は実施していない。
