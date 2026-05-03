import React, { useEffect, useMemo, useRef, useState } from "react";
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

type UiLang = "en" | "de";
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

type LocalizedText = {
  title: string;
  close: string;
  noSnipLoaded: string;
  loadingPreview: string;
  previewFailed: string;
  path: string;
  domain: string;
  commentPlaceholder: string;
  explain: string;
  translate: string;
  extractText: string;
  searchHelp: string;
  analyzing: string;
  detectedIntent: string;
  detectedGameOrApp: string;
  keyText: string;
  bestQuery: string;
  unknown: string;
  openBestGuideSearch: string;
  openBroaderSearch: string;
  imageSearch: string;
  youtubeSearch: string;
  copyResult: string;
  sendToBubble: string;
  snipPanel: string;
  contextualHelper: string;
};

const TEXTS: Record<UiLang, LocalizedText> = {
  en: {
    title: "Snip to Blob",
    close: "Close",
    noSnipLoaded: "No snip loaded yet.",
    loadingPreview: "Loading preview…",
    previewFailed: "Preview failed to load.",
    path: "Path",
    domain: "Domain",
    commentPlaceholder:
      "How do I solve this quest? / where is this item? / what does this error mean?",
    explain: "Explain",
    translate: "Translate",
    extractText: "Extract Text",
    searchHelp: "Search Help",
    analyzing: "Analyzing…",
    detectedIntent: "Detected intent",
    detectedGameOrApp: "Detected game/app",
    keyText: "Key text",
    bestQuery: "Best query",
    unknown: "unknown",
    openBestGuideSearch: "Open Best Guide Search",
    openBroaderSearch: "Open Broader Search",
    imageSearch: "Image Search",
    youtubeSearch: "YouTube Search",
    copyResult: "Copy Result",
    sendToBubble: "Send to Bubble",
    snipPanel: "snip panel",
    contextualHelper: "contextual helper",
  },
  de: {
    title: "Snip zu Blob",
    close: "Schließen",
    noSnipLoaded: "Noch kein Snip geladen.",
    loadingPreview: "Vorschau wird geladen…",
    previewFailed: "Vorschau konnte nicht geladen werden.",
    path: "Pfad",
    domain: "Domäne",
    commentPlaceholder:
      "Wie löse ich diese Quest? / wo ist dieses Item? / was bedeutet dieser Fehler?",
    explain: "Erklären",
    translate: "Übersetzen",
    extractText: "Text extrahieren",
    searchHelp: "Suchhilfe",
    analyzing: "Analysiere…",
    detectedIntent: "Erkannte Absicht",
    detectedGameOrApp: "Erkanntes Spiel/App",
    keyText: "Schlüsseltext",
    bestQuery: "Beste Suchanfrage",
    unknown: "unbekannt",
    openBestGuideSearch: "Beste Guide-Suche öffnen",
    openBroaderSearch: "Breitere Suche öffnen",
    imageSearch: "Bildersuche",
    youtubeSearch: "YouTube-Suche",
    copyResult: "Ergebnis kopieren",
    sendToBubble: "An Bubble senden",
    snipPanel: "snip panel",
    contextualHelper: "kontexthelfer",
  },
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
  const [uiLang, setUiLang] = useState<UiLang>("en");
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
  const [isMacOS] = useState(() =>
    /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent)
  );

  const busyRef = useRef(false);
  const t = TEXTS[uiLang];

  useEffect(() => {
    document.documentElement.classList.toggle("macos-lite", isMacOS);
  }, [isMacOS]);

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
        console.error("failed to load identity for snip panel ui", error);
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
          console.error("failed to refresh identity for snip panel ui", error);
        }
      });
    };

    void setupIdentityListener();

    return () => {
      unlistenIdentityUpdated?.();
    };
  }, []);

  useEffect(() => {
    busyRef.current = busy;
  }, [busy]);

  const hasPreview = useMemo(
    () => !!previewUrl && previewState === "ready",
    [previewUrl, previewState]
  );

  const resetPanelState = () => {
    setPreviewUrl("");
    setComment("");
    setResult("");
    setBusy(false);
    setSearchData(null);
    setPreviewError("");
    setPreviewState("idle");
    busyRef.current = false;
  };

  useEffect(() => {
    let unlisten: null | (() => void) = null;

    const setup = async () => {
      unlisten = await listen<SnipPayload>("snip-panel-data", async (event) => {
        const nextPath = event.payload.path || "";
        const normalizedPath = normalizeWindowsPath(nextPath);

        resetPanelState();

        setPath(nextPath);
        setAppName(event.payload.app || t.unknown);
        setWindowTitle(event.payload.windowTitle || "");
        setContextDomain(event.payload.contextDomain || "");

        const exists = await invoke<boolean>("snip_file_exists", {
          path: nextPath,
        }).catch(() => false);

        if (!exists || !nextPath) {
          setPreviewState("error");
          setPreviewError(
            nextPath
              ? `Snip file missing: ${nextPath}`
              : "No snip path received."
          );
        } else {
          setPreviewState("loading");
          setPreviewUrl(convertFileSrc(normalizedPath));
        }

        const win = getCurrentWindow();
        await win.show().catch(() => {});
        await win.setFocus().catch(() => {});
      });
    };

    void setup();

    return () => {
      unlisten?.();
    };
  }, [t.unknown]);

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
    if (!path || busyRef.current) return;

    busyRef.current = true;
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
      busyRef.current = false;
      setBusy(false);
    }
  };

  const closePanel = async () => {
    await getCurrentWindow().hide().catch(console.error);
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
          --glass-fill: rgba(255,255,255,0.06);
          --glass-fill-hover: rgba(255,255,255,0.12);
          --glass-border: rgba(255,255,255,0.14);
          --glass-border-soft: rgba(255,255,255,0.08);

          --blue: rgba(10, 132, 255, 0.92);
          --blue-soft: rgba(10, 132, 255, 0.14);
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

        .snip-shell {
          width: 100%;
          height: 100%;
          padding: 12px;
          background: transparent;
        }

        .snip-panel {
          position: relative;
          width: 100%;
          height: 100%;
          border-radius: 30px;
          overflow: hidden;
          isolation: isolate;
          background: var(--glass-bg);
          backdrop-filter: blur(26px) saturate(150%);
          -webkit-backdrop-filter: blur(26px) saturate(150%);
          border: 1px solid var(--glass-border);
          box-shadow:
            inset 0 1px 1px rgba(255,255,255,0.16),
            inset 0 -1px 1px rgba(0,0,0,0.18);
        }

        .macos-lite .snip-panel {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
          background: rgba(18, 20, 26, 0.74);
        }

        .snip-panel::before {
          content: "";
          position: absolute;
          inset: 0;
          pointer-events: none;
          border-radius: inherit;
          background:
            radial-gradient(circle at 12% 0%, rgba(255,255,255,0.12), transparent 30%),
            radial-gradient(circle at 100% 100%, rgba(117,163,255,0.10), transparent 22%);
        }

        .snip-header {
          position: relative;
          z-index: 1;
          display: grid;
          grid-template-columns: 1fr auto;
          align-items: center;
          gap: 12px;
          padding: 14px 16px;
          border-bottom: 1px solid rgba(255,255,255,0.06);
          background: linear-gradient(
            180deg,
            rgba(255,255,255,0.05),
            rgba(255,255,255,0.01)
          );
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
          border-color: rgba(255,255,255,0.16);
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
          background: rgba(255,255,255,0.12);
          border-radius: 999px;
        }

        .glass-card {
          border-radius: 22px;
          border: 1px solid rgba(255,255,255,0.08);
          background: rgba(255,255,255,0.05);
          overflow: hidden;
          backdrop-filter: blur(14px) saturate(135%);
          -webkit-backdrop-filter: blur(14px) saturate(135%);
        }

        .macos-lite .glass-card {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
        }

        .preview-card {
          min-height: 260px;
          display: grid;
          place-items: center;
          position: relative;
          background:
            linear-gradient(
              180deg,
              rgba(255,255,255,0.06),
              rgba(255,255,255,0.03)
            ),
            rgba(255,255,255,0.02);
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
          background: transparent;
          color: var(--text-main);
          outline: none;
          font: 500 13px/1.55 Inter, system-ui, sans-serif;
          backdrop-filter: blur(12px);
          -webkit-backdrop-filter: blur(12px);
        }

        .macos-lite .comment-box {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
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

        .action-btn {
          height: 44px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.08);
          color: #eef4ff;
          border-radius: 16px;
          padding: 0 14px;
          cursor: pointer;
          font-weight: 700;
          font-size: 13px;
          transition: all 160ms ease;
          backdrop-filter: blur(10px);
          -webkit-backdrop-filter: blur(10px);
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
        }

        .macos-lite .action-btn {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
        }

        .action-btn:hover {
          background: rgba(255,255,255,0.14);
          border-color: rgba(255,255,255,0.16);
          transform: translateY(-1px);
        }

        .action-btn:active {
          transform: translateY(0);
        }

        .action-btn-primary {
          background:
            linear-gradient(
              180deg,
              rgba(10,132,255,0.22),
              rgba(10,132,255,0.12)
            );
          border: 1px solid rgba(10,132,255,0.26);
        }

        .action-btn-primary:hover {
          background:
            linear-gradient(
              180deg,
              rgba(10,132,255,0.28),
              rgba(10,132,255,0.16)
            );
        }

        .busy-line {
          font-size: 12px;
          color: var(--text-soft);
          padding: 0 2px;
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
          line-height: 1.55;
          color: rgba(238,244,255,0.82);
          background: rgba(10,132,255,0.08);
          border: 1px solid rgba(10,132,255,0.18);
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
          color: rgba(255,255,255,0.64);
          cursor: pointer;
          transition: color 0.16s ease;
        }

        .tiny-link:hover {
          color: rgba(255,255,255,0.92);
        }

        .tiny-link-static {
          cursor: default;
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
                <div className="snip-title">{t.title}</div>
                <div className="snip-subtitle">{appName}</div>
                {!!windowTitle && (
                  <div className="snip-window-title">{windowTitle}</div>
                )}
              </div>
            </div>

            <button className="snip-close" onClick={closePanel} title={t.close}>
              <X size={16} />
            </button>
          </div>

          <div className="snip-content">
            <div className="glass-card preview-card">
              {!path && <div className="preview-empty">{t.noSnipLoaded}</div>}

              {!!path && previewState === "loading" && (
                <div className="preview-empty">{t.loadingPreview}</div>
              )}

              {!!path && previewState === "error" && (
                <div className="preview-error">
                  {previewError || t.previewFailed}
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
                      `${t.previewFailed}\n\nPath:\n${path}\n\nURL:\n${previewUrl}`
                    );
                  }}
                  className="preview-image"
                  style={{ display: hasPreview ? "block" : "none" }}
                />
              )}
            </div>

            {!!path && (
              <div className="info-chip">
                <strong>{t.path}:</strong> {path}
                {!!contextDomain && (
                  <>
                    <br />
                    <strong>{t.domain}:</strong> {contextDomain}
                  </>
                )}
              </div>
            )}

            <textarea
              value={comment}
              onChange={(e) => setComment(e.target.value)}
              placeholder={t.commentPlaceholder}
              className="comment-box"
            />

            <div className="actions-grid">
              <button
                className="action-btn"
                onClick={() => runAction("explain")}
              >
                <Sparkles size={16} />
                {t.explain}
              </button>

              <button
                className="action-btn"
                onClick={() => runAction("translate")}
              >
                <Languages size={16} />
                {t.translate}
              </button>

              <button className="action-btn" onClick={() => runAction("ocr")}>
                <ScanText size={16} />
                {t.extractText}
              </button>

              <button
                className="action-btn action-btn-primary"
                onClick={() => runAction("search")}
              >
                <Search size={16} />
                {t.searchHelp}
              </button>
            </div>

            {busy && <div className="busy-line">{t.analyzing}</div>}

            {!!result && (
              <>
                <div className="result-box">{result}</div>

                {!!searchData?.searchQuery && (
                  <div className="search-box">
                    <div className="search-summary">
                      <strong>{t.detectedIntent}:</strong>{" "}
                      {searchData.intent || t.unknown}
                      <br />
                      <strong>{t.detectedGameOrApp}:</strong>{" "}
                      {searchData.gameOrApp || t.unknown}
                      <br />
                      <strong>{t.keyText}:</strong>{" "}
                      {searchData.keyText || t.unknown}
                      <br />
                      <strong>{t.bestQuery}:</strong> {searchData.searchQuery}
                    </div>

                    <button
                      className="action-btn action-btn-primary"
                      onClick={() => openWebSearch(searchData.searchQuery)}
                    >
                      <ExternalLink size={16} />
                      {t.openBestGuideSearch}
                    </button>

                    {!!searchData.altQuery1 &&
                      searchData.altQuery1 !== "unknown" &&
                      searchData.altQuery1 !== "unbekannt" && (
                        <button
                          className="action-btn"
                          onClick={() => openWebSearch(searchData.altQuery1)}
                        >
                          <ExternalLink size={16} />
                          {t.openBroaderSearch}
                        </button>
                      )}

                    <div className="actions-grid">
                      <button
                        className="action-btn"
                        onClick={() =>
                          openImageSearch(
                            searchData.altQuery2 || searchData.searchQuery
                          )
                        }
                      >
                        <ImageIcon size={16} />
                        {t.imageSearch}
                      </button>

                      <button
                        className="action-btn"
                        onClick={() =>
                          openYouTubeSearch(
                            searchData.altQuery2 || searchData.searchQuery
                          )
                        }
                      >
                        <Youtube size={16} />
                        {t.youtubeSearch}
                      </button>
                    </div>
                  </div>
                )}

                <div className="actions-grid">
                  <button
                    className="action-btn"
                    onClick={async () => {
                      await writeText(result).catch(console.error);
                    }}
                  >
                    <Copy size={16} />
                    {t.copyResult}
                  </button>

                  <button
                    className="action-btn"
                    onClick={async () => {
                      await invoke("emit_snip_result_to_bubble", {
                        text: result,
                      }).catch(console.error);
                    }}
                  >
                    <Send size={16} />
                    {t.sendToBubble}
                  </button>
                </div>
              </>
            )}

            <div className="bottom-links">
              <span className="tiny-link tiny-link-static">{t.snipPanel}</span>
              <span className="tiny-link tiny-link-static">
                {t.contextualHelper}
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
