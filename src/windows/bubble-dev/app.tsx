import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  X,
  GripHorizontal,
  Mic,
  Globe,
  Music2,
  Dice5,
  TimerReset,
  MonitorSmartphone,
  Search,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

type DevPayload = {
  lastRoute?: string;
  voiceShortcut?: string;
  model?: string;
};

type CommandGroup = {
  title: string;
  icon: JSX.Element;
  items: Array<{
    command: string;
    description: string;
  }>;
};

const commandGroups: CommandGroup[] = [
  {
    title: "Voice / General",
    icon: <Mic size={15} />,
    items: [
      { command: "what time is it", description: "Aktuelle Uhrzeit" },
      { command: "what date is it", description: "Aktuelles Datum" },
      { command: "weather in Berlin", description: "Wetter für Ort abrufen" },
      { command: "take screenshot", description: "Snip-Modus öffnen" },
    ],
  },
  {
    title: "Browser",
    icon: <Globe size={15} />,
    items: [
      { command: "google cats", description: "Google-Suche starten" },
      { command: "youtube lo fi", description: "YouTube-Suche" },
      { command: "open github", description: "Website / App öffnen" },
      { command: "new tab", description: "Neuen Tab öffnen" },
      { command: "close tab", description: "Aktiven Tab schließen" },
      { command: "go back", description: "Browser zurück" },
      { command: "go forward", description: "Browser vor" },
      { command: "scroll down", description: "Seite scrollen" },
      { command: "scroll up", description: "Seite hoch" },
      {
        command: "click first result",
        description: "Erstes Suchergebnis klicken",
      },
      { command: "browser context", description: "Infos über aktuelle Seite" },
    ],
  },
  {
    title: "Media / Streaming",
    icon: <Music2 size={15} />,
    items: [
      { command: "play youtube", description: "Play/Pause" },
      { command: "pause youtube", description: "Play/Pause" },
      { command: "skip ad", description: "YouTube Werbung überspringen" },
      { command: "next video", description: "Nächstes YouTube-Video" },
      { command: "seek forward", description: "10s vorspulen" },
      { command: "seek backward", description: "10s zurückspulen" },
      { command: "volume up", description: "Lauter" },
      { command: "volume down", description: "Leiser" },
      { command: "mute", description: "Ton aus" },
      { command: "unmute", description: "Ton an" },
      {
        command: "recommend a comedy on netflix",
        description: "Streaming Empfehlung",
      },
    ],
  },
  {
    title: "Fun / Mini Commands",
    icon: <Dice5 size={15} />,
    items: [
      { command: "flip a coin", description: "Münzwurf" },
      { command: "roll dice", description: "Würfel 1–6" },
      { command: "start a 5 minute timer", description: "Timer starten" },
    ],
  },
  {
    title: "Apps / System",
    icon: <MonitorSmartphone size={15} />,
    items: [
      { command: "open discord", description: "App oder Web-App öffnen" },
      { command: "open spotify", description: "Spotify öffnen" },
      { command: "open chrome", description: "Chrome öffnen" },
      { command: "open explorer", description: "Explorer öffnen" },
      { command: "open notepad", description: "Notepad öffnen" },
      { command: "open paint", description: "Paint öffnen" },
      { command: "open calc", description: "Rechner öffnen" },
      { command: "open settings", description: "Windows Einstellungen öffnen" },
    ],
  },
  {
    title: "Editing / Shortcuts",
    icon: <TerminalSquare size={15} />,
    items: [
      { command: "save", description: "Ctrl+S" },
      { command: "save as", description: "Ctrl+Shift+S" },
      { command: "open file", description: "Ctrl+O" },
      { command: "new file", description: "Ctrl+N" },
      { command: "undo", description: "Ctrl+Z" },
      { command: "redo", description: "Ctrl+Y" },
      { command: "confirm", description: "Enter" },
      { command: "clear", description: "Escape" },
    ],
  },
];

