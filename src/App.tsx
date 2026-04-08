import { useEffect, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { emitTo, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";

type ContextPayload = {
  text: string;
  hint?: string;
  autoRun?: boolean;
};

type BlobMood = "idle" | "happy" | "thinking" | "sleepy" | "music" | "love";

type PresenceState = "visible" | "sleeping" | "hidden" | "entering" | "exiting";

type BlobConfig = {
  cols: [string, string, string];
  eyeShape: "open" | "happy" | "half" | "wide" | "heart";
  brow: number;
  mouth: number;
  wobble: number;
  speed: number;
  heart: number;
  bulb: number;
  think: number;
  dance: number;
  sleepy: number;
};

const BLOB_MOODS: Record<BlobMood, BlobConfig> = {
  idle: {
    cols: ["#99c2ff", "#c8e6ff", "#e7d9ff"],
    eyeShape: "open",
    brow: 0,
    mouth: 8,
    wobble: 0.72,
    speed: 0.72,
    heart: 0,
    bulb: 0,
    think: 0,
    dance: 0,
    sleepy: 0,
  },
  happy: {
    cols: ["#a1c4fd", "#c2e9fb", "#fbc2eb"],
    eyeShape: "open",
    brow: -2,
    mouth: 16,
    wobble: 0.95,
    speed: 0.92,
    heart: 0.08,
    bulb: 0,
    think: 0,
    dance: 0,
    sleepy: 0,
  },
  thinking: {
    cols: ["#d7dde8", "#eef2f8", "#cad7ff"],
    eyeShape: "half",
    brow: -4,
    mouth: 4,
    wobble: 0.58,
    speed: 0.45,
    heart: 0,
    bulb: 0.08,
    think: 1,
    dance: 0,
    sleepy: 0,
  },
  sleepy: {
    cols: ["#accbee", "#e7f0fd", "#c3cfe2"],
    eyeShape: "half",
    brow: 3,
    mouth: 2,
    wobble: 0.35,
    speed: 0.28,
    heart: 0,
    bulb: 0,
    think: 0,
    dance: 0,
    sleepy: 1,
  },
  music: {
    cols: ["#84fab0", "#8fd3f4", "#c2e9fb"],
    eyeShape: "happy",
    brow: -2,
    mouth: 20,
    wobble: 1.45,
    speed: 1.65,
    heart: 0,
    bulb: 0,
    think: 0,
    dance: 1,
    sleepy: 0,
  },
  love: {
    cols: ["#ffb1bb", "#ffd8ef", "#fbc2eb"],
    eyeShape: "heart",
    brow: -4,
    mouth: 22,
    wobble: 0.62,
    speed: 0.92,
    heart: 1,
    bulb: 0,
    think: 0,
    dance: 0,
    sleepy: 0,
  },
};

function lerp(a: number, b: number, t: number) {
  return a + (b - a) * t;
}

function hexToRgb(hex: string) {
  const bigint = parseInt(hex.slice(1), 16);
  return [(bigint >> 16) & 255, (bigint >> 8) & 255, bigint & 255];
}

function rgbToHex(r: number, g: number, b: number) {
  return (
    "#" +
    ((1 << 24) | (r << 16) | (g << 8) | b)
      .toString(16)
      .slice(1)
      .padStart(6, "0")
  );
}

function lerpColor(c1: string, c2: string, t: number) {
  const rgb1 = hexToRgb(c1);
  const rgb2 = hexToRgb(c2);
  return rgbToHex(
    Math.round(lerp(rgb1[0], rgb2[0], t)),
    Math.round(lerp(rgb1[1], rgb2[1], t)),
    Math.round(lerp(rgb1[2], rgb2[2], t))
  );
}

function getCirclePoint(angle: number, radius: number) {
  return { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius };
}

function getHeartPoint(angle: number, radius: number) {
  const t = angle - Math.PI / 2;
  const x = radius * 1.15 * Math.pow(Math.sin(t), 3);
  const y =
    -radius *
      0.9 *
      (1 * Math.cos(t) -
        0.35 * Math.cos(2 * t) -
        0.2 * Math.cos(3 * t) -
        0.05 * Math.cos(4 * t)) -
    10;
  return { x, y };
}

function getBulbPoint(angle: number, radius: number) {
  const x = Math.cos(angle) * radius * 0.95;
  let y = Math.sin(angle) * radius * 1.02;

  const topBias = Math.max(0, -Math.sin(angle));
  const bottomBias = Math.max(0, Math.sin(angle));

  const widenTop = topBias * 16;
  const neckPull = Math.exp(-Math.pow((angle + Math.PI / 2) / 0.42, 2)) * 18;
  const capPull = Math.exp(-Math.pow((angle - Math.PI * 1.5) / 0.24, 2)) * 20;

  const bx = x + Math.cos(angle) * widenTop - Math.cos(angle) * neckPull * 0.35;
  y -= topBias * 18;
  y -= bottomBias * 6;
  y -= capPull;

  return { x: bx, y };
}

function getThinkPoint(angle: number, radius: number) {
  const x = Math.cos(angle) * radius;
  let y = Math.sin(angle) * radius;

  const bumpL = Math.exp(-Math.pow((angle - Math.PI * 1.15) / 0.35, 2)) * 12;
  const bumpR = Math.exp(-Math.pow((angle - Math.PI * 1.85) / 0.35, 2)) * 12;

  y -= bumpL + bumpR;
  return { x, y };
}

function getBlobPoints(t: number, cfg: BlobConfig, radius = 88, stiffness = 0) {
  const pts: { x: number; y: number }[] = [];
  const n = 44;

  for (let i = 0; i < n; i++) {
    const angle = (i / n) * Math.PI * 2;

    const circle = getCirclePoint(angle, radius);
    const heart = getHeartPoint(angle, radius);
    const bulb = getBulbPoint(angle, radius);
    const think = getThinkPoint(angle, radius);

    let bx = circle.x;
    let by = circle.y;

    bx = lerp(bx, heart.x, cfg.heart);
    by = lerp(by, heart.y, cfg.heart);

    bx = lerp(bx, bulb.x, cfg.bulb);
    by = lerp(by, bulb.y, cfg.bulb);

    bx = lerp(bx, think.x, cfg.think * 0.7);
    by = lerp(by, think.y, cfg.think * 0.7);

    const activeWobble =
      cfg.wobble * (1 - cfg.heart * 0.5) * (1 - stiffness * 0.88);

    let rOffset = 0;
    rOffset += Math.sin(angle * 3 + t * cfg.speed) * 8 * activeWobble;
    rOffset += Math.cos(angle * 2 - t * cfg.speed * 0.8) * 12 * activeWobble;
    rOffset +=
      Math.sin(angle * 5 + t * 4.5) * 4 * cfg.dance * (1 - stiffness * 0.7);
    rOffset *= 1 - cfg.sleepy * 0.2;

    const nx = Math.cos(angle);
    const ny = Math.sin(angle);

    pts.push({
      x: bx + nx * rOffset,
      y: by + ny * rOffset,
    });
  }

  return pts;
}

function drawHeartEye(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  scale = 1
) {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(scale, scale);

  ctx.beginPath();
  ctx.moveTo(0, 6);
  ctx.bezierCurveTo(-10, -6, -18, 2, 0, 18);
  ctx.bezierCurveTo(18, 2, 10, -6, 0, 6);
  ctx.closePath();
  ctx.fillStyle = "#ff6fa8";
  ctx.fill();

  ctx.beginPath();
  ctx.arc(-3, 5, 1.4, 0, Math.PI * 2);
  ctx.fillStyle = "rgba(255,255,255,0.85)";
  ctx.fill();
  ctx.restore();
}

function BlobAvatar({
  irisOffset,
  state,
  dragging,
  presenceState,
  onMouseDown,
  onPet,
  onContextMenu,
  onActivity,
}: {
  irisOffset: { x: number; y: number };
  state: BlobMood;
  dragging: boolean;
  presenceState: PresenceState;
  onMouseDown: (event: React.MouseEvent<HTMLCanvasElement>) => void;
  onPet: () => void;
  onContextMenu: (event: React.MouseEvent<HTMLCanvasElement>) => void;
  onActivity: () => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [time, setTime] = useState(0);
  const [blinkTimer, setBlinkTimer] = useState(3);
  const [petScore, setPetScore] = useState(0);
  const lastPetXRef = useRef<number | null>(null);
  const lastPetTimeRef = useRef(0);

  const cfgRef = useRef<BlobConfig>({
    ...BLOB_MOODS.idle,
    cols: [...BLOB_MOODS.idle.cols] as [string, string, string],
  });

  const targetCfg = useMemo(() => BLOB_MOODS[state], [state]);

  useEffect(() => {
    const id = window.setInterval(() => {
      setTime((v) => v + 0.02);
      setBlinkTimer((v) => {
        const next = v - 0.02;
        return next <= 0 ? Math.random() * 3 + 2 : next;
      });
    }, 16);

    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const W = canvas.width;
    const H = canvas.height;
    const CX = W / 2;
    const CY = H / 2 - 6;

    const cfg = cfgRef.current;
    const tSpeed = dragging ? 0.14 : 0.09;

    cfg.brow = lerp(cfg.brow, targetCfg.brow, tSpeed);
    cfg.mouth = lerp(cfg.mouth, targetCfg.mouth, tSpeed);
    cfg.wobble = lerp(cfg.wobble, targetCfg.wobble, tSpeed);
    cfg.speed = lerp(cfg.speed, targetCfg.speed, tSpeed);
    cfg.heart = lerp(cfg.heart, targetCfg.heart, tSpeed);
    cfg.bulb = lerp(cfg.bulb, targetCfg.bulb, tSpeed);
    cfg.think = lerp(cfg.think, targetCfg.think, tSpeed);
    cfg.dance = lerp(cfg.dance, targetCfg.dance, tSpeed);
    cfg.sleepy = lerp(cfg.sleepy, targetCfg.sleepy, tSpeed);
    cfg.eyeShape = targetCfg.eyeShape;
    cfg.cols = [
      lerpColor(cfg.cols[0], targetCfg.cols[0], tSpeed),
      lerpColor(cfg.cols[1], targetCfg.cols[1], tSpeed),
      lerpColor(cfg.cols[2], targetCfg.cols[2], tSpeed),
    ];

    ctx.clearRect(0, 0, W, H);

    const stiffness = dragging ? 1 : 0;
    const blink = blinkTimer < 0.15 ? 0.12 : 1;
    const danceBounce =
      Math.abs(Math.sin(time * 6)) * 10 * cfg.dance * (dragging ? 0.2 : 1);
    const danceSwing =
      Math.sin(time * 6) * 4 * cfg.dance * (dragging ? 0.2 : 1);
    const sleepyDrift =
      Math.sin(time * 1.2) * 4 * cfg.sleepy * (dragging ? 0.2 : 1);
    const floatY = dragging
      ? 0
      : Math.sin(time * 2) * 5 + danceBounce * 0.35 + sleepyDrift;

    const pts = getBlobPoints(time, cfg, 88, stiffness);

    ctx.save();
    ctx.translate(0, floatY);

    ctx.beginPath();
    ctx.moveTo(CX + pts[0].x, CY + pts[0].y);
    for (let i = 0; i < pts.length; i++) {
      const p1 = pts[i];
      const p2 = pts[(i + 1) % pts.length];
      const midX = CX + (p1.x + p2.x) / 2;
      const midY = CY + (p1.y + p2.y) / 2;
      ctx.quadraticCurveTo(CX + p1.x, CY + p1.y, midX, midY);
    }
    ctx.closePath();

    const grad = ctx.createRadialGradient(
      CX - 30,
      CY - 40,
      10,
      CX + 20,
      CY + 30,
      140
    );
    grad.addColorStop(0, cfg.cols[0]);
    grad.addColorStop(0.5, cfg.cols[1]);
    grad.addColorStop(1, cfg.cols[2]);

    ctx.shadowColor = cfg.cols[0];
    ctx.shadowBlur = dragging ? 10 : 18;
    ctx.fillStyle = grad;
    ctx.fill();
    ctx.shadowBlur = 0;

    const highlight = ctx.createRadialGradient(
      CX - 40,
      CY - 40,
      5,
      CX - 40,
      CY - 40,
      40
    );
    highlight.addColorStop(0, "rgba(255,255,255,0.8)");
    highlight.addColorStop(1, "rgba(255,255,255,0)");
    ctx.fillStyle = highlight;
    ctx.fill();

    const eyeOffsetX = 28;
    const eyeOffsetY = -8 - cfg.heart * 5 - cfg.think * 7;
    const faceLiftY = danceBounce * 0.35;
    const blushY = CY + 15 - cfg.heart * 5 + faceLiftY;

    [-1, 1].forEach((side) => {
      const browX = CX + 31 * side;
      const browY = CY - 26 - cfg.heart * 4 + faceLiftY;
      const slope = cfg.brow * (side === -1 ? 1 : -1);

      ctx.beginPath();
      ctx.moveTo(browX - 10, browY + slope + 1);
      ctx.quadraticCurveTo(
        browX,
        browY - 4.5 + slope * 0.25,
        browX + 10,
        browY - slope + 1
      );
      ctx.strokeStyle = "rgba(45,55,72,0.72)";
      ctx.lineWidth = 2.2;
      ctx.lineCap = "round";
      ctx.stroke();

      ctx.beginPath();
      ctx.ellipse(CX + 42 * side, blushY, 11, 5.5, 0, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(255, 110, 160, 0.22)";
      ctx.fill();
    });

    [-1, 1].forEach((side) => {
      const ex =
        CX +
        eyeOffsetX * side +
        irisOffset.x * 0.5 * (dragging ? 0.45 : 1) +
        danceSwing * side * cfg.dance;
      const ey =
        CY +
        eyeOffsetY +
        irisOffset.y * 0.5 * (dragging ? 0.45 : 1) -
        cfg.think * 3;

      if (cfg.eyeShape === "heart") {
        drawHeartEye(ctx, ex, ey, 0.9);
        return;
      }

      const eRadius = cfg.eyeShape === "wide" ? 12 : 9;
      const scaleY = cfg.eyeShape === "half" ? 0.4 : blink;

      ctx.save();
      ctx.translate(ex, ey);
      ctx.scale(1, Math.max(0.1, scaleY));

      if (cfg.eyeShape === "happy") {
        ctx.beginPath();
        ctx.arc(0, 2, eRadius, Math.PI, 0);
        ctx.lineWidth = 3;
        ctx.strokeStyle = "#2d3748";
        ctx.stroke();
      } else {
        ctx.beginPath();
        ctx.arc(0, 0, eRadius, 0, Math.PI * 2);
        ctx.fillStyle = "#fff";
        ctx.fill();

        ctx.beginPath();
        ctx.arc(
          irisOffset.x * (dragging ? 0.45 : 1),
          irisOffset.y * (dragging ? 0.45 : 1) - cfg.think * 2.5,
          eRadius * 0.55,
          0,
          Math.PI * 2
        );
        ctx.fillStyle = "#2d3748";
        ctx.fill();

        ctx.beginPath();
        ctx.arc(
          irisOffset.x * (dragging ? 0.45 : 1) - 2,
          irisOffset.y * (dragging ? 0.45 : 1) - cfg.think * 2.5 - 2,
          eRadius * 0.2,
          0,
          Math.PI * 2
        );
        ctx.fillStyle = "#fff";
        ctx.fill();
      }

      ctx.restore();
    });

    const mouthY = CY + 25 - cfg.heart * 5 + faceLiftY;
    ctx.beginPath();
    ctx.moveTo(CX - 12 + irisOffset.x * 0.18, mouthY + irisOffset.y * 0.18);
    ctx.quadraticCurveTo(
      CX + irisOffset.x * 0.18,
      mouthY + cfg.mouth + irisOffset.y * 0.18,
      CX + 12 + irisOffset.x * 0.18,
      mouthY + irisOffset.y * 0.18
    );
    ctx.lineWidth = 3;
    ctx.strokeStyle = "#2d3748";
    ctx.lineCap = "round";
    ctx.stroke();

    if (presenceState !== "hidden") {
      const bubbleText =
        state === "thinking"
          ? "..."
          : state === "music"
          ? "♪"
          : state === "love"
          ? "♡"
          : state === "sleepy"
          ? "zZ"
          : "";

      if (bubbleText) {
        const bx = CX + 68;
        const by = CY - 70;

        ctx.save();
        ctx.globalAlpha = presenceState === "sleeping" ? 0.7 : 0.9;
        ctx.translate(bx, by + Math.sin(time * 2) * 2);

        const rectX = -22;
        const rectY = -12;
        const rectW = 44;
        const rectH = 24;
        const r = 12;

        ctx.beginPath();
        ctx.moveTo(rectX + r, rectY);
        ctx.lineTo(rectX + rectW - r, rectY);
        ctx.quadraticCurveTo(rectX + rectW, rectY, rectX + rectW, rectY + r);
        ctx.lineTo(rectX + rectW, rectY + rectH - r);
        ctx.quadraticCurveTo(
          rectX + rectW,
          rectY + rectH,
          rectX + rectW - r,
          rectY + rectH
        );
        ctx.lineTo(rectX + r, rectY + rectH);
        ctx.quadraticCurveTo(rectX, rectY + rectH, rectX, rectY + rectH - r);
        ctx.lineTo(rectX, rectY + r);
        ctx.quadraticCurveTo(rectX, rectY, rectX + r, rectY);
        ctx.closePath();
        ctx.fillStyle = "rgba(255,255,255,0.9)";
        ctx.fill();

        ctx.beginPath();
        ctx.moveTo(-8, 10);
        ctx.lineTo(-14, 18);
        ctx.lineTo(-2, 10);
        ctx.fill();

        ctx.fillStyle = "#526684";
        ctx.font = "bold 14px sans-serif";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(bubbleText, 0, 0);

        ctx.restore();
      }
    }

    ctx.restore();
  }, [time, blinkTimer, irisOffset, targetCfg, dragging, presenceState, state]);

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    onActivity();

    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const cx = e.currentTarget.width / 2;
    const cy = e.currentTarget.height / 2 - 30;

    const dx = x - cx;
    const dy = y - cy;

    const inHead = (dx * dx) / (92 * 92) + (dy * dy) / (52 * 52) <= 1;
    if (!inHead) {
      setPetScore((v) => v * 0.85);
      lastPetXRef.current = null;
      return;
    }

    const now = performance.now();
    if (lastPetXRef.current !== null) {
      const moveX = Math.abs(x - lastPetXRef.current);
      const dt = now - lastPetTimeRef.current;

      setPetScore((prev) => {
        const next =
          moveX > 8 && moveX < 70 && dt < 180 ? prev + 1 : prev * 0.9;
        if (next > 6) {
          onPet();
          return 0;
        }
        return next;
      });
    }

    lastPetXRef.current = x;
    lastPetTimeRef.current = now;
  };

  return (
    <canvas
      ref={canvasRef}
      width={250}
      height={250}
      onMouseDown={onMouseDown}
      onMouseMove={handleMouseMove}
      onMouseLeave={() => {
        setPetScore((v) => v * 0.8);
        lastPetXRef.current = null;
      }}
      onContextMenu={onContextMenu}
      style={{
        display: "block",
        width: 250,
        height: 250,
        background: "transparent",
        cursor: dragging ? "grabbing" : "grab",
        pointerEvents: "auto",
      }}
    />
  );
}

async function ensureBubbleWindow() {
  const existing = await WebviewWindow.getByLabel("bubble");
  if (existing) return existing;

  return new WebviewWindow("bubble", {
    url: "bubble.html",
    title: "Companion Bubble",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    skipTaskbar: true,
    resizable: false,
    width: 520,
    height: 620,
    visible: false,
  });
}

async function ensureSpeechWindow() {
  const existing = await WebviewWindow.getByLabel("speech");
  if (existing) return existing;

  return new WebviewWindow("speech", {
    url: "speech.html",
    title: "Companion Speech",
    transparent: true,
    decorations: false,
    alwaysOnTop: true,
    shadow: false,
    skipTaskbar: true,
    resizable: false,
    width: 220,
    height: 120,
    visible: false,
  });
}

async function positionBubbleWindow() {
  const bubble = await ensureBubbleWindow();
  const main = getCurrentWindow();

  const pos = await main.outerPosition();
  const size = await main.outerSize();

  await bubble.setPosition(
    new LogicalPosition(pos.x + size.width - 8, pos.y + 8)
  );
}

async function positionSpeechWindow() {
  const speech = await ensureSpeechWindow();
  const main = getCurrentWindow();

  const pos = await main.outerPosition();
  const size = await main.outerSize();

  await speech.setPosition(
    new LogicalPosition(pos.x + size.width / 2 - 110, pos.y - 78)
  );
}

async function showBubbleWindow() {
  const bubble = await ensureBubbleWindow();
  await positionBubbleWindow();
  await bubble.show();
}

async function showSpeechWindow(text: string) {
  const speech = await ensureSpeechWindow();
  await positionSpeechWindow();
  await emitTo("speech", "speech-text", text);
  await speech.show();
}

async function sendContextToBubble(payload: ContextPayload) {
  await ensureBubbleWindow();
  await emitTo("bubble", "companion-context", payload);
}

function clampContextMenuPosition(x: number, y: number) {
  const menuWidth = 226;
  const menuHeight = 430;
  const padding = 12;

  const maxX = window.innerWidth - menuWidth - padding;
  const maxY = window.innerHeight - menuHeight - padding;

  return {
    x: Math.max(padding, Math.min(x, maxX)),
    y: Math.max(padding, Math.min(y, maxY)),
  };
}

export default function App() {
  const [copiedText, setCopiedText] = useState("Noch nichts kopiert");
  const [activeApp, setActiveApp] = useState("unknown");
  const [hint, setHint] = useState("Rechtsklick auf den Blob für Optionen");
  const [irisOffset, setIrisOffset] = useState({ x: 0, y: 0 });
  const [blobMood, setBlobMood] = useState<BlobMood>("idle");
  const [dragging, setDragging] = useState(false);
  const [pinned, setPinned] = useState(true);
  const [presenceState, setPresenceState] = useState<PresenceState>("visible");
  const [contextMenu, setContextMenu] = useState({
    open: false,
    x: 0,
    y: 0,
  });

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const speechTimerRef = useRef<number | null>(null);
  const blobTimerRef = useRef<number | null>(null);
  const sleepTimerRef = useRef<number | null>(null);
  const hideTimerRef = useRef<number | null>(null);

  const pulseBlob = (
    next: BlobMood,
    ms = 1600,
    fallback: BlobMood = "idle"
  ) => {
    setBlobMood(next);
    if (blobTimerRef.current) {
      window.clearTimeout(blobTimerRef.current);
    }
    blobTimerRef.current = window.setTimeout(() => {
      setBlobMood(fallback);
    }, ms);
  };

  const markActivity = () => {
    if (presenceState === "hidden" || presenceState === "sleeping") {
      setPresenceState("entering");
      window.setTimeout(() => {
        setPresenceState("visible");
      }, 420);
    }

    if (sleepTimerRef.current) window.clearTimeout(sleepTimerRef.current);
    if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);

    sleepTimerRef.current = window.setTimeout(() => {
      setPresenceState("sleeping");
      setBlobMood("sleepy");
    }, 2 * 60 * 1000);

    hideTimerRef.current = window.setTimeout(() => {
      setPresenceState("exiting");
      window.setTimeout(() => {
        setPresenceState("hidden");
      }, 420);
    }, 6 * 60 * 1000);
  };

  const closeMenu = () => {
    setContextMenu({ open: false, x: 0, y: 0 });
  };

  const mediaPlayPause = async () => {
    markActivity();
    await invoke("media_play_pause").catch(console.error);
    pulseBlob("music", 2200, "happy");
  };

  const mediaPrev = async () => {
    markActivity();
    await invoke("media_prev_track").catch(console.error);
    pulseBlob("music", 1800, "happy");
  };

  const mediaNext = async () => {
    markActivity();
    await invoke("media_next_track").catch(console.error);
    pulseBlob("music", 1800, "happy");
  };

  const volumeDown = async () => {
    markActivity();
    await invoke("change_system_volume", { delta: -0.05 }).catch(console.error);
    pulseBlob("happy", 900);
  };

  const volumeUp = async () => {
    markActivity();
    await invoke("change_system_volume", { delta: 0.05 }).catch(console.error);
    pulseBlob("happy", 900);
  };

  const toggleMute = async () => {
    markActivity();
    await invoke("toggle_system_mute").catch(console.error);
    pulseBlob("thinking", 1000);
  };

  useEffect(() => {
    markActivity();

    return () => {
      if (speechTimerRef.current) window.clearTimeout(speechTimerRef.current);
      if (blobTimerRef.current) window.clearTimeout(blobTimerRef.current);
      if (sleepTimerRef.current) window.clearTimeout(sleepTimerRef.current);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
    };
  }, []);

  useEffect(() => {
    const onUserActivity = () => markActivity();

    window.addEventListener("mousemove", onUserActivity);
    window.addEventListener("mousedown", onUserActivity);
    window.addEventListener("keydown", onUserActivity);

    return () => {
      window.removeEventListener("mousemove", onUserActivity);
      window.removeEventListener("mousedown", onUserActivity);
      window.removeEventListener("keydown", onUserActivity);
    };
  }, [presenceState]);

  useEffect(() => {
    const win = getCurrentWindow();
    win.setAlwaysOnTop(pinned).catch(console.error);
  }, [pinned]);

  useEffect(() => {
    const onWindowClick = () => closeMenu();
    window.addEventListener("click", onWindowClick);
    return () => window.removeEventListener("click", onWindowClick);
  }, []);

  useEffect(() => {
    let unlistenFn: null | (() => void) = null;

    const setup = async () => {
      unlistenFn = await listen<string>("companion-speech", async (event) => {
        const text = event.payload || "";
        if (!text.trim()) return;

        markActivity();
        pulseBlob("thinking", 900, "happy");
        setHint(text);

        await showSpeechWindow(text);

        if (speechTimerRef.current) {
          window.clearTimeout(speechTimerRef.current);
        }

        speechTimerRef.current = window.setTimeout(async () => {
          const speechWindow = await WebviewWindow.getByLabel("speech");
          if (speechWindow) {
            await speechWindow.hide().catch(() => {});
          }
        }, 4000);
      });
    };

    setup();

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [presenceState]);

  useEffect(() => {
    let unlistenMove: null | (() => void) = null;

    const setupMoveSync = async () => {
      const win = getCurrentWindow();
      unlistenMove = await win.onMoved(async () => {
        const bubble = await WebviewWindow.getByLabel("bubble");
        if (bubble) {
          await positionBubbleWindow().catch(console.error);
        }

        const speech = await WebviewWindow.getByLabel("speech");
        if (speech) {
          await positionSpeechWindow().catch(console.error);
        }
      });
    };

    setupMoveSync();

    return () => {
      if (unlistenMove) unlistenMove();
    };
  }, []);

  useEffect(() => {
    let unlistenHotkey: null | (() => void) = null;

    const setupHotkeyListener = async () => {
      unlistenHotkey = await listen<string>("companion-hotkey", async () => {
        try {
          markActivity();
          pulseBlob("thinking", 1100);

          const previous = (await readText().catch(() => "")) || "";
          await invoke("trigger_copy_shortcut");
          await new Promise((resolve) => setTimeout(resolve, 220));
          const selected = (await readText().catch(() => "")) || "";

          if (selected.trim()) {
            const trimmed = selected.slice(0, 1500);
            setCopiedText(trimmed);
            setHint("Markierter Text erkannt.");
            await showBubbleWindow();
            await sendContextToBubble({
              text: trimmed,
              hint: `Markierter Text automatisch übernommen. App: ${activeApp}`,
              autoRun: true,
            });
            pulseBlob("happy", 1400);
          } else {
            setHint("Kein markierter Text gefunden.");
            await showBubbleWindow();
            await sendContextToBubble({
              text: "",
              hint: `Kein markierter Text gefunden. App: ${activeApp}`,
            });
            pulseBlob("thinking", 1200);
          }

          if (previous && previous !== selected) {
            await writeText(previous).catch(() => {});
          }
        } catch (error) {
          setHint(`Hotkey-Fehler: ${String(error)}`);
          pulseBlob("thinking", 1200);
        }
      });
    };

    setupHotkeyListener();

    return () => {
      if (unlistenHotkey) unlistenHotkey();
    };
  }, [activeApp, presenceState]);

  useEffect(() => {
    const interval = window.setInterval(async () => {
      try {
        const app = await invoke<string>("get_active_app");
        setActiveApp(app);
      } catch {
        //
      }
    }, 1000);

    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    const interval = window.setInterval(async () => {
      try {
        const [mouseX, mouseY] = await invoke<[number, number]>(
          "get_cursor_position"
        );
        const wrap = wrapRef.current;
        if (!wrap) return;

        const win = getCurrentWindow();
        const winPos = await win.outerPosition();

        const rect = wrap.getBoundingClientRect();
        const centerX = winPos.x + rect.left + rect.width / 2;
        const centerY = winPos.y + rect.top + rect.height / 2 - 8;

        const dx = mouseX - centerX;
        const dy = mouseY - centerY;

        const maxX = 5.5;
        const maxY = 4.5;

        const normalizedX = Math.max(-1, Math.min(1, dx / 140));
        const normalizedY = Math.max(-1, Math.min(1, dy / 140));

        setIrisOffset({
          x: normalizedX * maxX,
          y: normalizedY * maxY,
        });
      } catch {
        //
      }
    }, 32);

    return () => window.clearInterval(interval);
  }, []);

  const refreshClipboard = async () => {
    try {
      markActivity();
      const text = await readText();
      if (!text?.trim()) {
        setHint("Im Clipboard ist gerade kein Text.");
        pulseBlob("thinking", 900);
        return;
      }

      const trimmed = text.slice(0, 1500);
      setCopiedText(trimmed);
      setHint("Clipboard aktualisiert.");
      await showBubbleWindow();
      await sendContextToBubble({
        text: trimmed,
        hint: `Clipboard manuell übernommen. App: ${activeApp}`,
      });
      pulseBlob("happy", 1200);
    } catch {
      setHint("Clipboard konnte nicht gelesen werden.");
      pulseBlob("thinking", 900);
    }
  };

  const openBubble = async () => {
    markActivity();
    await showBubbleWindow();
    await sendContextToBubble({
      text: copiedText === "Noch nichts kopiert" ? "" : copiedText,
      hint: `Bubble geöffnet. App: ${activeApp}`,
    });
    pulseBlob("happy", 1000);
  };

  const handleClose = async () => {
    try {
      const bubble = await WebviewWindow.getByLabel("bubble");
      if (bubble) await bubble.close();

      const speech = await WebviewWindow.getByLabel("speech");
      if (speech) await speech.close();

      await getCurrentWindow().close();
    } catch (error) {
      console.error(error);
    }
  };

  const handleAvatarMouseDown = async (
    event: React.MouseEvent<HTMLCanvasElement>
  ) => {
    if (event.button !== 0) return;
    markActivity();
    setDragging(true);

    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error(error);
    } finally {
      setDragging(false);
    }
  };

  const handleAvatarContextMenu = (
    event: React.MouseEvent<HTMLCanvasElement>
  ) => {
    event.preventDefault();
    event.stopPropagation();
    markActivity();

    const pos = clampContextMenuPosition(event.clientX, event.clientY);

    setContextMenu({
      open: true,
      x: pos.x,
      y: pos.y,
    });
  };

  return (
    <div
      className="scene-shell"
      style={{
        width: "100vw",
        height: "100vh",
        background: "transparent",
        overflow: "hidden",
        position: "relative",
      }}
    >
      <motion.div
        ref={wrapRef}
        className="companion-wrap"
        initial={{ opacity: 0, y: 22, scale: 0.86 }}
        animate={{
          opacity:
            presenceState === "hidden"
              ? 0.18
              : presenceState === "sleeping"
              ? 0.72
              : 1,
          x:
            presenceState === "hidden"
              ? 92
              : presenceState === "exiting"
              ? 48
              : presenceState === "entering"
              ? [-24, 8, 0]
              : 0,
          y:
            presenceState === "hidden"
              ? 34
              : presenceState === "sleeping"
              ? [0, -2, 0]
              : [0, -8, 0],
          scale:
            presenceState === "hidden"
              ? 0.72
              : presenceState === "sleeping"
              ? 0.9
              : 1,
          rotate: presenceState === "sleeping" ? -6 : 0,
        }}
        transition={{
          opacity: { duration: 0.4 },
          scale: { duration: 0.45 },
          rotate: { duration: 0.5 },
          x: { duration: 0.42, ease: "easeOut" },
          y: {
            duration: presenceState === "sleeping" ? 4.5 : 3.2,
            repeat: Infinity,
            ease: "easeInOut",
          },
        }}
        style={{
          position: "absolute",
          right: 16,
          bottom: 18,
          width: 250,
          height: 250,
          display: "grid",
          placeItems: "center",
        }}
      >
        <div
          className="shadow"
          style={{
            position: "absolute",
            bottom: 18,
            width: 120,
            height: 28,
            borderRadius: 999,
            background:
              "radial-gradient(circle, rgba(80, 110, 170, 0.18), rgba(80, 110, 170, 0))",
            filter: "blur(8px)",
            pointerEvents: "none",
          }}
        />

        <BlobAvatar
          irisOffset={irisOffset}
          state={blobMood}
          dragging={dragging}
          presenceState={presenceState}
          onMouseDown={handleAvatarMouseDown}
          onPet={() => pulseBlob("love", 1600, "happy")}
          onContextMenu={handleAvatarContextMenu}
          onActivity={markActivity}
        />
      </motion.div>

      {contextMenu.open && (
        <div
          className="context-menu"
          style={{
            left: contextMenu.x,
            top: contextMenu.y,
          }}
          onClick={(e) => e.stopPropagation()}
          onMouseLeave={closeMenu}
        >
          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await openBubble();
            }}
          >
            Bubble öffnen
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await refreshClipboard();
            }}
          >
            Clipboard übernehmen
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await mediaPlayPause();
            }}
          >
            Play / Pause
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await mediaPrev();
            }}
          >
            Vorheriger Track
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await mediaNext();
            }}
          >
            Nächster Track
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await volumeDown();
            }}
          >
            Leiser
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await volumeUp();
            }}
          >
            Lauter
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await toggleMute();
            }}
          >
            Mute umschalten
          </button>

          <button
            className="menu-btn"
            onClick={() => {
              closeMenu();
              setPinned((p) => !p);
              markActivity();
            }}
          >
            {pinned ? "Always on top aus" : "Always on top an"}
          </button>

          <button
            className="menu-btn"
            onClick={() => {
              closeMenu();
              setPresenceState("sleeping");
              setBlobMood("sleepy");
              markActivity();
            }}
          >
            Jetzt schlafen
          </button>

          <button
            className="menu-btn"
            onClick={async () => {
              closeMenu();
              await handleClose();
            }}
          >
            Schließen
          </button>

          <div className="menu-meta">
            {hint}
            <br />
            {activeApp}
          </div>
        </div>
      )}
    </div>
  );
}
