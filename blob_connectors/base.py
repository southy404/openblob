"""
blob/base.py — Shared base for all Blob connectors.

Each connector implements this class. The AI core only sees
Message objects and calls send_response() — regardless of the channel.
"""

from __future__ import annotations

import asyncio
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable, Awaitable

logger = logging.getLogger(__name__)


@dataclass
class Attachment:
    """File attachment, image, or any other binary content."""
    filename: str
    content_type: str          # e.g. "image/png", "application/pdf"
    data: bytes | None = None  # Inline data (small)
    url: str | None = None     # Or download link


@dataclass
class Message:
    """
    Normalized message — channel-independent.

    The core works ONLY with this object and never needs to know
    whether the message came via Telegram, Slack, Email, or Discord.
    """
    # Required fields
    session_id: str            # Unique conversation ID (per user/chat)
    user_id: str               # Sender ID in the respective system
    text: str                  # Message text (already sanitized)
    channel: str               # "telegram" | "slack" | "discord" | "email"

    # Optional metadata
    message_id: str = ""
    username: str = ""         # Display name, if available
    timestamp: datetime = field(default_factory=datetime.utcnow)
    attachments: list[Attachment] = field(default_factory=list)
    raw: Any = None            # Original payload for debugging / channel-specific features

    def __repr__(self) -> str:
        return f"<Message channel={self.channel} user={self.user_id} len={len(self.text)}>"


# Type alias for the handler registered by the core
MessageHandler = Callable[[Message], Awaitable[str]]


class BlobConnector(ABC):
    """
    Abstract base class for all Blob connectors.

    Subclasses must implement start(), stop(), and send_response().
    receive_message() is called internally and normalizes the
    raw channel payload into a Message object.
    """

    def __init__(self, name: str):
        self.name = name
        self._handler: MessageHandler | None = None
        self._running = False

    def register_handler(self, handler: MessageHandler) -> None:
        """The core registers its message handler here."""
        self._handler = handler

    async def _dispatch(self, raw: Any) -> None:
        """
        Internal dispatcher: normalize → call handler → send response.
        Error handling is centralized here, not in the subclasses.
        """
        if not self._handler:
            logger.warning("[%s] No handler registered, ignoring message.", self.name)
            return
        try:
            msg = await self.receive_message(raw)
            if msg is None:
                return  # Connector decided to ignore this message
            logger.info("[%s] ← %r", self.name, msg)
            response = await self._handler(msg)
            logger.info("[%s] → session=%s len=%d", self.name, msg.session_id, len(response))
            await self.send_response(msg, response)
        except Exception:
            logger.exception("[%s] Error while processing a message.", self.name)

    @abstractmethod
    async def receive_message(self, raw: Any) -> Message | None:
        """
        Convert a raw channel payload into a normalized Message object.
        Returns None if the message should be ignored
        (e.g. bot messages, edits, reactions).
        """

    @abstractmethod
    async def send_response(self, original: Message, response: str) -> None:
        """Send a response back to the sender of the original message."""

    @abstractmethod
    async def start(self) -> None:
        """Start the connector (polling, webhook server, IMAP loop, ...)."""

    @abstractmethod
    async def stop(self) -> None:
        """Shut down the connector cleanly."""

    async def __aenter__(self):
        await self.start()
        return self

    async def __aexit__(self, *_):
        await self.stop()