"""
blob/connectors/slack.py — Slack connector via Bolt for Python.

Installation:
    pip install slack-bolt

Required environment variables:
    SLACK_BOT_TOKEN    — xoxb-… token (OAuth & Permissions → Bot Token)
    SLACK_APP_TOKEN    — xapp-… token (for Socket Mode, no public server required)

Slack app setup:
    1. https://api.slack.com/apps → "Create New App" → "From scratch"
    2. Enable Socket Mode (Settings → Socket Mode)
    3. Event Subscriptions → Subscribe to bot events:
       - message.channels, message.groups, message.im, message.mpim
       - app_mention
    4. OAuth Scopes (Bot Token):
       - chat:write, channels:history, groups:history, im:history, users:read
    5. Install the app → copy the tokens
"""

from __future__ import annotations

import logging
import os
from typing import Any

from slack_bolt.async_app import AsyncApp
from slack_bolt.adapter.socket_mode.async_handler import AsyncSocketModeHandler

from blob_connectors.base import BlobConnector, Message

logger = logging.getLogger(__name__)


class SlackConnector(BlobConnector):
    """
    Slack bot via Socket Mode (no publicly reachable server required).

    Responds to:
    - Direct messages (DMs)
    - @mentions in channels
    - Optional response to all messages in a channel (configurable)
    """

    def __init__(
        self,
        bot_token: str | None = None,
        app_token: str | None = None,
        respond_to_all: bool = False,  # True = respond to all channel messages
    ):
        super().__init__("slack")
        self._bot_token = bot_token or os.environ["SLACK_BOT_TOKEN"]
        self._app_token = app_token or os.environ["SLACK_APP_TOKEN"]
        self._respond_to_all = respond_to_all
        self._bolt_app = AsyncApp(token=self._bot_token)
        self._socket_handler: AsyncSocketModeHandler | None = None
        self._bot_user_id: str | None = None

    # ------------------------------------------------------------------
    # BlobConnector interface
    # ------------------------------------------------------------------

    async def receive_message(self, raw: dict) -> Message | None:
        """Slack event payload → normalized Message object."""
        event = raw.get("event", raw)  # Direct event or wrapped event

        # Ignore the bot's own messages and subtypes (e.g. message_changed)
        if event.get("bot_id") or event.get("subtype"):
            return None
        if event.get("user") == self._bot_user_id:
            return None

        text: str = event.get("text", "").strip()
        channel_type = event.get("channel_type", "")
        channel_id = event.get("channel", "")
        user_id = event.get("user", "unknown")

        # For channel messages: only react to @mentions (unless respond_to_all)
        is_dm = channel_type == "im"
        is_mention = self._bot_user_id and f"<@{self._bot_user_id}>" in text
        if not is_dm and not is_mention and not self._respond_to_all:
            return None

        # Remove @mention from text
        if self._bot_user_id:
            text = text.replace(f"<@{self._bot_user_id}>", "").strip()

        if not text:
            return None

        # session_id = DM channel or thread, if present
        thread_ts = event.get("thread_ts") or event.get("ts", "")
        session_id = f"slack_{channel_id}_{thread_ts}" if thread_ts else f"slack_{channel_id}"

        return Message(
            session_id=session_id,
            user_id=user_id,
            text=text,
            channel="slack",
            message_id=event.get("ts", ""),
            raw=raw,
        )

    async def send_response(self, original: Message, response: str) -> None:
        """Post the response in the same thread/channel."""
        event = original.raw.get("event", original.raw)
        channel_id = event.get("channel")
        thread_ts = event.get("thread_ts") or event.get("ts")

        # Split Slack message into 3000-character chunks
        chunks = [response[i:i+3000] for i in range(0, len(response), 3000)]
        for chunk in chunks:
            await self._bolt_app.client.chat_postMessage(
                channel=channel_id,
                thread_ts=thread_ts,  # Replies in the thread
                text=chunk,
                mrkdwn=True,
            )

    async def start(self) -> None:
        logger.info("[slack] Starting Socket Mode…")

        # Determine bot user ID (for mention detection)
        auth = await self._bolt_app.client.auth_test()
        self._bot_user_id = auth["user_id"]
        logger.info("[slack] Bot user ID: %s", self._bot_user_id)

        # Register message events
        @self._bolt_app.event("message")
        async def on_message(event, say):
            await self._dispatch({"event": event})

        @self._bolt_app.event("app_mention")
        async def on_mention(event, say):
            await self._dispatch({"event": event})

        self._socket_handler = AsyncSocketModeHandler(self._bolt_app, self._app_token)
        self._running = True
        await self._socket_handler.start_async()

    async def stop(self) -> None:
        if self._socket_handler:
            logger.info("[slack] Stopping…")
            await self._socket_handler.close_async()
        self._running = False