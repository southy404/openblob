import React, { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen, emit, emitTo } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import {
  MessageCircle,
  Clipboard,
  Scissors,
  Play,
  SkipBack,
  SkipForward,
  Volume1,
  Volume2,
  VolumeX,
  Pin,
  Moon,
  Power,
  X,
  Square,
  LoaderCircle,
  Wrench,
} from "lucide-react";
import { showTranscriptWindow } from "../transcript/open";
import { ensureDevWindow } from "../bubble-dev/open";

type QuickMenuPayload = {
  hint?: string;
  activeApp?: string;
  pinned?: boolean;
};

type TranscriptStatus = {
  state: "Idle" | "Recording" | "Stopping" | "Summarizing" | "Error";
  active_session_id: string | null;
  segment_count: number;
};

type UiLang = "en" | "de";

type QuickAction = {
  id: string;
  label: Record<UiLang, string>;
  icon: React.ReactNode;
  danger?: boolean;
};

type LocalizedText = {
  ready: string;
  unknown: string;
  close: string;
  quickMenuTitle: string;
  startTranscript: string;
  stopTranscript: string;
  enableAlwaysOnTop: string;
  disableAlwaysOnTop: string;
  transcriptIdle: string;
  transcriptRecording: string;
  transcriptStopping: string;
  transcriptWorking: string;
  appChip: (app: string) => string;
  pinChipPinned: string;
  pinChipFloating: string;
  segmentsChip: (count: number) => string;
  transcriptErrorFallback: string;
  devWindow: string;
};

const TEXTS: Record<UiLang, LocalizedText> = {
  en: {
    ready: "Ready.",
    unknown: "unknown",
    close: "Close",
    quickMenuTitle: "Companion Quick Menu",
    startTranscript: "Start Transcript",
    stopTranscript: "Stop Transcript",
    enableAlwaysOnTop: "Enable Always on Top",
    disableAlwaysOnTop: "Disable Always on Top",
    transcriptIdle: "transcript idle",
    transcriptRecording: "transcript recording",
    transcriptStopping: "transcript stopping",
    transcriptWorking: "transcript working",
    appChip: (app) => `app ${app}`,
    pinChipPinned: "always on top",
    pinChipFloating: "floating",
    segmentsChip: (count) => `segments ${count}`,
    transcriptErrorFallback: "Transcript error",
    devWindow: "Open Dev Window",
  },
  de: {
    ready: "Bereit.",
    unknown: "unbekannt",
    close: "Schließen",
    quickMenuTitle: "Companion Quick Menu",
    startTranscript: "Transkript starten",
    stopTranscript: "Transkript stoppen",
    enableAlwaysOnTop: "Immer im Vordergrund aktivieren",
    disableAlwaysOnTop: "Immer im Vordergrund deaktivieren",
    transcriptIdle: "transkript inaktiv",
    transcriptRecording: "transkript aufnahme",
    transcriptStopping: "transkript stoppt",
    transcriptWorking: "transkript verarbeitet",
    appChip: (app) => `app ${app}`,
    pinChipPinned: "immer im vordergrund",
    pinChipFloating: "frei schwebend",
    segmentsChip: (count) => `segmente ${count}`,
    transcriptErrorFallback: "Transkriptfehler",
    devWindow: "Dev Window öffnen",
  },
};

const actions: QuickAction[] = [
  {
    id: "open-bubble",
    label: {
      en: "Open Bubble",
      de: "Bubble öffnen",
    },
    icon: <MessageCircle size={16} />,
  },
  {
    id: "open-transcript",
    label: {
      en: "Open Transcript",
      de: "Transkript öffnen",
    },
    icon: <MessageCircle size={16} />,
  },
  {
    id: "capture-clipboard",
    label: {
      en: "Capture Clipboard",
      de: "Zwischenablage erfassen",
    },
    icon: <Clipboard size={16} />,
  },
  {
    id: "snip-screen",
    label: {
      en: "Snip Screen",
      de: "Bildschirm snippen",
    },
    icon: <Scissors size={16} />,
  },
  {
    id: "media-play-pause",
    label: {
      en: "Play / Pause",
      de: "Play / Pause",
    },
    icon: <Play size={16} />,
  },
  {
    id: "media-prev",
    label: {
      en: "Previous Track",
      de: "Vorheriger Track",
    },
    icon: <SkipBack size={16} />,
  },
  {
    id: "media-next",
    label: {
      en: "Next Track",
      de: "Nächster Track",
    },
    icon: <SkipForward size={16} />,
  },
  {
    id: "volume-down",
    label: {
      en: "Volume Down",
      de: "Leiser",
    },
    icon: <Volume1 size={16} />,
  },
  {
    id: "volume-up",
    label: {
      en: "Volume Up",
      de: "Lauter",
    },
    icon: <Volume2 size={16} />,
  },
  {
    id: "toggle-mute",
    label: {
      en: "Toggle Mute",
      de: "Stumm umschalten",
    },
    icon: <VolumeX size={16} />,
  },
  {
    id: "sleep-now",
    label: {
      en: "Sleep Now",
      de: "Jetzt schlafen",
    },
    icon: <Moon size={16} />,
  },
  {
    id: "close-app",
    label: {
      en: "Close",
      de: "Schließen",
    },
    icon: <Power size={16} />,
    danger: true,
  },
];

