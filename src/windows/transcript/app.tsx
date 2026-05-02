import React, { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { listen, emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  AudioLines,
  Bot,
  CheckSquare,
  LoaderCircle,
  Mic,
  Play,
  Save,
  Square,
  UserRound,
  Waves,
  X,
} from "lucide-react";

type UiLang = "en" | "de";

type TranscriptSegment = {
  start_ms: number;
  end_ms: number;
  speaker?: string | null;
  text: string;
  confidence?: number | null;
};

type TranscriptSession = {
  id: string;
  state: string;
  started_at: string;
  ended_at?: string | null;
  context: {
    app_name?: string | null;
    window_title?: string | null;
  };
  segments: TranscriptSegment[];
};

type TranscriptStatus = {
  state: "Idle" | "Recording" | "Stopping" | "Summarizing" | "Error";
  active_session_id: string | null;
  segment_count: number;
};

type TranscriptPrereqs = {
  ok: boolean;
  default_input_device: string | null;
  whisper_exe: string | null;
  whisper_model: string | null;
  needs_virtual_audio_routing: boolean;
  message: string;
};

type SpeakerBlock = {
  speaker: string;
  text: string;
};

type ProcessedTranscriptResult = {
  faithful_transcript: string;
  speaker_blocks: SpeakerBlock[];
  summary: string;
  action_items: string[];
};

type LocalizedText = {
  title: string;
  close: string;

  startTranscript: string;
  stopTranscript: string;
  save: string;
  process: string;

  session: string;
  segments: string;
  app: string;

  processing: string;
  working: string;
  recording: string;
  idle: string;

  rawCleanTranscript: string;
  rawCleanTranscriptSubtitle: string;
  noTranscriptTextYet: string;

  processedOutput: string;
  processedOutputSubtitle: string;
  noProcessedOutputYet: string;
  processStrong: string;

  faithfulTranscript: string;
  speakerBlocks: string;
  noSpeakerBlocksReturned: string;
  summary: string;
  noSummaryReturned: string;
  actionItems: string;
  noActionItemsFound: string;

  liveSegments: string;
  liveSegmentsSubtitle: string;
  noTranscriptSegmentsYet: string;

  unknownApp: string;
};

const TEXTS: Record<UiLang, LocalizedText> = {
  en: {
    title: "Transcript Studio",
    close: "Close",

    startTranscript: "Start Transcript",
    stopTranscript: "Stop Transcript",
    save: "Save",
    process: "Process",

    session: "session",
    segments: "segments",
    app: "app",

    processing: "processing",
    working: "working",
    recording: "recording",
    idle: "idle",

    rawCleanTranscript: "Raw Clean Transcript",
    rawCleanTranscriptSubtitle:
      "direct cleaned stream from the current session",
    noTranscriptTextYet: "No transcript text yet.",

    processedOutput: "Processed Output",
    processedOutputSubtitle:
      "faithful transcript, speaker grouping and summary",
    noProcessedOutputYet: "No processed output yet. Click",
    processStrong: "Process",

    faithfulTranscript: "Faithful Transcript",
    speakerBlocks: "Speaker Blocks",
    noSpeakerBlocksReturned: "No speaker blocks returned.",
    summary: "Summary",
    noSummaryReturned: "No summary returned.",
    actionItems: "Action Items",
    noActionItemsFound: "No action items found.",

    liveSegments: "Live Segments",
    liveSegmentsSubtitle: "latest unique timestamped chunks",
    noTranscriptSegmentsYet: "No transcript segments yet.",

    unknownApp: "-",
  },
  de: {
    title: "Transcript Studio",
    close: "Schließen",

    startTranscript: "Transkript starten",
    stopTranscript: "Transkript stoppen",
    save: "Speichern",
    process: "Verarbeiten",

    session: "Sitzung",
    segments: "Segmente",
    app: "App",

    processing: "verarbeite",
    working: "arbeite",
    recording: "nimmt auf",
    idle: "inaktiv",

    rawCleanTranscript: "Rohes bereinigtes Transkript",
    rawCleanTranscriptSubtitle:
      "direkter bereinigter Stream der aktuellen Sitzung",
    noTranscriptTextYet: "Noch kein Transkripttext vorhanden.",

    processedOutput: "Verarbeitetes Ergebnis",
    processedOutputSubtitle:
      "treues Transkript, Sprechergruppierung und Zusammenfassung",
    noProcessedOutputYet: "Noch kein verarbeitetes Ergebnis. Klicke auf",
    processStrong: "Verarbeiten",

    faithfulTranscript: "Treues Transkript",
    speakerBlocks: "Sprecherblöcke",
    noSpeakerBlocksReturned: "Keine Sprecherblöcke zurückgegeben.",
    summary: "Zusammenfassung",
    noSummaryReturned: "Keine Zusammenfassung zurückgegeben.",
    actionItems: "Aufgaben",
    noActionItemsFound: "Keine Aufgaben gefunden.",

    liveSegments: "Live-Segmente",
    liveSegmentsSubtitle: "letzte eindeutige Zeitblöcke",
    noTranscriptSegmentsYet: "Noch keine Transkriptsegmente vorhanden.",

    unknownApp: "-",
  },
};

function formatMs(ms: number) {
  const total = Math.floor(ms / 1000);
  const min = Math.floor(total / 60);
  const sec = total % 60;
  return `${String(min).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

function segmentKey(seg: TranscriptSegment) {
  return `${seg.start_ms}-${seg.end_ms}-${seg.text.trim()}`;
}

function TranscriptApp() {
  const [uiLang, setUiLang] = useState<UiLang>("en");
  const [status, setStatus] = useState<TranscriptStatus | null>(null);
  const [session, setSession] = useState<TranscriptSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [prereqs, setPrereqs] = useState<TranscriptPrereqs | null>(null);
  const [setupBusy, setSetupBusy] = useState(false);
  const [isMacOS] = useState(() =>
    /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent)
  );

  const [faithfulTranscript, setFaithfulTranscript] = useState("");
  const [speakerBlocks, setSpeakerBlocks] = useState<SpeakerBlock[]>([]);
  const [summary, setSummary] = useState("");
  const [actionItems, setActionItems] = useState<string[]>([]);

  const t = TEXTS[uiLang];

  useEffect(() => {
    document.documentElement.classList.toggle("macos-lite", isMacOS);
  }, [isMacOS]);

  const refreshPrereqs = async () => {
    try {
      const p = await invoke<TranscriptPrereqs>("transcript_check_prereqs");
      setPrereqs(p);
      return p;
    } catch (err) {
      setError(String(err));
      return null;
    }
  };

  const refresh = async () => {
    try {
      const nextStatus = await invoke<TranscriptStatus>(
        "get_transcript_status"
      );
      const nextSession = await invoke<TranscriptSession | null>(
        "get_current_transcript"
      );

      setStatus(nextStatus);
      if (nextSession) {
        setSession(nextSession);
      }
    } catch (err) {
      setError(String(err));
    }
  };

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
        console.error("failed to load identity for transcript ui", error);
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
          console.error("failed to refresh identity for transcript ui", error);
        }
      });
    };

    void setupIdentityListener();

    return () => {
      unlistenIdentityUpdated?.();
    };
  }, []);

  useEffect(() => {
    void refreshPrereqs();
    void refresh();

    let unlistenSegment: null | (() => void) = null;
    let unlistenError: null | (() => void) = null;
    let unlistenStatus: null | (() => void) = null;

    const setup = async () => {
      unlistenStatus = await listen<TranscriptStatus>(
        "transcript://status",
        async (event) => {
          const nextStatus = event.payload;

          setStatus(nextStatus);

          if (nextStatus.state === "Recording") {
            await emit("blob-state", { state: "transcript", active: true });
          } else {
            await emit("blob-state", { state: "transcript", active: false });
          }

          const nextSession = await invoke<TranscriptSession | null>(
            "get_current_transcript"
          );

          if (nextSession) {
            setSession(nextSession);
          }
        }
      );
      unlistenSegment = await listen<TranscriptSegment>(
        "transcript://segment",
        (event) => {
          const segment = event.payload;

          setSession((prev) => {
            if (!prev) return prev;

            const exists = prev.segments.some(
              (s) => segmentKey(s) === segmentKey(segment)
            );

            if (exists) return prev;

            return {
              ...prev,
              segments: [...prev.segments, segment],
            };
          });

          setStatus((prev) =>
            prev
              ? {
                  ...prev,
                  state: "Recording",
                  segment_count: (prev.segment_count ?? 0) + 1,
                }
              : prev
          );
        }
      );

      unlistenError = await listen<string>("transcript://error", (event) => {
        setError(String(event.payload || "Transcript error"));
      });
    };

    void setup();

    return () => {
      unlistenStatus?.();
      unlistenSegment?.();
      unlistenError?.();
    };
  }, []);

  const uniqueSegments = useMemo(() => {
    const seen = new Set<string>();
    const result: TranscriptSegment[] = [];

    for (const seg of session?.segments ?? []) {
      const key = segmentKey(seg);
      if (seen.has(key)) continue;
      seen.add(key);
      result.push(seg);
    }

    return result;
  }, [session]);

  const cleanTranscript = useMemo(() => {
    return uniqueSegments
      .map((seg) => seg.text.trim())
      .filter(Boolean)
      .join("\n");
  }, [uniqueSegments]);

  const derivedSegmentCount = uniqueSegments.length;
  const isRecording = status?.state === "Recording";
  const canProcess = derivedSegmentCount > 0 && !busy;
  const canSave = isRecording && !busy;

  const startTranscript = async () => {
    try {
      const p = await refreshPrereqs();
      if (p && !p.ok) {
        setError(p.message);
        return;
      }

      setBusy(true);
      setError(null);
      setFaithfulTranscript("");
      setSpeakerBlocks([]);
      setSummary("");
      setActionItems([]);

      await invoke("start_transcript", {
        source: "system",
        appName: session?.context?.app_name ?? "unknown",
        windowTitle: session?.context?.window_title ?? "Transcript Window",
      });

      await emit("transcript://status", await invoke("get_transcript_status"));
      await emit("blob-state", { state: "transcript", active: true });

      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const downloadDefaultModel = async () => {
    try {
      setSetupBusy(true);
      setError(null);
      await invoke("transcript_download_default_model");
      const p = await refreshPrereqs();
      if (p && !p.ok) setError(p.message);
    } catch (err) {
      setError(String(err));
    } finally {
      setSetupBusy(false);
    }
  };

  const openAudioMidiSetup = async () => {
    try {
      await invoke("transcript_open_audio_midi_setup");
    } catch (err) {
      setError(String(err));
    }
  };

  const openSoundSettings = async () => {
    try {
      await invoke("transcript_open_sound_settings");
    } catch (err) {
      setError(String(err));
    }
  };

  const openMicPrivacy = async () => {
    try {
      await invoke("transcript_open_microphone_privacy_settings");
    } catch (err) {
      setError(String(err));
    }
  };

  const openAccessibilityPrivacy = async () => {
    try {
      await invoke("transcript_open_accessibility_privacy_settings");
    } catch (err) {
      setError(String(err));
    }
  };

  const openBlackHoleDownload = async () => {
    try {
      await invoke("transcript_open_blackhole_download");
    } catch (err) {
      setError(String(err));
    }
  };

  const stopTranscript = async () => {
    try {
      setBusy(true);
      setError(null);

      await invoke("stop_transcript");

      await emit("transcript://status", await invoke("get_transcript_status"));
      await emit("blob-state", { state: "transcript", active: false });

      setStatus((prev) =>
        prev
          ? {
              ...prev,
              state: "Idle",
              active_session_id: null,
            }
          : prev
      );

      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const saveTranscript = async () => {
    try {
      setBusy(true);
      setError(null);
      await invoke("save_current_transcript");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const processTranscript = async () => {
    try {
      setBusy(true);
      setError(null);

      const result = await invoke<ProcessedTranscriptResult>(
        "process_transcript"
      );

      setFaithfulTranscript(result.faithful_transcript || "");
      setSpeakerBlocks(result.speaker_blocks || []);
      setSummary(result.summary || "");
      setActionItems(result.action_items || []);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const closeWindow = async () => {
    await getCurrentWindow()
      .hide()
      .catch(() => {});
  };

  const transcriptStatusLabel = useMemo(() => {
    if (busy && isRecording) return t.processing;
    if (busy) return t.working;
    if (isRecording) return t.recording;
    return t.idle;
  }, [busy, isRecording, t]);

  return (
    <>
      <style>{`
        :root {
          color-scheme: dark;
          --text-main: rgba(255,255,255,0.96);
          --text-soft: rgba(255,255,255,0.74);
          --text-dim: rgba(255,255,255,0.48);
          --glass-bg: rgba(18, 22, 30, 0.34);
          --glass-bg-strong: rgba(18, 22, 30, 0.52);
          --glass-fill: rgba(255,255,255,0.06);
          --glass-fill-strong: rgba(255,255,255,0.08);
          --glass-fill-hover: rgba(255,255,255,0.12);
          --glass-border: rgba(255,255,255,0.14);
          --glass-border-soft: rgba(255,255,255,0.08);
          --blue: rgba(10,132,255,0.92);
          --danger: rgba(255,69,58,0.92);
          --success: rgba(52,199,89,0.92);
          --warn: rgba(255,159,10,0.92);
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

        .body-scroll {
          position: relative;
          z-index: 2;
          flex: 1;
          min-height: 0;
          overflow-y: auto;
          padding-bottom: 16px;
          scrollbar-width: thin;
          scrollbar-color: rgba(255,255,255,0.18) transparent;
        }

        .body-scroll::-webkit-scrollbar {
          width: 10px;
        }

        .body-scroll::-webkit-scrollbar-thumb {
          background: rgba(255,255,255,0.14);
          border-radius: 999px;
        }

        .shell {
          width: 100%;
          height: 100%;
          padding: 18px;
          background: transparent;
        }

        .window {
          position: relative;
          width: 100%;
          height: 100%;
          display: flex;
          flex-direction: column;
          border-radius: 30px;
          overflow: hidden;
          isolation: isolate;
          background: var(--glass-bg);
          backdrop-filter: blur(24px) saturate(150%);
          -webkit-backdrop-filter: blur(24px) saturate(150%);
          border: 1px solid var(--glass-border);
          box-shadow:
            inset 0 1px 1px rgba(255,255,255,0.16),
            inset 0 -1px 1px rgba(0,0,0,0.18);
        }

        .macos-lite .window {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
          background: rgba(18, 20, 26, 0.74);
        }

        .window::before {
          content: "";
          position: absolute;
          inset: 0;
          pointer-events: none;
          border-radius: inherit;
          background:
            radial-gradient(circle at 12% 0%, rgba(255,255,255,0.12), transparent 28%),
            radial-gradient(circle at 100% 100%, rgba(117,163,255,0.10), transparent 24%);
        }

        .topbar {
          position: relative;
          z-index: 3;
          display: grid;
          grid-template-columns: 1fr auto;
          gap: 12px;
          align-items: center;
          padding: 14px 16px 12px;
          border-bottom: 1px solid rgba(255,255,255,0.06);
          background: linear-gradient(
            180deg,
            rgba(255,255,255,0.05),
            rgba(255,255,255,0.01)
          );
          -webkit-app-region: drag;
        }

        .title-wrap {
          min-width: 0;
        }

        .title {
          font-size: 15px;
          font-weight: 800;
          letter-spacing: 0.01em;
        }

        .subtitle {
          margin-top: 4px;
          font-size: 11px;
          color: rgba(255,255,255,0.58);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .window-actions {
          display: flex;
          align-items: center;
          gap: 8px;
          -webkit-app-region: no-drag;
        }

        .icon-btn {
          width: 40px;
          height: 40px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.08);
          color: white;
          display: grid;
          place-items: center;
          cursor: pointer;
          transition: all 0.18s ease;
          -webkit-app-region: no-drag;
        }

        .icon-btn:hover {
          background: rgba(255,255,255,0.14);
          border-color: rgba(255,255,255,0.16);
        }

        .toolbar {
          position: relative;
          z-index: 2;
          display: flex;
          align-items: center;
          gap: 10px;
          flex-wrap: wrap;
          padding: 14px 16px 0;
          -webkit-app-region: no-drag;
        }

        .btn {
          min-height: 42px;
          border-radius: 14px;
          border: 1px solid var(--glass-border-soft);
          background: rgba(255,255,255,0.07);
          color: var(--text-main);
          padding: 0 14px;
          display: inline-flex;
          align-items: center;
          gap: 8px;
          cursor: pointer;
          font-size: 13px;
          font-weight: 650;
          transition: all 0.18s ease;
          backdrop-filter: blur(14px) saturate(135%);
          -webkit-backdrop-filter: blur(14px) saturate(135%);
          -webkit-app-region: no-drag;
        }

        .macos-lite .btn {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
        }

        .btn:hover {
          background: var(--glass-fill-hover);
          border-color: rgba(255,255,255,0.16);
          transform: translateY(-1px);
        }

        .btn:disabled {
          opacity: 0.56;
          cursor: not-allowed;
          transform: none;
        }

        .btn-start {
          border-color: rgba(52,199,89,0.22);
          background: rgba(52,199,89,0.12);
        }

        .btn-stop {
          border-color: rgba(255,69,58,0.22);
          background: rgba(255,69,58,0.12);
        }

        .meta {
          position: relative;
          z-index: 2;
          display: flex;
          gap: 8px;
          flex-wrap: wrap;
          padding: 12px 16px 0;
          -webkit-app-region: no-drag;
        }

        .chip {
          padding: 6px 10px;
          border-radius: 999px;
          background: rgba(255,255,255,0.07);
          border: 1px solid rgba(255,255,255,0.08);
          font-size: 11px;
          color: rgba(255,255,255,0.76);
          backdrop-filter: blur(10px);
          -webkit-backdrop-filter: blur(10px);
        }

        .macos-lite .chip {
          backdrop-filter: none;
          -webkit-backdrop-filter: none;
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

        .error {
          position: relative;
          z-index: 2;
          margin: 12px 16px 0;
          padding: 12px 14px;
          border-radius: 16px;
          font-size: 12px;
          line-height: 1.45;
          color: rgba(255,255,255,0.94);
          background: rgba(255,69,58,0.14);
          border: 1px solid rgba(255,69,58,0.22);
          white-space: pre-wrap;
        }

        .content {
          position: relative;
          z-index: 2;
          display: grid;
          grid-template-columns: 0.95fr 1.05fr;
          gap: 14px;
          padding: 14px 16px 16px;
          min-height: 520px;
        }

        @media (max-width: 980px) {
          .content {
            grid-template-columns: 1fr;
          }
        }

        .panel {
          min-width: 0;
          min-height: 0;
          display: flex;
          flex-direction: column;
          border-radius: 24px;
          background: rgba(255,255,255,0.04);
          border: 1px solid rgba(255,255,255,0.08);
          box-shadow: inset 0 1px 1px rgba(255,255,255,0.05);
          overflow: hidden;
        }

        .panel-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 12px;
          padding: 14px 14px 12px;
          border-bottom: 1px solid rgba(255,255,255,0.06);
          background: linear-gradient(
            180deg,
            rgba(255,255,255,0.04),
            rgba(255,255,255,0.01)
          );
        }

        .panel-title {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 13px;
          font-weight: 750;
        }

        .panel-subtitle {
          font-size: 11px;
          color: rgba(255,255,255,0.5);
        }

        .panel-scroll {
          flex: 1;
          min-height: 0;
          overflow-y: auto;
          padding: 14px;
          scrollbar-width: thin;
          scrollbar-color: rgba(255,255,255,0.18) transparent;
        }

        .panel-scroll::-webkit-scrollbar {
          width: 10px;
        }

        .panel-scroll::-webkit-scrollbar-thumb {
          background: rgba(255,255,255,0.14);
          border-radius: 999px;
        }

        .raw-text {
          white-space: pre-wrap;
          line-height: 1.62;
          font-size: 14px;
          color: rgba(255,255,255,0.94);
        }

        .section {
          margin-bottom: 14px;
          padding: 12px;
          border-radius: 18px;
          background: rgba(255,255,255,0.03);
          border: 1px solid rgba(255,255,255,0.07);
        }

        .section-label {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 12px;
          font-weight: 750;
          color: rgba(255,255,255,0.82);
          margin-bottom: 8px;
        }

        .section-text {
          white-space: pre-wrap;
          line-height: 1.62;
          font-size: 14px;
          color: rgba(255,255,255,0.94);
        }

        .speaker-card {
          margin-bottom: 10px;
          padding: 12px;
          border-radius: 16px;
          background: rgba(255,255,255,0.025);
          border: 1px solid rgba(255,255,255,0.06);
        }

        .speaker-name {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          font-size: 12px;
          font-weight: 750;
          color: rgba(255,255,255,0.82);
          margin-bottom: 8px;
        }

        .speaker-text {
          white-space: pre-wrap;
          line-height: 1.6;
          font-size: 14px;
          color: rgba(255,255,255,0.94);
        }

        .action-list {
          margin: 0;
          padding-left: 18px;
          line-height: 1.7;
          color: rgba(255,255,255,0.94);
        }

        .live-list {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .live-segment {
          padding: 10px 12px;
          border: 1px solid rgba(255,255,255,0.06);
          border-radius: 14px;
          background: rgba(255,255,255,0.02);
        }

        .live-time {
          font-size: 11px;
          color: rgba(255,255,255,0.56);
          margin-bottom: 4px;
        }

        .live-text {
          font-size: 13px;
          line-height: 1.48;
          color: rgba(255,255,255,0.94);
        }

        .empty {
          color: rgba(255,255,255,0.54);
          font-size: 13px;
          line-height: 1.5;
        }

        .spin {
          animation: spin 0.9s linear infinite;
        }

        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>

      <div className="shell">
        <div className="window">
          <div className="topbar" data-tauri-drag-region>
            <div className="title-wrap">
              <div className="title">{t.title}</div>
              <div
                className="subtitle"
                title={`${session?.context?.app_name ?? t.unknownApp} • ${
                  session?.context?.window_title ?? t.unknownApp
                }`}
              >
                {session?.context?.app_name ?? t.unknownApp} •{" "}
                {session?.context?.window_title ?? t.unknownApp}
              </div>
            </div>

            <div className="window-actions">
              <button
                className="icon-btn"
                onClick={closeWindow}
                title={t.close}
              >
                <X size={16} />
              </button>
            </div>
          </div>

          <div className="body-scroll">
            <div className="toolbar">
              {!isRecording ? (
                <button
                  className="btn btn-start"
                  onClick={startTranscript}
                  disabled={busy}
                >
                  {busy ? (
                    <LoaderCircle size={16} className="spin" />
                  ) : (
                    <Play size={16} />
                  )}
                  {t.startTranscript}
                </button>
              ) : (
                <button
                  className="btn btn-stop"
                  onClick={stopTranscript}
                  disabled={busy}
                >
                  {busy ? (
                    <LoaderCircle size={16} className="spin" />
                  ) : (
                    <Square size={16} />
                  )}
                  {t.stopTranscript}
                </button>
              )}

              <button
                className="btn"
                onClick={saveTranscript}
                disabled={!canSave}
              >
                <Save size={16} />
                {t.save}
              </button>

              <button
                className="btn"
                onClick={processTranscript}
                disabled={!canProcess}
              >
                {busy ? (
                  <LoaderCircle size={16} className="spin" />
                ) : (
                  <Bot size={16} />
                )}
                {t.process}
              </button>
            </div>

            <div className="meta">
              <div className="chip">
                {t.session} {session?.id ?? "-"}
              </div>
              <div className="chip">
                {t.segments} {derivedSegmentCount}
              </div>
              <div className="chip">
                {t.app} {session?.context?.app_name ?? t.unknownApp}
              </div>
              <div
                className={`chip ${
                  busy ? "chip-busy" : isRecording ? "chip-recording" : ""
                }`}
              >
                {transcriptStatusLabel}
              </div>
            </div>

            {prereqs && (prereqs.needs_virtual_audio_routing || !prereqs.ok) && (
              <div className="error" style={{ background: "rgba(255,159,10,0.12)" }}>
                <div style={{ marginBottom: 8 }}>{prereqs.message}</div>
                <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
                  <button
                    className="btn"
                    onClick={downloadDefaultModel}
                    disabled={setupBusy}
                  >
                    {setupBusy ? (
                      <LoaderCircle size={16} className="spin" />
                    ) : (
                      <CheckSquare size={16} />
                    )}
                    Download default model
                  </button>
                  <button className="btn" onClick={openBlackHoleDownload}>
                    <Mic size={16} />
                    Get BlackHole
                  </button>
                  <button className="btn" onClick={openAudioMidiSetup}>
                    <AudioLines size={16} />
                    Audio MIDI Setup
                  </button>
                  <button className="btn" onClick={openSoundSettings}>
                    <UserRound size={16} />
                    Sound Settings
                  </button>
                  <button className="btn" onClick={openMicPrivacy}>
                    <CheckSquare size={16} />
                    Mic Privacy
                  </button>
                  <button className="btn" onClick={openAccessibilityPrivacy}>
                    <CheckSquare size={16} />
                    Accessibility
                  </button>
                  {prereqs.default_input_device ? (
                    <div className="chip">
                      Input: {prereqs.default_input_device}
                    </div>
                  ) : null}
                </div>
              </div>
            )}

            {error && <div className="error">{error}</div>}

            <div className="content">
              <div className="panel">
                <div className="panel-header">
                  <div>
                    <div className="panel-title">
                      <Waves size={15} />
                      {t.rawCleanTranscript}
                    </div>
                    <div className="panel-subtitle">
                      {t.rawCleanTranscriptSubtitle}
                    </div>
                  </div>
                </div>

                <div className="panel-scroll">
                  {cleanTranscript ? (
                    <div className="raw-text">{cleanTranscript}</div>
                  ) : (
                    <div className="empty">{t.noTranscriptTextYet}</div>
                  )}
                </div>
              </div>

              <div className="panel">
                <div className="panel-header">
                  <div>
                    <div className="panel-title">
                      <AudioLines size={15} />
                      {t.processedOutput}
                    </div>
                    <div className="panel-subtitle">
                      {t.processedOutputSubtitle}
                    </div>
                  </div>
                </div>

                <div className="panel-scroll">
                  {faithfulTranscript ? (
                    <>
                      <div className="section">
                        <div className="section-label">
                          <Mic size={14} />
                          {t.faithfulTranscript}
                        </div>
                        <div className="section-text">{faithfulTranscript}</div>
                      </div>

                      <div className="section">
                        <div className="section-label">
                          <UserRound size={14} />
                          {t.speakerBlocks}
                        </div>

                        {speakerBlocks.length ? (
                          speakerBlocks.map((block, index) => (
                            <div
                              key={`${block.speaker}-${index}`}
                              className="speaker-card"
                            >
                              <div className="speaker-name">
                                <UserRound size={13} />
                                {block.speaker}
                              </div>
                              <div className="speaker-text">{block.text}</div>
                            </div>
                          ))
                        ) : (
                          <div className="empty">
                            {t.noSpeakerBlocksReturned}
                          </div>
                        )}
                      </div>

                      <div className="section">
                        <div className="section-label">
                          <Bot size={14} />
                          {t.summary}
                        </div>
                        <div className="section-text">
                          {summary || t.noSummaryReturned}
                        </div>
                      </div>

                      <div className="section" style={{ marginBottom: 0 }}>
                        <div className="section-label">
                          <CheckSquare size={14} />
                          {t.actionItems}
                        </div>

                        {actionItems.length ? (
                          <ul className="action-list">
                            {actionItems.map((item, index) => (
                              <li key={`${item}-${index}`}>{item}</li>
                            ))}
                          </ul>
                        ) : (
                          <div className="empty">{t.noActionItemsFound}</div>
                        )}
                      </div>
                    </>
                  ) : (
                    <div className="empty">
                      {t.noProcessedOutputYet}{" "}
                      <strong>{t.processStrong}</strong>.
                    </div>
                  )}
                </div>
              </div>
            </div>

            <div
              style={{
                position: "relative",
                zIndex: 2,
                padding: "0 16px 16px",
              }}
            >
              <div className="panel" style={{ minHeight: 220 }}>
                <div className="panel-header">
                  <div>
                    <div className="panel-title">
                      <Waves size={15} />
                      {t.liveSegments}
                    </div>
                    <div className="panel-subtitle">
                      {t.liveSegmentsSubtitle}
                    </div>
                  </div>
                </div>

                <div className="panel-scroll">
                  {uniqueSegments.length ? (
                    <div className="live-list">
                      {uniqueSegments.map((seg, i) => (
                        <div
                          key={`${segmentKey(seg)}-${i}`}
                          className="live-segment"
                        >
                          <div className="live-time">
                            {formatMs(seg.start_ms)} - {formatMs(seg.end_ms)}
                          </div>
                          <div className="live-text">{seg.text}</div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="empty">{t.noTranscriptSegmentsYet}</div>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(<TranscriptApp />);
