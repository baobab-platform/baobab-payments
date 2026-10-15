"""Gate D1 driver helpers: HTTP, the databases and the evidence record.

Standard library only. Every call is to a harness-published loopback port;
Payments itself reaches CP, IAM and HyperSwitch over TLS through the edge.
"""
from __future__ import annotations

import json
import subprocess
import time
import urllib.error
import urllib.request
import uuid

PAYMENTS = "http://127.0.0.1:18081"
CP = "http://127.0.0.1:18082"
HYPERSWITCH = "http://127.0.0.1:18080"
ISSUER = "http://127.0.0.1:18000"
PROCESSOR = "http://127.0.0.1:19000"


class Response:
    def __init__(self, status: int, body, headers: dict):
        self.status, self.body, self.headers = status, body, headers

    def __repr__(self) -> str:
        return f"<{self.status} {json.dumps(self.body)[:400]}>"


def call(method: str, url: str, body=None, headers: dict | None = None, form: str | None = None,
         timeout: float = 60) -> Response:
    data, hdrs = None, dict(headers or {})
    if form is not None:
        data = form.encode()
        hdrs.setdefault("Content-Type", "application/x-www-form-urlencoded")
    elif body is not None:
        data = json.dumps(body).encode()
        hdrs.setdefault("Content-Type", "application/json")
    request = urllib.request.Request(url, data=data, method=method, headers=hdrs)
    try:
        with urllib.request.urlopen(request, timeout=timeout) as r:
            raw, status, rh = r.read(), r.status, dict(r.headers)
    except urllib.error.HTTPError as e:
        raw, status, rh = e.read(), e.code, dict(e.headers)
    try:
        parsed = json.loads(raw) if raw else None
    except ValueError:
        parsed = raw.decode(errors="replace")
    return Response(status, parsed, {k.lower(): v for k, v in rh.items()})


def wait_for(url: str, seconds: int = 120) -> None:
    deadline = time.time() + seconds
    while time.time() < deadline:
        try:
            if call("GET", url, timeout=5).status < 500:
                return
        except OSError:
            pass
        time.sleep(1)
    raise RuntimeError(f"{url} did not come up")


class Compose:
    def __init__(self, file: str):
        self.base = ["docker", "compose", "-f", file, "-p", "baobab-gate-d1"]

    def run(self, *args: str, env: dict | None = None, check: bool = True, input_: str | None = None) -> str:
        import os
        full_env = dict(os.environ, **(env or {}))
        p = subprocess.run(self.base + list(args), input=input_, capture_output=True, text=True, env=full_env)
        if check and p.returncode != 0:
            raise RuntimeError(f"compose {' '.join(args)} failed: {p.stderr[-2000:]}")
        return p.stdout

    def sql(self, service: str, user: str, db: str, statement: str, check: bool = True) -> list[list[str]]:
        out = self.run("exec", "-T", service, "psql", "-X", "-q", "-At", "-F", "\t", "-v", "ON_ERROR_STOP=1",
                       "-U", user, "-d", db, "-c", statement, check=check)
        return [line.split("\t") for line in out.splitlines() if line]

    def scalar(self, service: str, user: str, db: str, statement: str) -> str:
        rows = self.sql(service, user, db, statement)
        return rows[0][0] if rows and rows[0] else ""


def key(label: str) -> str:
    """A fresh Idempotency-Key (16-128 of [A-Za-z0-9._:-])."""
    return f"d1-{label}-{uuid.uuid4().hex[:20]}"
