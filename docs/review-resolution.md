# DisplayDeck 設計レビュー対応表

最終更新: 2026-08-01  
対象レビュー: `docs/design-review.md`  
状態: 設計修正完了。実装・Windows実機スパイク未承認、未実施。

## 1. 対応状況の意味

| 状況 | 意味 |
| --- | --- |
| 設計解消・検証待ち | 指摘された設計上の未定義部分を規範契約、停止条件、test/evidence gateへ反映した。実装または実機での成立はまだ証明していない |
| 解消 | 記述・scope・traceabilityの修正で指摘を解消した |
| 人間判断待ち | agentが確定してはならないproduct/support/budget/accessibility decision。safe baselineは提案に留める |

件数:

- Critical: 1件。設計解消・検証待ち 1件
- High: 6件。設計解消・検証待ち 6件
- Medium: 5件。設計解消 5件
- Low: 2件。解消 2件
- Question: 6件。人間判断待ち 6件

Critical/High の「設計解消」はrelease可能またはmutation実装可能という意味ではない。Phase 1/2の無変更spikeと、別承認されたPhase 7実機mutation spikeのclosure evidenceが揃うまでverified/closedへ進めない。

## 2. Critical / High

### DDR-001

- 指摘ID: DDR-001
- 重要度: Critical
- 対応状況: 設計解消・検証待ち
- 対応内容: 単一helperを、blocking display APIを直接呼ばないrollback supervisorとone-shot mutation/readback/recovery workerの別processへ分離した。各workerは`GO`待機でspawnし、PID/process creation time/image/role/epoch/nonceをintentへdurable化する。supervisor crash後もnew supervisorが同一processを再open・照合し、消滅を証明してからだけfresh workerを起動する。open/query/終了確認不能時は`RECOVERY_BLOCKED_BY_INFLIGHT_CALL`として並行Win32 operationを禁止する。
- 更新した文書: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: driver/kernel内callをprocess terminationでquiesceできない、またはtakeover側がprocessを再open/queryできない可能性。Phase 2/7でPID reuse/access denied/Job競合も再現し、exit未確認が一件でもあれば当該環境をrelease対象から外すかarchitectureを再設計する。

### DDR-002

- 指摘ID: DDR-002
- 重要度: High
- 対応状況: 設計解消・検証待ち
- 対応内容: pre-provisioned dual-slot write-ahead journalを規定した。state-required field mask、C0/P0、requested trial R、exactly one expected observation、target/topology、active-worker identity、operation/readback/decision evidenceをschema化した。recovery-blockedは`resumeFromState`と停止前intent/attempt/decision payloadを保持する。inactive slot全書込み、`FlushFileBuffers`、close/reopen検証、必要なintentだけone-use GO、exit確認、fresh readback、次state durable化の順序を固定した。PREPARED divergence、全intent、awaiting時C0/P0、persisted drift、terminal observation drift、blocked resumeを含むstartup decision tableを追加した。
- 更新した文書: `docs/requirements.md`、`docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: storage/firmware/AVによるdurability差、両slot破損。同時破損・未知schemaでは自動mutationせずfail closedとする。全write/call境界のfault evidenceが必要。

### DDR-003

- 指摘ID: DDR-003
- 重要度: High
- 対応状況: 設計解消・人間境界承認待ち
- 対応内容: renderer/main lossは生存supervisor、worker lossはsupervisor、supervisor lossはmutex解放後かつ旧worker quiescence証明後のmain recovery、全process lossはnext manual launch recoveryと、保証主体を分けた。全process loss、OS crash、power lossを15秒保証外とするsafe baselineを明記しつつ、境界を受容するsupport cellでもblind reboot/sign-out/物理手順からP0へ5回戻ることをrelease必須条件にした。15秒保証へ含める場合はauto-start/service/scheduled mechanismと別threat modelが必要である。
- 更新した文書: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 全process lossを保証対象に含めるかはproduct decisionであり、DDR-Q02としてagentが確定しない。
- 残っているリスク: supervisorを含む強制終了後、ユーザーが画面を見られず再起動操作もできない可能性。境界を受容しない場合、初期版mutationはrelease不可。

### DDR-004

- 指摘ID: DDR-004
- 重要度: High
- 対応状況: 設計解消・検証待ち
- 対応内容: `C0=current baseline`、`P0=persisted baseline`、`R=trial`を定義した。初期版はregistry不変のsession-only keepへscopeを削り、通常rollbackはC0をdynamic exact restoreしてC0/P0を別々にreadbackする。exact失敗時は同一targetのcaptured P0だけをdegraded recovery候補とし、通常成功にしない。将来persistenceではP0を先、C0を後に補償する順序を固定した。
- 更新した文書: `docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/ui-design.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: C0 exact restore不可時にsession-only currentを失う。P0 fallbackの実API副作用はPhase 7で要検証。persistenceはDDR-Q01と別設計承認まで初期範囲外。

