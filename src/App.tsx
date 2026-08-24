import { useCallback, useEffect, useMemo, useState } from "react";
import { api, type ChangeStatus, type DisplaySnapshot } from "./services/tauriApi";

export default function App() {
  const [snapshot, setSnapshot] = useState<DisplaySnapshot | null>(null);
  const [status, setStatus] = useState<ChangeStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      setSnapshot(await api.getDisplaySnapshot());
    } catch (reason) {
      setError(String(reason));
    }
  }, []);

  useEffect(() => {
    void Promise.all([api.bootHandshake(), api.getDisplaySnapshot()])
      .then(([nextStatus, nextSnapshot]) => {
        setStatus(nextStatus);
        setSnapshot(nextSnapshot);
      })
      .catch((reason) => setError(String(reason)));
  }, []);

  useEffect(() => {
    if (!status || ["IDLE", "KEPT_SESSION", "REVERTED", "FAILED_CLOSED"].includes(status.state)) {
      return;
    }
    const timer = window.setInterval(() => {
      void api.getStatus().then(setStatus).catch((reason) => setError(String(reason)));
    }, 500);
    return () => window.clearInterval(timer);
  }, [status]);

  useEffect(() => {
    if (!status?.transactionId || status.state !== "AWAITING_DECISION" || status.presentationStage >= 2) {
      return;
    }
    let cancelled = false;
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        if (cancelled) return;
        void api
          .acknowledge(status.viewRevision, status.transactionId!, "REVERT_READY")
          .then((stage1) =>
            api.acknowledge(stage1.viewRevision, stage1.transactionId!, "CONFIRMATION_READY"),
          )
          .then(setStatus)
          .catch((reason) => setError(String(reason)));
      }),
    );
    return () => {
      cancelled = true;
    };
  }, [status]);

  const activeDisplays = useMemo(
    () => snapshot?.displays.filter((display) => display.attachedToDesktop) ?? [],
    [snapshot],
  );

  const act = async (operation: () => Promise<ChangeStatus>) => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await operation());
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  };

  return (
    <main>
      <header className="topbar">
        <div>
          <p className="eyebrow">DISPLAYDECK / GATE B PREPARATION</p>
          <h1>ディスプレイの状態</h1>
        </div>
        <span className="readonly">読み取り専用</span>
      </header>

      <section className="notice" aria-live="polite">
        <strong>Windowsの設定は変更しません。</strong>
        <span>Gate B承認済み。D07とexact cellの事前判定が揃うまでApplyは無効です。</span>
      </section>

      {error && <p className="error" role="alert">{error}</p>}

      <section className="actions" aria-label="操作">
        <button type="button" onClick={() => void refresh()} disabled={busy}>再読み込み</button>
        <button type="button" disabled title="D07 / exact cell判定待ち">Apply（事前判定待ち）</button>
        <button
          type="button"
          className="primary"
          disabled={busy || !status || !["IDLE", "KEPT_SESSION", "REVERTED", "FAILED_CLOSED"].includes(status.state)}
          onClick={() => status && void act(() => api.beginSimulation(status.viewRevision))}
        >
          15秒の安全動作をシミュレート
        </button>
      </section>

      {status && status.state !== "IDLE" && (
        <section className="transaction" aria-live="polite">
          <div>
            <span className="label">Fake transaction</span>
            <strong>{status.message}</strong>
            {status.remainingMs !== null && <span>残り {Math.ceil(status.remainingMs / 1000)} 秒</span>}
          </div>
          {status.transactionId && status.state === "AWAITING_DECISION" && (
            <div className="decision-buttons">
              <button
                type="button"
                disabled={busy}
                onClick={() => void act(() => api.revert(status.viewRevision, status.transactionId!))}
              >戻す</button>
              <button
                type="button"
                className="primary"
                disabled={busy || status.presentationStage < 2 || (status.remainingMs ?? 0) <= 0}
                onClick={() => void act(() => api.confirm(status.viewRevision, status.transactionId!))}
              >この状態を維持</button>
            </div>
          )}
        </section>
      )}

      <section className="summary">
        <div><span>取得状態</span><strong>{snapshot?.captureStatus ?? "取得中"}</strong></div>
        <div><span>接続中</span><strong>{activeDisplays.length} 台</strong></div>
        <div><span>変更可否</span><strong>不可</strong></div>
      </section>

      <section aria-labelledby="display-heading">
        <div className="section-heading">
          <h2 id="display-heading">検出したディスプレイ</h2>
          <span>{snapshot?.displays.length ?? 0} adapters</span>
        </div>
        <div className="display-grid">
          {snapshot?.displays.map((display) => (
            <article className="display-card" key={display.adapterIndex}>
              <div className="display-title">
                <div>
                  <span>{display.deviceName}</span>
                  <h3>{display.friendlyName || "名称不明"}</h3>
                </div>
                <span className={display.attachedToDesktop ? "connected" : "detached"}>
                  {display.attachedToDesktop ? "接続中" : "未接続"}
                </span>
              </div>
              <dl>
                <div><dt>現在</dt><dd>{formatMode(display.currentMode)}</dd></div>
                <div><dt>照合</dt><dd>{display.currentMembership}</dd></div>
                <div><dt>候補</dt><dd>{display.candidates.length} records</dd></div>
              </dl>
              {display.candidates.length > 0 && (
                <div className="candidate-list">
                  {display.candidates.slice(0, 40).map((candidate) => (
                    <div key={candidate.enumerationIndex}>
                      <span>{candidate.widthPixels ?? "?"} × {candidate.heightPixels ?? "?"}</span>
                      <span>{candidate.refreshLabel}</span>
                    </div>
                  ))}
                  {display.candidates.length > 40 && <small>ほか {display.candidates.length - 40} records</small>}
                </div>
              )}
            </article>
          ))}
        </div>
      </section>

      {snapshot && snapshot.blockers.length > 0 && (
        <details>
          <summary>変更不能の理由（{snapshot.blockers.length}件）</summary>
          <ul>{snapshot.blockers.map((blocker) => <li key={blocker}>{blocker}</li>)}</ul>
        </details>
      )}
    </main>
  );
}

function formatMode(mode: DisplaySnapshot["displays"][number]["currentMode"]): string {
  if (!mode) return "取得できませんでした";
  return `${mode.widthPixels ?? "?"} × ${mode.heightPixels ?? "?"} / ${mode.refreshHz ?? "?"} Hz`;
}
