from __future__ import annotations

import asyncio
import json
import logging
import os
import sys
from pathlib import Path

import aiohttp

# Path-Fix
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Optional: load .env
try:
    from dotenv import load_dotenv
    load_dotenv()
except ImportError:
    pass

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("blob.runner")

# ------------------------------------------------------------------
# OpenBlob-Paths
# ------------------------------------------------------------------

def _config_dir() -> Path:
    return Path(os.environ.get("APPDATA", "~")) / "OpenBlob" / "config"

def _memory_dir() -> Path:
    return Path(os.environ.get("APPDATA", "~")) / "OpenBlob" / "memory"

# ------------------------------------------------------------------
# Config + Profile
# ------------------------------------------------------------------

def load_companion_config() -> dict:
    try:
        path = _config_dir() / "companion_config.json"
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        pass
    return {}

def load_user_profile() -> dict:
    try:
        path = _config_dir() / "user_profile.json"
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        pass
    return {}

# ------------------------------------------------------------------
# Memory
# ------------------------------------------------------------------

def load_semantic_memory() -> dict:
    try:
        path = _memory_dir() / "semantic_memory.json"
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        pass
    return {}

def load_episodic_memory(limit: int = 10) -> list[dict]:
    entries = []
    try:
        path = _memory_dir() / "episodic_memory.jsonl"
        if path.exists():
            lines = path.read_text(encoding="utf-8").strip().splitlines()
            for line in lines[-limit:]:
                try:
                    entries.append(json.loads(line))
                except Exception:
                    pass
    except Exception:
        pass
    return entries

def load_bonding_state() -> dict:
    try:
        path = _memory_dir() / "bonding_state.json"
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        pass
    return {}

def load_personality_state() -> dict:
    try:
        path = _memory_dir() / "personality_state.json"
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        pass
    return {}

async def load_rust_memory_context(query: str, channel: str, limit: int = 12) -> str:
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(
                "http://127.0.0.1:7842/memory/context",
                params={
                    "q": query,
                    "app": channel,
                    "domain": "external",
                    "limit": limit,
                },
                timeout=aiohttp.ClientTimeout(total=1.5),
            ) as resp:
                if resp.status != 200:
                    return ""
                data = await resp.json()
                return str(data.get("memory") or "").strip()
    except Exception:
        return ""

async def record_connector_memory(message, reply: str) -> None:
    try:
        async with aiohttp.ClientSession() as session:
            await session.post(
                "http://127.0.0.1:7842/memory/event",
                json={
                    "kind": "connector_message",
                    "channel": message.channel,
                    "app_name": message.channel,
                    "domain": "external",
                    "user_input": message.text,
                    "summary": reply,
                    "outcome": "success",
                    "importance": 0.6,
                },
                timeout=aiohttp.ClientTimeout(total=1.0),
            )
    except Exception:
        pass

# ------------------------------------------------------------------
# System-Prompt Builder
# ------------------------------------------------------------------

def build_system_prompt(channel: str, memory_context: str = "") -> dict:
    config   = load_companion_config()
    profile  = load_user_profile()
    semantic = load_semantic_memory()
    episodes = load_episodic_memory(10)
    bonding  = load_bonding_state()
    persona  = load_personality_state()

    blob_name  = config.get("blob_name", "").strip() or "OpenBlob"
    owner_name = profile.get("display_name", "").strip()
    language   = config.get("preferred_language", "en").strip() or "en"

    # Personality
    mood = "idle"
    energy     = persona.get("energy", 0.72)
    affection  = persona.get("affection", 0.36)
    playfulness = persona.get("playfulness", 0.48)
    if energy < 0.28:
        mood = "sleepy"
    elif affection > 0.78:
        mood = "affectionate"
    elif playfulness > 0.72:
        mood = "playful"

    # Bonding
    rel_level  = bonding.get("relationship_level", 0.12)
    trust      = bonding.get("trust_score", 0.18)
    help_count = bonding.get("successful_help_count", 0)

    parts = [
        f"Du bist {blob_name}, ein lokaler KI-Assistent.",
    ]

    if owner_name:
        parts.append(f"Dein Besitzer heisst {owner_name}.")

    parts.append(f"Bevorzugte Sprache: {language}.")
    parts.append(f"Der User kontaktiert dich gerade via {channel}.")
    parts.append(f"Deine aktuelle Stimmung: {mood}.")

    if rel_level > 0.3:
        parts.append(
            f"Ihr kennt euch schon gut (Beziehungslevel: {rel_level:.0%}, "
            f"Vertrauen: {trust:.0%}, {help_count} erfolgreiche Interaktionen)."
        )

    if memory_context.strip():
        parts.append("Nutze diesen lokalen Langzeitgedaechtnis-Kontext nur, wenn er relevant ist.")
        parts.append(memory_context.strip())
    else:
        if semantic.get("inferred_user_style"):
            parts.append(f"Kommunikationsstil des Users: {semantic['inferred_user_style']}.")

        if semantic.get("favorite_apps"):
            apps = ", ".join(semantic["favorite_apps"][:5])
            parts.append(f"Haeufig genutzte Apps: {apps}.")

        if semantic.get("recurring_topics"):
            topics = ", ".join(semantic["recurring_topics"][:5])
            parts.append(f"Wiederkehrende Themen: {topics}.")

        if semantic.get("notes"):
            notes = " | ".join(semantic["notes"][:3])
            parts.append(f"Notizen ueber den User: {notes}.")

        if episodes:
            ep_text = "; ".join(
                f"{e.get('kind','?')} via {e.get('app_name','?')}: {e.get('summary','?')}"
                for e in episodes[-5:]
            )
            parts.append(f"Letzte Interaktionen: {ep_text}.")

    parts.append("Antworte direkt und hilfreich.")

    return {"role": "system", "content": " ".join(parts)}

