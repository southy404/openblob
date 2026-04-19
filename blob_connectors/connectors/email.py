"""
blob/connectors/email.py — Email connector via IMAP (receiving) + SMTP (sending).

Installation:
    pip install aiosmtplib aioimaplib beautifulsoup4

    Only stdlib required if you use the sync variant:
    pip install beautifulsoup4

Required environment variables:
    EMAIL_ADDRESS      — Sender/recipient address
    EMAIL_PASSWORD     — App password (Google: myaccount.google.com/apppasswords)
    IMAP_HOST          — e.g. imap.gmail.com
    SMTP_HOST          — e.g. smtp.gmail.com
    IMAP_PORT          — Default: 993 (SSL)
    SMTP_PORT          — Default: 587 (STARTTLS)

Gmail note:
    - Enable 2-factor authentication
    - Generate an app password (not your regular password)
    - Enable "IMAP" in Gmail settings → Forwarding/POP/IMAP
"""

from __future__ import annotations

import asyncio
import email
import imaplib
import logging
import os
import smtplib
import ssl
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from email.utils import parseaddr, formataddr
from typing import Any

from bs4 import BeautifulSoup

from blob_connectors.base import BlobConnector, Message

logger = logging.getLogger(__name__)

# Quoting prefixes that are ignored when replying
QUOTE_PREFIXES = (">", "On ", "Am ", "Le ", "El ")


