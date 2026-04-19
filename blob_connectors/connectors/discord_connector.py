"""
blob/connectors/discord.py — Discord connector via discord.py.

Installation:
    pip install discord.py

Required environment variables:
    DISCORD_BOT_TOKEN  — Token from the Discord Developer Portal

Discord app setup:
    1. https://discord.com/developers/applications → "New Application"
    2. Bot → "Add Bot" → copy the token
    3. Enable privileged gateway intents:
       - MESSAGE CONTENT INTENT  ← important, otherwise no message text
       - SERVER MEMBERS INTENT (optional)
    4. OAuth2 → URL Generator:
       Scopes: bot
       Bot Permissions: Send Messages, Read Message History, Use Slash Commands
    5. Open the generated link → invite the bot to the server
"""

from __future__ import annotations

import logging
import os
from typing import Any

import discord
from discord.ext import commands

from blob_connectors.base import Attachment, BlobConnector, Message

logger = logging.getLogger(__name__)


class DiscordConnector(BlobConnector):
    """
    Discord bot.

    Responds to:
    - Direct messages (DMs)
    - @mentions in servers
    - Slash command /ask <question> (optional, easily extendable)

    Long responses are split into 2000-character chunks (Discord limit).
    """

    MAX_MSG_LEN = 2000

    def __init__(
        self,
        token: str | None = None,
        command_prefix: str = "!",
    ):
        super().__init__("discord")
        self._token = token or os.environ["DISCORD_BOT_TOKEN"]

        # Configure intents (MESSAGE CONTENT is required)
        intents = discord.Intents.default()
        intents.message_content = True
        intents.dm_messages = True

        self._client = commands.Bot(
            command_prefix=command_prefix,
            intents=intents,
            help_command=None,
        )
        self._setup_events()

    # ------------------------------------------------------------------
    # BlobConnector interface
    # ------------------------------------------------------------------

    async def receive_message(self, raw: discord.Message) -> Message | None:
        """discord.Message → normalized Message object."""
        # Ignore the bot's own messages
        if raw.author == self._client.user or raw.author.bot:
            return None

        is_dm = isinstance(raw.channel, discord.DMChannel)
        is_mention = self._client.user in raw.mentions

        # In servers, only react to mentions
        if not is_dm and not is_mention:
            return None

        # Remove @mention from text
        text = raw.content
        if self._client.user:
            text = text.replace(f"<@{self._client.user.id}>", "")
            text = text.replace(f"<@!{self._client.user.id}>", "")
        text = text.strip()

        # Collect attachments
        attachments = [
            Attachment(
                filename=a.filename,
                content_type=a.content_type or "application/octet-stream",
                url=a.url,
            )
            for a in raw.attachments
        ]

        if not text and not attachments:
            return None

        # Session: DM = per user, server = per channel (or thread)
        if is_dm:
            session_id = f"discord_dm_{raw.author.id}"
        elif isinstance(raw.channel, discord.Thread):
            session_id = f"discord_thread_{raw.channel.id}"
        else:
            session_id = f"discord_channel_{raw.channel.id}"

        return Message(
            session_id=session_id,
            user_id=str(raw.author.id),
            username=raw.author.display_name,
            text=text,
            channel="discord",
            message_id=str(raw.id),
            attachments=attachments,
            raw=raw,
        )

    async def send_response(self, original: Message, response: str) -> None:
        """Post a response to the original message."""
        raw: discord.Message = original.raw

        # "typing…" indicator
        async with raw.channel.typing():
            chunks = self._split(response)
            # First chunk as reply, rest as follow-up
            first = True
            for chunk in chunks:
                if first:
                    await raw.reply(chunk, mention_author=False)
                    first = False
                else:
                    await raw.channel.send(chunk)

    async def start(self) -> None:
        logger.info("[discord] Connecting…")
        self._running = True
        # run() is blocking — call it as a task
        await self._client.start(self._token)

    async def stop(self) -> None:
        logger.info("[discord] Disconnecting…")
        await self._client.close()
        self._running = False

    # ------------------------------------------------------------------
    # Internal
    # ------------------------------------------------------------------

    def _setup_events(self) -> None:
        """Register event handlers on the discord.py client."""

        @self._client.event
        async def on_ready():
            logger.info("[discord] Logged in as %s (ID: %s)", self._client.user, self._client.user.id)
            logger.info("[discord] Intents: %s", self._client.intents)

        @self._client.event
        async def on_message(message: discord.Message):
            logger.info("[discord] Message received from %s: %s", message.author, message.content)
            await self._dispatch(message)
            await self._client.process_commands(message)

        # Optional slash command /ask
        @self._client.command(name="ask")
        async def ask_command(ctx: commands.Context, *, question: str):
            """Ask the Blob a question."""
            # Create a fake message so receive_message works
            ctx.message.content = question
            await self._dispatch(ctx.message)

    @staticmethod
    def _split(text: str, limit: int = MAX_MSG_LEN) -> list[str]:
        """Split text into chunks of ≤ limit if possible at line breaks."""
        if len(text) <= limit:
            return [text]
        chunks, buf = [], []
        for line in text.splitlines(keepends=True):
            if sum(len(l) for l in buf) + len(line) > limit:
                chunks.append("".join(buf))
                buf = []
            buf.append(line)
        if buf:
            chunks.append("".join(buf))
        return chunks