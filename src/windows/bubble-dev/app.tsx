import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  X,
  GripHorizontal,
  Mic,
  Globe,
  Music2,
  Dice5,
  MonitorSmartphone,
  Search,
  TerminalSquare,
  ChevronDown,
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
  const [blobName, setBlobName] = useState("");
  const [ownerName, setOwnerName] = useState("");
  const [language, setLanguage] = useState("en");
  const [saving, setSaving] = useState(false);
  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>({
    "Voice / General": true,
    Browser: false,
    "Media / Streaming": false,
    "Fun / Mini Commands": false,
    "Apps / System": false,
    "Editing / Shortcuts": false,
  });

  const appWindow = useMemo(() => getCurrentWindow(), []);

  useEffect(() => {
    const applyGlass = async () => {
      try {
        const win = getCurrentWindow();
        await invoke("apply_glass_effect", { window: win });
      } catch (error) {
        console.error("failed to apply dev glass effect", error);
      }
    };

    void applyGlass();
  }, []);

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

  useEffect(() => {
    const loadIdentity = async () => {
      try {
        const result = (await invoke("get_identity")) as [
          string,
          string,
          string
        ];
        const [blob, owner, lang] = result;
        setBlobName(blob);
        setOwnerName(owner);
        setLanguage(lang);
      } catch (err) {
        console.error("failed to load identity", err);
      }
    };

    void loadIdentity();
  }, []);

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

  const saveIdentity = async () => {
    try {
      setSaving(true);

      await invoke("update_identity", {
        blobName,
        ownerName,
        language,
      });

      const result = (await invoke("get_identity")) as [string, string, string];
      const [blob, owner, lang] = result;

      setBlobName(blob);
      setOwnerName(owner);
      setLanguage(lang);
    } catch (err) {
      console.error("failed to save identity", err);
    } finally {
      setSaving(false);
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

  useEffect(() => {
    const q = search.trim();
    if (!q) return;

    setOpenGroups((prev) => {
      const next = { ...prev };
      for (const group of filteredGroups) {
        next[group.title] = true;
      }
      return next;
    });
  }, [search, filteredGroups]);

  const totalCommands = commandGroups.reduce(
    (sum, group) => sum + group.items.length,
    0
  );

  const visibleCommands = filteredGroups.reduce(
    (sum, group) => sum + group.items.length,
    0
  );

  const toggleGroup = (title: string) => {
    setOpenGroups((prev) => ({
      ...prev,
      [title]: !prev[title],
    }));
  };

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
          --text-main: #ffffff;
          --text-soft: rgba(255,255,255,0.74);
          --text-dim: rgba(255,255,255,0.48);
          --glass-bg: rgba(24, 24, 28, 0.30);
          --glass-bg-strong: rgba(24, 24, 28, 0.42);
          --glass-stroke: rgba(255,255,255,0.14);
          --glass-stroke-soft: rgba(255,255,255,0.08);
          --glass-fill: rgba(255,255,255,0.06);
          --glass-fill-hover: rgba(255,255,255,0.10);
          --blue: rgba(10, 132, 255, 0.92);
        }

        html,
        body,
        #root {
          margin: 0;
          width: 100%;
          height: 100%;
          background: transparent;
          overflow: hidden;
          font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", Inter, sans-serif;
          color: var(--text-main);
        }

        * {
          box-sizing: border-box;
        }

        .shell {
          width: 100%;
          height: 100%;
          padding: 12px;
        }

        .panel {
          position: relative;
          width: 100%;
          height: 100%;
          border-radius: 28px;
          overflow: hidden;
          isolation: isolate;
          background: var(--glass-bg);
          backdrop-filter: blur(18px) saturate(155%);
          -webkit-backdrop-filter: blur(18px) saturate(155%);
          border: 1px solid var(--glass-stroke);
          box-shadow:
            inset 0 1px 1px rgba(255,255,255,0.16),
            inset 0 -1px 1px rgba(0,0,0,0.18);
        }

        .panel::before {
          content: "";
          position: absolute;
          inset: 0;
          pointer-events: none;
          border-radius: inherit;
          background:
            radial-gradient(circle at 12% 0%, rgba(255,255,255,0.10), transparent 28%),
            radial-gradient(circle at 100% 100%, rgba(255,255,255,0.05), transparent 24%);
        }

        .header {
          position: relative;
          z-index: 1;
          display: grid;
          grid-template-columns: minmax(0, 1fr) auto;
          align-items: center;
          gap: 12px;
          padding: 14px 16px;
          border-bottom: 1px solid rgba(255,255,255,0.06);
          background: linear-gradient(
            180deg,
            rgba(255,255,255,0.04),
            rgba(255,255,255,0.01)
          );
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
          width: 36px;
          height: 36px;
          border-radius: 13px;
          display: grid;
          place-items: center;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.08);
          color: rgba(255,255,255,0.88);
          flex-shrink: 0;
        }

        .titleWrap {
          min-width: 0;
        }

        .title {
          font-size: 14px;
          font-weight: 700;
          letter-spacing: 0.01em;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .subtitle {
          font-size: 11px;
          color: rgba(255,255,255,0.58);
          margin-top: 3px;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .close {
          width: 40px;
          height: 40px;
          border-radius: 13px;
          border: 1px solid rgba(255,255,255,0.12);
          background: rgba(255,255,255,0.10);
          color: white;
          display: grid;
          place-items: center;
          cursor: pointer;
          transition: all 0.2s ease;
          flex-shrink: 0;
        }

        .close:hover {
          background: rgba(255,255,255,0.16);
          border-color: rgba(255,255,255,0.18);
        }

      .content {
        position: relative;
        z-index: 1;
        height: calc(100% - 71px);
        padding: 14px;
        display: grid;
        grid-template-columns: 1fr;
        grid-auto-rows: max-content;
        gap: 12px;
        min-height: 0;
        overflow: auto;
        align-content: start;
      }

        .stats {
          display: grid;
          grid-template-columns: repeat(4, minmax(0, 1fr));
          gap: 10px;
        }

        .card {
          border-radius: 20px;
          border: 1px solid var(--glass-stroke-soft);
          background: rgba(255,255,255,0.05);
          padding: 13px;
          backdrop-filter: blur(18px) saturate(155%);
          -webkit-backdrop-filter: blur(18px) saturate(155%);
          min-width: 0;
        }

        .label {
          font-size: 10px;
          text-transform: uppercase;
          letter-spacing: 0.08em;
          opacity: 0.58;
          margin-bottom: 6px;
        }

        .value {
          font-size: 13px;
          line-height: 1.45;
          word-break: break-word;
          color: var(--text-soft);
        }

        .value.strong {
          font-size: 16px;
          font-weight: 700;
          color: var(--text-main);
        }

        .identityCard {
          display: grid;
          gap: 12px;
        }

        .identityGrid {
          display: grid;
          grid-template-columns: repeat(3, minmax(0, 1fr)) auto;
          gap: 10px;
          align-items: end;
        }

        .field {
          display: flex;
          flex-direction: column;
          gap: 6px;
          min-width: 0;
        }

        .fieldLabel {
          font-size: 11px;
          color: rgba(255,255,255,0.56);
          letter-spacing: 0.03em;
        }

        .textInput {
          width: 100%;
          height: 42px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.07);
          color: white;
          outline: none;
          padding: 0 12px;
          font-size: 13px;
          min-width: 0;
        }

        .textInput::placeholder {
          color: rgba(255,255,255,0.38);
        }

        .textInput:focus {
          border-color: rgba(255,255,255,0.18);
          background: rgba(255,255,255,0.10);
        }

        .saveBtn {
          height: 42px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.16);
          background: var(--blue);
          color: white;
          font-weight: 700;
          padding: 0 16px;
          cursor: pointer;
          transition: all 0.2s ease;
          white-space: nowrap;
        }

        .saveBtn:hover {
          filter: brightness(1.05);
        }

        .saveBtn:disabled {
          opacity: 0.65;
          cursor: default;
        }

        .searchWrap {
          display: flex;
          align-items: center;
          gap: 10px;
          border-radius: 18px;
          border: 1px solid rgba(255,255,255,0.08);
          background: rgba(255,255,255,0.06);
          padding: 12px 14px;
          min-width: 0;
        }

        .searchIcon {
          opacity: 0.7;
          flex-shrink: 0;
        }

        .searchInput {
          width: 100%;
          min-width: 0;
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
          min-height: 180px;
          padding-right: 4px;
          display: grid;
          align-content: start;
          gap: 10px;
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
          backdrop-filter: blur(12px) saturate(130%);
          -webkit-backdrop-filter: blur(12px) saturate(130%);
          min-height: 52px;
        }

        .groupToggle {
          width: 100%;
          border: 0;
          background: rgba(255,255,255,0.03);
          color: white;
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 10px;
          padding: 12px 14px;
          cursor: pointer;
          text-align: left;
        }

        .groupToggle:hover {
          background: rgba(255,255,255,0.06);
        }

        .groupHeaderLeft {
          min-width: 0;
          display: flex;
          align-items: center;
          gap: 10px;
        }

        .groupMeta {
          min-width: 0;
          display: flex;
          flex-direction: column;
          gap: 2px;
        }

        .groupTitle {
          font-size: 13px;
          font-weight: 700;
          color: rgba(255,255,255,0.95);
        }

        .groupCount {
          font-size: 11px;
          color: rgba(255,255,255,0.56);
        }

        .chev {
          flex-shrink: 0;
          opacity: 0.72;
          transition: transform 0.18s ease;
        }

        .chev.open {
          transform: rotate(180deg);
        }

        .groupBody {
          display: grid;
          border-top: 1px solid rgba(255,255,255,0.06);
        }

        .cmdRow {
          display: grid;
          grid-template-columns: minmax(220px, 1fr) minmax(0, 1.1fr);
          gap: 14px;
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

        @media (max-width: 1100px) {
          .stats {
            grid-template-columns: repeat(2, minmax(0, 1fr));
          }

          .identityGrid {
            grid-template-columns: repeat(2, minmax(0, 1fr));
          }

          .saveBtn {
            width: 100%;
          }
        }

        @media (max-width: 760px) {
          .shell {
            padding: 8px;
          }

          .panel {
            border-radius: 22px;
          }

          .header {
            padding: 12px;
          }

          .content {
            padding: 12px;
            grid-template-rows: auto auto auto minmax(0, 1fr);
          }

          .content::-webkit-scrollbar {
            width: 10px;
          }

          .content::-webkit-scrollbar-thumb {
            background: rgba(255,255,255,0.12);
            border-radius: 999px;
          }

          .stats {
            grid-template-columns: 1fr;
          }

          .identityGrid {
            grid-template-columns: 1fr;
          }

          .cmdRow {
            grid-template-columns: 1fr;
            gap: 6px;
          }

          .subtitle {
            white-space: normal;
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

              <div className="titleWrap">
                <div className="title">OpenBlob Dev Mode</div>
                <div className="subtitle">
                  Commands, routing, voice, identity, debug info
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
                <div className="value strong">
                  {search.trim()
                    ? `${visibleCommands} / ${totalCommands}`
                    : totalCommands}
                </div>
              </div>
            </div>

            <div className="card identityCard">
              <div className="label">Companion Identity</div>

              <div className="identityGrid">
                <div className="field">
                  <div className="fieldLabel">Blob Name</div>
                  <input
                    className="textInput"
                    value={blobName}
                    onChange={(e) => setBlobName(e.target.value)}
                    placeholder="Blob Name"
                  />
                </div>

                <div className="field">
                  <div className="fieldLabel">Owner Name</div>
                  <input
                    className="textInput"
                    value={ownerName}
                    onChange={(e) => setOwnerName(e.target.value)}
                    placeholder="Owner Name"
                  />
                </div>

                <div className="field">
                  <div className="fieldLabel">Language</div>
                  <input
                    className="textInput"
                    value={language}
                    onChange={(e) => setLanguage(e.target.value)}
                    placeholder="Language (en/de)"
                  />
                </div>

                <button
                  className="saveBtn"
                  onClick={() => void saveIdentity()}
                  disabled={saving}
                  type="button"
                >
                  {saving ? "Saving..." : "Save"}
                </button>
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
                filteredGroups.map((group) => {
                  const isOpen = !!openGroups[group.title];

                  return (
                    <div className="group" key={group.title}>
                      <button
                        className="groupToggle"
                        type="button"
                        onClick={() => toggleGroup(group.title)}
                      >
                        <div className="groupHeaderLeft">
                          {group.icon}
                          <div className="groupMeta">
                            <div className="groupTitle">{group.title}</div>
                            <div className="groupCount">
                              {group.items.length} command
                              {group.items.length === 1 ? "" : "s"}
                            </div>
                          </div>
                        </div>

                        <ChevronDown
                          size={16}
                          className={`chev ${isOpen ? "open" : ""}`}
                        />
                      </button>

                      {isOpen && (
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
                      )}
                    </div>
                  );
                })
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

createRoot(document.getElementById("root")!).render(<DevWindow />);
