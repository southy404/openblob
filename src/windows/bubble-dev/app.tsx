import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen, emit } from "@tauri-apps/api/event";
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
  Radio,
  TerminalSquare,
  ChevronDown,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

type DevPayload = {
  lastRoute?: string;
  voiceShortcut?: string;
  model?: string;
};

type UiLang = "en" | "de";

type RouteState = "none" | "command" | "ollama";
type WakeWordProvider = "none" | "porcupine" | "mic-test" | "mock" | "disabled";
type WakeWordStatusName =
  | "disabled"
  | "stopped"
  | "starting"
  | "listening"
  | "detected"
  | "no_input_device"
  | "permission_error"
  | "provider_missing"
  | "error";

type WakeWordSettings = {
  wake_word_enabled: boolean;
  wake_word_phrase: string;
  wake_word_sensitivity: number;
  wake_word_provider: WakeWordProvider;
};

type WakeWordStatus = {
  status: WakeWordStatusName;
  state: WakeWordStatusName;
  message: string;
  enabled: boolean;
  phrase: string;
  provider: WakeWordProvider;
  sensitivity: number;
  listening: boolean;
  detected: boolean;
  provider_configured: boolean;
  selected_input_device?: string | null;
  available_input_devices: string[];
  last_error?: string | null;
  last_started_at?: string | null;
  last_stopped_at?: string | null;
  last_audio_at?: string | null;
  audio_chunks_seen: number;
  input_level?: number | null;
};

type LocalizedText = {
  windowTitle: string;
  windowSubtitle: string;
  close: string;

  lastRoute: string;
  voiceShortcut: string;
  model: string;
  commands: string;

  companionIdentity: string;
  blobName: string;
  ownerName: string;
  language: string;
  save: string;
  saving: string;

  wakeWord: string;
  wakeWordEnabled: string;
  wakeWordPhrase: string;
  wakeWordSensitivity: string;
  wakeWordProvider: string;
  wakeWordStatus: string;
  wakeWordSave: string;
  wakeWordRefresh: string;
  wakeWordStart: string;
  wakeWordStop: string;
  wakeWordInputDevice: string;
  wakeWordLastAudio: string;
  wakeWordChunks: string;
  wakeWordInputLevel: string;
  wakeWordLastError: string;

  searchPlaceholder: string;
  noCommandsFound: string;
  commandCount: (count: number) => string;

  routeNone: string;

  languageOptions: Array<{ value: UiLang; label: string }>;
};

type CommandItem = {
  command: string;
  description: string;
};

type CommandGroupKey =
  | "voiceGeneral"
  | "system"
  | "browser"
  | "media"
  | "fun"
  | "apps"
  | "editing";

type LocalizedCommandGroup = {
  key: CommandGroupKey;
  title: string;
  icon: JSX.Element;
  items: CommandItem[];
};

