import { invoke } from "@tauri-apps/api/core";

export interface DisplayMode {
  widthPixels: number | null;
  heightPixels: number | null;
  refreshHz: number | null;
  orientation: number | null;
}

export interface Candidate {
  enumerationIndex: number;
  widthPixels: number | null;
  heightPixels: number | null;
  refreshLabel: string;
  eligibility: string;
}

export interface Display {
  adapterIndex: number;
  deviceName: string;
  friendlyName: string;
  attachedToDesktop: boolean;
  primary: boolean;
  currentMode: DisplayMode | null;
  currentMembership: string;
  candidates: Candidate[];
}

export interface DisplaySnapshot {
  schemaVersion: number;
  platform: string;
  captureStatus: string;
  mutationAllowed: false;
  blockers: string[];
  displays: Display[];
}

export type TransactionState =
  | "IDLE"
  | "STARTING"
  | "APPLY_IN_FLIGHT"
  | "AWAITING_DECISION"
  | "KEEP_AUTHORIZED"
  | "REVERT_IN_FLIGHT"
  | "KEPT_SESSION"
  | "REVERTED"
  | "FAILED_CLOSED";

export interface ChangeStatus {
  schemaVersion: number;
  viewRevision: string;
  mutationAllowed: false;
  simulationAllowed: true;
  transactionId: string | null;
  state: TransactionState;
  remainingMs: number | null;
  presentationStage: number;
  message: string;
}

export interface DiagnosticExport {
  schemaVersion: number;
  path: string;
  bytes: number;
}

interface StatusRequest {
  schemaVersion: 1;
  mode: "BOOT_HANDSHAKE" | "ORDINARY_RESYNC";
  frontendBootNonce: string;
}

const frontendBootNonce = randomHex(16);

export const api = {
  getDisplaySnapshot: () => invoke<DisplaySnapshot>("get_display_snapshot"),
  bootHandshake: () =>
    invoke<ChangeStatus>("get_display_change_status", {
      request: statusRequest("BOOT_HANDSHAKE"),
    }),
  getStatus: () =>
    invoke<ChangeStatus>("get_display_change_status", {
      request: statusRequest("ORDINARY_RESYNC"),
    }),
  beginSimulation: (viewRevision: string) =>
    invoke<ChangeStatus>("begin_display_change", {
      request: {
        schemaVersion: 1,
        viewRevision,
        simulation: true,
        durationMs: 15_000,
      },
    }),
  acknowledge: (
    viewRevision: string,
    transactionId: string,
    stage: "REVERT_READY" | "CONFIRMATION_READY",
  ) =>
    invoke<ChangeStatus>("ack_display_change_presentation", {
      request: { schemaVersion: 1, viewRevision, transactionId, stage },
    }),
  confirm: (viewRevision: string, transactionId: string) =>
    invoke<ChangeStatus>("confirm_display_change", {
      request: { schemaVersion: 1, viewRevision, transactionId },
    }),
  revert: (viewRevision: string, transactionId: string) =>
    invoke<ChangeStatus>("revert_display_change", {
      request: { schemaVersion: 1, viewRevision, transactionId },
    }),
  exportDiagnostics: () => invoke<DiagnosticExport>("export_diagnostics"),
};

function statusRequest(mode: StatusRequest["mode"]): StatusRequest {
  return { schemaVersion: 1, mode, frontendBootNonce };
}

function randomHex(length: number): string {
  const bytes = crypto.getRandomValues(new Uint8Array(length));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
