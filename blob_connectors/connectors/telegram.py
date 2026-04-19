"""
blob/connectors/telegram.py — Telegram connector via python-telegram-bot.

Installation:
    pip install python-telegram-bot

Required environment variables:
    TELEGRAM_BOT_TOKEN  — Token from BotFather (https://t.me/BotFather)

Quick start:
    connector = TelegramConnector(token=os.environ["TELEGRAM_BOT_TOKEN"])
    connector.register_handler(my_ai_handler)
    await connector.start()   # blocks (long polling)
"""

from __future__ import annotations

import logging
import os
from typing import Any

from telegram import Update
from telegram.constants import ChatAction
from telegram.ext import Application, ContextTypes, MessageHandler, filters

from blob_connectors.base import Attachment, BlobConnector, Message

logger = logging.getLogger(__name__)


class TelegramConnector(BlobConnector):
    """
    Telegram bot via long polling.

    Receives text and photo messages, sends responses as text.
    Long responses are automatically split into 4096-character chunks
    (Telegram limit).
    """

    MAX_MSG_LEN = 4096

    def __init__(self, token: str | None = None):
        super().__init__("telegram")
        self._token = token or os.environ["TELEGRAM_BOT_TOKEN"]
        self._app: Application | None = None

    # ------------------------------------------------------------------
    # BlobConnector interface
    # ------------------------------------------------------------------

    async def receive_message(self, raw: Update) -> Message | None:
        """Telegram update → normalized Message object."""
        if not raw.message or not raw.effective_user:
            return None

        tg_msg = raw.message
        user = raw.effective_user

        # Collect photos as attachments (metadata only, no download here)
        attachments: list[Attachment] = []
        if tg_msg.photo:
            # Use the largest available photo
            photo = tg_msg.photo[-1]
            attachments.append(Attachment(
                filename=f"photo_{photo.file_id}.jpg",
                content_type="image/jpeg",
                url=f"tg://file/{photo.file_id}",  # Placeholder, retrievable via Bot API
            ))

        # Group chats: only react to direct mentions or replies
        chat = tg_msg.chat
        text = tg_msg.text or tg_msg.caption or ""
        if chat.type in ("group", "supergroup"):
            bot_username = self._app.bot.username if self._app else ""
            is_mention = bot_username and f"@{bot_username}" in text
            is_reply_to_bot = (
                tg_msg.reply_to_message
                and tg_msg.reply_to_message.from_user
                and tg_msg.reply_to_message.from_user.is_bot
            )
            if not is_mention and not is_reply_to_bot:
                return None  # Group post not relevant to us
            # Remove mention from the text
            if bot_username:
                text = text.replace(f"@{bot_username}", "").strip()

        if not text and not attachments:
            return None  # Stickers, voice, etc. — ignored for now

        return Message(
            session_id=f"tg_{chat.id}",
            user_id=str(user.id),
            username=user.username or user.first_name or str(user.id),
            text=text,
            channel="telegram",
            message_id=str(tg_msg.message_id),
            attachments=attachments,
            raw=raw,
        )

    async def send_response(self, original: Message, response: str) -> None:
        """Send a response to the Telegram chat (with typing indicator)."""
        update: Update = original.raw
        chat_id = update.effective_chat.id

        # Show "typing…" indicator
        await self._app.bot.send_chat_action(chat_id=chat_id, action=ChatAction.TYPING)

        # Split into chunks if the response is too long
        for chunk in self._split(response):
            await update.effective_chat.send_message(
                text=chunk,
                parse_mode="Markdown",
                reply_to_message_id=update.message.message_id,
            )

    async def start(self) -> None:
        logger.info("[telegram] Starting long polling…")
        self._app = Application.builder().token(self._token).build()

        # Handler for all text and photo messages
        self._app.add_handler(
            MessageHandler(
                filters.TEXT | filters.PHOTO,
                self._on_update,
            )
        )

        self._running = True
        # run_polling() is blocking — wrap it in its own task
        await self._app.initialize()
        await self._app.start()
        await self._app.updater.start_polling(drop_pending_updates=True)
        logger.info("[telegram] Bot is running (@%s)", (await self._app.bot.get_me()).username)

    async def stop(self) -> None:
        if self._app:
            logger.info("[telegram] Stopping…")
            await self._app.updater.stop()
            await self._app.stop()
            await self._app.shutdown()
        self._running = False

    # ------------------------------------------------------------------
    # Internal
    # ------------------------------------------------------------------

    async def _on_update(self, update: Update, _: ContextTypes.DEFAULT_TYPE) -> None:
        """python-telegram-bot handler — delegates to _dispatch()."""
        await self._dispatch(update)

    @staticmethod
    def _split(text: str, limit: int = MAX_MSG_LEN) -> list[str]:
        """Split text into chunks of ≤ limit characters (at line breaks)."""
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