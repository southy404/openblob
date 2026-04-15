import React, { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { emitTo, listen } from "@tauri-apps/api/event";
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
} from "lucide-react";

type QuickMenuPayload = {
  hint?: string;
  activeApp?: string;
  pinned?: boolean;
};

type QuickAction = {
  id: string;
  label: string;
  icon: React.ReactNode;
};

const actions: QuickAction[] = [
  {
    id: "open-bubble",
    label: "Open Bubble",
    icon: <MessageCircle size={16} />,
  },
  {
    id: "capture-clipboard",
    label: "Capture Clipboard",
    icon: <Clipboard size={16} />,
  },
  {
    id: "snip-screen",
    label: "Snip Screen",
    icon: <Scissors size={16} />,
  },
  {
    id: "media-play-pause",
    label: "Play / Pause",
    icon: <Play size={16} />,
  },
  {
    id: "media-prev",
    label: "Previous Track",
    icon: <SkipBack size={16} />,
  },
  {
    id: "media-next",
    label: "Next Track",
    icon: <SkipForward size={16} />,
  },
  {
    id: "volume-down",
    label: "Volume Down",
    icon: <Volume1 size={16} />,
  },
  {
    id: "volume-up",
    label: "Volume Up",
    icon: <Volume2 size={16} />,
  },
  {
    id: "toggle-mute",
    label: "Toggle Mute",
    icon: <VolumeX size={16} />,
  },
  {
    id: "sleep-now",
    label: "Sleep Now",
    icon: <Moon size={16} />,
  },
  {
    id: "close-app",
    label: "Close",
    icon: <Power size={16} />,
  },
];

function QuickMenuApp() {
  const [hint, setHint] = useState("Ready.");
  const [activeApp, setActiveApp] = useState("unknown");
  const [pinned, setPinned] = useState(true);

  const mountedRef = useRef(false);

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

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
          --text-main: rgba(255,255,255,0.96);
          --text-soft: rgba(255,255,255,0.72);
          --text-dim: rgba(255,255,255,0.48);
          --glass-bg: rgba(18, 22, 30, 0.34);
          --glass-bg-strong: rgba(18, 22, 30, 0.50);
          --glass-fill: rgba(255,255,255,0.07);
          --glass-fill-hover: rgba(255,255,255,0.13);
          --glass-border: rgba(255,255,255,0.14);
          --glass-border-soft: rgba(255,255,255,0.08);
          --blue: rgba(10, 132, 255, 0.92);
          --danger: rgba(255, 69, 58, 0.92);
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
        box-shadow: none;
        }

        .quick-close:hover {
          background: rgba(255,255,255,0.14);
          border-color: rgba(255,255,255,0.16);
        }

        .quick-scroll {
          position: relative;
          z-index: 1;
          height: calc(100% - 136px);
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

        .quick-btn-danger:hover {
          background: rgba(255, 69, 58, 0.14);
          border-color: rgba(255, 69, 58, 0.26);
        }

        .quick-btn-pin-active {
          border-color: rgba(10,132,255,0.28);
          background: rgba(10,132,255,0.14);
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
            {actions.map((action) => {
              const isDanger = action.id === "close-app";

              return (
                <button
                  key={action.id}
                  className={`quick-btn ${isDanger ? "quick-btn-danger" : ""}`}
                  onClick={() => sendAction(action.id)}
                >
                  {action.icon}
                  {action.label}
                </button>
              );
            })}

            <button
              className={`quick-btn ${pinned ? "quick-btn-pin-active" : ""}`}
              onClick={() => {
                const nextPinned = !pinned;
                setPinned(nextPinned);
                void sendAction("toggle-pin");
              }}
            >
              <Pin size={16} />
              {pinned ? "Disable Always on Top" : "Enable Always on Top"}
            </button>
          </div>

          <div className="quick-footer">
            <div className="chip">app {activeApp}</div>
            <div className="chip">{pinned ? "always on top" : "floating"}</div>
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
