import React, { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Search,
  Languages,
  ScanText,
  Sparkles,
  Copy,
  Send,
  ExternalLink,
  Image as ImageIcon,
  Youtube,
  X,
} from "lucide-react";

type SnipMode = "explain" | "translate" | "ocr" | "search";

type SnipPayload = {
  path: string;
  app?: string;
  windowTitle?: string;
  contextDomain?: string;
};

type SearchData = {
  intent: string;
  gameOrApp: string;
  keyText: string;
  searchQuery: string;
  altQuery1: string;
  altQuery2: string;
  answer: string;
};

function normalizeWindowsPath(path: string) {
  return path.replace(/\\/g, "/");
}

function extractField(text: string, label: string) {
  const regex = new RegExp(`${label}:\\s*([\\s\\S]*?)(?=\\n[A-Z _]+:|$)`, "i");
  const match = text.match(regex);
  return match?.[1]?.trim() || "";
}

function parseSearchResult(text: string): SearchData | null {
  const intent = extractField(text, "INTENT");
  const gameOrApp =
    extractField(text, "GAME OR APP") || extractField(text, "GAME_OR_APP");
  const keyText = extractField(text, "KEY_TEXT");
  const searchQuery =
    extractField(text, "SEARCH QUERY") || extractField(text, "SEARCH_QUERY");
  const altQuery1 =
    extractField(text, "ALT QUERY 1") || extractField(text, "ALT_QUERY_1");
  const altQuery2 =
    extractField(text, "ALT QUERY 2") || extractField(text, "ALT_QUERY_2");
  const answer = extractField(text, "ANSWER");

  if (!searchQuery) return null;

  return {
    intent,
    gameOrApp,
    keyText,
    searchQuery,
    altQuery1,
    altQuery2,
    answer,
  };
}

function buildGoogleSearchUrl(query: string) {
  return `https://www.google.com/search?q=${encodeURIComponent(
    query
  )}&t=${Date.now()}`;
}

function buildGoogleImageSearchUrl(query: string) {
  return `https://www.google.com/search?tbm=isch&q=${encodeURIComponent(
    query
  )}&t=${Date.now()}`;
}

function buildYouTubeSearchUrl(query: string) {
  return `https://www.youtube.com/results?search_query=${encodeURIComponent(
    query
  )}`;
}

