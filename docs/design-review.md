# DisplayDeck 設計レビュー

レビュー日: 2026-08-01  
レビュー種別: 実装担当者とは独立したシニアエンジニアレビュー  
対象: `AGENTS.md` および指定された `docs/` 配下 8 文書  
実施範囲: 設計レビューのみ。実装、既存設計文書の変更、package 導入、build/test、Windows display mutation は行っていない。

## 1. 結論

**最終判定: APPROVED WITH CONDITIONS**

Electron の権限分離、renderer を未信頼とする姿勢、opaque token の再解決、非永続の一時適用、helper 所有の deadline、read-after-write、初期リリースからの scale/multi-monitor/DLDSR mutation 除外は妥当である。採用候補の Win32 API も実在し、単一 active path の解像度・refresh rate 変更を Windows 実機で成立させる技術的見込みはある。

ただし、現状の設計のまま display mutation を実装してはならない。特に、Win32 呼び出し中に transaction helper 自身が停止するケース、crash-consistent journal、旧 helper と recovery helper の競合防止、current mode と persisted mode が異なる場合の補償順序、GDI の整数 Hz と CCD の有理数 Hz の照合規則が未確定である。これらは UI や通常の error handling では補えず、black screen からの復旧保証に直接影響する。

この判定は Phase 1 以降への無条件な実装承認ではない。Critical/High の修正を設計へ反映し、人間判断事項を記録した後に再レビューすることを条件とする。Windows mutation は、その後も専用実機上の限定 spike として別途承認される必要がある。

## 2. レビュー基準と一次資料の照合

本レビューでは、文書の記述を次のように扱った。

- **確認事実**: Microsoft、Electron、Node.js、各 tool/package の一次資料で確認できる契約。
- **設計評価**: 文書間の整合性、failure window、信頼境界から導いたレビュー判断。
- **要検証**: 公開 API の契約だけでは決まらず、Windows build、GPU、driver、display の実機 evidence が必要な事項。

主要な照合結果は次のとおりである。