async function broadcastTranscriptStatus() {
  try {
    const status = await invoke<TranscriptStatus>("get_transcript_status");

    await emit("transcript://status", status);

    await emit("blob-state", {
      state: "transcript",
      active: status.state === "Recording",
    });

    return status;
  } catch (error) {
    console.error("failed to broadcast transcript status", error);
    return null;
  }
}

function QuickMenuApp() {
  const [uiLang, setUiLang] = useState<UiLang>("en");
  const [hint, setHint] = useState(TEXTS.en.ready);
  const [activeApp, setActiveApp] = useState(TEXTS.en.unknown);
  const [pinned, setPinned] = useState(true);

  const [transcriptRunning, setTranscriptRunning] = useState(false);
  const [transcriptBusy, setTranscriptBusy] = useState(false);
  const [transcriptSegments, setTranscriptSegments] = useState(0);
  const [errorText, setErrorText] = useState<string | null>(null);

  const t = TEXTS[uiLang];

  const transcriptChip = useMemo(() => {
    if (transcriptBusy && transcriptRunning) return t.transcriptStopping;
    if (transcriptBusy) return t.transcriptWorking;
    return transcriptRunning ? t.transcriptRecording : t.transcriptIdle;
  }, [t, transcriptBusy, transcriptRunning]);

  useEffect(() => {
    const applyGlass = async () => {
      try {
        const win = getCurrentWindow();
        await invoke("apply_glass_effect", { window: win });
      } catch (error) {
        console.error("failed to apply glass effect", error);
      }
    };

    void applyGlass();
  }, []);

  useEffect(() => {
    const loadIdentity = async () => {
      try {
        const result = (await invoke("get_identity")) as [
          string,
          string,
          string
        ];
        const [, , lang] = result;
        setUiLang(lang === "de" ? "de" : "en");
      } catch (error) {
        console.error("failed to load identity for quick menu ui", error);
        setUiLang("en");
      }
    };

    void loadIdentity();

    let unlistenIdentityUpdated: null | (() => void) = null;

    const setupIdentityListener = async () => {
      unlistenIdentityUpdated = await listen("identity-updated", async () => {
        try {
          const result = (await invoke("get_identity")) as [
            string,
            string,
            string
          ];
          const [, , lang] = result;
          setUiLang(lang === "de" ? "de" : "en");
        } catch (error) {
          console.error("failed to refresh identity for quick menu ui", error);
        }
      });
    };

    void setupIdentityListener();

    return () => {
      unlistenIdentityUpdated?.();
    };
  }, []);

  useEffect(() => {
    let unlistenData: null | (() => void) = null;
    let unlistenHide: null | (() => void) = null;
    let unlistenTranscriptSegment: null | (() => void) = null;
    let unlistenTranscriptError: null | (() => void) = null;
    let unlistenTranscriptStatus: null | (() => void) = null;

    const refreshTranscriptStatus = async () => {
      try {
        const status = await invoke<TranscriptStatus>("get_transcript_status");
        setTranscriptRunning(status.state === "Recording");
        setTranscriptBusy(
          status.state === "Stopping" || status.state === "Summarizing"
        );
        setTranscriptSegments(status.segment_count ?? 0);
      } catch (error) {
        console.error("failed to get transcript status", error);
      }
    };

    const setup = async () => {
      await refreshTranscriptStatus();

      unlistenData = await listen<QuickMenuPayload>(
        "quick-menu-data",
        async (event) => {
          setHint(event.payload.hint || t.ready);
          setActiveApp(event.payload.activeApp || t.unknown);
          setPinned(Boolean(event.payload.pinned));
          setErrorText(null);

          await refreshTranscriptStatus();

          const win = getCurrentWindow();
          await win.show().catch(() => {});
          await win.setFocus().catch(() => {});
        }
      );

      unlistenHide = await listen("quick-menu-hide", async () => {
        await getCurrentWindow()
          .hide()
          .catch(() => {});
      });

      unlistenTranscriptSegment = await listen("transcript://segment", () => {
        setTranscriptRunning(true);
        setTranscriptBusy(false);
        setTranscriptSegments((prev) => prev + 1);
      });

      unlistenTranscriptError = await listen<string>(
        "transcript://error",
        (event) => {
          const message = String(event.payload || t.transcriptErrorFallback);
          setErrorText(message);
          setTranscriptBusy(false);
        }
      );

      unlistenTranscriptStatus = await listen<TranscriptStatus>(
        "transcript://status",
        (event) => {
          const status = event.payload;

          setTranscriptRunning(status.state === "Recording");
          setTranscriptBusy(
            status.state === "Stopping" || status.state === "Summarizing"
          );
          setTranscriptSegments(status.segment_count ?? 0);
        }
      );
    };

    void setup();

    const onKey = async (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        await closeMenu();
      }
    };

    const onBlur = async () => {
      await getCurrentWindow()
        .hide()
        .catch(() => {});
    };

    window.addEventListener("keydown", onKey);
    window.addEventListener("blur", onBlur);

    return () => {
      unlistenData?.();
      unlistenHide?.();
      unlistenTranscriptSegment?.();
      unlistenTranscriptError?.();
      unlistenTranscriptStatus?.();
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", onBlur);
    };
  }, [t]);

  const closeMenu = async () => {
    await emitTo("main", "quick-menu-action", {
      action: "close-menu",
    }).catch(console.error);

    await getCurrentWindow()
      .hide()
      .catch(() => {});
  };

  const hideMenuUnlessPinnedAction = async (action: string) => {
    if (action !== "toggle-pin") {
      await getCurrentWindow()
        .hide()
        .catch(() => {});
    }
  };

  const openDevWindow = async () => {
    try {
      const dev = await ensureDevWindow();
      await dev.show();
      await dev.setFocus().catch(() => {});
      await getCurrentWindow()
        .hide()
        .catch(() => {});
    } catch (error) {
      console.error("failed to open dev window", error);
    }
  };

  const sendAction = async (action: string) => {
    try {
      setErrorText(null);

      if (action === "open-transcript") {
        await showTranscriptWindow();
        await hideMenuUnlessPinnedAction(action);
        return;
      }

      if (action === "start-transcript") {
        if (transcriptRunning || transcriptBusy) return;

        setTranscriptBusy(true);

        await invoke("start_transcript", {
          source: "system",
          appName: activeApp,
          windowTitle: `${activeApp} Transcript`,
        });

        const status = await broadcastTranscriptStatus();

        setTranscriptRunning(status?.state === "Recording");
        setTranscriptBusy(false);
        setTranscriptSegments(status?.segment_count ?? 0);

        await showTranscriptWindow();
        await hideMenuUnlessPinnedAction(action);
        return;
      }

      if (action === "stop-transcript") {
        if (!transcriptRunning || transcriptBusy) return;

        setTranscriptBusy(true);
        await emit("transcript://status", {
          state: "Stopping",
          active_session_id: null,
          segment_count: transcriptSegments,
        });

        await emit("blob-state", {
          state: "transcript",
          active: false,
        });

        await invoke("stop_transcript");

        const status = await broadcastTranscriptStatus();

        setTranscriptRunning(status?.state === "Recording");
        setTranscriptBusy(false);
        setTranscriptSegments(status?.segment_count ?? 0);

        await hideMenuUnlessPinnedAction(action);
        return;
      }

      if (action === "toggle-pin") {
        await emitTo("main", "quick-menu-action", { action });
        return;
      }

      await emitTo("main", "quick-menu-action", { action });
      await hideMenuUnlessPinnedAction(action);
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : String(error ?? "Unknown error");
      setErrorText(message);
      setTranscriptBusy(false);
      console.error("quick action failed", error);
    }
  };

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
          --text-main: rgba(255,255,255,0.96);
          --text-soft: rgba(255,255,255,0.72);
          --text-dim: rgba(255,255,255,0.48);
          --glass-bg: rgba(18, 22, 30, 0.34);
          --glass-fill-hover: rgba(255,255,255,0.13);
          --glass-border: rgba(255,255,255,0.14);
          --glass-border-soft: rgba(255,255,255,0.08);
          --blue: rgba(10, 132, 255, 0.92);
          --danger: rgba(255, 69, 58, 0.92);
          --success: rgba(52, 199, 89, 0.92);
          --warn: rgba(255, 159, 10, 0.92);
        }

        * {
          box-sizing: border-box;
        }

        html, body, #root {
          width: 100%;
          height: 100%;
          margin: 0;
          overflow: hidden;
          background: transparent;
          font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", Inter, sans-serif;
          color: var(--text-main);
        }

        .quick-shell {
          width: 100%;
          height: 100%;
          background: transparent;
        }

        .quick-menu {
          position: relative;
          width: 100%;
          height: 100%;
          overflow: hidden;
          border-radius: 30px;
          isolation: isolate;
          background: var(--glass-bg);
          backdrop-filter: blur(24px) saturate(150%);
          -webkit-backdrop-filter: blur(24px) saturate(150%);
          border: 1px solid var(--glass-border);
          box-shadow:
            inset 0 1px 1px rgba(255,255,255,0.16),
            inset 0 -1px 1px rgba(0,0,0,0.18);
        }

        .quick-menu::before {
          content: "";
          position: absolute;
          inset: 0;
          pointer-events: none;
          border-radius: inherit;
          background:
            radial-gradient(circle at 14% 0%, rgba(255,255,255,0.12), transparent 30%),
            radial-gradient(circle at 100% 100%, rgba(117,163,255,0.10), transparent 24%);
        }

        .quick-header {
          position: relative;
          z-index: 1;
          display: grid;
          grid-template-columns: 1fr auto;
          gap: 12px;
          align-items: center;
          padding: 14px 14px 12px;
          border-bottom: 1px solid rgba(255,255,255,0.06);
          background: linear-gradient(
            180deg,
            rgba(255,255,255,0.05),
            rgba(255,255,255,0.01)
          );
        }

        .quick-title {
          font-size: 14px;
          font-weight: 800;
          letter-spacing: 0.01em;
          color: var(--text-main);
        }

        .quick-subtitle {
          margin-top: 4px;
          font-size: 11px;
          color: rgba(255,255,255,0.58);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .quick-close {
          width: 40px;
          height: 40px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.08);
          color: white;
          display: grid;
          place-items: center;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .quick-close:hover {
          background: rgba(255,255,255,0.14);
          border-color: rgba(255,255,255,0.16);
        }

        .quick-scroll {
          position: relative;
          z-index: 1;
          height: calc(100% - 152px);
          overflow-y: auto;
          padding: 12px;
          display: flex;
          flex-direction: column;
          gap: 10px;
          scrollbar-width: thin;
          scrollbar-color: rgba(255,255,255,0.18) transparent;
        }

        .quick-scroll::-webkit-scrollbar {
          width: 10px;
        }

        .quick-scroll::-webkit-scrollbar-thumb {
          background: rgba(255,255,255,0.14);
          border-radius: 999px;
        }

        .quick-btn {
          width: 100%;
          min-height: 50px;
          border: 1px solid var(--glass-border-soft);
          border-radius: 18px;
          background: rgba(255,255,255,0.06);
          color: var(--text-main);
          padding: 0 14px;
          text-align: left;
          font-size: 13px;
          font-weight: 650;
          cursor: pointer;
          transition: all 160ms ease;
          display: flex;
          align-items: center;
          gap: 10px;
          backdrop-filter: blur(14px) saturate(135%);
          -webkit-backdrop-filter: blur(14px) saturate(135%);
        }

        .quick-btn:hover {
          background: var(--glass-fill-hover);
          border-color: rgba(255,255,255,0.16);
          transform: translateY(-1px);
        }

        .quick-btn:active {
          transform: translateY(0);
        }

        .quick-btn:disabled {
          opacity: 0.58;
          cursor: not-allowed;
          transform: none;
        }

        .quick-btn-danger:hover {
          background: rgba(255, 69, 58, 0.14);
          border-color: rgba(255, 69, 58, 0.26);
        }

        .quick-btn-pin-active {
          border-color: rgba(10,132,255,0.28);
          background: rgba(10,132,255,0.14);
        }

        .quick-btn-transcript-start {
          border-color: rgba(52,199,89,0.22);
          background: rgba(52,199,89,0.12);
        }

        .quick-btn-transcript-stop {
          border-color: rgba(255,69,58,0.22);
          background: rgba(255,69,58,0.12);
        }

        .quick-error {
          margin: 0 12px;
          padding: 10px 12px;
          border-radius: 14px;
          font-size: 11px;
          line-height: 1.35;
          color: rgba(255,255,255,0.92);
          background: rgba(255,69,58,0.14);
          border: 1px solid rgba(255,69,58,0.22);
          position: relative;
          z-index: 1;
        }

        .quick-footer {
          position: relative;
          z-index: 1;
          padding: 10px 14px 14px;
          border-top: 1px solid rgba(255,255,255,0.06);
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
        }

        .chip {
          padding: 6px 10px;
          border-radius: 999px;
          background: rgba(255,255,255,0.07);
          border: 1px solid rgba(255,255,255,0.08);
          font-size: 11px;
          color: rgba(255,255,255,0.74);
          backdrop-filter: blur(10px);
          -webkit-backdrop-filter: blur(10px);
        }

        .chip-recording {
          border-color: rgba(52,199,89,0.22);
          background: rgba(52,199,89,0.12);
          color: rgba(255,255,255,0.92);
        }

        .chip-busy {
          border-color: rgba(255,159,10,0.22);
          background: rgba(255,159,10,0.12);
          color: rgba(255,255,255,0.92);
        }

        .spin {
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>

      <div className="quick-shell">
        <div className="quick-menu">
          <div className="quick-header">
            <div>
              <div className="quick-title">{t.quickMenuTitle}</div>
              <div className="quick-subtitle" title={`${hint} • ${activeApp}`}>
                {hint} • {activeApp}
              </div>
            </div>

            <button className="quick-close" onClick={closeMenu} title={t.close}>
              <X size={16} />
            </button>
          </div>

          {errorText && <div className="quick-error">{errorText}</div>}

          <div className="quick-scroll">
            {!transcriptRunning ? (
              <button
                className="quick-btn quick-btn-transcript-start"
                onClick={() => sendAction("start-transcript")}
                disabled={transcriptBusy}
              >
                {transcriptBusy ? (
                  <LoaderCircle size={16} className="spin" />
                ) : (
                  <MessageCircle size={16} />
                )}
                {t.startTranscript}
              </button>
            ) : (
              <button
                className="quick-btn quick-btn-transcript-stop"
                onClick={() => sendAction("stop-transcript")}
                disabled={transcriptBusy}
              >
                {transcriptBusy ? (
                  <LoaderCircle size={16} className="spin" />
                ) : (
                  <Square size={16} />
                )}
                {t.stopTranscript}
              </button>
            )}

            <button className="quick-btn" onClick={() => void openDevWindow()}>
              <Wrench size={16} />
              {t.devWindow}
            </button>

            {actions.map((action) => (
              <button
                key={action.id}
                className={`quick-btn ${
                  action.danger ? "quick-btn-danger" : ""
                }`}
                onClick={() => sendAction(action.id)}
              >
                {action.icon}
                {action.label[uiLang]}
              </button>
            ))}

            <button
              className={`quick-btn ${pinned ? "quick-btn-pin-active" : ""}`}
              onClick={() => {
                const nextPinned = !pinned;
                setPinned(nextPinned);
                void sendAction("toggle-pin");
              }}
            >
              <Pin size={16} />
              {pinned ? t.disableAlwaysOnTop : t.enableAlwaysOnTop}
            </button>
          </div>

          <div className="quick-footer">
            <div className="chip">{t.appChip(activeApp)}</div>
            <div className="chip">
              {pinned ? t.pinChipPinned : t.pinChipFloating}
            </div>
            <div
              className={`chip ${
                transcriptBusy
                  ? "chip-busy"
                  : transcriptRunning
                  ? "chip-recording"
                  : ""
              }`}
            >
              {transcriptChip}
            </div>
            <div className="chip">{t.segmentsChip(transcriptSegments)}</div>
          </div>
        </div>
      </div>
    </>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QuickMenuApp />
  </React.StrictMode>
);