const TEXTS: Record<UiLang, LocalizedText> = {
  en: {
    windowTitle: "OpenBlob Dev Mode",
    windowSubtitle: "Commands, routing, voice, identity, debug info",
    close: "Close",

    lastRoute: "Last Route",
    voiceShortcut: "Voice Shortcut",
    model: "Model",
    commands: "Commands",

    companionIdentity: "Companion Identity",
    blobName: "Blob Name",
    ownerName: "Owner Name",
    language: "Language",
    save: "Save",
    saving: "Saving...",

    wakeWord: "Wake Word",
    wakeWordEnabled: "Enabled",
    wakeWordPhrase: "Phrase",
    wakeWordSensitivity: "Sensitivity",
    wakeWordProvider: "Provider",
    wakeWordStatus: "Status",
    wakeWordSave: "Save Wake Word",
    wakeWordRefresh: "Refresh",
    wakeWordStart: "Start Listener",
    wakeWordStop: "Stop Listener",
    wakeWordInputDevice: "Input",
    wakeWordLastAudio: "Last audio",
    wakeWordChunks: "Chunks",
    wakeWordInputLevel: "Level",
    wakeWordLastError: "Last error",

    searchPlaceholder:
      "Filter commands, e.g. youtube, timer, browser, volume, shutdown ...",
    noCommandsFound: "No commands found for this search.",
    commandCount: (count) => `${count} command${count === 1 ? "" : "s"}`,

    routeNone: "none",

    languageOptions: [
      { value: "en", label: "English" },
      { value: "de", label: "Deutsch" },
    ],
  },
  de: {
    windowTitle: "OpenBlob Dev Mode",
    windowSubtitle: "Befehle, Routing, Sprache, Identität, Debug-Infos",
    close: "Schließen",

    lastRoute: "Letzte Route",
    voiceShortcut: "Voice Shortcut",
    model: "Modell",
    commands: "Befehle",

    companionIdentity: "Companion-Identität",
    blobName: "Blob-Name",
    ownerName: "Besitzername",
    language: "Sprache",
    save: "Speichern",
    saving: "Speichert...",

    wakeWord: "Wake Word",
    wakeWordEnabled: "Aktiviert",
    wakeWordPhrase: "Phrase",
    wakeWordSensitivity: "Empfindlichkeit",
    wakeWordProvider: "Provider",
    wakeWordStatus: "Status",
    wakeWordSave: "Wake Word speichern",
    wakeWordRefresh: "Aktualisieren",
    wakeWordStart: "Listener starten",
    wakeWordStop: "Listener stoppen",
    wakeWordInputDevice: "Eingang",
    wakeWordLastAudio: "Letztes Audio",
    wakeWordChunks: "Chunks",
    wakeWordInputLevel: "Pegel",
    wakeWordLastError: "Letzter Fehler",

    searchPlaceholder:
      "Befehle filtern, z. B. youtube, timer, browser, volume, shutdown ...",
    noCommandsFound: "Keine Befehle für diese Suche gefunden.",
    commandCount: (count) => `${count} Befehl${count === 1 ? "" : "e"}`,

    routeNone: "keine",

    languageOptions: [
      { value: "en", label: "English" },
      { value: "de", label: "Deutsch" },
    ],
  },
};

