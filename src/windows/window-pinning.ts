import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export const BLOB_ALWAYS_ON_TOP_STORAGE_KEY = "openblob-blob-always-on-top";
export const BLOB_ALWAYS_ON_TOP_EVENT = "blob-always-on-top-changed";

export type BlobAlwaysOnTopPayload = {
  enabled: boolean;
};

type PinnableWindow = {
  setAlwaysOnTop: (enabled: boolean) => Promise<void>;
};

export function readBlobAlwaysOnTop() {
  try {
    const value = window.localStorage.getItem(BLOB_ALWAYS_ON_TOP_STORAGE_KEY);
    return value === null ? true : value === "true";
  } catch {
    return true;
  }
}

export function persistBlobAlwaysOnTop(enabled: boolean) {
  try {
    window.localStorage.setItem(
      BLOB_ALWAYS_ON_TOP_STORAGE_KEY,
      String(enabled)
    );
  } catch {}
}

export async function setWindowAlwaysOnTopSafely(
  win: PinnableWindow | null | undefined,
  enabled: boolean
) {
  if (!win) return;

  try {
    await win.setAlwaysOnTop(enabled);
  } catch (error) {
    console.error("failed to apply always-on-top", error);
  }
}

export async function applyBlobWindowPinning(
  enabled = readBlobAlwaysOnTop(),
  labels = ["main", "bubble", "bubble-subtitle", "speech"]
) {
  await Promise.all(
    labels.map(async (label) => {
      const win = await WebviewWindow.getByLabel(label).catch(() => null);
      await setWindowAlwaysOnTopSafely(win, enabled);
    })
  );
}