class EmailConnector(BlobConnector):
    """
    Email bot via IMAP polling + SMTP sending.

    Polls the mailbox every `poll_interval` seconds for unread emails.
    Replies as a reply (Re: …) with the original thread header.

    Security: Only replies to senders in `allowed_senders` if set.
    """

    def __init__(
        self,
        address: str | None = None,
        password: str | None = None,
        imap_host: str | None = None,
        smtp_host: str | None = None,
        imap_port: int | None = None,
        smtp_port: int | None = None,
        poll_interval: float = 30.0,
        allowed_senders: list[str] | None = None,  # None = all allowed
        mailbox: str = "INBOX",
    ):
        super().__init__("email")
        self._address = address or os.environ["EMAIL_ADDRESS"]
        self._password = password or os.environ["EMAIL_PASSWORD"]
        self._imap_host = imap_host or os.environ.get("IMAP_HOST", "imap.gmail.com")
        self._smtp_host = smtp_host or os.environ.get("SMTP_HOST", "smtp.gmail.com")
        self._imap_port = imap_port or int(os.environ.get("IMAP_PORT", "993"))
        self._smtp_port = smtp_port or int(os.environ.get("SMTP_PORT", "587"))
        self._poll_interval = poll_interval
        self._allowed_senders = {s.lower() for s in (allowed_senders or [])}
        self._mailbox = mailbox
        self._poll_task: asyncio.Task | None = None

    # ------------------------------------------------------------------
    # BlobConnector interface
    # ------------------------------------------------------------------

    async def receive_message(self, raw: dict) -> Message | None:
        """Parsed email → normalized Message object."""
        sender_name, sender_addr = parseaddr(raw["from"])
        sender_addr = sender_addr.lower()

        # Allowlist check
        if self._allowed_senders and sender_addr not in self._allowed_senders:
            logger.debug("[email] Ignoring sender: %s", sender_addr)
            return None
        
        # Ignore no-reply and system emails
        NO_REPLY_PATTERNS = ("no-reply", "noreply", "mailer-daemon", "postmaster", "notifications")
        if any(p in sender_addr for p in NO_REPLY_PATTERNS):
            logger.debug("[email] Ignoring system email from: %s", sender_addr)
            return None

        # Ignore own emails (prevent reply loop)
        if sender_addr == self._address.lower():
            return None

        subject = raw.get("subject", "(no subject)")
        body = raw.get("body", "").strip()

        if not body:
            return None

        # Derive thread ID from Message-ID or subject
        message_id = raw.get("message_id", "")
        in_reply_to = raw.get("in_reply_to", "")
        thread_id = in_reply_to or message_id or subject

        return Message(
            session_id=f"email_{sender_addr}_{self._normalize_thread(thread_id)}",
            user_id=sender_addr,
            username=sender_name or sender_addr,
            text=body,
            channel="email",
            message_id=message_id,
            raw=raw,
        )

    async def send_response(self, original: Message, response: str) -> None:
        """Send the response as a reply."""
        raw = original.raw
        sender_name, sender_addr = parseaddr(raw["from"])
        subject = raw.get("subject", "")
        if not subject.startswith("Re:"):
            subject = f"Re: {subject}"

        msg = MIMEMultipart("alternative")
        msg["From"] = formataddr(("Blob", self._address))
        msg["To"] = formataddr((sender_name, sender_addr))
        msg["Subject"] = subject
        msg["In-Reply-To"] = raw.get("message_id", "")
        msg["References"] = raw.get("message_id", "")

        # Plain text + simple HTML
        msg.attach(MIMEText(response, "plain", "utf-8"))
        html_body = response.replace("\n", "<br>")
        msg.attach(MIMEText(f"<html><body>{html_body}</body></html>", "html", "utf-8"))

        # SMTP in thread pool (blocking I/O)
        await asyncio.get_event_loop().run_in_executor(
            None, self._smtp_send, sender_addr, msg
        )
        logger.info("[email] Response sent to %s (subject: %s)", sender_addr, subject)

    async def start(self) -> None:
        logger.info("[email] Starting IMAP polling (interval: %ss)…", self._poll_interval)
        self._running = True
        self._poll_task = asyncio.create_task(self._poll_loop())

    async def stop(self) -> None:
        logger.info("[email] Stopping…")
        self._running = False
        if self._poll_task:
            self._poll_task.cancel()
            try:
                await self._poll_task
            except asyncio.CancelledError:
                pass

    # ------------------------------------------------------------------
    # IMAP polling
    # ------------------------------------------------------------------

    async def _poll_loop(self) -> None:
        while self._running:
            try:
                mails = await asyncio.get_event_loop().run_in_executor(
                    None, self._fetch_unseen
                )
                for mail_data in mails:
                    await self._dispatch(mail_data)
            except Exception:
                logger.exception("[email] Error during polling.")
            await asyncio.sleep(self._poll_interval)

    def _fetch_unseen(self) -> list[dict]:
        """IMAP: Fetch unread emails and return them as dicts."""
        results = []
        ssl_context = ssl.create_default_context()
        with imaplib.IMAP4_SSL(self._imap_host, self._imap_port, ssl_context=ssl_context) as imap:
            imap.login(self._address, self._password)
            imap.select(self._mailbox)

            _, msg_ids = imap.search(None, "UNSEEN")
            for msg_id in msg_ids[0].split():
                _, msg_data = imap.fetch(msg_id, "(RFC822)")
                raw_email = msg_data[0][1]
                parsed = email.message_from_bytes(raw_email)

                body = self._extract_body(parsed)
                results.append({
                    "from": parsed.get("From", ""),
                    "subject": parsed.get("Subject", ""),
                    "message_id": parsed.get("Message-ID", ""),
                    "in_reply_to": parsed.get("In-Reply-To", ""),
                    "body": body,
                })
                # Mark as read
                imap.store(msg_id, "+FLAGS", "\\Seen")

        return results

    # ------------------------------------------------------------------
    # SMTP sending
    # ------------------------------------------------------------------

    def _smtp_send(self, recipient: str, msg: MIMEMultipart) -> None:
        with smtplib.SMTP(self._smtp_host, self._smtp_port) as smtp:
            smtp.starttls()
            smtp.login(self._address, self._password)
            smtp.sendmail(self._address, [recipient], msg.as_string())

    # ------------------------------------------------------------------
    # Helper methods
    # ------------------------------------------------------------------

    @staticmethod
    def _extract_body(msg: email.message.Message) -> str:
        """Extract plain text from MIME message, remove quoting."""
        body = ""
        if msg.is_multipart():
            for part in msg.walk():
                ctype = part.get_content_type()
                if ctype == "text/plain":
                    body = part.get_payload(decode=True).decode("utf-8", errors="replace")
                    break
                elif ctype == "text/html" and not body:
                    html = part.get_payload(decode=True).decode("utf-8", errors="replace")
                    body = BeautifulSoup(html, "html.parser").get_text(separator="\n")
        else:
            payload = msg.get_payload(decode=True)
            if payload:
                body = payload.decode("utf-8", errors="replace")

        # Remove quoted lines
        lines = [
            line for line in body.splitlines()
            if not any(line.startswith(p) for p in QUOTE_PREFIXES)
        ]
        return "\n".join(lines).strip()

    @staticmethod
    def _normalize_thread(thread_id: str) -> str:
        """Reduce thread ID to safe characters."""
        return "".join(c if c.isalnum() else "_" for c in thread_id)[:64]