function SnipPanel() {
  const [path, setPath] = useState("");
  const [previewUrl, setPreviewUrl] = useState("");
  const [comment, setComment] = useState("");
  const [result, setResult] = useState("");
  const [busy, setBusy] = useState(false);
  const [appName, setAppName] = useState("unknown");
  const [windowTitle, setWindowTitle] = useState("");
  const [contextDomain, setContextDomain] = useState("");
  const [previewState, setPreviewState] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const [previewError, setPreviewError] = useState("");
  const [searchData, setSearchData] = useState<SearchData | null>(null);

  const hasPreview = useMemo(
    () => !!previewUrl && previewState !== "error",
    [previewUrl, previewState]
  );

  useEffect(() => {
    let unlisten: null | (() => void) = null;

    const setup = async () => {
      unlisten = await listen<SnipPayload>("snip-panel-data", async (event) => {
        const nextPath = event.payload.path || "";
        const normalizedPath = normalizeWindowsPath(nextPath);

        const exists = await invoke<boolean>("snip_file_exists", {
          path: nextPath,
        }).catch(() => false);

        const nextPreviewUrl = exists ? convertFileSrc(normalizedPath) : "";

        setPath(nextPath);
        setPreviewUrl("");
        setComment("");
        setResult("");
        setBusy(false);
        setSearchData(null);
        setAppName(event.payload.app || "unknown");
        setWindowTitle(event.payload.windowTitle || "");
        setContextDomain(event.payload.contextDomain || "");
        setPreviewError("");

        if (exists && nextPreviewUrl) {
          setPreviewState("loading");
          setPreviewUrl(nextPreviewUrl);
        } else {
          setPreviewState("error");
          setPreviewError(
            nextPath
              ? `Snip file missing: ${nextPath}`
              : "No snip path received."
          );
        }

        await getCurrentWindow().show();
        await getCurrentWindow().setFocus();
      });
    };

    void setup();

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const openWebSearch = async (query?: string) => {
    const finalQuery = query || searchData?.searchQuery;
    if (!finalQuery) return;
    await openUrl(buildGoogleSearchUrl(finalQuery)).catch(console.error);
  };

  const openImageSearch = async (query?: string) => {
    const finalQuery = query || searchData?.searchQuery;
    if (!finalQuery) return;
    await openUrl(buildGoogleImageSearchUrl(finalQuery)).catch(console.error);
  };

  const openYouTubeSearch = async (query?: string) => {
    const finalQuery = query || searchData?.searchQuery;
    if (!finalQuery) return;
    await openUrl(buildYouTubeSearchUrl(finalQuery)).catch(console.error);
  };

  const runAction = async (mode: SnipMode) => {
    if (!path || busy) return;

    setBusy(true);
    setResult("");

    if (mode !== "search") {
      setSearchData(null);
    }

    try {
      const response = await invoke<string>("analyze_snip", {
        mode,
        comment,
        imagePath: path,
        appName,
        windowTitle,
      });

      setResult(response);

      if (mode === "search") {
        setSearchData(parseSearchResult(response));
      }
    } catch (error) {
      setResult(`Snip analysis failed: ${String(error)}`);
      if (mode === "search") {
        setSearchData(null);
      }
    } finally {
      setBusy(false);
    }
  };

  const closePanel = async () => {
    await getCurrentWindow().hide().catch(console.error);
  };

  const glassButtonBase: React.CSSProperties = {
    height: 44,
    border: "1px solid rgba(255,255,255,0.10)",
    background: "rgba(255,255,255,0.08)",
    color: "#eef4ff",
    borderRadius: 16,
    padding: "0 14px",
    cursor: "pointer",
    fontWeight: 700,
    fontSize: 13,
    transition: "all 160ms ease",
    backdropFilter: "blur(10px)",
    WebkitBackdropFilter: "blur(10px)",
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    gap: 8,
  };

  const primaryButtonStyle: React.CSSProperties = {
    ...glassButtonBase,
    background:
      "linear-gradient(180deg, rgba(117,163,255,0.22), rgba(117,163,255,0.12))",
    border: "1px solid rgba(140,180,255,0.26)",
  };

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
          --panel-base: rgba(15, 19, 28, 0.90);
          --panel-tint-top: rgba(255, 255, 255, 0.18);
          --panel-tint-bottom: rgba(255, 255, 255, 0.08);
          --panel-border: rgba(255, 255, 255, 0.14);
          --panel-border-soft: rgba(255, 255, 255, 0.08);

          --text-main: rgba(238, 244, 255, 0.98);
          --text-soft: rgba(205, 217, 241, 0.78);
          --text-dim: rgba(205, 217, 241, 0.56);

          --glass-chip: rgba(255, 255, 255, 0.06);
          --glass-chip-strong: rgba(255, 255, 255, 0.10);
          --accent: rgba(117, 163, 255, 1);
          --accent-soft: rgba(117, 163, 255, 0.16);
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
          color: var(--text-main);
        }

        .snip-shell {
          width: 100%;
          height: 100%;
          padding: 14px;
          background: transparent;
        }

        .snip-panel {
          width: 100%;
          height: 100%;
          border-radius: 30px;
          overflow: hidden;
          border: 1px solid rgba(255,255,255,0.14);
          background:
            linear-gradient(
              180deg,
              rgba(255,255,255,0.18),
              rgba(255,255,255,0.08)
            ),
            rgba(15, 19, 28, 0.90);
          backdrop-filter: blur(30px) saturate(145%);
          -webkit-backdrop-filter: blur(30px) saturate(145%);
          isolation: isolate;
        }

        .snip-panel::before {
          content: "";
          position: absolute;
          inset: 0;
          border-radius: inherit;
          pointer-events: none;
          background:
            radial-gradient(circle at 10% 0%, rgba(255,255,255,0.16), transparent 28%),
            radial-gradient(circle at 100% 100%, rgba(117,163,255,0.12), transparent 22%);
        }

        .snip-panel::after {
          content: "";
          position: absolute;
          inset: 0;
          border-radius: inherit;
          pointer-events: none;
          box-shadow:
            inset 1px 1px 0 rgba(255,255,255,0.24),
            inset -1px -1px 0 rgba(255,255,255,0.04);
        }

        .snip-header {
          position: relative;
          z-index: 1;
          display: grid;
          grid-template-columns: 1fr auto;
          align-items: center;
          gap: 12px;
          padding: 14px 16px;
          border-bottom: 1px solid var(--panel-border-soft);
        }

        .snip-header-left {
          min-width: 0;
          display: flex;
          align-items: center;
          gap: 12px;
        }

        .snip-brand {
          width: 42px;
          height: 42px;
          border-radius: 14px;
          display: grid;
          place-items: center;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.08);
          color: var(--text-main);
          flex-shrink: 0;
        }

        .snip-title {
          font-size: 15px;
          font-weight: 800;
          letter-spacing: 0.01em;
        }

        .snip-subtitle {
          margin-top: 2px;
          font-size: 12px;
          color: var(--text-soft);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .snip-window-title {
          margin-top: 2px;
          font-size: 11px;
          color: var(--text-dim);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .snip-close {
          width: 40px;
          height: 40px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.08);
          color: white;
          display: grid;
          place-items: center;
          cursor: pointer;
          transition: all 160ms ease;
        }

        .snip-close:hover {
          background: rgba(255,255,255,0.14);
        }

        .snip-content {
          position: relative;
          z-index: 1;
          height: calc(100% - 71px);
          overflow: auto;
          padding: 14px;
          display: grid;
          gap: 12px;
        }

        .snip-content::-webkit-scrollbar {
          width: 10px;
        }

        .snip-content::-webkit-scrollbar-thumb {
          background: rgba(255,255,255,0.10);
          border-radius: 999px;
        }

        .glass-card {
          border-radius: 22px;
          border: 1px solid rgba(255,255,255,0.08);
          background:
            linear-gradient(
              180deg,
              rgba(255,255,255,0.08),
              rgba(255,255,255,0.04)
            ),
            rgba(255,255,255,0.02);
          overflow: hidden;
        }

        .preview-card {
          min-height: 260px;
          display: grid;
          place-items: center;
          position: relative;
        }

        .preview-empty {
          font-size: 13px;
          color: var(--text-soft);
          padding: 20px;
          text-align: center;
        }

        .preview-error {
          padding: 16px;
          font-size: 12px;
          line-height: 1.55;
          color: var(--text-soft);
          white-space: pre-wrap;
        }

        .preview-image {
          width: 100%;
          display: block;
          max-height: 360px;
          object-fit: contain;
          background: rgba(0,0,0,0.18);
        }

        .info-chip {
          padding: 12px 14px;
          border-radius: 18px;
          font-size: 11px;
          line-height: 1.5;
          color: var(--text-soft);
          background: rgba(255,255,255,0.04);
          border: 1px solid rgba(255,255,255,0.06);
          word-break: break-word;
        }

        .info-chip strong {
          color: var(--text-main);
        }

        .comment-box {
          width: 100%;
          min-height: 100px;
          resize: vertical;
          border-radius: 18px;
          padding: 14px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.05);
          color: var(--text-main);
          outline: none;
          font: 500 13px/1.5 Inter, system-ui, sans-serif;
        }

        .comment-box::placeholder {
          color: var(--text-dim);
        }

        .comment-box:focus {
          border-color: rgba(140,180,255,0.26);
          box-shadow: 0 0 0 4px rgba(117,163,255,0.10);
        }

        .actions-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 8px;
        }

        .result-box {
          padding: 14px;
          border-radius: 20px;
          background: rgba(255,255,255,0.05);
          border: 1px solid rgba(255,255,255,0.08);
          white-space: pre-wrap;
          line-height: 1.6;
          font-size: 13px;
        }

        .search-box {
          display: grid;
          gap: 8px;
        }

        .search-summary {
          padding: 12px 14px;
          border-radius: 18px;
          font-size: 12px;
          line-height: 1.5;
          color: rgba(238,244,255,0.82);
          background: rgba(117,163,255,0.08);
          border: 1px solid rgba(140,180,255,0.18);
        }

        .search-summary strong {
          color: var(--text-main);
        }

        .bottom-links {
          display: flex;
          justify-content: center;
          gap: 14px;
          flex-wrap: wrap;
          min-height: 16px;
        }

        .tiny-link {
          appearance: none;
          border: 0;
          background: transparent;
          padding: 0;
          font-size: 11px;
          color: rgba(255,255,255,0.68);
          cursor: pointer;
        }

        .tiny-link:hover {
          color: rgba(255,255,255,0.94);
        }

        .tiny-link-static {
          cursor: default;
        }

        .busy-line {
          font-size: 12px;
          color: var(--text-soft);
          padding: 0 2px;
        }

        @media (max-width: 640px) {
          .actions-grid {
            grid-template-columns: 1fr;
          }
        }
      `}</style>

      <div className="snip-shell">
        <div className="snip-panel">
          <div className="snip-header">
            <div className="snip-header-left">
              <div className="snip-brand">
                <Sparkles size={18} />
              </div>

              <div style={{ minWidth: 0 }}>
                <div className="snip-title">Snip to Blob</div>
                <div className="snip-subtitle">{appName}</div>
                {!!windowTitle && (
                  <div className="snip-window-title">{windowTitle}</div>
                )}
              </div>
            </div>

            <button
              className="snip-close"
              onClick={closePanel}
              title="Schließen"
            >
              <X size={16} />
            </button>
          </div>

          <div className="snip-content">
            <div className="glass-card preview-card">
              {!path && (
                <div className="preview-empty">No snip loaded yet.</div>
              )}

              {!!path && previewState === "loading" && (
                <div className="preview-empty">Loading preview…</div>
              )}

              {!!path && previewState === "error" && (
                <div className="preview-error">
                  {previewError || "Preview failed to load."}
                </div>
              )}

              {!!path && !!previewUrl && (
                <img
                  key={previewUrl}
                  src={previewUrl}
                  alt="Snip preview"
                  onLoad={() => setPreviewState("ready")}
                  onError={() => {
                    setPreviewState("error");
                    setPreviewError(
                      `Preview failed to load.\n\nPath:\n${path}\n\nURL:\n${previewUrl}`
                    );
                  }}
                  className="preview-image"
                  style={{ display: hasPreview ? "block" : "none" }}
                />
              )}
            </div>

            {!!path && (
              <div className="info-chip">
                <strong>Path:</strong> {path}
                {!!contextDomain && (
                  <>
                    <br />
                    <strong>Domain:</strong> {contextDomain}
                  </>
                )}
              </div>
            )}

            <textarea
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              placeholder="How do I solve this quest? / where is this item? / what does this error mean?"
              className="comment-box"
            />

            <div className="actions-grid">
              <button
                style={glassButtonBase}
                onClick={() => runAction("explain")}
              >
                <Sparkles size={16} />
                Explain
              </button>

              <button
                style={glassButtonBase}
                onClick={() => runAction("translate")}
              >
                <Languages size={16} />
                Translate
              </button>

              <button style={glassButtonBase} onClick={() => runAction("ocr")}>
                <ScanText size={16} />
                Extract Text
              </button>

              <button
                style={primaryButtonStyle}
                onClick={() => runAction("search")}
              >
                <Search size={16} />
                Search Help
              </button>
            </div>

            {busy && <div className="busy-line">Analyzing…</div>}

            {!!result && (
              <>
                <div className="result-box">{result}</div>

                {!!searchData?.searchQuery && (
                  <div className="search-box">
                    <div className="search-summary">
                      <strong>Detected intent:</strong>{" "}
                      {searchData.intent || "unknown"}
                      <br />
                      <strong>Detected game/app:</strong>{" "}
                      {searchData.gameOrApp || "unknown"}
                      <br />
                      <strong>Key text:</strong>{" "}
                      {searchData.keyText || "unknown"}
                      <br />
                      <strong>Best query:</strong> {searchData.searchQuery}
                    </div>

                    <button
                      style={primaryButtonStyle}
                      onClick={() => openWebSearch(searchData.searchQuery)}
                    >
                      <ExternalLink size={16} />
                      Open Best Guide Search
                    </button>

                    {!!searchData.altQuery1 &&
                      searchData.altQuery1 !== "unknown" && (
                        <button
                          style={glassButtonBase}
                          onClick={() => openWebSearch(searchData.altQuery1)}
                        >
                          <ExternalLink size={16} />
                          Open Broader Search
                        </button>
                      )}

                    <div className="actions-grid">
                      <button
                        style={glassButtonBase}
                        onClick={() =>
                          openImageSearch(
                            searchData.altQuery2 || searchData.searchQuery
                          )
                        }
                      >
                        <ImageIcon size={16} />
                        Image Search
                      </button>

                      <button
                        style={glassButtonBase}
                        onClick={() =>
                          openYouTubeSearch(
                            searchData.altQuery2 || searchData.searchQuery
                          )
                        }
                      >
                        <Youtube size={16} />
                        YouTube Search
                      </button>
                    </div>
                  </div>
                )}

                <div className="actions-grid">
                  <button
                    style={glassButtonBase}
                    onClick={async () => {
                      await writeText(result).catch(console.error);
                    }}
                  >
                    <Copy size={16} />
                    Copy Result
                  </button>

                  <button
                    style={glassButtonBase}
                    onClick={async () => {
                      await invoke("emit_snip_result_to_bubble", {
                        text: result,
                      }).catch(console.error);
                    }}
                  >
                    <Send size={16} />
                    Send to Bubble
                  </button>
                </div>
              </>
            )}

            <div className="bottom-links">
              <span className="tiny-link tiny-link-static">snip panel</span>
              <span className="tiny-link tiny-link-static">
                later: screenshot shortcut link
              </span>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <SnipPanel />
  </React.StrictMode>
);