function getCommandGroups(lang: UiLang): LocalizedCommandGroup[] {
  if (lang === "de") {
    return [
      {
        key: "voiceGeneral",
        title: "Sprache / Allgemein",
        icon: <Mic size={15} />,
        items: [
          {
            command: "wie spät ist es",
            description: "Aktuelle Uhrzeit abrufen",
          },
          {
            command: "welches datum ist heute",
            description: "Aktuelles Datum abrufen",
          },
          {
            command: "wie ist das wetter in Berlin",
            description: "Wetter für einen Ort abrufen",
          },
          {
            command: "mach einen screenshot",
            description: "Snip-Modus öffnen",
          },
        ],
      },
      {
        key: "system",
        title: "System",
        icon: <MonitorSmartphone size={15} />,
        items: [
          {
            command: "öffne downloads",
            description: "Downloads-Ordner öffnen",
          },
          {
            command: "öffne einstellungen",
            description: "Windows-Einstellungen öffnen",
          },
          {
            command: "öffne explorer",
            description: "Datei-Explorer öffnen",
          },
          {
            command: "bildschirm sperren",
            description: "Aktuelle Windows-Sitzung sperren",
          },
          {
            command: "herunterfahren",
            description: "Bestätigung anfordern und dann den PC herunterfahren",
          },
          {
            command: "neu starten",
            description: "Bestätigung anfordern und dann den PC neu starten",
          },
          {
            command: "ja",
            description: "Eine ausstehende geschützte Aktion bestätigen",
          },
          {
            command: "nein",
            description: "Eine ausstehende geschützte Aktion abbrechen",
          },
          {
            command: "abbrechen",
            description: "Eine ausstehende geschützte Aktion abbrechen",
          },
        ],
      },
      {
        key: "browser",
        title: "Browser",
        icon: <Globe size={15} />,
        items: [
          {
            command: "google katzen",
            description: "Google-Suche starten",
          },
          {
            command: "youtube lo fi",
            description: "YouTube-Suche starten",
          },
          {
            command: "öffne github",
            description: "Bekannte Website oder App öffnen",
          },
          {
            command: "neuer tab",
            description: "Neuen Tab öffnen",
          },
          {
            command: "tab schließen",
            description: "Aktiven Tab schließen",
          },
          {
            command: "zurück",
            description: "Im Browser zurückgehen",
          },
          {
            command: "vor",
            description: "Im Browser vorgehen",
          },
          {
            command: "runterscrollen",
            description: "Seite nach unten scrollen",
          },
          {
            command: "hochscrollen",
            description: "Seite nach oben scrollen",
          },
          {
            command: "klicke auf das erste ergebnis",
            description: "Erstes sichtbares Ergebnis anklicken",
          },
          {
            command: "browser kontext",
            description: "Infos über die aktuelle Seite abrufen",
          },
        ],
      },
      {
        key: "media",
        title: "Medien",
        icon: <Music2 size={15} />,
        items: [
          {
            command: "spiele youtube",
            description: "YouTube abspielen oder pausieren",
          },
          {
            command: "pausiere youtube",
            description: "YouTube abspielen oder pausieren",
          },
          {
            command: "werbung überspringen",
            description: "YouTube-Werbung überspringen",
          },
          {
            command: "nächstes video",
            description: "Nächstes YouTube-Video abspielen",
          },
          {
            command: "spule vor",
            description: "10 Sekunden vorspulen",
          },
          {
            command: "spule zurück",
            description: "10 Sekunden zurückspulen",
          },
          {
            command: "lauter",
            description: "Lautstärke erhöhen",
          },
          {
            command: "leiser",
            description: "Lautstärke verringern",
          },
          {
            command: "stumm",
            description: "Audio stummschalten",
          },
          {
            command: "laut",
            description: "Stummschaltung aufheben",
          },
          {
            command: "empfiehl mir eine komödie auf netflix",
            description: "Streaming-Empfehlung abrufen",
          },
        ],
      },
      {
        key: "fun",
        title: "Spaß / Mini-Befehle",
        icon: <Dice5 size={15} />,
        items: [
          {
            command: "wirf eine münze",
            description: "Münze werfen",
          },
          {
            command: "würfeln",
            description: "Sechsseitigen Würfel werfen",
          },
          {
            command: "starte einen timer für 5 minuten",
            description: "Timer starten",
          },
        ],
      },
      {
        key: "apps",
        title: "Apps",
        icon: <MonitorSmartphone size={15} />,
        items: [
          {
            command: "öffne discord",
            description: "App oder Web-App öffnen",
          },
          {
            command: "öffne spotify",
            description: "Spotify öffnen",
          },
          {
            command: "öffne chrome",
            description: "Chrome öffnen",
          },
          {
            command: "öffne notepad",
            description: "Notepad öffnen",
          },
          {
            command: "öffne paint",
            description: "Paint öffnen",
          },
          {
            command: "öffne rechner",
            description: "Rechner öffnen",
          },
        ],
      },
      {
        key: "editing",
        title: "Bearbeiten / Shortcuts",
        icon: <TerminalSquare size={15} />,
        items: [
          {
            command: "speichern",
            description: "Ctrl+S drücken",
          },
          {
            command: "speichern unter",
            description: "Ctrl+Shift+S drücken",
          },
          {
            command: "datei öffnen",
            description: "Ctrl+O drücken",
          },
          {
            command: "neue datei",
            description: "Ctrl+N drücken",
          },
          {
            command: "rückgängig",
            description: "Ctrl+Z drücken",
          },
          {
            command: "wiederholen",
            description: "Ctrl+Y drücken",
          },
          {
            command: "bestätigen",
            description: "Enter drücken",
          },
          {
            command: "abbrechen",
            description: "Escape drücken",
          },
        ],
      },
    ];
  }

  return [
    {
      key: "voiceGeneral",
      title: "Voice / General",
      icon: <Mic size={15} />,
      items: [
        { command: "what time is it", description: "Get current time" },
        { command: "what date is it", description: "Get current date" },
        {
          command: "weather in Berlin",
          description: "Get weather for a location",
        },
        { command: "take screenshot", description: "Open snip mode" },
      ],
    },
    {
      key: "system",
      title: "System",
      icon: <MonitorSmartphone size={15} />,
      items: [
        { command: "open downloads", description: "Open the Downloads folder" },
        { command: "open settings", description: "Open Windows Settings" },
        { command: "open explorer", description: "Open File Explorer" },
        {
          command: "lock screen",
          description: "Lock the current Windows session",
        },
        {
          command: "shutdown",
          description: "Ask for confirmation, then shut down the PC",
        },
        {
          command: "restart",
          description: "Ask for confirmation, then restart the PC",
        },
        { command: "yes", description: "Confirm a pending protected action" },
        { command: "no", description: "Cancel a pending protected action" },
        { command: "cancel", description: "Cancel a pending protected action" },
      ],
    },
    {
      key: "browser",
      title: "Browser",
      icon: <Globe size={15} />,
      items: [
        { command: "google cats", description: "Start a Google search" },
        { command: "youtube lo fi", description: "Start a YouTube search" },
        {
          command: "open github",
          description: "Open a known website or app",
        },
        { command: "new tab", description: "Open a new tab" },
        { command: "close tab", description: "Close the active tab" },
        { command: "go back", description: "Go back in the browser" },
        { command: "go forward", description: "Go forward in the browser" },
        { command: "scroll down", description: "Scroll the page down" },
        { command: "scroll up", description: "Scroll the page up" },
        {
          command: "click first result",
          description: "Click the first visible result",
        },
        {
          command: "browser context",
          description: "Get info about the current page",
        },
      ],
    },
    {
      key: "media",
      title: "Media",
      icon: <Music2 size={15} />,
      items: [
        { command: "play youtube", description: "Play or pause YouTube" },
        { command: "pause youtube", description: "Play or pause YouTube" },
        { command: "skip ad", description: "Skip a YouTube ad" },
        { command: "next video", description: "Play the next YouTube video" },
        { command: "seek forward", description: "Seek forward by 10 seconds" },
        {
          command: "seek backward",
          description: "Seek backward by 10 seconds",
        },
        { command: "volume up", description: "Increase volume" },
        { command: "volume down", description: "Decrease volume" },
        { command: "mute", description: "Mute audio" },
        { command: "unmute", description: "Unmute audio" },
        {
          command: "recommend a comedy on netflix",
          description: "Get a streaming recommendation",
        },
      ],
    },
    {
      key: "fun",
      title: "Fun / Mini Commands",
      icon: <Dice5 size={15} />,
      items: [
        { command: "flip a coin", description: "Flip a coin" },
        { command: "roll dice", description: "Roll a six-sided die" },
        { command: "start a 5 minute timer", description: "Start a timer" },
      ],
    },
    {
      key: "apps",
      title: "Apps",
      icon: <MonitorSmartphone size={15} />,
      items: [
        { command: "open discord", description: "Open an app or web app" },
        { command: "open spotify", description: "Open Spotify" },
        { command: "open chrome", description: "Open Chrome" },
        { command: "open notepad", description: "Open Notepad" },
        { command: "open paint", description: "Open Paint" },
        { command: "open calc", description: "Open Calculator" },
      ],
    },
    {
      key: "editing",
      title: "Editing / Shortcuts",
      icon: <TerminalSquare size={15} />,
      items: [
        { command: "save", description: "Press Ctrl+S" },
        { command: "save as", description: "Press Ctrl+Shift+S" },
        { command: "open file", description: "Press Ctrl+O" },
        { command: "new file", description: "Press Ctrl+N" },
        { command: "undo", description: "Press Ctrl+Z" },
        { command: "redo", description: "Press Ctrl+Y" },
        { command: "confirm", description: "Press Enter" },
        { command: "clear", description: "Press Escape" },
      ],
    },
  ];
}

