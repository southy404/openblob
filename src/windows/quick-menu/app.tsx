import React, { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
} from "lucide-react";

type QuickMenuPayload = {
  hint?: string;
  activeApp?: string;
  pinned?: boolean;
};

function QuickMenuApp() {
  const [hint, setHint] = useState("Ready.");
  const [activeApp, setActiveApp] = useState("unknown");
  const [pinned, setPinned] = useState(true);

  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;

    let unlistenData: null | (() => void) = null;
    let unlistenHide: null | (() => void) = null;

    const setup = async () => {
      unlistenData = await listen<QuickMenuPayload>(
        "quick-menu-data",
        async (event) => {
          setHint(event.payload.hint || "Ready.");
          setActiveApp(event.payload.activeApp || "unknown");
          setPinned(Boolean(event.payload.pinned));

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
      mountedRef.current = false;
      unlistenData?.();
      unlistenHide?.();
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", onBlur);
    };
  }, []);

  const closeMenu = async () => {
    await emitTo("main", "quick-menu-action", {
      action: "close-menu",
    }).catch(console.error);

    await getCurrentWindow()
      .hide()
      .catch(() => {});
  };

  const sendAction = async (action: string) => {
    await emitTo("main", "quick-menu-action", { action }).catch(console.error);

    if (action !== "toggle-pin") {
      await getCurrentWindow()
        .hide()
        .catch(() => {});
    }
  };

  const glassButton: React.CSSProperties = {
    width: "100%",
    minHeight: 48,
    border: "1px solid rgba(255,255,255,0.10)",
    borderRadius: 18,
    background: "rgba(255,255,255,0.08)",
    color: "rgba(255,255,255,0.96)",
    padding: "0 14px",
    textAlign: "left",
    fontSize: 14,
    fontWeight: 600,
    cursor: "pointer",
    transition: "all 140ms ease",
    display: "flex",
    alignItems: "center",
    gap: 10,
  };

  const chipStyle: React.CSSProperties = {
    padding: "6px 10px",
    borderRadius: 999,
    background: "rgba(255,255,255,0.08)",
    border: "1px solid rgba(255,255,255,0.08)",
    fontSize: 11,
    color: "rgba(255,255,255,0.76)",
  };

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
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
          font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        }

        .quick-shell {
          width: 100%;
          height: 100%;
          padding: 12px;
          background: transparent;
        }

        .quick-menu {
        position: relative;
        width: 100%;
        height: 100%;
        overflow: hidden;
        border-radius: 30px;
        border: 1px solid rgba(255,255,255,0.18);
        background:
            linear-gradient(
            180deg,
            rgba(255,255,255,0.18),
            rgba(255,255,255,0.08)
            ),
            rgba(16,20,28,0.84);
        backdrop-filter: blur(24px) saturate(145%);
        -webkit-backdrop-filter: blur(24px) saturate(145%);
        box-shadow: none;
        }

        .quick-menu::before {
          content: "";
          position: absolute;
          inset: 0;
          border-radius: inherit;
          pointer-events: none;
          background:
            radial-gradient(circle at 18% 0%, rgba(255,255,255,0.14), transparent 34%),
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
          border-bottom: 1px solid rgba(255,255,255,0.08);
        }

        .quick-title {
          font-size: 13px;
          font-weight: 800;
          color: rgba(255,255,255,0.96);
          letter-spacing: 0.01em;
        }

        .quick-subtitle {
          margin-top: 4px;
          font-size: 11px;
          color: rgba(236,240,248,0.64);
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
        box-shadow: none;
        }

        .quick-close:hover {
          background: rgba(255,255,255,0.14);
        }

        .quick-scroll {
          position: relative;
          z-index: 1;
          height: calc(100% - 132px);
          overflow-y: auto;
          padding: 12px;
          display: flex;
          flex-direction: column;
          gap: 8px;
          scrollbar-width: thin;
          scrollbar-color: rgba(255,255,255,0.24) transparent;
        }

        .quick-btn:hover {
          background: rgba(255,255,255,0.14) !important;
          border-color: rgba(255,255,255,0.18) !important;
          box-shadow:
            inset 1px 1px 0 rgba(255,255,255,0.10),
            0 6px 18px rgba(0,0,0,0.16);
        }

        .quick-footer {
          position: relative;
          z-index: 1;
          padding: 10px 14px 14px;
          border-top: 1px solid rgba(255,255,255,0.08);
          display: flex;
          flex-wrap: wrap;
          gap: 8px;
        }
      `}</style>

      <div className="quick-shell">
        <div className="quick-menu">
          <div className="quick-header">
            <div>
              <div className="quick-title">Companion Quick Menu</div>
              <div className="quick-subtitle" title={`${hint} • ${activeApp}`}>
                {hint} • {activeApp}
              </div>
            </div>

            <button className="quick-close" onClick={closeMenu} title="Close">
              <X size={16} />
            </button>
          </div>

          <div className="quick-scroll">
            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("open-bubble")}
            >
              <MessageCircle size={16} />
              Open Bubble
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("capture-clipboard")}
            >
              <Clipboard size={16} />
              Capture Clipboard
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("snip-screen")}
            >
              <Scissors size={16} />
              Snip Screen
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("media-play-pause")}
            >
              <Play size={16} />
              Play / Pause
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("media-prev")}
            >
              <SkipBack size={16} />
              Previous Track
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("media-next")}
            >
              <SkipForward size={16} />
              Next Track
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("volume-down")}
            >
              <Volume1 size={16} />
              Volume Down
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("volume-up")}
            >
              <Volume2 size={16} />
              Volume Up
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("toggle-mute")}
            >
              <VolumeX size={16} />
              Toggle Mute
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => {
                const nextPinned = !pinned;
                setPinned(nextPinned);
                void sendAction("toggle-pin");
              }}
            >
              <Pin size={16} />
              {pinned ? "Disable Always on Top" : "Enable Always on Top"}
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("sleep-now")}
            >
              <Moon size={16} />
              Sleep Now
            </button>

            <button
              className="quick-btn"
              style={glassButton}
              onClick={() => sendAction("close-app")}
            >
              <Power size={16} />
              Close
            </button>
          </div>

          <div className="quick-footer">
            <div style={chipStyle}>app {activeApp}</div>
            <div style={chipStyle}>{pinned ? "always on top" : "floating"}</div>
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