### DDR-005

- 指摘ID: DDR-005
- 重要度: High
- 対応状況: 設計解消・Phase 1検証待ち
- 対応内容: current/persistedを別recordにし、field-presence mask、width/height、bits-per-pixel、frequency marker、display flags/scanline、position、orientation、fixed-outputと正規化CCD evidenceを持つversioned baseline schemaを定義した。pointer、raw buffer、`dmDriverExtra`を禁止した。CURRENT_NOT_LISTEDはtrial前の再構築・preflight・一意readbackが成立しない限りapply不可とした。
- 更新した文書: `docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: 一部driverではallowlist fieldだけでexact modeを再構築できない。その環境では機能を狭め、現在値表示だけにする。

### DDR-006

- 指摘ID: DDR-006
- 重要度: High
- 対応状況: 設計解消・Phase 1検証待ち
- 対応内容: `CandidateIdentity`、`DisplayLabel`、`ApplyTuple`、`ExpectedObservation`を別modelにし、`canApply=true`には完全なobservation tupleをexactly one要求した。eligibilityを`product-allowed`、mapping qualificationだけ欠ける`lab-unqualified`、target/apply/current-restore/preferred境界等がunsafeな`hard-excluded`へ分けた。Phase 1 read-onlyはcurrentと文書化APIからの導出規則だけを評価し、lab-unqualifiedだけを別承認Phase 7の専用tokenで観測する。exact support fingerprintへboundしたqualification evidenceまでproduct tokenを発行せず、hard-excludedはlabでも変更しない。CCD rationalは約分後exact比較し、epsilonや整数丸めを禁止した。
- 更新した文書: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: GDI整数HzからCCD rationalへ文書化read-only導出規則を作れず、qualification対象外では全candidateが適用不可になる可能性。安全側のread-only製品化を許容し、推測で解放しない。

### DDR-007

- 指摘ID: DDR-007
- 重要度: High
- 対応状況: 設計解消・検証待ち
- 対応内容: 対話user/session単位のOS-wide named mutex、durable epoch、128-bit owner nonce、operation sequence、one-use GO、durable active-worker identityを定義した。new supervisorはmutex解放/abandonment前にtakeoverせず、journal PIDをcreation time/imageで再同定し旧processの消滅を証明できるまでepochを進めない。PID reuse先を終了せず、stale/late resultを破棄し、one-shot workerに後続keep/reapply能力を持たせない。
- 更新した文書: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: named object namespace/ACL、same-user DoS、packaging Jobとの競合。Phase 2で確定する。外部Windows Settingsはfence外なので各operation前後のfresh readbackが必要。

## 3. Medium

### DDR-008

- 指摘ID: DDR-008
- 重要度: Medium
- 対応状況: 設計解消・packaged verification待ち
- 対応内容: production schemeを`app`、authorityを`displaydeck`、entry pathを`/index.html`と`/confirm.html`に固定した。ready前に`standard/secure`だけを登録し、`bypassCSP`等を無効化した。GET/HEAD、URL component、canonical asset manifest、asset別exact MIMEを検査し、CSPのexact directive set、nosniff/no-referrerをresponse headerで強制する。
- 更新した文書: `docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: 選定build toolが追加privilegeを要求する可能性。追加時はversioned design reviewを再度行う。

