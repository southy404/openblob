import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { X, GripHorizontal } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

type DevPayload = {
  lastRoute?: string;
  voiceShortcut?: string;
  model?: string;
};

function DevWindow() {
  const [lastRoute, setLastRoute] = useState("none");
  const [voiceShortcut, setVoiceShortcut] = useState("Alt + M");
  const [model, setModel] = useState("llama3.1:8b");

  const appWindow = useMemo(() => getCurrentWindow(), []);

  useEffect(() => {
    let unlisten: null | (() => void) = null;

    const setup = async () => {
      unlisten = await listen<DevPayload>("bubble-dev-data", (event) => {
        const payload = event.payload;
        if (payload.lastRoute) setLastRoute(payload.lastRoute);
        if (payload.voiceShortcut) setVoiceShortcut(payload.voiceShortcut);
        if (payload.model) setModel(payload.model);
      });
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void appWindow.hide();
      }
    };

    void setup();
    window.addEventListener("keydown", onKeyDown);

    return () => {
      if (unlisten) unlisten();
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [appWindow]);

  const closeWindow = async () => {
    try {
      await appWindow.hide();
    } catch (error) {
      console.error("failed to hide dev window:", error);
    }
  };

  const startWindowDrag = async () => {
    try {
      await appWindow.startDragging();
    } catch (error) {
      console.error("failed to start dragging:", error);
    }
  };

  return (
    <>
      <style>{`
        html, body, #root {
          margin: 0;
          width: 100%;
          height: 100%;
          background: transparent;
          overflow: hidden;
          font-family: Inter, system-ui, sans-serif;
          color: rgba(255,255,255,0.96);
        }

        * { box-sizing: border-box; }

        .shell {
          width: 100%;
          height: 100%;
          padding: 14px;
        }

        .panel {
          position: relative;
          width: 100%;
          height: 100%;
          border-radius: 28px;
          overflow: hidden;
          border: 1px solid rgba(255,255,255,0.18);
          background:
            linear-gradient(
              180deg,
              rgba(255,255,255,0.22),
              rgba(255,255,255,0.12)
            ),
            rgba(14, 18, 26, 0.88);
          backdrop-filter: blur(38px) saturate(155%);
          -webkit-backdrop-filter: blur(38px) saturate(155%);
          isolation: isolate;
        }

        .panel::before {
          content: "";
          position: absolute;
          inset: 0;
          pointer-events: none;
          border-radius: inherit;
          background:
            radial-gradient(circle at 14% 0%, rgba(255,255,255,0.16), transparent 30%),
            radial-gradient(circle at 100% 100%, rgba(255,255,255,0.08), transparent 26%);
        }

        .panel::after {
          content: "";
          position: absolute;
          inset: 0;
          pointer-events: none;
          border-radius: inherit;
          box-shadow:
            inset 1px 1px 0 rgba(255,255,255,0.30),
            inset -1px -1px 0 rgba(255,255,255,0.04);
        }

        .header {
          position: relative;
          z-index: 1;
          display: grid;
          grid-template-columns: 1fr auto;
          align-items: center;
          gap: 12px;
          padding: 14px 16px;
          border-bottom: 1px solid rgba(255,255,255,0.08);
        }

        .dragbar {
          min-width: 0;
          display: flex;
          align-items: center;
          gap: 10px;
          min-height: 42px;
          padding-right: 8px;
          user-select: none;
          cursor: grab;
        }

        .dragbar:active {
          cursor: grabbing;
        }

        .drag-icon {
          width: 34px;
          height: 34px;
          border-radius: 12px;
          display: grid;
          place-items: center;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.08);
          color: rgba(255,255,255,0.86);
          flex-shrink: 0;
        }

        .title {
          font-size: 14px;
          font-weight: 700;
        }

        .subtitle {
          font-size: 11px;
          color: rgba(255,255,255,0.58);
          margin-top: 3px;
        }

        .close {
          width: 40px;
          height: 40px;
          border-radius: 12px;
          border: 1px solid rgba(255,255,255,0.12);
          background: rgba(255,255,255,0.12);
          color: white;
          display: grid;
          place-items: center;
          cursor: pointer;
        }

        .close:hover {
          background: rgba(255,255,255,0.18);
        }

        .content {
          position: relative;
          z-index: 1;
          padding: 16px;
          display: grid;
          gap: 14px;
        }

        .card {
          border-radius: 18px;
          border: 1px solid rgba(255,255,255,0.08);
          background: rgba(255,255,255,0.06);
          padding: 14px;
        }

        .label {
          font-size: 11px;
          text-transform: uppercase;
          opacity: 0.58;
          margin-bottom: 6px;
        }

        .value {
          font-size: 13px;
          line-height: 1.5;
        }
      `}</style>

      <div className="shell">
        <div className="panel">
          <div className="header">
            <div
              className="dragbar"
              onMouseDown={(e) => {
                if (e.button !== 0) return;
                e.preventDefault();
                void startWindowDrag();
              }}
            >
              <div className="drag-icon">
                <GripHorizontal size={16} />
              </div>

              <div>
                <div className="title">OpenBlob Dev Mode</div>
                <div className="subtitle">
                  Verschiebbar · Escape oder X zum Schließen
                </div>
              </div>
            </div>

            <button
              className="close"
              onMouseDown={(e) => e.stopPropagation()}
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                void closeWindow();
              }}
              type="button"
              title="Schließen"
            >
              <X size={16} />
            </button>
          </div>

          <div className="content">
            <div className="card">
              <div className="label">Letzte Route</div>
              <div className="value">{lastRoute}</div>
            </div>

            <div className="card">
              <div className="label">Voice Shortcut</div>
              <div className="value">{voiceShortcut}</div>
            </div>

            <div className="card">
              <div className="label">Model</div>
              <div className="value">{model}</div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

createRoot(document.getElementById("root")!).render(<DevWindow />);
