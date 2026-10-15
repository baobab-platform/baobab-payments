#!/usr/bin/env python3
"""Baobab Gate D1 deterministic test processor. CI only, never deployed.

HyperSwitch's own, unmodified Stripe connector is pointed at this service
(stripe.base_url), so the path under test stays

    Baobab Payments -> real HyperSwitch -> Stripe connector -> this processor.

It implements only the Stripe wire subset those connector flows call:
customers, payment intents (manual and automatic capture), capture
(full or one partial, remainder released), cancel, refunds and reads. It keeps
explicit authorised / captured / voided / refunded state, refuses what a card
processor refuses (overcapture, double capture, capture after void, void after
capture, over-refund), and records every call in an append-only log served at
GET /_evidence.

It never contacts any payment rail and never stores or logs a card number:
only the last four digits survive the request parser. Faults are injected only
through POST /_fault, which the harness drives.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qsl, urlsplit

# A test card that the issuer declines (Stripe's documented test number).
DECLINE_CARD_SUFFIX = "0002"
REFUSE_NON_INTEGRATION = "BAOBAB_ENVIRONMENT"

LOCK = threading.Lock()
STATE: dict = {"intents": {}, "refunds": {}, "customers": {}, "seq": 0}
LOG: list[dict] = []
FAULTS: list[dict] = []
# hold_response: the effect is applied, then the response waits until the
# harness releases it, so a client can be killed inside that window.
HELD = threading.Event()
RELEASE = threading.Event()


# Transaction ids are unique per process start, as a processor's are: a
# restarted processor never reissues an id HyperSwitch already holds.
RUN_NONCE = os.urandom(4).hex()


def next_id(prefix: str) -> str:
    STATE["seq"] += 1
    return f"{prefix}_d1{RUN_NONCE}{STATE['seq']:06d}"


def form(body: str) -> dict:
    """Flatten the form body, keeping only the last four card digits."""
    out = {}
    for key, value in parse_qsl(body, keep_blank_values=True):
        if key.endswith("[card][number]"):
            out["card_last4"] = value[-4:]
            continue
        if key.endswith("[card][cvc]"):
            continue
        out[key] = value
    return out


def intent_view(intent: dict) -> dict:
    return {
        "id": intent["id"], "object": "payment_intent", "amount": intent["amount"],
        "amount_received": intent["amount_received"], "amount_capturable": intent["amount_capturable"],
        "currency": intent["currency"], "status": intent["status"], "created": intent["created"],
        "customer": intent.get("customer"), "metadata": intent["metadata"],
        "capture_method": intent["capture_method"],
        "last_payment_error": intent.get("last_payment_error"),
    }


def refund_view(refund: dict) -> dict:
    return {"id": refund["id"], "object": "refund", "amount": refund["amount"], "currency": refund["currency"],
            "payment_intent": refund["payment_intent"], "status": refund["status"], "metadata": refund["metadata"],
            "failure_reason": None}


def error(status: int, code: str, message: str) -> tuple[int, dict]:
    return status, {"error": {"type": "invalid_request_error", "code": code, "message": message}}


def handle(method: str, path: str, data: dict) -> tuple[int, dict]:
    parts = [p for p in path.split("/") if p]
    intents, refunds = STATE["intents"], STATE["refunds"]
    if method == "POST" and parts == ["v1", "customers"]:
        cid = next_id("cus")
        STATE["customers"][cid] = {"id": cid}
        return 200, {"id": cid, "object": "customer"}
    if method == "POST" and parts == ["v1", "payment_intents"]:
        amount = int(data.get("amount", "0"))
        if amount <= 0:
            return error(400, "parameter_invalid_integer", "amount must be positive")
        manual = data.get("capture_method") == "manual"
        intent = {
            "id": next_id("pi"), "amount": amount, "currency": data.get("currency", "usd").lower(),
            "created": int(time.time()), "customer": data.get("customer"), "capture_method": data.get("capture_method", "automatic"),
            "metadata": {k[len("metadata["):-1]: v for k, v in data.items() if k.startswith("metadata[")},
            "amount_received": 0, "amount_capturable": 0, "status": "requires_payment_method",
        }
        if data.get("confirm") == "true":
            if data.get("card_last4") == DECLINE_CARD_SUFFIX:
                intent["last_payment_error"] = {"code": "card_declined", "decline_code": "generic_decline",
                                                "message": "Your card was declined.", "type": "card_error"}
                intents[intent["id"]] = intent
                return 402, {"error": {**intent["last_payment_error"], "payment_intent": intent_view(intent)}}
            if manual:
                intent.update(status="requires_capture", amount_capturable=amount)
            else:
                intent.update(status="succeeded", amount_received=amount)
        intents[intent["id"]] = intent
        return 200, intent_view(intent)
    if len(parts) >= 3 and parts[:2] == ["v1", "payment_intents"]:
        intent = intents.get(parts[2])
        if intent is None:
            return error(404, "resource_missing", "No such payment_intent")
        if method == "GET" and len(parts) == 3:
            return 200, intent_view(intent)
        if method == "POST" and parts[3:] == ["capture"]:
            if intent["status"] != "requires_capture":
                return error(400, "payment_intent_unexpected_state",
                             f"This PaymentIntent could not be captured because it has a status of {intent['status']}.")
            amount = int(data.get("amount_to_capture", intent["amount_capturable"]))
            if amount <= 0 or amount > intent["amount_capturable"]:
                return error(400, "amount_too_large", "amount_to_capture exceeds the capturable amount")
            # One capture per authorisation; the remainder is released.
            intent.update(status="succeeded", amount_received=amount, amount_capturable=0)
            return 200, intent_view(intent)
        if method == "POST" and parts[3:] == ["cancel"]:
            if intent["status"] != "requires_capture":
                return error(400, "payment_intent_unexpected_state",
                             f"You cannot cancel this PaymentIntent because it has a status of {intent['status']}.")
            intent.update(status="canceled", amount_capturable=0)
            return 200, intent_view(intent)
    if method == "POST" and parts == ["v1", "refunds"]:
        intent = intents.get(data.get("payment_intent", ""))
        if intent is None:
            return error(404, "resource_missing", "No such payment_intent")
        if intent["status"] != "succeeded":
            return error(400, "charge_not_captured", "This charge has not been captured.")
        amount = int(data.get("amount", intent["amount_received"]))
        already = sum(r["amount"] for r in refunds.values()
                      if r["payment_intent"] == intent["id"] and r["status"] == "succeeded")
        if amount <= 0 or already + amount > intent["amount_received"]:
            return error(400, "charge_already_refunded", "Refund exceeds the captured amount")
        refund = {"id": next_id("re"), "amount": amount, "currency": intent["currency"], "payment_intent": intent["id"],
                  "status": "succeeded",
                  "metadata": {k[len("metadata["):-1]: v for k, v in data.items() if k.startswith("metadata[")}}
        refunds[refund["id"]] = refund
        return 200, refund_view(refund)
    if method == "GET" and len(parts) == 3 and parts[:2] == ["v1", "refunds"]:
        refund = refunds.get(parts[2])
        return (200, refund_view(refund)) if refund else error(404, "resource_missing", "No such refund")
    return error(404, "resource_missing", f"Unsupported test-processor route {method} {path}")


def take_fault(method: str, path: str) -> str | None:
    for fault in FAULTS:
        if fault["remaining"] > 0 and fault["method"] == method and fault["path_contains"] in path:
            fault["remaining"] -= 1
            return fault["mode"]
    return None


def summary() -> dict:
    return {
        "intents": [intent_view(i) for i in STATE["intents"].values()],
        "refunds": [refund_view(r) for r in STATE["refunds"].values()],
        "calls": LOG,
    }


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def send_json(self, status: int, body: dict) -> None:
        raw = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def dispatch(self) -> None:
        length = int(self.headers.get("Content-Length") or 0)
        raw = self.rfile.read(length).decode() if length else ""
        path = urlsplit(self.path).path
        if path == "/_health":
            return self.send_json(200, {"status": "UP"})
        if path == "/_evidence" and self.command == "GET":
            with LOCK:
                return self.send_json(200, summary())
        if path == "/_fault" and self.command == "POST":
            fault = json.loads(raw or "{}")
            assert fault["mode"] in ("lose_response", "drop_request", "hold_response")
            with LOCK:
                FAULTS.append({"mode": fault["mode"], "method": fault.get("method", "POST"),
                               "path_contains": fault["path_contains"], "remaining": int(fault.get("count", 1))})
            return self.send_json(200, {"armed": fault})
        if path == "/_held" and self.command == "GET":
            return self.send_json(200, {"held": HELD.is_set()})
        if path == "/_release" and self.command == "POST":
            RELEASE.set()
            return self.send_json(200, {"released": True})
        data = form(raw)
        with LOCK:
            fault = take_fault(self.command, path)
            entry = {"seq": len(LOG) + 1, "at": time.time(), "method": self.command, "path": path,
                     "fields": sorted(data), "fault": fault}
            if fault == "drop_request":
                # The request never reached the processor: no effect at all.
                entry["status"] = 503
                LOG.append(entry)
                status, body = 503, {"error": {"type": "api_error", "message": "synthetic outage"}}
            else:
                status, body = handle(self.command, path, data)
                entry["status"] = status
                entry["result_id"] = body.get("id") if isinstance(body, dict) else None
                LOG.append(entry)
        if fault == "hold_response":
            RELEASE.clear()
            HELD.set()
            RELEASE.wait(timeout=120)
            HELD.clear()
        if fault == "lose_response":
            # The effect is applied, then the response is lost on the wire.
            self.close_connection = True
            self.connection.shutdown(2)
            return
        self.send_json(status, body)

    do_GET = dispatch
    do_POST = dispatch

    def log_message(self, fmt: str, *args) -> None:  # never echo request lines
        pass


def main() -> None:
    if os.environ.get(REFUSE_NON_INTEGRATION) != "integration":
        sys.exit("the Gate D1 test processor runs only with BAOBAB_ENVIRONMENT=integration")
    port = int(os.environ.get("PORT", "9000"))
    ThreadingHTTPServer(("0.0.0.0", port), Handler).serve_forever()


if __name__ == "__main__":
    main()