### DDR-009

- 指摘ID: DDR-009
- 重要度: Medium
- 対応状況: 設計解消・Phase 2/3検証待ち
- 対応内容: 4-byte little-endian length + UTF-8 JSON、request 16KiB、response 256KiB、depth/property/array/string/token、process-lifetime両方向frame/total byte、stderr、handshake/inspect/partial-frame/terminal-exit timeoutをprotocol v1の具体値として固定した。one-use GO後の正常stdin half-close、terminal+EOF、unexpected EOF、trailing bytes、exit statusを区別し、stderr 1MiB fault後もdiscard-drainする。stdout/controlとstderr/diagnosticを分離し並行drain、mutation前後で異なるfailure handlingを定義した。
- 更新した文書: `docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: 実候補数が上限を超える環境。truncateせず `LIMIT_EXCEEDED/canApply=false` とする。

### DDR-010

- 指摘ID: DDR-010
- 重要度: Medium
- 対応状況: 設計解消・E2E/実機検証待ち
- 対応内容: confirmation windowをmutation前にhidden prewarmする。apply readback後は`AWAITING_PRESENTATION`をdurable化し、supervisorのpending発行monotonic時刻を2秒timeoutの唯一の起点にする。第一段はRevert enabled/focused、Keep disabledでpresentation ackし、durable`AWAITING_CONFIRMATION` generation後だけKeepを有効化して両button enabled/Revert focus維持を第二段ackする。両段を同じ2秒枠かつ最終ack時残12秒以上で完了できなければ即時rollbackし、15秒deadlineを延長しない。
- 更新した文書: `docs/requirements.md`、`docs/architecture.md`、`docs/ui-design.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: ready ackは物理的な可視性を証明しない。最終保証はsupervisor deadlineである。

### DDR-011

- 指摘ID: DDR-011
- 重要度: Medium
- 対応状況: 解消
- 対応内容: 「将来multi-monitorでもIPC無変更」という主張を撤回した。V1は単一 `ApplyIntentV1` だけとし、monitorごとの反復を禁止した。将来は全active pathを一体化するversioned batch intentまたは `preparePlanV2/applyPlanV2` 相当のopaque plan contractと別reviewを要求する。
- 更新した文書: `docs/requirements.md`、`docs/architecture.md`、`docs/implementation-plan.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: 将来V2でAPI/UI migrationが必要。初期版に先行surfaceは作らない。

### DDR-012

- 指摘ID: DDR-012
- 重要度: Medium
- 対応状況: 設計解消・support値の人間決定待ち
- 対応内容: W11-01〜07を削除不能なcoverage classとし、実行前に各IDをexactly oneの物理構成へinstantiateするRC qualification manifest契約を定義した。manifestはexact OS/KB/driver/GPU/display/connection/C0/P0/requested/expected tuple/transition/fault IDを持ち、range、alternative、`latest`、主観的risk名、placeholderを禁止する。全transitionとfaultの反復数、full-process blind recovery、zero-tolerance、waiver不可、共同scope除外、evidence schemaも固定した。
- 更新した文書: `docs/requirements.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: exact Windows build/KBとWindows 10採否はDDR-Q03として人間判断に残す。
- 残っているリスク: DDR-Q03のexact機材/build値が未入力なのでRC manifestは現時点で`UNFROZEN / NOT EXECUTABLE`。matrix外hardwareをsupportできず、driver updateごとに再qualificationが必要。

## 4. Low

### DDR-013