function DevWindow() {
  const [uiLang, setUiLang] = useState<UiLang>("en");
  const [lastRoute, setLastRoute] = useState<RouteState>("none");
  const [voiceShortcut, setVoiceShortcut] = useState("Alt + M");
  const [model, setModel] = useState("llama3.1:8b");
  const [search, setSearch] = useState("");
  const [blobName, setBlobName] = useState("");
  const [ownerName, setOwnerName] = useState("");
  const [language, setLanguage] = useState<UiLang>("en");
  const [saving, setSaving] = useState(false);
  const [wakeWordSettings, setWakeWordSettings] = useState<WakeWordSettings>({
    wake_word_enabled: false,
    wake_word_phrase: "hey blob",
    wake_word_sensitivity: 0.5,
    wake_word_provider: "none",
  });
  const [wakeWordStatus, setWakeWordStatus] = useState<WakeWordStatus>({
    status: "disabled",
    state: "disabled",
    message: "Wake word is disabled.",
    enabled: false,
    phrase: "hey blob",
    provider: "none",
    sensitivity: 0.5,
    listening: false,
    detected: false,
    provider_configured: false,
    selected_input_device: null,
    available_input_devices: [],
    last_error: null,
    last_started_at: null,
    last_stopped_at: null,
    last_audio_at: null,
    audio_chunks_seen: 0,
    input_level: null,
  });
  const [wakeWordSaving, setWakeWordSaving] = useState(false);

  const t = TEXTS[uiLang];
  const commandGroups = useMemo(() => getCommandGroups(uiLang), [uiLang]);

  const [openGroups, setOpenGroups] = useState<
    Record<CommandGroupKey, boolean>
  >({
    voiceGeneral: true,
    system: false,
    browser: false,
    media: false,
    fun: false,
    apps: false,
    editing: false,
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
        const normalizedLang: UiLang = lang === "de" ? "de" : "en";

        setBlobName(blob);
        setOwnerName(owner);
        setLanguage(normalizedLang);
        setUiLang(normalizedLang);
      } catch (err) {
        console.error("failed to load identity", err);
      }
    };

    void loadIdentity();
  }, []);

  const refreshWakeWord = async () => {
    try {
      const [settings, status] = await Promise.all([
        invoke<WakeWordSettings>("get_wake_word_settings"),
        invoke<WakeWordStatus>("get_wake_word_status"),
      ]);

      setWakeWordSettings(settings);
      setWakeWordStatus(status);
    } catch (err) {
      console.error("failed to load wake word settings", err);
      setWakeWordStatus({
        status: "error",
        state: "error",
        message: String(err),
        enabled: wakeWordSettings.wake_word_enabled,
        phrase: wakeWordSettings.wake_word_phrase,
        provider: wakeWordSettings.wake_word_provider,
        sensitivity: wakeWordSettings.wake_word_sensitivity,
        listening: false,
        detected: false,
        provider_configured: false,
        selected_input_device: null,
        available_input_devices: [],
        last_error: String(err),
        last_started_at: null,
        last_stopped_at: null,
        last_audio_at: null,
        audio_chunks_seen: 0,
        input_level: null,
      });
    }
  };

  useEffect(() => {
    void refreshWakeWord();

    const interval = window.setInterval(() => {
      void invoke<WakeWordStatus>("get_wake_word_status")
        .then(setWakeWordStatus)
        .catch((err) => {
          console.error("failed to refresh wake word status", err);
        });
    }, 3000);

    return () => window.clearInterval(interval);
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

      await emit("identity-updated");

      const result = (await invoke("get_identity")) as [string, string, string];
      const [blob, owner, lang] = result;
      const normalizedLang: UiLang = lang === "de" ? "de" : "en";

      setBlobName(blob);
      setOwnerName(owner);
      setLanguage(normalizedLang);
      setUiLang(normalizedLang);
    } catch (err) {
      console.error("failed to save identity", err);
    } finally {
      setSaving(false);
    }
  };

  const saveWakeWordSettings = async () => {
    try {
      setWakeWordSaving(true);

      const saved = await invoke<WakeWordSettings>("update_wake_word_settings", {
        settings: wakeWordSettings,
      });
      setWakeWordSettings(saved);
      const nextStatus = await invoke<WakeWordStatus>("get_wake_word_status");
      setWakeWordStatus(nextStatus);
    } catch (err) {
      console.error("failed to save wake word settings", err);
      setWakeWordStatus({
        status: "error",
        state: "error",
        message: String(err),
        enabled: wakeWordSettings.wake_word_enabled,
        phrase: wakeWordSettings.wake_word_phrase,
        provider: wakeWordSettings.wake_word_provider,
        sensitivity: wakeWordSettings.wake_word_sensitivity,
        listening: false,
        detected: false,
        provider_configured: false,
        selected_input_device: null,
        available_input_devices: [],
        last_error: String(err),
        last_started_at: null,
        last_stopped_at: null,
        last_audio_at: null,
        audio_chunks_seen: 0,
        input_level: null,
      });
    } finally {
      setWakeWordSaving(false);
    }
  };

  const startWakeWordListener = async () => {
    try {
      const nextStatus = await invoke<WakeWordStatus>("start_wake_word_listener");
      setWakeWordStatus(nextStatus);
    } catch (err) {
      console.error("failed to start wake word listener", err);
    }
  };

  const stopWakeWordListener = async () => {
    try {
      const nextStatus = await invoke<WakeWordStatus>("stop_wake_word_listener");
      setWakeWordStatus(nextStatus);
    } catch (err) {
      console.error("failed to stop wake word listener", err);
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
  }, [search, commandGroups]);

  useEffect(() => {
    const q = search.trim();
    if (!q) return;

    setOpenGroups((prev) => {
      const next = { ...prev };
      for (const group of filteredGroups) {
        next[group.key] = true;
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

  const toggleGroup = (key: CommandGroupKey) => {
    setOpenGroups((prev) => ({
      ...prev,
      [key]: !prev[key],
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

        .wakeGrid {
          display: grid;
          grid-template-columns: minmax(130px, 0.7fr) minmax(150px, 1fr) minmax(130px, 0.8fr) minmax(160px, 1fr) auto;
          gap: 10px;
          align-items: end;
        }

        .checkField {
          display: flex;
          min-height: 42px;
          align-items: center;
          gap: 10px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.07);
          padding: 0 12px;
          color: rgba(255,255,255,0.82);
          font-size: 13px;
        }

        .checkField input {
          width: 16px;
          height: 16px;
          accent-color: #75A3FF;
        }

        .rangeInput {
          width: 100%;
          accent-color: #75A3FF;
        }

        .statusPill {
          min-height: 42px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.10);
          background: rgba(255,255,255,0.07);
          padding: 8px 12px;
          display: flex;
          align-items: center;
          gap: 9px;
          color: rgba(255,255,255,0.78);
          font-size: 12px;
          line-height: 1.35;
        }

        .statusDot {
          width: 8px;
          height: 8px;
          border-radius: 999px;
          background: rgba(255,255,255,0.38);
          flex-shrink: 0;
        }

        .statusPill.listening .statusDot {
          background: #64f0ad;
        }

        .statusPill.detected .statusDot {
          background: #75A3FF;
        }

        .statusPill.error .statusDot {
          background: #ff8a8a;
        }

        .statusPill.no_input_device .statusDot,
        .statusPill.permission_error .statusDot,
        .statusPill.provider_missing .statusDot {
          background: #ffd166;
        }

        .wakeMetrics {
          display: grid;
          grid-template-columns: repeat(4, minmax(0, 1fr));
          gap: 10px;
        }

        .wakeMetric {
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.08);
          background: rgba(255,255,255,0.045);
          padding: 10px 12px;
          min-width: 0;
        }

        .wakeMetric .metricLabel {
          font-size: 10px;
          text-transform: uppercase;
          letter-spacing: 0.08em;
          opacity: 0.52;
          margin-bottom: 5px;
        }

        .wakeMetric .metricValue {
          font-size: 12px;
          line-height: 1.4;
          color: rgba(255,255,255,0.76);
          word-break: break-word;
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

        .textInput,
        .selectInput {
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

        .textInput:focus,
        .selectInput:focus {
          border-color: rgba(255,255,255,0.18);
          background: rgba(255,255,255,0.10);
        }

        .selectInput option {
          background: #1f1f24;
          color: white;
        }

        .saveBtn {
          height: 42px;
          border-radius: 14px;
          border: 1px solid rgba(255,255,255,0.12);
          background: rgba(255,255,255,0.09);
          color: var(--text-main);
          font-weight: 700;
          padding: 0 16px;
          cursor: pointer;
          transition: background 0.18s ease, border-color 0.18s ease, opacity 0.18s ease;
          white-space: nowrap;
          box-shadow:
            inset 0 1px 1px rgba(255,255,255,0.18),
            inset 0 -1px 1px rgba(0,0,0,0.14);
        }

        .saveBtn:hover {
          background: rgba(255,255,255,0.12);
          border-color: rgba(255,255,255,0.16);
        }

        .saveBtn:active {
          background: rgba(255,255,255,0.14);
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

          .wakeGrid {
            grid-template-columns: repeat(2, minmax(0, 1fr));
          }

          .wakeMetrics {
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

          .wakeGrid {
            grid-template-columns: 1fr;
          }

          .wakeMetrics {
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
                <div className="title">{t.windowTitle}</div>
                <div className="subtitle">{t.windowSubtitle}</div>
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
              title={t.close}
            >
              <X size={16} />
            </button>
          </div>

          <div className="content">
            <div className="stats">
              <div className="card">
                <div className="label">{t.lastRoute}</div>
                <div className="value">
                  {lastRoute === "none" ? t.routeNone : lastRoute}
                </div>
              </div>

              <div className="card">
                <div className="label">{t.voiceShortcut}</div>
                <div className="value">{voiceShortcut}</div>
              </div>

              <div className="card">
                <div className="label">{t.model}</div>
                <div className="value">{model}</div>
              </div>

              <div className="card">
                <div className="label">{t.commands}</div>
                <div className="value strong">
                  {search.trim()
                    ? `${visibleCommands} / ${totalCommands}`
                    : totalCommands}
                </div>
              </div>
            </div>

            <div className="card identityCard">
              <div className="label">{t.companionIdentity}</div>

              <div className="identityGrid">
                <div className="field">
                  <div className="fieldLabel">{t.blobName}</div>
                  <input
                    className="textInput"
                    value={blobName}
                    onChange={(e) => setBlobName(e.target.value)}
                    placeholder={t.blobName}
                  />
                </div>

                <div className="field">
                  <div className="fieldLabel">{t.ownerName}</div>
                  <input
                    className="textInput"
                    value={ownerName}
                    onChange={(e) => setOwnerName(e.target.value)}
                    placeholder={t.ownerName}
                  />
                </div>

                <div className="field">
                  <div className="fieldLabel">{t.language}</div>
                  <select
                    className="selectInput"
                    value={language}
                    onChange={(e) => {
                      const nextLang = (
                        e.target.value === "de" ? "de" : "en"
                      ) as UiLang;
                      setLanguage(nextLang);
                      setUiLang(nextLang);
                    }}
                  >
                    {t.languageOptions.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </div>

                <button
                  className="saveBtn"
                  onClick={() => void saveIdentity()}
                  disabled={saving}
                  type="button"
                >
                  {saving ? t.saving : t.save}
                </button>
              </div>
            </div>

            <div className="card identityCard">
              <div className="label">{t.wakeWord}</div>

              <div className="wakeGrid">
                <div className="field">
                  <div className="fieldLabel">{t.wakeWordEnabled}</div>
                  <label className="checkField">
                    <input
                      type="checkbox"
                      checked={wakeWordSettings.wake_word_enabled}
                      onChange={(e) =>
                        setWakeWordSettings((prev) => ({
                          ...prev,
                          wake_word_enabled: e.target.checked,
                        }))
                      }
                    />
                    <span>{wakeWordSettings.wake_word_enabled ? "on" : "off"}</span>
                  </label>
                </div>

                <div className="field">
                  <div className="fieldLabel">{t.wakeWordPhrase}</div>
                  <input
                    className="textInput"
                    value={wakeWordSettings.wake_word_phrase}
                    onChange={(e) =>
                      setWakeWordSettings((prev) => ({
                        ...prev,
                        wake_word_phrase: e.target.value,
                      }))
                    }
                  />
                </div>

                <div className="field">
                  <div className="fieldLabel">{t.wakeWordProvider}</div>
                  <select
                    className="selectInput"
                    value={wakeWordSettings.wake_word_provider}
                    onChange={(e) =>
                      setWakeWordSettings((prev) => ({
                        ...prev,
                        wake_word_provider: e.target.value as WakeWordProvider,
                      }))
                    }
                  >
                    <option value="none">none</option>
                    <option value="disabled">disabled</option>
                    <option value="mic-test">mic-test</option>
                    <option value="mock">mock</option>
                    <option value="porcupine">porcupine</option>
                  </select>
                </div>

                <div className="field">
                  <div className="fieldLabel">
                    {t.wakeWordSensitivity}{" "}
                    {Math.round(wakeWordSettings.wake_word_sensitivity * 100)}%
                  </div>
                  <input
                    className="rangeInput"
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={wakeWordSettings.wake_word_sensitivity}
                    onChange={(e) =>
                      setWakeWordSettings((prev) => ({
                        ...prev,
                        wake_word_sensitivity: Number(e.target.value),
                      }))
                    }
                  />
                </div>

                <button
                  className="saveBtn"
                  onClick={() => void saveWakeWordSettings()}
                  disabled={wakeWordSaving}
                  type="button"
                >
                  {wakeWordSaving ? t.saving : t.wakeWordSave}
                </button>

                <button
                  className="saveBtn"
                  onClick={() => void startWakeWordListener()}
                  disabled={!wakeWordSettings.wake_word_enabled}
                  type="button"
                >
                  {t.wakeWordStart}
                </button>

                <button
                  className="saveBtn"
                  onClick={() => void stopWakeWordListener()}
                  type="button"
                >
                  {t.wakeWordStop}
                </button>
              </div>

              <div className={`statusPill ${wakeWordStatus.state}`}>
                <Radio size={14} />
                <span className="statusDot" />
                <span>
                  {t.wakeWordStatus}: {wakeWordStatus.state}
                  {wakeWordStatus.message ? ` - ${wakeWordStatus.message}` : ""}
                </span>
                <button
                  className="saveBtn"
                  onClick={() => void refreshWakeWord()}
                  type="button"
                >
                  {t.wakeWordRefresh}
                </button>
              </div>

              <div className="wakeMetrics">
                <div className="wakeMetric">
                  <div className="metricLabel">{t.wakeWordInputDevice}</div>
                  <div className="metricValue">
                    {wakeWordStatus.selected_input_device ||
                      wakeWordStatus.available_input_devices[0] ||
                      "none"}
                  </div>
                </div>

                <div className="wakeMetric">
                  <div className="metricLabel">{t.wakeWordLastAudio}</div>
                  <div className="metricValue">
                    {wakeWordStatus.last_audio_at || "none"}
                  </div>
                </div>

                <div className="wakeMetric">
                  <div className="metricLabel">{t.wakeWordChunks}</div>
                  <div className="metricValue">
                    {wakeWordStatus.audio_chunks_seen}
                  </div>
                </div>

                <div className="wakeMetric">
                  <div className="metricLabel">{t.wakeWordInputLevel}</div>
                  <div className="metricValue">
                    {wakeWordStatus.input_level == null
                      ? "none"
                      : `${Math.round(wakeWordStatus.input_level * 100)}%`}
                  </div>
                </div>

                {wakeWordStatus.last_error && (
                  <div className="wakeMetric">
                    <div className="metricLabel">{t.wakeWordLastError}</div>
                    <div className="metricValue">{wakeWordStatus.last_error}</div>
                  </div>
                )}
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
                placeholder={t.searchPlaceholder}
              />
            </div>

            <div className="commandsArea">
              {filteredGroups.length === 0 ? (
                <div className="empty">{t.noCommandsFound}</div>
              ) : (
                filteredGroups.map((group) => {
                  const isOpen = !!openGroups[group.key];

                  return (
                    <div className="group" key={group.key}>
                      <button
                        className="groupToggle"
                        type="button"
                        onClick={() => toggleGroup(group.key)}
                      >
                        <div className="groupHeaderLeft">
                          {group.icon}
                          <div className="groupMeta">
                            <div className="groupTitle">{group.title}</div>
                            <div className="groupCount">
                              {t.commandCount(group.items.length)}
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
                              key={`${group.key}-${item.command}-${item.description}`}
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