function DevWindow() {
  const [lastRoute, setLastRoute] = useState("none");
  const [voiceShortcut, setVoiceShortcut] = useState("Alt + M");
  const [model, setModel] = useState("llama3.1:8b");
  const [search, setSearch] = useState("");

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

  const filteredGroups = useMemo(() => {
    const q = search.trim().toLowerCase();

    if (!q) return commandGroups;

    return commandGroups
      .map((group) => ({
        ...group,
        items: group.items.filter(
          (item) =>
            item.command.toLowerCase().includes(q) ||
            item.description.toLowerCase().includes(q) ||
            group.title.toLowerCase().includes(q)
        ),
      }))
      .filter((group) => group.items.length > 0);
  }, [search]);

  const totalCommands = commandGroups.reduce(
    (sum, group) => sum + group.items.length,
    0
  );

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
          height: calc(100% - 71px);
          padding: 16px;
          display: grid;
          grid-template-rows: auto auto 1fr;
          gap: 14px;
          overflow: hidden;
        }

        .stats {
          display: grid;
          grid-template-columns: repeat(4, minmax(0, 1fr));
          gap: 12px;
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
          letter-spacing: 0.08em;
          opacity: 0.58;
          margin-bottom: 6px;
        }

        .value {
          font-size: 13px;
          line-height: 1.5;
          word-break: break-word;
        }

        .value.strong {
          font-size: 16px;
          font-weight: 700;
        }

        .searchWrap {
          display: flex;
          align-items: center;
          gap: 10px;
          border-radius: 16px;
          border: 1px solid rgba(255,255,255,0.08);
          background: rgba(255,255,255,0.06);
          padding: 12px 14px;
        }

        .searchIcon {
          opacity: 0.7;
          flex-shrink: 0;
        }

        .searchInput {
          width: 100%;
          border: none;
          outline: none;
          background: transparent;
          color: white;
          font-size: 13px;
        }

        .searchInput::placeholder {
          color: rgba(255,255,255,0.42);
        }

        .commandsArea {
          min-height: 0;
          overflow: auto;
          padding-right: 4px;
          display: grid;
          gap: 12px;
        }

        .commandsArea::-webkit-scrollbar {
          width: 10px;
        }

        .commandsArea::-webkit-scrollbar-thumb {
          background: rgba(255,255,255,0.12);
          border-radius: 999px;
        }

        .group {
          border-radius: 20px;
          border: 1px solid rgba(255,255,255,0.08);
          background: rgba(255,255,255,0.05);
          overflow: hidden;
        }

        .groupHeader {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 12px 14px;
          border-bottom: 1px solid rgba(255,255,255,0.06);
          background: rgba(255,255,255,0.03);
          font-size: 13px;
          font-weight: 700;
        }

        .groupBody {
          display: grid;
        }

        .cmdRow {
          display: grid;
          grid-template-columns: minmax(220px, 0.95fr) 1fr;
          gap: 16px;
          padding: 12px 14px;
          border-top: 1px solid rgba(255,255,255,0.05);
        }

        .cmdRow:first-child {
          border-top: none;
        }

        .cmd {
          font-size: 12px;
          font-weight: 700;
          color: rgba(255,255,255,0.95);
          word-break: break-word;
        }

        .desc {
          font-size: 12px;
          color: rgba(255,255,255,0.68);
          word-break: break-word;
        }

        .empty {
          border-radius: 18px;
          border: 1px dashed rgba(255,255,255,0.14);
          background: rgba(255,255,255,0.04);
          padding: 24px;
          text-align: center;
          color: rgba(255,255,255,0.65);
          font-size: 13px;
        }

        @media (max-width: 900px) {
          .stats {
            grid-template-columns: repeat(2, minmax(0, 1fr));
          }

          .cmdRow {
            grid-template-columns: 1fr;
            gap: 6px;
          }
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
                  Commands, Routing, Voice, Debug Info
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
            <div className="stats">
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

              <div className="card">
                <div className="label">Commands</div>
                <div className="value strong">{totalCommands}</div>
              </div>
            </div>

            <div className="searchWrap">
              <div className="searchIcon">
                <Search size={15} />
              </div>
              <input
                className="searchInput"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Commands filtern, z. B. youtube, timer, browser, volume ..."
              />
            </div>

            <div className="commandsArea">
              {filteredGroups.length === 0 ? (
                <div className="empty">
                  Keine Commands für diese Suche gefunden.
                </div>
              ) : (
                filteredGroups.map((group) => (
                  <div className="group" key={group.title}>
                    <div className="groupHeader">
                      {group.icon}
                      <span>{group.title}</span>
                    </div>

                    <div className="groupBody">
                      {group.items.map((item) => (
                        <div
                          className="cmdRow"
                          key={`${group.title}-${item.command}-${item.description}`}
                        >
                          <div className="cmd">{item.command}</div>
                          <div className="desc">{item.description}</div>
                        </div>
                      ))}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

createRoot(document.getElementById("root")!).render(<DevWindow />);
