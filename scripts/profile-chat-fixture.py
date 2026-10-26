#!/usr/bin/env python3
"""Create an isolated Prism chat database for offline UI profiling."""

from __future__ import annotations

import argparse
import json
import sqlite3
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SUPPORT = ROOT / "build" / "profile" / "chat-support"


def activity(message_index: int, text: str) -> str:
    text_end = text.index("\n\n```text")
    payload = {
        "agent": "offline-profile-agent",
        "started": 1_700_000_000_000 + message_index,
        "ended": 1_700_000_000_500 + message_index,
        "outcome": "done",
        "blocks": [
            {"kind": "text", "start": 0, "end": text_end},
            {
                "kind": "thought",
                "text": "Inspecting the fixture's markdown, code, and tool result rendering path.",
            },
            {
                "kind": "tool",
                "id": f"offline-tool-{message_index}",
                "title": "Read local fixture",
                "status": "done",
                "started": 1_700_000_000_100 + message_index,
                "ended": 1_700_000_000_300 + message_index,
                "input": '{"path":"fixture.rs"}',
                "output": "fn fixture_entry() -> Result<()> {\n    Ok(())\n}",
            },
        ],
        "limited": False,
        "dropped_blocks": 0,
        "dropped_thoughts": 0,
        "dropped_text_end": 0,
        "progress": {
            "plan": [
                {"content": "Inspect local fixture", "status": "completed"},
                {"content": "Summarize findings", "status": "completed"},
            ],
            "children": [],
            "usage": {"used": 2048, "size": 8192, "cost": None, "currency": ""},
            "goal": None,
            "limited": False,
        },
    }
    return json.dumps(payload, separators=(",", ":"))


def message_text(index: int, role: str) -> str:
    if role == "user":
        return (
            f"## Offline profiling turn {index}\n\n"
            "Review this local fixture and explain the behavior.\n\n"
            "```rust\n"
            "fn decode_record(bytes: &[u8]) -> Result<Record, Error> {\n"
            "    let record = Record::parse(bytes)?;\n"
            "    Ok(record)\n"
            "}\n"
            "```\n"
        )
    return (
        f"### Response {index}\n\n"
        "The offline fixture exercises markdown paragraphs, fenced Rust code, "
        "thought blocks, plans, and a completed tool result.\n\n"
        "```text\n"
        "address    instruction                 comment\n"
        "0x1000     mov rax, [rbx]              load record\n"
        "0x1004     ret                         return\n"
        "```\n"
    )


def create_database(path: Path, message_count: int, *, replace: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and not replace:
        raise FileExistsError(f"database already exists: {path}; pass --replace to overwrite it")
    if replace and path.exists():
        path.unlink()
        for suffix in ("-wal", "-shm"):
            sidecar = path.with_name(path.name + suffix)
            if sidecar.exists():
                sidecar.unlink()

    connection = sqlite3.connect(path)
    try:
        connection.executescript(
            """
            PRAGMA journal_mode=WAL;
            CREATE TABLE projects(
                id TEXT PRIMARY KEY, title TEXT, path TEXT, state TEXT, opened INTEGER
            );
            CREATE TABLE chats(
                id INTEGER PRIMARY KEY, project TEXT, title TEXT, updated INTEGER
            );
            CREATE TABLE messages(
                id INTEGER PRIMARY KEY, chat INTEGER, role TEXT, text TEXT
            );
            CREATE INDEX message_chat ON messages(chat,id);
            CREATE TABLE chat_preferences(
                chat INTEGER PRIMARY KEY, pinned INTEGER NOT NULL DEFAULT 0,
                archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE chat_selection(project TEXT PRIMARY KEY, chat INTEGER);
            CREATE TABLE chat_drafts(
                chat INTEGER PRIMARY KEY, draft TEXT NOT NULL,
                attachments TEXT NOT NULL DEFAULT '[]'
            );
            CREATE TABLE chat_queue(chat INTEGER PRIMARY KEY, data TEXT NOT NULL);
            CREATE TABLE message_activity(message INTEGER PRIMARY KEY, data TEXT NOT NULL);
            """
        )
        connection.execute(
            "INSERT INTO chats(id,project,title,updated) VALUES(1,NULL,?,?)",
            (f"Offline profile ({message_count} messages)", 1_700_000_000),
        )
        connection.execute(
            "INSERT INTO chat_selection(project,chat) VALUES('',1)"
        )

        for index in range(message_count):
            role = "user" if index % 2 == 0 else "assistant"
            text = message_text(index // 2 + 1, role)
            cursor = connection.execute(
                "INSERT INTO messages(chat,role,text) VALUES(1,?,?)",
                (role, text),
            )
            if role == "assistant":
                connection.execute(
                    "INSERT INTO message_activity(message,data) VALUES(?,?)",
                    (cursor.lastrowid, activity(index, text)),
                )

        connection.commit()
    finally:
        connection.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--support", type=Path, default=DEFAULT_SUPPORT)
    parser.add_argument("--messages", type=int, default=500)
    parser.add_argument(
        "--replace",
        action="store_true",
        help="replace the fixture database at the validated support path",
    )
    args = parser.parse_args()
    if args.messages < 2:
        parser.error("--messages must be at least 2")

    support = args.support.resolve()
    profile_root = (ROOT / "build" / "profile").resolve()
    try:
        support.relative_to(profile_root)
    except ValueError:
        parser.error("refusing to write outside build/profile")
    try:
        create_database(support / "prism.db", args.messages, replace=args.replace)
    except OSError as error:
        parser.error(str(error))
    print(f"Created {args.messages} messages at {support / 'prism.db'}")
    print(f"Use PRISM_SUPPORT={support} when launching Prism.")


if __name__ == "__main__":
    main()