- 指摘ID: DDR-013
- 重要度: Low
- 対応状況: 解消
- 対応内容: 初期版のscaleをdisabled slider/selectから、known/unknown/unsupported、現在値、未対応理由を持つ非操作read-only status rowへ変更した。draft/planned/apply intent、Tab order、write IPCにscaleを含めず、scale取得不能/変更未対応だけではresolution/Hz applyを無効化しない。
- 更新した文書: `docs/requirements.md`、`docs/ui-design.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: ユーザーがscale変更を期待する可能性。visible説明でscopeを明示する。

### DDR-014

- 指摘ID: DDR-014
- 重要度: Low
- 対応状況: 解消
- 対応内容: 全FR/ACを1 ID 1行でarchitecture component、stable test ID、test level、phase/gate、planned evidence ID、owner、statusへ結ぶtraceability matrixを追加した。evidence未生成を `planned` とし、失敗runを再実行で上書きしない規則を追加した。
- 更新した文書: `docs/testing-strategy.md`、`docs/implementation-plan.md`
- 対応しない場合の理由: 該当なし
- 残っているリスク: 実装時に要件IDとmatrixがdriftする可能性。将来の文書検査でmissing/duplicate/orphanをrelease failureにする。

## 5. Question

### DDR-Q01

- 指摘ID: DDR-Q01
- 重要度: Question
- 対応状況: 人間判断待ち
- 対応内容: safe baselineとして初期版をsession-only keep、persisted baseline不変、再起動後非保証へscope削減した。persistenceはpost-v1の別phaseへ移した。
- 更新した文書: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/ui-design.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: persistenceのproduct意味はProduct ownerのdecisionであり、agentが確定しない。
- 残っているリスク: 再起動後も保存されるというユーザー期待。owner承認前は初期scope確定・release不可。

### DDR-Q02

- 指摘ID: DDR-Q02
- 重要度: Question
- 対応状況: 人間判断待ち
- 対応内容: 推奨測定contractをpost-apply readback`t0`から15秒間Keep受付、deadlineでtimeout decision、durable `REVERT_DECIDED`と最初のrollback callはdeadline後250ms以内とした。これにより15秒の確認時間とcall開始処理時間の因果を分離した。operation guard 5秒はverified pending / quiesced rollback start / recovery-blockedの三分岐である。baseline完了は別SLO、full process lossは15秒保証外だがblind P0 recovery必須とした。
- 更新した文書: `docs/requirements.md`、`docs/architecture.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: 外部保証文言、jitter、all-process scopeはProduct/Safety ownerのdecisionである。
- 残っているリスク: ユーザーが15秒で復元完了すると解釈する可能性。決定なしではmutation release不可。

### DDR-Q03

- 指摘ID: DDR-Q03
- 重要度: Question
- 対応状況: 人間判断待ち
- 対応内容: safe baseline候補をWindows 11 x64とし、Windows 10/arm64/x86を初期範囲外へ移した。W11-MIN/CURRENTのexact build/KBと各W11-01〜07の実機値はRC manifest freeze前のdecisionとし、未入力中は`NOT EXECUTABLE`とした。
- 更新した文書: `docs/requirements.md`、`docs/windows-display-research.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: support期限、test capacity、artifact費用はProduct/Release ownerのdecisionである。
- 残っているリスク: 対象縮小、Windows 10需要、OS update後の再qualification。

### DDR-Q04

- 指摘ID: DDR-Q04
- 重要度: Question
- 対応状況: 人間判断待ち
- 対応内容: portable/per-user writable配布を初期版から外し、protected machine-wide install、全PE/DLL/installerのAuthenticode/timestamp、runtime hash + `WinVerifyTrust` publisher検証、runtime asInvoker、active/pending/degraded/failed/blocked/unknown journal中update/uninstall拒否をsafe baselineにした。offline/revocation policyは未決定として残した。
- 更新した文書: `docs/requirements.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: certificate/installer/signing serviceの予算と運用主体は人間判断である。
- 残っているリスク: 予算不成立、AV/SmartScreen、publisher rotation。決定なしではpublic release不可。