# ------------------------------------------------------------------
# Conversation-History (In-Memory)
# ------------------------------------------------------------------

session_histories: dict[str, list] = {}

# ------------------------------------------------------------------
# AI-Handler
# ------------------------------------------------------------------

async def ai_handler(message) -> str:
    # 1. OpenBlob Command-Router 
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                "http://127.0.0.1:7842/command",
                json={
                    "input": message.text,
                    "channel": message.channel,
                },
                timeout=aiohttp.ClientTimeout(total=2),
            ) as resp:
                data = await resp.json()
                if data.get("action_taken"):
                    action = data.get("result", "")
                    return f"Ausgefuehrt: {action}"
    except Exception:
        pass  # No Command -> Ollama

    # 2. Ollama-Fallback with full Memory-Context
    history = session_histories.setdefault(message.session_id, [])
    history.append({"role": "user", "content": message.text})

    memory_context = await load_rust_memory_context(message.text, message.channel)
    system_prompt = build_system_prompt(message.channel, memory_context)

    async with aiohttp.ClientSession() as session:
        async with session.post(
            "http://localhost:11434/api/chat",
            json={
                "model": "llama3.1:8b",
                "messages": [system_prompt] + history[-20:],
                "stream": False,
            },
        ) as resp:
            data = await resp.json()
            reply = data["message"]["content"]

    history.append({"role": "assistant", "content": reply})
    await record_connector_memory(message, reply)
    return reply

# ------------------------------------------------------------------
# Connector-Setup
# ------------------------------------------------------------------

def build_connectors() -> list:
    from blob_connectors.base import BlobConnector
    connectors: list[BlobConnector] = []

    if os.getenv("TELEGRAM_BOT_TOKEN"):
        from blob_connectors.connectors.telegram import TelegramConnector
        connectors.append(TelegramConnector())
        logger.info("✓ Telegram-Connector aktiviert")
    else:
        logger.info("- Telegram: TELEGRAM_BOT_TOKEN nicht gesetzt, uebersprungen")

    if os.getenv("SLACK_BOT_TOKEN") and os.getenv("SLACK_APP_TOKEN"):
        from blob_connectors.connectors.slack import SlackConnector
        connectors.append(SlackConnector())
        logger.info("✓ Slack-Connector aktiviert")
    else:
        logger.info("- Slack: SLACK_BOT_TOKEN / SLACK_APP_TOKEN nicht gesetzt, uebersprungen")

    if os.getenv("DISCORD_BOT_TOKEN"):
        from blob_connectors.connectors.discord_connector import DiscordConnector
        connectors.append(DiscordConnector())
        logger.info("✓ Discord-Connector aktiviert")
    else:
        logger.info("- Discord: DISCORD_BOT_TOKEN nicht gesetzt, uebersprungen")

    if os.getenv("EMAIL_ADDRESS") and os.getenv("EMAIL_PASSWORD"):
        from blob_connectors.connectors.email import EmailConnector
        connectors.append(EmailConnector(poll_interval=60.0))
        logger.info("✓ Email-Connector aktiviert (Polling alle 60s)")
    else:
        logger.info("- Email: EMAIL_ADDRESS / EMAIL_PASSWORD nicht gesetzt, uebersprungen")

    return connectors

# ------------------------------------------------------------------
# Main
# ------------------------------------------------------------------

async def main():
    connectors = build_connectors()

    if not connectors:
        logger.error("Keine Connectoren konfiguriert. Bitte Umgebungsvariablen setzen.")
        return

    for connector in connectors:
        connector.register_handler(ai_handler)

    logger.info("Starte %d Connector(en)...", len(connectors))

    tasks = [asyncio.create_task(connector.start()) for connector in connectors]

    try:
        await asyncio.Event().wait()
    except (KeyboardInterrupt, asyncio.CancelledError):
        logger.info("Unterbrochen, fahre herunter...")
    finally:
        for connector in connectors:
            await connector.stop()
        logger.info("All Connectors gestoppt.")


if __name__ == "__main__":
    asyncio.run(main())