- `ChangeDisplaySettingsExW` は flags `0` の動的変更、`CDS_TEST`、USER profile を更新する `CDS_UPDATEREGISTRY`、`DISP_CHANGE_RESTART` 等を文書化している。設計の基本方向は一次資料と整合する。ただし API success は可視性を保証せず、current/persisted の補償手順はアプリ側で定義する必要がある。[Microsoft: ChangeDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-changedisplaysettingsexw)
- `QueryDisplayConfig` は temporary mode と persistence database の内容が異なり得ること、hotplug race で `ERROR_INSUFFICIENT_BUFFER` が生じ得ること、remote session/current desktop access で `ERROR_ACCESS_DENIED` になり得ることを明記している。[Microsoft: QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)
- `SetDisplayConfig` は supplied path の active display を排他的に有効化するため、不完全な path array が他 display を無効化し得る。`SDC_APPLY | SDC_USE_DATABASE_CURRENT` は最後に保存された構成への復帰であって、適用直前の session-only current mode と同義ではない。[Microsoft: SetDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setdisplayconfig)
- Electron は context isolation、sandbox、IPC sender validation、navigation/new-window 制限、custom protocol、fuses を推奨しており、本設計の process boundary は概ね整合する。[Electron Security Checklist](https://www.electronjs.org/docs/latest/tutorial/security)
- Electron の custom scheme は `standard` 等の privilege を ready 前に明示登録しないと、relative URL や origin の前提が変わる。CSP bypass 等の不要 privilege を付けない具体化が必要である。[Electron protocol API](https://www.electronjs.org/docs/latest/api/protocol)
- Node.js の `spawn`/`execFile` は既定で shell を使わない。本設計の fixed absolute path、`shell: false`、ユーザー入力を command line へ連結しない方針は妥当である。[Node.js child_process](https://nodejs.org/api/child_process.html)
- Electron Forge の Vite plugin は現行公式文書でも experimental とされているため、比較 spike と version pin の判断は妥当である。[Electron Forge Vite Plugin](https://www.electronforge.io/config/plugins/vite)
- 調査文書で参照する `node-win-screen-resolution`、`win32-displayconfig`、`DisplayConfig` は実在する。ただし、いずれも本製品が必要とする独立 watchdog、transaction、厳密 rollback を一式提供する根拠はなく、中核依存にしない判断は妥当である。
- Windows 10 Home/Pro は 2025-10-14 に通常 support を終了している。2026 年時点での対応には ESU/LTSC/edition/build の明示が必要である。[Microsoft Windows 10 lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro)

## 3. 指摘事項

### DDR-001 — helper 自身の停止時に watchdog も失われる

- **ID**: DDR-001
- **重要度**: Critical
- **対象文書**: `docs/architecture.md`、`docs/implementation-plan.md`、`docs/testing-strategy.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: architecture 4.6、10.1、11.2、implementation-plan Phase 7、testing-strategy 9、risks T-03/Q-21
- **問題点**: native helper が Win32 mutation、readback、deadline、rollback のすべてを所有する一方、mutation API または driver call が helper 内で停止したときにも rollback を実行できる独立した実行主体が定義されていない。「apply fail-safe を作動」とあるが、同一 process/thread、別 thread、別 process のどれか、停止した call と同時に rollback API を呼んでよいか、誰が旧操作を fence するかが未定である。
- **問題になる具体的な状況**: `ChangeDisplaySettingsExW` が画面を切り替えた後に戻らない、または直後の CCD/GDI readback が driver 内で停止する。画面は black screen だが helper は `pending` を返せず、15 秒 deadline も開始または処理できない。Electron main も同時に終了していれば recovery helper も起動しない。
- **推奨修正**: mutation worker と watchdog/supervisor の failure domain を分ける設計を追加する。少なくとも、guard の起動確認、mutation 前 deadline、worker hang 判定、recovery actor、OS-wide fencing、旧 worker が遅れて復帰した場合の無効化、同時 Win32 call の禁止条件を state machine と sequence diagram で定義する。Phase 7 より前に、実機 spike で「call が返らない」fault を再現し、分離方式で復元できない場合は architecture を再設計する。
- **修正しなかった場合の影響**: helper-owned watchdog という最重要保証が helper の単一障害で消え、black screen が 15 秒を越えて継続する。製品の最優先要件を満たさず、復旧不能または手動 reboot が必要になる。

### DDR-002 — journal の crash consistency が設計されていない

- **ID**: DDR-002
- **重要度**: High
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/implementation-plan.md`
- **対象セクション**: architecture 4.6、10、11、security 8.2/11、implementation-plan Phase 7/8
- **問題点**: journal の schema、atomic replace、checksum は述べられているが、どの state を OS mutation より前に durable にするか、file と directory metadata をいつ flush するか、各 crash window を起動時にどう解釈するかがない。`PREPARED`、`APPLIED/PENDING`、`COMMITTING`、`REVERTING`、terminal state の write-ahead 規則が設計契約になっていない。
- **問題になる具体的な状況**: baseline を書いた直後、disk cache が永続化される前に mode が変わり process tree が kill される。次回起動時には journal がない、旧 state のまま、または checksum は正しいが意味的に途中の状態となる。逆に keep 後の registry 更新直後に crash し、journal が `PENDING` のため、ユーザーが維持した mode を無条件に戻す可能性もある。
- **推奨修正**: write-ahead protocol を明文化する。各 transition について「durable journal write/flush → OS operation → readback → durable state update」の順序、Windows 上の flush/atomic replace 方針、sequence number、schema version、起動時 decision table を定義する。fault injection は各 durable write と各 Win32 call の間すべてに置く。
- **修正しなかった場合の影響**: recovery が必要な transaction を見失うか、既に安全な状態を誤って変更する。再起動復旧の再現性がなくなり、test に通った経路以外で復元不能になる。

### DDR-003 — 全 process 終了・再起動後の復旧契約が不十分

- **ID**: DDR-003
- **重要度**: High
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: requirements 3.1/FR-304、architecture 11.2、implementation-plan Phase 8、risks T-04/Q-06
- **問題点**: 設計は「次回起動時に UI より先に復旧」とするが、アプリは常駐せず、自動起動もしない。全 process tree kill 後に temporary mode が残った場合、画面が見えないユーザーがアプリを再起動できる保証がない。OS reboot/sign-out が temporary mode を必ず解除するかも未検証である。
- **問題になる具体的な状況**: ユーザーまたは updater が Electron と helper をまとめて強制終了し、display は非表示 mode のまま残る。journal は存在するが、アプリが次に手動起動されるまで recovery は動かず、画面が見えないため起動操作もできない。
- **推奨修正**: 15 秒保証の境界を product requirement として決定する。全-process loss を保証対象に含めるなら、署名済みの最小 auto-start recovery component、service、scheduled mechanism 等を別 threat model と uninstall/update lifecycle 付きで設計する。含めないなら、temporary apply が reboot/sign-out/desktop switch で確実に persisted baseline へ戻る対象環境だけを実機 allowlist 化し、保証外条件と物理 recovery 手順を明記する。
- **修正しなかった場合の影響**: 「アプリが強制終了されても戻る」という利用者の自然な期待と実装保証がずれ、配布後に手動復旧できない black screen を残す。

### DDR-004 — current と persisted の補償順序が未定義

- **ID**: DDR-004
- **重要度**: High
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`
- **対象セクション**: requirements FR-210/211/301/302、architecture 10/11.1、windows-display-research 6
- **問題点**: current mode と registry/persisted mode を別々に保存する方針は正しいが、failure state ごとの復元対象と順序がない。`SDC_USE_DATABASE_CURRENT` は persisted 構成へ戻す fallback であり、適用直前の session-only current mode を復元するものではない。
- **問題になる具体的な状況**: アプリ起動前に別ツールが 144 Hz を一時適用し、registry は 60 Hz のままだったとする。DisplayDeck の trial が timeout しただけなら current=144 Hz、registry=60 Hz を保つべきである。commit failure 後に current だけ戻す、または database fallback を先に使うと、current を 60 Hz に変えたり、誤った mode を再起動後へ残したりする。
- **推奨修正**: state 別の補償表を追加する。最低限、(a) mutation 前失敗、(b) temporary apply 後/keep 前、(c) registry update 開始後、(d) registry 更新済み・current 不一致、(e) current 更新済み・registry 不一致を分ける。commit failure では persisted baseline の復元と current baseline の動的復元の順序を固定し、各 step 後に両方を readback する。database fallback は exact current が不要になる一般 fallback として扱わない。
- **修正しなかった場合の影響**: 一部だけ変更された状態で「復元済み」と誤判定し、ユーザーの以前の session-only mode または永続設定を破壊する。

### DDR-005 — crash 後に exact baseline を再構築できる journal 表現がない

- **ID**: DDR-005
- **重要度**: High
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/security.md`、`docs/windows-display-research.md`
- **対象セクション**: requirements FR-104/204/304、architecture 6.2/11.1、security 8.2、windows-display-research 6/12
- **問題点**: architecture は active current の完全 mode を保存する一方、security は古い raw buffer を再投入せず「最小 mode identity」を fresh candidate へ照合するとする。current mode が通常候補にない場合、crash 後の recovery helper が exact mode を再構築できる情報量と検証規則が定義されていない。
- **問題になる具体的な状況**: driver が現在使用中の mode を `EnumDisplaySettingsEx` の通常候補に返さない状態で trial を開始し、helper が適用後に crash する。journal に width/height/integer Hz 程度しかなければ、orientation、scanline、bit depth、target signal 等を含む元 tuple を一意に復元できない。fresh candidate membership を必須にすると元 mode 自体を拒否する。
- **推奨修正**: pointer や `dmDriverExtra` の生 buffer ではなく、再構築に必要な allowlisted `DEVMODE` fields、CCD identity/fingerprint、current/persisted の別 record、capture OS/protocol version を持つ versioned baseline schema を定義する。current-not-listed の復元許可条件、field validation、再列挙との一致条件、曖昧時の停止条件を明文化する。
- **修正しなかった場合の影響**: 通常時には戻せても、helper crash や app restart をまたぐと exact rollback が成立せず、重要な復旧経路だけが実装不能になる。

### DDR-006 — GDI candidate と CCD readback の同値判定が未定義

- **ID**: DDR-006
- **重要度**: High
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/testing-strategy.md`
- **対象セクション**: requirements FR-004/207、architecture 6.1/10、windows-display-research 3.2/4/12、testing-strategy 3.1/8.3
- **問題点**: GDI の candidate は整数 `dmDisplayFrequency`、CCD の current/target refresh は有理数である。文書は 59.94 と 60 を分離すると要求するが、どの evidence で GDI record を特定の CCD rational rate に結び付け、apply/readback の一致を判定するかがない。DRR では virtual と physical refresh も分かれ得る。
- **問題になる具体的な状況**: GDI が 60 Hz と返す candidate を適用し、CCD readback が 60000/1001 を返す。これを一致とするとユーザーが 60.000 を選んだのに 59.94 を受理し得る。不一致とすると正常な 59.94 mode を毎回 rollback する。119.88/120 や DRR でも同じ問題が起きる。
- **推奨修正**: Windows read-only spike の結果を前提に、candidate identity、表示 label、apply tuple、readback equivalence を別概念として定義する。曖昧な一対多 mapping では候補を統合せず、正確に選択・検証できない candidate は初期版の `canApply` から外す。必要なら DXGI 等は read-only 補助 evidence として比較し、API 名だけで採用しない。
- **修正しなかった場合の影響**: 誤った Hz の確定、正常変更の無限 rollback、driver/OS ごとに異なる success 判定が発生し、主要機能の信頼性を証明できない。

### DDR-007 — live helper と recovery helper の fencing がない

- **ID**: DDR-007
- **重要度**: High
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`
- **対象セクション**: architecture 4.5/7/11.2、security 5.2/11、testing-strategy 7.2/9.2
- **問題点**: application/helper 単位の mutex と main による recovery 起動は記載されているが、OS process をまたぐ ownership transfer と fencing token がない。main の single-instance lock は、既に存在する transaction helper、hang した helper、startup recovery helper の競合を防がない。
- **問題になる具体的な状況**: transaction helper が apply 後に応答停止し、main が recovery helper を起動する。recovery helper が baseline へ戻した直後、旧 helper が復帰して pending 処理または commit を続ける。両者が同じ journal を更新し、最終 mode が実行順に依存する。
- **推奨修正**: interactive user/session 単位の OS-wide named mutex または同等 lock、単調増加 epoch/lease、journal ownership、recovery takeover protocol を定義する。旧 owner は lease を失った後に Win32 mutation を行えないことを各 mutation 直前に検証する。lock 放棄、process death、stale journal、simultaneous startup を fault table に加える。
- **修正しなかった場合の影響**: rollback と commit が競合し、timeout 後に危険 mode が再適用または永続化される可能性がある。

### DDR-008 — custom protocol の security contract が具体化されていない

- **ID**: DDR-008
- **重要度**: Medium
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`
- **対象セクション**: architecture 4.4/5.3、security 3.2/3.3、testing-strategy 5.2/12
- **問題点**: `app://displaydeck` の exact origin を IPC authorization に使うが、scheme を `standard`/`secure` としていつ登録するか、不要 privilege を何にするか、CSP を response header としてどう強制するかが未定義である。Electron の custom scheme は privilege 設定によって origin/relative URL の挙動が変わる。
- **問題になる具体的な状況**: non-standard scheme のまま asset path や origin comparison が想定と異なり、開発用例外を追加して IPC sender check を緩める。または `bypassCSP` を付けたまま production package を作り、設計した CSP が効かない。
- **推奨修正**: ready 前の scheme registration、必要 privilege の exact allowlist、`bypassCSP=false`、response header CSP、host/path canonicalization、dev scheme との完全分離を security contract に追加する。packaged artifact で exact URL と CSP readback を検証する。
- **修正しなかった場合の影響**: IPC sender validation と CSP の前提が build tool や実装者判断で変わり、renderer compromise の防御が弱くなる。

### DDR-009 — helper protocol の framing と具体上限が未確定

- **ID**: DDR-009
- **重要度**: Medium
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/implementation-plan.md`
- **対象セクション**: architecture 4.5/5.3、security 6.2/7、implementation-plan Phase 1/5
- **問題点**: versioned/bounded JSON protocol の方針はあるが、framing、最大 message/total bytes、最大 nesting/count、stdout backpressure、partial write、timeout 後の child ownership が設計値になっていない。
- **問題になる具体的な状況**: helper bug が newline を出さない巨大 response を書き、main と helper が pipe backpressure で相互停止する。pending display transaction 中なら、状態通知と rollback supervision の両方へ影響する。
- **推奨修正**: protocol v1 の framing、numeric upper bounds、request/response sequence、stream close semantics、stdout/stderr drain、timeout 時の state transition を Phase 1 の成果物として固定する。transaction control channel と診断出力を分離することも検討する。
- **修正しなかった場合の影響**: native bug や malformed output が Electron main の停止へ波及し、rollback の監視経路を不安定にする。

### DDR-010 — confirmation UI を提示できない場合の即時 rollback が明記されていない

- **ID**: DDR-010
- **重要度**: Medium
- **対象文書**: `docs/architecture.md`、`docs/ui-design.md`
- **対象セクション**: architecture 10、ui-design 9
- **問題点**: helper は readback 後に deadline を開始し、その後 main が confirmation window を作る。window の作成、load、preload handshake、配置、表示のどこまでを「確認 UI が利用可能」とするか、その失敗を即時 revert へ結ぶ acknowledgment がない。
- **問題になる具体的な状況**: mode apply は成功したが Electron の confirmation renderer が crash loop し、dialog は一度も表示されない。最終的には 15 秒で戻るとしても、main は即時失敗を認識しているのに危険な trial を期限まで維持する。
- **推奨修正**: confirmation-ready handshake と短い表示準備 timeout を定義し、失敗、navigation、preload mismatch、window destroy、focus不能を helper への即時 revert trigger にする。deadline は延長しない。
- **修正しなかった場合の影響**: UI を確認できない状態を不要に最大 15 秒維持し、利用者の復旧体験と accessibility を悪化させる。

### DDR-011 — 将来の multi-monitor で IPC 無変更という主張は成立しない

- **ID**: DDR-011
- **重要度**: Medium
- **対象文書**: `docs/architecture.md`、`docs/requirements.md`
- **対象セクション**: architecture 1/6.2/13、requirements 6.3
- **問題点**: architecture は将来の multi-monitor 版でも renderer/IPC contract を変更しないとするが、現在の `ApplyIntent` は monitor token と mode token を 1 組しか持たない。一括変更、全 active path の before/after、複数 step の確認要約、partial compensation を表現できない。
- **問題になる具体的な状況**: 将来、internal panel を 60 Hz、external monitor を 144 Hz へ一括適用する要件が追加される。現在の API では 2 transaction に分かれ、途中 failure で topology 全体を原子的に補償できず、UI も一括差分を表示できない。
- **推奨修正**: 「初期 renderer を全面的に作り直さない」程度へ目標を修正し、将来は versioned batch intent または trusted side で生成する opaque plan token が必要と明記する。初期版に multi-monitor mutation surface を先行実装する必要はない。
- **修正しなかった場合の影響**: 将来拡張時に安全な transaction 単位を壊すか、互換性を守るため不適切な逐次 apply を採用する。

### DDR-012 — release hardware matrix と反復基準が有限に定義されていない

- **ID**: DDR-012
- **重要度**: Medium
- **対象文書**: `docs/testing-strategy.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: testing-strategy 8/9/10/15、risks Q-09/Q-10/Q-11
- **問題点**: test 観点は非常に充実しているが、最小 Windows build、driver branch、GPU/connection の必須組合せ、timeout/race/hotplug の反復回数、許容 failure 率、evidence 保存形式が未確定である。「全 matrix 合格」だけでは matrix を後から狭められる。
- **問題になる具体的な状況**: NVIDIA HDMI の一度の rollback 成功だけで release gate を通し、Intel internal panel の 50 回 stress で 1 回だけ helper race が出る。再実行で成功したため flaky と処理され、配布後に同じ timing failure が再発する。
- **推奨修正**: support policy と同時に有限の mandatory matrix、driver/build pin、反復数、zero-tolerance 項目、quarantine/waiver 権限、evidence template を決める。rollback-failed は 1 回でも原因解明または scope 除外まで release blocker とする既存原則を数値基準へ落とす。
- **修正しなかった場合の影響**: 充実した test list が実行不能な希望一覧になり、release 判定が担当者ごとに変わる。

### DDR-013 — scale は read-only row の方が disabled slider より明確

- **ID**: DDR-013
- **重要度**: Low
- **対象文書**: `docs/ui-design.md`、`docs/requirements.md`
- **対象セクション**: ui-design 2.1/6.4、requirements FR-106
- **問題点**: 初期版で変更不能な scale を slider と select で表示すると、操作可能な候補が存在するように見え、keyboard/screen reader 利用者にも冗長である。文書自身も read-only row との選択を未決定にしている。
- **問題になる具体的な状況**: 125% と表示された disabled slider に focus できず、ユーザーが「権限不足」「一時的故障」「初期版未対応」のどれか判断できない。
- **推奨修正**: 初期版は現在値と取得 status、未対応理由を持つ read-only row にする。将来 setter が承認された時点で candidate control を追加する。
- **修正しなかった場合の影響**: 機能上の危険は小さいが、未対応機能への誤期待と accessibility 上の混乱が残る。

### DDR-014 — requirement と test の機械的 traceability が不足

- **ID**: DDR-014
- **重要度**: Low
- **対象文書**: `docs/requirements.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`
- **対象セクション**: requirements 5/9、testing-strategy 全体、implementation-plan Phase 共通の完了定義
- **問題点**: 内容上の対応は多いが、FR/AC ごとの test level、owner、phase、evidence ID の対応表がない。
- **問題になる具体的な状況**: AC-009 の main 強制終了は test list に存在するが、どの packaged build、どの process-kill 方法、どの evidence が AC 完了を証明するか review 時に追跡できない。
- **推奨修正**: 設計修正後に FR/AC → architecture component → test case → phase gate → evidence の traceability matrix を追加する。
- **修正しなかった場合の影響**: 要件の抜けというより、release review で「どの test が何を証明したか」の監査コストが高くなる。

## 4. 人間の判断が必要な事項

### DDR-Q01 — keep 後の persistence

- **ID**: DDR-Q01
- **重要度**: Question
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: requirements FR-210/10、architecture 10、risks Q-07
- **問題点**: 「維持する」が current session の維持なのか、USER profile へ保存して reboot/sign-out を越えるのか未決定である。
- **問題になる具体的な状況**: ユーザーは「維持する」で永続化したと思うが、再起動後に元へ戻る。反対に、一時試用のつもりだった mode が registry に残る。
- **推奨修正**: 初期版は安全性を優先し、再起動を越える persistence を外して session-only keep とする案を第一候補にする。永続化を採るなら DDR-004 の compensation と reboot/sign-out matrix を実装前に承認する。
- **修正しなかった場合の影響**: transaction の完了条件と rollback baseline が確定せず、Phase 2 の state model から実装できない。

### DDR-Q02 — 15 秒保証の SLA

- **ID**: DDR-Q02
- **重要度**: Question
- **対象文書**: `docs/requirements.md`、`docs/testing-strategy.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: requirements FR-208/AC-008、testing-strategy 9.2、risks Q-06/Q-09
- **問題点**: 15 秒が「rollback 開始まで」か「baseline readback 完了まで」か、apply/readback 中の pre-deadline を何秒とするかが未決定である。
- **問題になる具体的な状況**: deadline ちょうどに rollback を開始しても driver が 20 秒かかり、ユーザーからは 35 秒 black screen に見える。
- **推奨修正**: product wording と測定点を固定する。推奨は「readback 成功から 15 秒で rollback 開始」を外部保証とし、mutation 開始から confirmation pending までにも独立上限を設ける。baseline 完了時間は実測 SLO として別に公開する。
- **修正しなかった場合の影響**: acceptance test とユーザー説明が一致せず、「15 秒で戻る」という誤認を招く。

### DDR-Q03 — Windows 10/11 と CPU architecture の support 範囲

- **ID**: DDR-Q03
- **重要度**: Question
- **対象文書**: `docs/requirements.md`、`docs/testing-strategy.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: requirements 6.3/7/10、testing-strategy 8.1、risks Q-10/Q-11
- **問題点**: Windows 10 の edition/ESU/LTSC/build、Windows 11 の最小 build、x64/arm64 が未決定である。
- **問題になる具体的な状況**: 通常 support 終了後の Windows 10 Home 22H2 で helper を配布し、security update または driver combination を再現できない。arm64 package に x64 helper を同梱して起動不能になる。
- **推奨修正**: 初期版は Windows 11 x64 を primary support とし、Windows 10 は具体的な ESU/LTSC edition/build と test capacity が承認された場合だけ追加する。arm64 は別 artifact/matrix が用意できるまで外す。
- **修正しなかった場合の影響**: test matrix と配布 artifact が固定できず、support 不能な環境へ危険な mutation 機能を提供する。

### DDR-Q04 — install path、署名、配布予算

- **ID**: DDR-Q04
- **重要度**: Question
- **対象文書**: `docs/security.md`、`docs/risks-and-open-questions.md`、`docs/implementation-plan.md`
- **対象セクション**: security 8/9/10、risks D-01〜D-09/Q-12/Q-23、implementation-plan Phase 1/10
- **問題点**: runtime は standard user/asInvoker とする一方、helper を user-writable でない directory に置く installer、全 PE 署名、timestamp、runtime verification の予算と配布方式が未決定である。
- **問題になる具体的な状況**: portable または per-user writable directory に置いた helper を同一 user process が置換し、hash 検証と spawn の間の TOCTOU を突く。あるいは unsigned helper が AV に隔離され、trial 中の recovery 起動だけ失敗する。
- **推奨修正**: 初期配布は portable を除外し、protected install directory、全 PE/DLL/installer の Authenticode、timestamp、upgrade/uninstall 時の pending transaction 処理を必須条件として予算承認する。installer の UAC と runtime elevation は分けて説明する。
- **修正しなかった場合の影響**: helper trust と recovery availability を保証できず、security review と release gateを完了できない。

### DDR-Q05 — confirmation の初期 focus と accessibility

- **ID**: DDR-Q05
- **重要度**: Question
- **対象文書**: `docs/ui-design.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: ui-design 4/9/11、risks U-07/U-09/Q-14
- **問題点**: 「元に戻す」を初期 focus にする安全案と、15 秒が支援技術利用者に十分かが承認されていない。
- **問題になる具体的な状況**: display flash 後にユーザーが反射的に Enter を押し、`維持する` が既定 focus なら見えない mode を確定する。screen reader の読み上げ中に期限が切れる可能性もある。
- **推奨修正**: 初期 focus は「元に戻す」、Enter は focus button のみ、keep に default action/shortcut を与えない方針を承認する。15 秒を維持する場合は即時 heading 読み上げと短い説明、keyboard path を実機 Narrator で検証する。
- **修正しなかった場合の影響**: 誤 keep と accessibility 上の操作不能を acceptance 上排除できない。

### DDR-Q06 — DLDSR-like candidate の表示方針

- **ID**: DDR-Q06
- **重要度**: Question
- **対象文書**: `docs/requirements.md`、`docs/windows-display-research.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: requirements 7/10、windows-display-research 8、risks Q-18
- **問題点**: driver が通常 GDI candidate として返す preferred mode 超の解像度を表示するか隠すか未決定で、公開 flag だけでは DLDSR と識別できない。
- **問題になる具体的な状況**: 4K display で 5K/8K 相当 candidate が列挙されるが、custom resolution か DLDSR かを識別できない。一般 mode として表示すると black-screen matrix が増え、隠すと正当な candidate を失う。
- **推奨修正**: 初期 spike で vendor/GPU/connection ごとの列挙結果を収集し、識別不能な preferred 超 candidate は初期版で apply 対象外にする安全側 policy を推奨する。DLDSR の名称や保証は付けない。
- **修正しなかった場合の影響**: 初期 scope が暗黙に vendor-specific mode まで拡大し、black-screen test matrix と support claim が制御不能になる。

## 5. 実装開始前に必須の修正

1. DDR-001 の helper hang/crash 時の独立 rollback actor と failure-domain 分離を設計する。
2. DDR-002 の durable write-ahead journal と全 crash window の recovery decision table を設計する。
3. DDR-003 の全-process loss/reboot 復旧保証の境界を決定し、必要な起動主体を設計する。
4. DDR-004 の current/persisted 別補償表と exact/fallback の順序を固定する。
5. DDR-005 の crash 後にも exact restore 可能な versioned baseline schema を定義する。
6. DDR-006 の GDI candidate ↔ CCD readback mapping/equality を read-only spike で確定し、曖昧候補を fail closed にする。
7. DDR-007 の OS process 横断 mutex、lease/epoch、recovery takeover を設計する。
8. DDR-Q01〜Q06 の owner、decision、date、受容残余 risk を decision log に記録する。
9. 修正後、requirements/architecture/security/testing/risks の相互整合を再レビューし、人間が実装開始を明示承認する。

## 6. 初期リリースから削るべき機能

既に設計から外している次の項目は、そのまま除外を維持すべきである。

- 画面拡大率の変更、custom scaling、未公開 DisplayConfig packet、registry 直接操作
- 2 台以上の active monitor の mutation、clone/extend/position/primary の変更
- DLDSR/DSR の固有識別・保証、vendor API、unsafe/raw mode
- HDR、bit depth、color space、orientation、VRR/DRR の変更
- remote session、service、CLI、常駐 profile switch、auto-update

さらに本レビューでは次を初期リリースから外すことを推奨する。

- **再起動を越える persistence**: DDR-Q01 を解決し compensation を実証するまで、keep は current session の維持に限定する。
- **Windows 10 の一般対応**: 具体的な ESU/LTSC edition/build と実機 matrix が承認されるまで Windows 11 x64 に限定する。
- **portable/per-user writable 配布**: protected install と署名/更新 lifecycle が確立するまで除外する。
- **preferred mode を超える識別不能 candidate の apply**: DLDSR-like mode policy が実機 evidence で決まるまで除外する。
- **disabled scale slider**: 機能ではなく read-only status row として表示する。

## 7. Windows 実機で先に行う技術検証

優先順は次のとおりである。

1. mutation なしで CCD active path、GDI device、monitor identity、current/registry `DEVMODE`、Settings UI の対応を確認する。
2. 59.94/60、119.88/120、DRR on/off で GDI record と CCD rational readback の対応を確定する。
3. current mode が candidate list にない場合と current != registry の baseline を採取し、再構築可能な schema を決める。
4. standard user、local console、RDP、multiple active path、virtual display、hotplug で fail-closed classification を確認する。
5. dedicated machine で flags `0` の temporary apply が USER registry を変えないことを確認する。
6. mutation 前 guard、apply/readback hang、parent kill、helper kill、旧 helper 復帰、recovery helper takeover を fault injection する。
7. timeout、explicit revert、commit failure で current/persisted の各 baseline が state 別手順どおり戻ることを確認する。
8. process-tree kill、sign-out、reboot、sleep/hibernate、driver reset 後の current/persisted/journal を確認する。
9. black-screen lab で UI を一切使わず、out-of-band 観測により rollback start と baseline readback を証明する。
10. signed packaged artifact で child lifetime、pipe EOF、Program Files ACL、AV/WDAC/AppLocker、upgrade/uninstall 中の pending transaction を確認する。

## 8. 推奨する最初の技術スパイク

最初の spike は build tool 比較ではなく、**Windows 11 x64・standard user・単一物理 monitor 上の read-only native helper** とする。

成果物は次の evidence に限定する。

- CCD source/target/path と GDI `DISPLAYn` の対応表
- current、registry、全 candidate の構造化 dump。ただし device path/EDID 等は外部共有時に redact
- Settings UI との resolution/Hz 比較
- 59.94/60 と source/target signal の対応
- current-not-listed、current != registry の再現結果
- hotplug/RDP/multiple path の safe classification
- C++/C#/Rust のうち Win32 fidelity、crash isolation、packaging、署名に最も適した最小選定 evidence

この read-only spike で DDR-005/006 の前提が成立しない場合、temporary mutation prototype へ進まず設計を更新する。その次の spike として、UI/Electron と切り離した supervisor + mutation worker の non-persistent apply/rollback を専用実機で検証する。

## 9. 設計の良い点

- renderer を未信頼とし、`nodeIntegration: false`、`contextIsolation: true`、sandbox、専用 preload、operation-specific IPC を一貫して要求している。
- renderer から raw width/height/Hz、device path、helper path、generic IPC を受けず、opaque/session-scoped token を trusted code で直前再解決する。
- main と confirmation window の capability を分け、keep を confirmation webContents と active transaction に bind している。
- shell/PowerShell を production path から外し、fixed packaged path と bounded structured protocol を採る判断は妥当である。
- `CDS_TEST` 成功や API success を可視性・適用成功と同一視せず、apply/commit/rollback の readback を要求している。
- temporary apply と persistence を分離し、renderer countdown を権威にしていない。
- scale mutation、multi-monitor mutation、DLDSR 固有対応を初期版から除外し、Windows の scale setter に過度な楽観がない。
- current と persisted baseline の差、monitor identity の不安定性、fractional refresh、hotplug、process loss を既知 risk として明示している。
- macOS mock と Windows platform adapter の port 分離により、UI 開発が実 display mutation へ依存しない。
- physical GPU/monitor、black-screen、cable/dock、process kill、Narrator、packaged artifact を CI と分けた test 戦略は現実的である。
- rollback failure を通常の apply error より重大に扱い、journal/evidence を保持する原則が明確である。

## 10. 件数と最終判定の要約

1. **Critical 件数**: 1 件
2. **High 件数**: 6 件
3. **実装開始前に必須の修正**: helper failure-domain 分離、crash-consistent journal、全-process/reboot recovery 契約、current/persisted 補償表、baseline schema、GDI/CCD 同値判定、cross-process fencing、人間判断事項の記録
4. **初期リリースから削るべき機能**: scale/multi-monitor/DLDSR mutation に加え、当面の reboot persistence、未承認 Windows 10、portable/per-user writable 配布、識別不能な preferred 超 candidate
5. **Windows 実機で先に作るべき技術検証**: read-only identity/mode mapping、fractional Hz、current/registry 差、hang/crash/fencing、process-tree kill/reboot、black-screen rollback
6. **推奨する最初の技術スパイク**: Windows 11 x64 standard-user 環境の read-only CCD/GDI native helper
7. **設計の良い点**: Electron 境界、opaque token、temporary apply、helper deadline、readback、初期 scope、mock 分離、物理 lab gate はいずれも強い
8. **最終判定**: **APPROVED WITH CONDITIONS**。基本アーキテクチャは実現可能で安全志向だが、Critical/High の未定義部分を実装者の裁量に残したまま mutation 実装へ進むことは承認できない。設計修正と限定 read-only spike の evidence を再レビューした後にのみ、non-persistent mutation spike を許可すべきである。
