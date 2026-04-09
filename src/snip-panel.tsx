import React, { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen } from "@tauri-apps/api/event";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";

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
  const extractedText = extractField(text, "EXTRACTED_TEXT");

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

    setup();

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

  const buttonStyle: React.CSSProperties = {
    border: "1px solid rgba(255,255,255,0.1)",
    background: "rgba(255,255,255,0.06)",
    color: "#eef4ff",
    borderRadius: 14,
    padding: "12px 14px",
    cursor: "pointer",
    fontWeight: 700,
    fontSize: 13,
    transition: "all 160ms ease",
    backdropFilter: "blur(10px)",
    boxShadow: "0 8px 24px rgba(0,0,0,0.18)",
  };

  const primaryButtonStyle: React.CSSProperties = {
    ...buttonStyle,
    background:
      "linear-gradient(180deg, rgba(117,163,255,0.22), rgba(117,163,255,0.12))",
    border: "1px solid rgba(140,180,255,0.28)",
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        background:
          "radial-gradient(circle at top, rgba(70,100,190,0.18), transparent 30%), linear-gradient(180deg, rgba(8,12,20,0.97), rgba(13,18,30,0.94))",
        backdropFilter: "blur(24px)",
        border: "1px solid rgba(255,255,255,0.08)",
        color: "#eef4ff",
        padding: 16,
        boxSizing: "border-box",
        fontFamily: "Inter, system-ui, sans-serif",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 14,
        }}
      >
        <div>
          <div style={{ fontSize: 17, fontWeight: 800, letterSpacing: 0.2 }}>
            Snip to Blob
          </div>
          <div style={{ fontSize: 12, opacity: 0.72 }}>{appName}</div>
          {!!windowTitle && (
            <div style={{ fontSize: 11, opacity: 0.56, marginTop: 2 }}>
              {windowTitle}
            </div>
          )}
        </div>

        <button
          onClick={() => getCurrentWindow().hide()}
          style={{
            width: 36,
            height: 36,
            borderRadius: 12,
            border: "1px solid rgba(255,255,255,0.08)",
            background: "rgba(255,255,255,0.06)",
            color: "#fff",
            cursor: "pointer",
            fontSize: 18,
            lineHeight: 1,
          }}
        >
          ×
        </button>
      </div>

      <div
        style={{
          borderRadius: 20,
          overflow: "hidden",
          border: "1px solid rgba(255,255,255,0.08)",
          marginBottom: 12,
          background:
            "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(255,255,255,0.02))",
          minHeight: 220,
          display: "grid",
          placeItems: "center",
          position: "relative",
        }}
      >
        {!path && (
          <div style={{ fontSize: 13, opacity: 0.68 }}>No snip loaded yet.</div>
        )}

        {!!path && previewState === "loading" && (
          <div style={{ fontSize: 13, opacity: 0.72 }}>Loading preview…</div>
        )}

        {!!path && previewState === "error" && (
          <div
            style={{
              padding: 14,
              fontSize: 12,
              lineHeight: 1.5,
              opacity: 0.82,
              whiteSpace: "pre-wrap",
            }}
          >
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
            style={{
              width: "100%",
              display: hasPreview ? "block" : "none",
              maxHeight: 340,
              objectFit: "contain",
              background: "rgba(0,0,0,0.18)",
            }}
          />
        )}
      </div>

      {!!path && (
        <div
          style={{
            marginBottom: 12,
            padding: 10,
            borderRadius: 14,
            fontSize: 11,
            lineHeight: 1.45,
            color: "rgba(238,244,255,0.72)",
            background: "rgba(255,255,255,0.035)",
            border: "1px solid rgba(255,255,255,0.06)",
            wordBreak: "break-all",
          }}
        >
          <strong style={{ color: "#eef4ff" }}>Path:</strong> {path}
          {!!contextDomain && (
            <>
              <br />
              <strong style={{ color: "#eef4ff" }}>Domain:</strong>{" "}
              {contextDomain}
            </>
          )}
        </div>
      )}

      <textarea
        value={comment}
        onChange={(e) => setComment(e.target.value)}
        placeholder="How do I solve this quest? / where is this item? / what does this error mean?"
        style={{
          width: "100%",
          minHeight: 96,
          resize: "vertical",
          borderRadius: 16,
          padding: 12,
          boxSizing: "border-box",
          border: "1px solid rgba(255,255,255,0.1)",
          background: "rgba(255,255,255,0.04)",
          color: "#eef4ff",
          outline: "none",
          marginBottom: 12,
          font: "500 13px/1.45 Inter, system-ui, sans-serif",
        }}
      />

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 8,
          marginBottom: 12,
        }}
      >
        <button style={buttonStyle} onClick={() => runAction("explain")}>
          Explain
        </button>
        <button style={buttonStyle} onClick={() => runAction("translate")}>
          Translate
        </button>
        <button style={buttonStyle} onClick={() => runAction("ocr")}>
          Extract Text
        </button>
        <button style={primaryButtonStyle} onClick={() => runAction("search")}>
          Search Help
        </button>
      </div>

      {busy && (
        <div style={{ fontSize: 12, opacity: 0.75, marginBottom: 8 }}>
          Analyzing…
        </div>
      )}

      {!!result && (
        <>
          <div
            style={{
              marginTop: 12,
              padding: 12,
              borderRadius: 16,
              background: "rgba(255,255,255,0.05)",
              border: "1px solid rgba(255,255,255,0.08)",
              whiteSpace: "pre-wrap",
              lineHeight: 1.55,
              fontSize: 13,
            }}
          >
            {result}
          </div>

          {!!searchData?.searchQuery && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr",
                gap: 8,
                marginTop: 12,
              }}
            >
              <div
                style={{
                  padding: 10,
                  borderRadius: 14,
                  fontSize: 12,
                  lineHeight: 1.45,
                  color: "rgba(238,244,255,0.78)",
                  background: "rgba(117,163,255,0.08)",
                  border: "1px solid rgba(140,180,255,0.18)",
                }}
              >
                <strong style={{ color: "#eef4ff" }}>Detected intent:</strong>{" "}
                {searchData.intent || "unknown"}
                <br />
                <strong style={{ color: "#eef4ff" }}>
                  Detected game/app:
                </strong>{" "}
                {searchData.gameOrApp || "unknown"}
                <br />
                <strong style={{ color: "#eef4ff" }}>Key text:</strong>{" "}
                {searchData.keyText || "unknown"}
                <br />
                <strong style={{ color: "#eef4ff" }}>Best query:</strong>{" "}
                {searchData.searchQuery}
              </div>

              <button
                style={primaryButtonStyle}
                onClick={() => openWebSearch(searchData.searchQuery)}
              >
                Open Best Guide Search
              </button>

              {!!searchData.altQuery1 && searchData.altQuery1 !== "unknown" && (
                <button
                  style={buttonStyle}
                  onClick={() => openWebSearch(searchData.altQuery1)}
                >
                  Open Broader Search
                </button>
              )}

              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: 8,
                }}
              >
                <button
                  style={buttonStyle}
                  onClick={() =>
                    openImageSearch(
                      searchData.altQuery2 || searchData.searchQuery
                    )
                  }
                >
                  Image Search
                </button>

                <button
                  style={buttonStyle}
                  onClick={() =>
                    openYouTubeSearch(
                      searchData.altQuery2 || searchData.searchQuery
                    )
                  }
                >
                  YouTube Search
                </button>
              </div>
            </div>
          )}

          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr 1fr",
              gap: 8,
              marginTop: 12,
            }}
          >
            <button
              style={buttonStyle}
              onClick={async () => {
                await writeText(result).catch(console.error);
              }}
            >
              Copy Result
            </button>

            <button
              style={buttonStyle}
              onClick={async () => {
                await invoke("emit_snip_result_to_bubble", {
                  text: result,
                }).catch(console.error);
              }}
            >
              Send to Bubble
            </button>
          </div>
        </>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <SnipPanel />
  </React.StrictMode>
);
