#!/usr/bin/env python3
"""
SSQL Launch Website & Interactive Playground Server.

Serves `website/` static assets (including the compiled `pkg/ssql_wasm_bg.wasm`
WebAssembly compiler) and provides an offline-capable `/api/execute` and
`/api/reset` in-memory SQLite endpoint pre-seeded with the University Bookstore
dataset.

Usage:
    python3 website/server.py [--port 8080]
"""

import argparse
import json
import mimetypes
import os
import sqlite3
import sys
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

WEBSITE_DIR = Path(__file__).resolve().parent
if str(WEBSITE_DIR) not in sys.path:
    sys.path.insert(0, str(WEBSITE_DIR))

import build_site  # noqa: E402

import math
import re


def _register_sqlite_udfs(conn: sqlite3.Connection) -> None:
    conn.create_function(
        "POWER",
        2,
        lambda a, b: float(a) ** float(b) if a is not None and b is not None else None,
    )
    conn.create_function(
        "SQRT",
        1,
        lambda a: math.sqrt(float(a)) if a is not None and float(a) >= 0 else None,
    )
    conn.create_function(
        "MOD",
        2,
        lambda a, b: (float(a) % float(b)) if a is not None and b not in (None, 0) else None,
    )
    conn.create_function(
        "REGEXP",
        2,
        lambda pat, val: bool(re.search(str(pat), str(val)))
        if pat is not None and val is not None
        else False,
    )


# The default database is the University Bookstore, set up from the SQL that
# build_site.py compiled from its schema.सं and seed.सं (data/corpus.json).
BOOKSTORE_PROJECT_ID = "university_bookstore/SAMSQL"


def _lookup_project_setup_sql(project_id: str) -> str | None:
    corpus_file = WEBSITE_DIR / "data" / "corpus.json"
    if not corpus_file.exists():
        return None
    try:
        corpus = json.loads(corpus_file.read_text(encoding="utf-8"))
        for proj in corpus.get("projects", []):
            if proj.get("id") == project_id:
                return proj.get("setup_sql")
    except Exception:
        return None
    return None


def reset_db(project_id: str | None = None, setup_sql: str | None = None):
    global DB_CONN
    try:
        DB_CONN.close()
    except Exception:
        pass
    DB_CONN = sqlite3.connect(":memory:", check_same_thread=False)
    _register_sqlite_udfs(DB_CONN)
    target_sql = setup_sql
    if not target_sql and project_id and project_id != "__featured__":
        target_sql = _lookup_project_setup_sql(project_id)
    if not target_sql:
        target_sql = _lookup_project_setup_sql(BOOKSTORE_PROJECT_ID) or ""
    DB_CONN.executescript(target_sql)
    DB_CONN.commit()


DB_CONN = sqlite3.connect(":memory:", check_same_thread=False)
reset_db()


def _split_sql_statements(sql: str) -> list[str]:
    stmts: list[str] = []
    buf: list[str] = []
    for line in sql.splitlines():
        stripped = line.strip()
        if stripped.startswith("--"):
            continue
        buf.append(line)
        candidate = "\n".join(buf).strip()
        if candidate and sqlite3.complete_statement(candidate):
            stmts.append(candidate)
            buf = []
    leftover = "\n".join(buf).strip()
    if leftover:
        stmts.append(leftover)
    return stmts


mimetypes.add_type("application/wasm", ".wasm")
mimetypes.add_type("text/javascript", ".js")
mimetypes.add_type("application/octet-stream", ".vsix")
mimetypes.add_type("application/octet-stream", ".whl")


class PlaygroundHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(WEBSITE_DIR), **kwargs)

    def end_headers(self):
        if self.path.startswith("/downloads/"):
            fname = self.path.split("/")[-1].split("?")[0]
            if fname:
                self.send_header("Content-Disposition", f'attachment; filename="{fname}"')
        super().end_headers()

    def do_POST(self):
        if self.path == "/api/reset":
            length = int(self.headers.get("Content-Length", "0"))
            raw_body = self.rfile.read(length).decode("utf-8") if length > 0 else "{}"
            try:
                payload = json.loads(raw_body) if raw_body.strip() else {}
            except Exception:
                payload = {}
            reset_db(
                project_id=payload.get("project_id"),
                setup_sql=payload.get("setup_sql"),
            )
            self._send_json(200, {"ok": True})
            return

        if self.path == "/api/execute":
            length = int(self.headers.get("Content-Length", "0"))
            raw_body = self.rfile.read(length).decode("utf-8") if length > 0 else "{}"
            try:
                payload = json.loads(raw_body)
                sql = payload.get("sql", "").strip()
                cur = DB_CONN.cursor()
                stmts = _split_sql_statements(sql)
                columns: list[str] = []
                rows: list[list] = []
                for stmt in stmts:
                    q_count = stmt.count("?")
                    if q_count > 0:
                        default_val = "978-1234567890" if "isbn" in stmt.lower() else 20
                        cur.execute(stmt, [default_val] * q_count)
                    else:
                        cur.execute(stmt)
                    if cur.description:
                        columns = [col[0] for col in cur.description]
                        rows = [list(r) for r in cur.fetchall()]
                    else:
                        DB_CONN.commit()
                self._send_json(200, {"ok": True, "columns": columns, "rows": rows})
            except Exception as exc:
                self._send_json(200, {"ok": False, "error": str(exc)})
            return

        self._send_json(404, {"ok": False, "error": "Unknown endpoint"})

    def _send_json(self, status_code: int, data: dict):
        encoded = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)


def main():
    parser = argparse.ArgumentParser(description="Serve the SSQL Launch Website & Playground")
    parser.add_argument("--port", type=int, default=8080, help="Port to listen on (default: 8080)")
    args = parser.parse_args()

    build_site.ensure_up_to_date(force=False)
    os.chdir(WEBSITE_DIR)
    server = ThreadingHTTPServer(("127.0.0.1", args.port), PlaygroundHandler)
    print(f"🚀 SSQL Launch Website & Playground running at http://127.0.0.1:{args.port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down server.")
        server.server_close()


if __name__ == "__main__":
    main()
