import { useEffect, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { emitTo, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { openSnipOverlay } from "./windows/snip-overlay/open";
import { ensureSnipPanelWindow } from "./windows/snip-panel/open";
import { ensureBubbleWindow } from "./windows/bubble/open";
import {
  ensureTimerOverlayWindow,
  showTimerOverlayWindow,
} from "./windows/timer-overlay/open";

import {
  showQuickMenuWindow,
  hideQuickMenuWindow,
} from "./windows/quick-menu/open";

type ContextPayload = {
  text: string;
  hint?: string;
  autoRun?: boolean;
};

type SnipCreatedPayload = {
  path: string;
  rect?: { x: number; y: number; width: number; height: number };
};

type BlobMood = "idle" | "happy" | "thinking" | "sleepy" | "music" | "love";

type PresenceState =
  | "visible"
  | "sleeping"
  | "hidden"
  | "entering"
  | "exiting"
  | "hidden_peek";

type HideGameState = "idle" | "seeking" | "found";

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
  hideGameState,
  peekVisible,
  foundBubbleText,
  onMouseDown,
  onPet,
  onContextMenu,
  onActivity,
}: {
  irisOffset: { x: number; y: number };
  state: BlobMood;
  dragging: boolean;
  presenceState: PresenceState;
  hideGameState: HideGameState;
  peekVisible: boolean;
  foundBubbleText: string;
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

    if (presenceState === "hidden_peek" && hideGameState === "seeking") {
      if (!peekVisible) {
        return;
      }

      const eyeY = H / 2 - 6;
      const leftX = W / 2 - 16;
      const rightX = W / 2 + 16;

      ctx.save();

      const glow = ctx.createRadialGradient(W / 2, eyeY, 1, W / 2, eyeY, 28);
      glow.addColorStop(0, "rgba(160,210,255,0.18)");
      glow.addColorStop(1, "rgba(160,210,255,0)");
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(W / 2, eyeY, 28, 0, Math.PI * 2);
      ctx.fill();

      ctx.beginPath();
      ctx.arc(leftX, eyeY, 7, 0, Math.PI * 2);
      ctx.fillStyle = "#ffffff";
      ctx.fill();

      ctx.beginPath();
      ctx.arc(leftX, eyeY, 3, 0, Math.PI * 2);
      ctx.fillStyle = "#243042";
      ctx.fill();

      ctx.beginPath();
      ctx.arc(leftX - 1.2, eyeY - 1.2, 1.1, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(255,255,255,0.9)";
      ctx.fill();

      ctx.beginPath();
      ctx.arc(rightX, eyeY, 7, 0, Math.PI * 2);
      ctx.fillStyle = "#ffffff";
      ctx.fill();

      ctx.beginPath();
      ctx.arc(rightX, eyeY, 3, 0, Math.PI * 2);
      ctx.fillStyle = "#243042";
      ctx.fill();

      ctx.beginPath();
      ctx.arc(rightX - 1.2, eyeY - 1.2, 1.1, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(255,255,255,0.9)";
      ctx.fill();

      ctx.restore();
      return;
    }

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
    if (foundBubbleText) {
      const bx = CX + 10;
      const by = CY - 78;

      ctx.save();
      ctx.translate(bx, by + Math.sin(time * 2) * 1.5);

      const text = foundBubbleText;
      ctx.font = "bold 12px sans-serif";
      const metrics = ctx.measureText(text);
      const padX = 12;
      const rectW = metrics.width + padX * 2;
      const rectH = 28;
      const rectX = -rectW / 2;
      const rectY = -rectH / 2;
      const r = 14;

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
      ctx.fillStyle = "rgba(255,255,255,0.94)";
      ctx.fill();

      ctx.beginPath();
      ctx.moveTo(-8, rectH / 2 - 2);
      ctx.lineTo(-13, rectH / 2 + 8);
      ctx.lineTo(-1, rectH / 2 - 2);
      ctx.fill();

      ctx.fillStyle = "#526684";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(text, 0, 1);

      ctx.restore();
    }
    ctx.restore();
  }, [
    time,
    blinkTimer,
    irisOffset,
    targetCfg,
    dragging,
    presenceState,
    state,
    hideGameState,
    peekVisible,
    foundBubbleText,
  ]);

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
  const canvasSize = hideGameState === "seeking" ? 120 : 250;
  return (
    <canvas
      ref={canvasRef}
      width={canvasSize}
      height={canvasSize}
      onMouseDown={onMouseDown}
      onMouseMove={handleMouseMove}
      onMouseLeave={() => {
        setPetScore((v) => v * 0.8);
        lastPetXRef.current = null;
      }}
      onContextMenu={onContextMenu}
      style={{
        display: "block",
        width: canvasSize,
        height: canvasSize,
        background: "transparent",
        cursor: dragging ? "grabbing" : "grab",
        pointerEvents: "auto",
      }}
    />
  );
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

  const bubbleWidth = 1040;
  const bubbleHeight = 135;
  const bottomMargin = 0;

  const screenWidth = window.screen.availWidth;
  const screenHeight = window.screen.availHeight;

  const x = Math.round((screenWidth - bubbleWidth) / 2);
  const y = Math.round(screenHeight - bubbleHeight - bottomMargin);

  await bubble.setPosition(new LogicalPosition(x, y));
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

  await bubble.show();
  await bubble.setFocus().catch(() => {});

  window.setTimeout(async () => {
    try {
      await emitTo("bubble", "bubble-show");
    } catch (error) {
      console.error("bubble-show emit failed", error);
    }
  }, 80);
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

export default function App() {
  const [copiedText, setCopiedText] = useState("Nothing copied yet");
  const [activeApp, setActiveApp] = useState("unknown");
  const [hint, setHint] = useState("Right-click the blob for options");
  const [irisOffset, setIrisOffset] = useState({ x: 0, y: 0 });
  const [blobMood, setBlobMood] = useState<BlobMood>("idle");
  const [dragging, setDragging] = useState(false);
  const [pinned, setPinned] = useState(true);
  const [presenceState, setPresenceState] = useState<PresenceState>("visible");
  const [hideGameState, setHideGameState] = useState<HideGameState>("idle");
  const [peekVisible, setPeekVisible] = useState(false);

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const speechTimerRef = useRef<number | null>(null);
  const blobTimerRef = useRef<number | null>(null);
  const sleepTimerRef = useRef<number | null>(null);
  const hideTimerRef = useRef<number | null>(null);
  const hideBlinkTimerRef = useRef<number | null>(null);
  const hideEndTimerRef = useRef<number | null>(null);
  const [foundBubbleText, setFoundBubbleText] = useState("");
  const foundBubbleTimerRef = useRef<number | null>(null);
  const lastWindowPosRef = useRef<LogicalPosition | null>(null);
  const lastWindowSizeRef = useRef<{ width: number; height: number } | null>(
    null
  );

  const pulseBlob = (
    next: BlobMood,
    ms = 1600,
    fallback: BlobMood = "idle"
  ) => {
    setBlobMood(next);
    if (foundBubbleTimerRef.current)
      window.clearTimeout(foundBubbleTimerRef.current);
    if (blobTimerRef.current) {
      window.clearTimeout(blobTimerRef.current);
    }
    blobTimerRef.current = window.setTimeout(() => {
      setBlobMood(fallback);
    }, ms);
  };

  const stopHideAndSeek = () => {
    if (hideBlinkTimerRef.current) {
      window.clearInterval(hideBlinkTimerRef.current);
      hideBlinkTimerRef.current = null;
    }

    if (hideEndTimerRef.current) {
      window.clearTimeout(hideEndTimerRef.current);
      hideEndTimerRef.current = null;
    }

    setPeekVisible(false);
  };

  const moveWindowToHideSpot = async () => {
    const win = getCurrentWindow();

    const currentPos = await win.outerPosition();
    const currentSize = await win.outerSize();

    lastWindowPosRef.current = new LogicalPosition(currentPos.x, currentPos.y);
    lastWindowSizeRef.current = {
      width: currentSize.width,
      height: currentSize.height,
    };

    const hideWindowWidth = 120;
    const hideWindowHeight = 120;
    const margin = 8;

    const screenW = window.screen.availWidth || window.screen.width;
    const screenH = window.screen.availHeight || window.screen.height;

    const maxX = Math.max(margin, screenW - hideWindowWidth - margin);
    const maxY = Math.max(margin, screenH - hideWindowHeight - margin);

    const randomX = Math.floor(margin + Math.random() * (maxX - margin));
    const randomY = Math.floor(margin + Math.random() * (maxY - margin));

    await win.setSize({
      type: "Logical",
      width: hideWindowWidth,
      height: hideWindowHeight,
    });

    await win.setPosition(new LogicalPosition(randomX, randomY));
  };

  const restoreMainWindowPosition = async () => {
    const win = getCurrentWindow();

    const previousSize = lastWindowSizeRef.current;
    const previousPos = lastWindowPosRef.current;

    await win.setSize({
      type: "Logical",
      width: previousSize?.width ?? 260,
      height: previousSize?.height ?? 260,
    });

    await win.setPosition(
      previousPos ??
        new LogicalPosition(
          window.screen.availWidth - 300,
          window.screen.availHeight - 320
        )
    );
  };

  const restoreMainWindowSizeAtCurrentSpot = async () => {
    const win = getCurrentWindow();
    const currentPos = await win.outerPosition();
    const currentSize = await win.outerSize();

    const targetWidth = lastWindowSizeRef.current?.width ?? 260;
    const targetHeight = lastWindowSizeRef.current?.height ?? 260;

    const centerX = currentPos.x + currentSize.width / 2;
    const centerY = currentPos.y + currentSize.height / 2;

    const nextX = Math.round(centerX - targetWidth / 2);
    const nextY = Math.round(centerY - targetHeight / 2);

    await win.setSize({
      type: "Logical",
      width: targetWidth,
      height: targetHeight,
    });

    await win.setPosition(new LogicalPosition(nextX, nextY));
  };

  const startHideAndSeek = async () => {
    stopHideAndSeek();
    await closeMenu();

    setHideGameState("seeking");
    setPresenceState("hidden_peek");
    setPeekVisible(false);
    setHint("Find me...");

    void moveWindowToHideSpot().catch(console.error);

    pulseBlob("thinking", 1200, "idle");

    hideBlinkTimerRef.current = window.setInterval(() => {
      setPeekVisible(true);
      window.setTimeout(() => setPeekVisible(false), 220);
    }, 1800);

    hideEndTimerRef.current = window.setTimeout(() => {
      stopHideAndSeek();
      setHideGameState("idle");
      setPresenceState("entering");
      setHint("You did not find me in time.");
      pulseBlob("sleepy", 1200, "idle");

      void restoreMainWindowPosition().catch(console.error);

      window.setTimeout(() => {
        setPresenceState("visible");
      }, 420);
    }, 45000);
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

  const closeMenu = async () => {
    await hideQuickMenuWindow().catch(() => {});
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

  const openSnip = async () => {
    markActivity();
    await closeMenu();
    setHint("Snip mode opened");
    pulseBlob("thinking", 1000);
    await openSnipOverlay().catch((error) => {
      console.error(error);
      setHint(`Snip overlay error: ${String(error)}`);
    });
  };

  useEffect(() => {
    let unlistenStarted: null | (() => void) = null;
    let unlistenFinished: null | (() => void) = null;

    const setup = async () => {
      unlistenStarted = await listen<{
        seconds: number;
        label: string;
        startedAt: number;
      }>("companion-timer-started", async (event) => {
        const win = await ensureTimerOverlayWindow();

        await showTimerOverlayWindow().catch(console.error);

        await emitTo(
          "timer-overlay",
          "timer-overlay-start",
          event.payload
        ).catch(console.error);

        markActivity();
        setHint("Timer gestartet");
        pulseBlob("thinking", 900, "happy");
      });

      unlistenFinished = await listen<{
        seconds: number;
        text: string;
      }>("companion-timer-finished", async (event) => {
        await emitTo(
          "timer-overlay",
          "timer-overlay-finished",
          event.payload
        ).catch(console.error);

        markActivity();
        setHint(event.payload?.text || "Timer fertig");
        pulseBlob("happy", 1800, "idle");
      });
    };

    void setup();

    return () => {
      unlistenStarted?.();
      unlistenFinished?.();
    };
  }, []);

  useEffect(() => {
    markActivity();

    return () => {
      if (speechTimerRef.current) window.clearTimeout(speechTimerRef.current);
      if (blobTimerRef.current) window.clearTimeout(blobTimerRef.current);
      if (sleepTimerRef.current) window.clearTimeout(sleepTimerRef.current);
      if (hideTimerRef.current) window.clearTimeout(hideTimerRef.current);
      if (hideBlinkTimerRef.current)
        window.clearInterval(hideBlinkTimerRef.current);
      if (hideEndTimerRef.current) window.clearTimeout(hideEndTimerRef.current);
    };
  }, []);

  useEffect(() => {
    ensureBubbleWindow().catch(console.error);
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
    const onWindowClick = () => {
      void closeMenu();
    };

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
    let unlistenToggle: null | (() => void) = null;
    let unlistenSnipHotkey: null | (() => void) = null;

    const setupHotkeyListener = async () => {
      unlistenToggle = await listen("companion-toggle", async () => {
        console.log("[main] companion-toggle received");

        try {
          markActivity();
          pulseBlob("thinking", 900);

          const bubble = await ensureBubbleWindow();
          const isVisible = await bubble.isVisible();

          console.log("[main] bubble visible before toggle:", isVisible);

          if (isVisible) {
            await emitTo("bubble", "bubble-hide");
            console.log("[main] bubble-hide emitted");
          } else {
            await positionBubbleWindow().catch(console.error);
            await bubble.show();
            await bubble.setFocus().catch(() => {});

            window.setTimeout(async () => {
              try {
                await emitTo("bubble", "bubble-show");
                console.log("[main] bubble-show emitted");
              } catch (error) {
                console.error("bubble-show emit failed", error);
              }
            }, 80);
          }
        } catch (error) {
          console.error("toggle bubble failed", error);
        }
      });

      unlistenSnipHotkey = await listen("companion-snip-hotkey", async () => {
        await openSnip();
      });
    };

    void setupHotkeyListener();

    return () => {
      if (unlistenToggle) unlistenToggle();
      if (unlistenSnipHotkey) unlistenSnipHotkey();
    };
  }, []);

  useEffect(() => {
    let unlistenHideAndSeek: null | (() => void) = null;

    const setup = async () => {
      unlistenHideAndSeek = await listen("start-hide-and-seek", async () => {
        markActivity();
        await startHideAndSeek();
      });
    };

    setup();

    return () => {
      if (unlistenHideAndSeek) unlistenHideAndSeek();
    };
  }, []);

  useEffect(() => {
    let unlistenSnipCreated: null | (() => void) = null;

    const setupSnipCreated = async () => {
      unlistenSnipCreated = await listen<SnipCreatedPayload>(
        "snip-created",
        async (event) => {
          try {
            const path = event.payload.path;
            if (!path) return;

            const context = await invoke<{
              app_name: string;
              window_title: string;
              context_domain: string;
            }>("get_active_snip_context").catch(() => ({
              app_name: "unknown",
              window_title: "",
              context_domain: "desktop",
            }));

            const panel = await ensureSnipPanelWindow();

            await panel.show();
            await panel.setFocus();

            await emitTo("snip-panel", "snip-panel-data", {
              path,
              app: context.app_name || "unknown",
              windowTitle: context.window_title || "",
              contextDomain: context.context_domain || "desktop",
            });

            setHint("Snip captured");
            pulseBlob("happy", 1200);
            markActivity();
          } catch (error) {
            console.error("snip-panel open failed:", error);
            setHint("Snip panel failed");
            pulseBlob("thinking", 1200);
          }
        }
      );
    };

    setupSnipCreated();

    return () => {
      if (unlistenSnipCreated) unlistenSnipCreated();
    };
  }, []);

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

  useEffect(() => {
    const onKeyDown = async (event: KeyboardEvent) => {
      if (event.ctrlKey && event.altKey && event.key.toLowerCase() === "s") {
        event.preventDefault();
        await openSnip();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [presenceState]);

  useEffect(() => {
    let unlistenQuickMenu: null | (() => void) = null;

    const setup = async () => {
      unlistenQuickMenu = await listen<{ action: string }>(
        "quick-menu-action",
        async (event) => {
          const action = event.payload?.action;
          console.log("[APP] quick-menu-action received:", action);

          try {
            switch (action) {
              case "open-bubble":
                await openBubble();
                break;

              case "capture-clipboard":
                await refreshClipboard();
                break;

              case "snip-screen":
                await openSnip();
                break;

              case "media-play-pause":
                await mediaPlayPause();
                break;

              case "media-prev":
                await mediaPrev();
                break;

              case "media-next":
                await mediaNext();
                break;

              case "volume-down":
                await volumeDown();
                break;

              case "volume-up":
                await volumeUp();
                break;

              case "toggle-mute":
                await toggleMute();
                break;

              case "toggle-pin":
                setPinned((p) => !p);
                markActivity();
                break;

              case "sleep-now":
                if (sleepTimerRef.current) {
                  window.clearTimeout(sleepTimerRef.current);
                }
                if (hideTimerRef.current) {
                  window.clearTimeout(hideTimerRef.current);
                }

                setPresenceState("sleeping");
                setBlobMood("sleepy");
                setHint("Sleeping...");
                break;

              case "close-app":
                await handleClose();
                break;

              case "close-menu":
              default:
                break;
            }
          } catch (error) {
            console.error("quick-menu action failed", error);
          }
        }
      );
    };

    void setup();

    return () => {
      unlistenQuickMenu?.();
    };
  }, []);

  const refreshClipboard = async () => {
    try {
      markActivity();
      const text = await readText();
      if (!text?.trim()) {
        setHint("Clipboard is empty");
        pulseBlob("thinking", 900);
        return;
      }

      const trimmed = text.slice(0, 1500);
      setCopiedText(trimmed);
      setHint("Clipboard updated");
      await showBubbleWindow();
      await sendContextToBubble({
        text: trimmed,
        hint: `Clipboard captured manually. App: ${activeApp}`,
      });
      pulseBlob("happy", 1200);
    } catch {
      setHint("Could not read clipboard");
      pulseBlob("thinking", 900);
    }
  };

  const openBubble = async () => {
    markActivity();
    await showBubbleWindow();
    await sendContextToBubble({
      text: copiedText === "Nothing copied yet" ? "" : copiedText,
      hint: `Bubble opened. App: ${activeApp}`,
    });
    pulseBlob("happy", 1000);
  };

  const handleClose = async () => {
    try {
      const quickMenu = await WebviewWindow.getByLabel("quick-menu");
      if (quickMenu) await quickMenu.close();

      const bubble = await WebviewWindow.getByLabel("bubble");
      if (bubble) await bubble.close();

      const speech = await WebviewWindow.getByLabel("speech");
      if (speech) await speech.close();

      const snipPanel = await WebviewWindow.getByLabel("snip-panel");
      if (snipPanel) await snipPanel.close();

      const snipOverlay = await WebviewWindow.getByLabel("snip-overlay");
      if (snipOverlay) await snipOverlay.close();

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

    if (hideGameState === "seeking") {
      stopHideAndSeek();
      setHideGameState("found");
      setPresenceState("entering");
      setHint("You found me. You win!");
      pulseBlob("happy", 1600, "idle");

      setFoundBubbleText("You found me!");
      if (foundBubbleTimerRef.current) {
        window.clearTimeout(foundBubbleTimerRef.current);
      }
      foundBubbleTimerRef.current = window.setTimeout(() => {
        setFoundBubbleText("");
      }, 2200);

      void restoreMainWindowSizeAtCurrentSpot().catch(console.error);

      window.setTimeout(() => {
        setPresenceState("visible");
        setHideGameState("idle");
      }, 520);

      return;
    }

    setDragging(true);

    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error(error);
    } finally {
      setDragging(false);
    }
  };

  const handleAvatarContextMenu = async (
    event: React.MouseEvent<HTMLCanvasElement>
  ) => {
    event.preventDefault();
    event.stopPropagation();
    markActivity();

    try {
      const win = getCurrentWindow();
      const winPos = await win.outerPosition();

      const screenX = winPos.x + event.clientX;
      const screenY = winPos.y + event.clientY;

      await closeMenu();
      await showQuickMenuWindow(screenX, screenY);

      window.setTimeout(async () => {
        try {
          await emitTo("quick-menu", "quick-menu-data", {
            hint,
            activeApp,
            pinned,
          });
        } catch (error) {
          console.error("quick-menu-data emit failed", error);
        }
      }, 90);
    } catch (error) {
      console.error("open quick menu failed", error);
    }
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
              ? 0
              : presenceState === "hidden_peek"
              ? peekVisible
                ? 0.22
                : 0.04
              : presenceState === "sleeping"
              ? 0.72
              : 1,

          x:
            presenceState === "hidden"
              ? 0
              : presenceState === "exiting"
              ? 18
              : presenceState === "entering"
              ? [-24, 8, 0]
              : 0,

          y:
            presenceState === "hidden"
              ? 0
              : presenceState === "sleeping"
              ? [0, -2, 0]
              : presenceState === "exiting"
              ? 16
              : presenceState === "entering"
              ? [10, -4, 0]
              : 0,

          scale:
            presenceState === "hidden"
              ? 0.5
              : presenceState === "hidden_peek"
              ? 0.34
              : presenceState === "entering"
              ? [0.75, 1.08, 1]
              : presenceState === "sleeping"
              ? 0.9
              : presenceState === "exiting"
              ? 0.82
              : 1,

          rotate: presenceState === "sleeping" ? -6 : 0,
        }}
        transition={{
          opacity: { duration: 0.35 },
          scale: { duration: 0.4 },
          rotate: { duration: 0.5 },
          x: { duration: 0.4, ease: "easeOut" },
          y:
            presenceState === "sleeping"
              ? {
                  duration: 4.5,
                  repeat: Infinity,
                  ease: "easeInOut",
                }
              : presenceState === "entering"
              ? {
                  duration: 0.45,
                  ease: "easeOut",
                }
              : {
                  duration: 0.3,
                  ease: "easeOut",
                },
        }}
        style={{
          position: "absolute",
          right: 16,
          bottom: 18,
          width: hideGameState === "seeking" ? 82 : 250,
          height: hideGameState === "seeking" ? 82 : 250,
          display: "grid",
          placeItems: "center",
          pointerEvents: presenceState === "hidden" ? "none" : "auto",
        }}
      >
        {hideGameState !== "seeking" && (
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
        )}

        <BlobAvatar
          irisOffset={irisOffset}
          state={blobMood}
          dragging={dragging}
          presenceState={presenceState}
          hideGameState={hideGameState}
          peekVisible={peekVisible}
          foundBubbleText={foundBubbleText}
          onMouseDown={handleAvatarMouseDown}
          onPet={() => pulseBlob("love", 1600, "happy")}
          onContextMenu={handleAvatarContextMenu}
          onActivity={markActivity}
        />
      </motion.div>
    </div>
  );
}
