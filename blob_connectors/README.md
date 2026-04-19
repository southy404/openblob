# Blob Connectors

Channel integrations for your AI core (Blob). Telegram, Slack, Discord, and Email — all through a unified interface.

## Structure

```
blob_connectors/
├── base.py                    ← Message-Modell + BlobConnector-Interface
├── run.py                     ← Starter (all connectors in parallel)
├── requirements.txt
└── connectors/
    ├── telegram.py            ← python-telegram-bot, Long-Polling
    ├── slack.py               ← slack-bolt, Socket Mode
    ├── discord_connector.py   ← discord.py
    └── email.py               ← IMAP-Polling + SMTP
```

## Quickstart

```bash
pip install -r requirements.txt

# Set tokens (or place them in a .env file)
export TELEGRAM_BOT_TOKEN=...
export SLACK_BOT_TOKEN=xoxb-...
export SLACK_APP_TOKEN=xapp-...
export DISCORD_BOT_TOKEN=...
export EMAIL_ADDRESS=bot@example.com
export EMAIL_PASSWORD=...

python run.py
```

Only connectors with configured tokens will be started — so you can begin with just one.

## Plug in your own handler

Replace in `run.py` the `ai_handler`-function:

```python
async def ai_handler(message: Message) -> str:
    # message.text      — Message text
    # message.channel   — "telegram" | "slack" | "discord" | "email"
    # message.session_id — Conversation ID (for memory/context)
    # message.username  — Sender display name

    response = await my_blob.process(
        text=message.text,
        session=message.session_id,
    )
    return response
```

## Build your own connector

```python
from blob_connectors.base import BlobConnector, Message

class MyConnector(BlobConnector):
    def __init__(self):
        super().__init__("myplatform")

    async def receive_message(self, raw) -> Message | None:
        # Normalize raw into Message
        # Return None = ignore

    async def send_response(self, original: Message, response: str) -> None:
        # Send the response

    async def start(self) -> None:
        # Start polling/webhook

    async def stop(self) -> None:
        # Shut down cleanly
```

## Setup-Guides

### Telegram

1. [@BotFather](https://t.me/BotFather) → `/newbot` → opy the token
2. `TELEGRAM_BOT_TOKEN=...` set

### Slack

1. [api.slack.com/apps](https://api.slack.com/apps) → New App → From scratch
2. Enable Socket Mode → generate App-Level Token (xapp-…)
3. Event Subscriptions → `message.im`, `message.channels`, `app_mention`
4. OAuth Scopes: `chat:write`, `channels:history`, `im:history`
5. Install the app → copy the Bot Token (xoxb-…)

### Discord

1. [discord.com/developers](https://discord.com/developers/applications) → New Application → Bot
2. **Message Content Intent** enable (Privileged Gateway Intents)
3. OAuth2 → Bot → Permissions: Send Messages, Read Message History
4. Copy the token → invite the bot to your server

### Email (Gmail)

1. Google account → enable 2FA
2. [Generate App-Passwort](https://myaccount.google.com/apppasswords)
3. Gmail Settings → Forwarding/POP/IMAP → enable IMAP
4. Set `EMAIL_ADDRESS` + `EMAIL_PASSWORD` (app password!)