### DDR-Q05

- 指摘ID: DDR-Q05
- 重要度: Question
- 対応状況: 人間判断待ち
- 対応内容: safe baselineを「元に戻す」初期focus、Keep default/global shortcutなし、Enterはfocused buttonのみ、Esc/Alt+F4 revert、accessible-ready ack、Narrator gateとした。
- 更新した文書: `docs/requirements.md`、`docs/ui-design.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: focus policyと固定15秒のaccessibility受容はProduct/Accessibility/Safety ownerのdecisionである。
- 残っているリスク: screen readerの読み上げ中に期限が切れる可能性。Narrator evidenceで不成立ならDDR-Q02も再度開く。

### DDR-Q06

- 指摘ID: DDR-Q06
- 重要度: Question
- 対応状況: 人間判断待ち
- 対応内容: preferred/native境界超、分類不能、exactly one observation未証明、qualification fingerprint不一致のcandidateを初期版apply対象外とするsafe baselineを定義した。currentに該当する場合はcurrent card/recovery baselineとして扱えるが、slider/select entryや新規selection tokenをrenderer/IPCへ発行しない。UIへ追加のread-only診断表示を出すかはPhase 1後に判断する。
- 更新した文書: `docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/ui-design.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- 対応しない場合の理由: legitimate custom modeを隠すUXとsafetyのtradeoffは人間判断である。
- 残っているリスク: candidate除外による機能制限、classifierのvendor/driver依存。DLDSR名称・保証は付けない。

## 6. 変更後の初期リリース baseline

人間承認に提示するsafe baselineは次のとおりである。

- 人間承認後にexact RC manifestへfreezeされたWindows 11 x64 support profilesだけ（現時点はunfrozen）
- standard user、local console、single active path
- resolution/refreshのread-only inspectと、exact-one mappingかつsupport-fingerprint qualification済みcandidateだけのsession-only temporary apply
- rollback supervisor + one-shot worker、durable active-worker identity、dual-slot WAL、named mutex/epoch fencing
- 5秒operation guard（pending / quiesced rollback / blocked）、post-readback 15秒Keep受付 + deadline後250ms call-start proposal
- exact C0 restore、P0不変。検証済みP0へのdegraded recoveryはcritical扱い
- macOS deterministic mock
- scaleはread-only status row

初期範囲外:

- registry/USER profileへのpersistence
- Windows 10、arm64/x86、portable/per-user writable配布
- multi-monitor mutation、scale mutation、DLDSR-like/分類不能candidate apply
- service/auto-start recoveryによるfull-process-loss 15秒保証
- preset、OBS/game、tray、auto-update、HDR/VRR/DRR制御

## 7. 最初の技術スパイク

最初はWindows 11 x64・standard user・単一物理display上のread-only native spikeである。CCD/GDI identity、current/registry、全candidate、currentの59.94/60、DRR、current-not-listed、C0!=P0、preferred超candidate、RDP/multiple path/hotplugのclassificationを採取する。非current candidateはpost-apply observationを得たとみなさず、文書化read-only APIから完全tupleを一意導出できるかを調べる。C++/C#/RustのWin32 fidelityとprocess isolationも比較する。

このspikeでDDR-005/006の前提が成立しない場合、推測mappingを追加せず設計を更新する。次に、displayを変更しないsupervisor/worker/WAL/fencing fault prototypeでPID reuse、takeover、全startup branch、protocol close/budgetを検証する。非current candidateのpositive qualificationを含むWindows mutation spikeはその後の別承認である。

## 8. 実装開始判定

現時点では一般実装もWindows mutationも開始可能ではない。DDR-Q01〜Q06、全-process保証境界、Phase 1の対象機材を人間が判断し、修正文書を再承認した後に限りPhase 1 read-only実装を開始できる。Phase 1の承認はPhase 7 mutation、production signing、配布の承認を含まない。
