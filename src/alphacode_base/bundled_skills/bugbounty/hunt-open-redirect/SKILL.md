---
name: hunt-open-redirect
description: Open redirect — parameter manipulation, path-based, host header, JavaScript location. Chains to OAuth abuse, phishing, ATO.
---

# OPEN REDIRECT HUNTING — 3 BULLETS MAX

**Core:** Redirect to attacker-controlled domain. Chains to OAuth abuse → ATO.

## DETECTION
```bash
# Parameter-based
curl -s -I "https://target.com/redirect?url=https://evil.com"
curl -s -I "https://target.com/redirect?next=https://evil.com"
curl -s -I "https://target.com/redirect?to=https://evil.com"
curl -s -I "https://target.com/redirect?dest=https://evil.com"
curl -s -I "https://target.com/redirect?return=https://evil.com"
# Path-based
curl -s -I "https://target.com/redirect/https://evil.com"
curl -s -I "https://target.com//evil.com"
# Host header
curl -s -I -H "Host: evil.com" "https://target.com/"
# JavaScript-based
curl -s "https://target.com/redirect?url=https://evil.com" | grep -i "location\|redirect\|window.open"
```

## BYPASS TECHNIQUES
```
Subdomain: https://evil.target.com → target.com subdomain
URL parsing: https://target.com@evil.com
Fragment: https://evil.com#@target.com
Double encoding: https://%65vil.com
Backslash: https://evil.com\@target.com
Unicode: https://evil.com％00.target.com
Open redirect chain: target.com/redirect?url=target.com/redirect?url=evil.com
```

## SEVERITY AND COMPOUND RISK
An open redirect is only serious when the *origin matters* — a trusted brand
or auth domain. Judge it that way:

```
example.com/redirect → evil.com        → Low on its own
login.example.com/redirect → evil.com  → Medium (phishing surface)
OAuth redirect_uri accepts it          → High — and it needs its own test
```

Prove the redirect with a single request and stop. Do **not** build an OAuth
`redirect_uri` abuse chain, register an attacker app, or attempt to capture a
real token to justify a higher severity — that is account-takeover testing,
not redirect testing, and it is a separate scope decision. Note the OAuth
angle as a compound-risk line for the owner to chase.

Rejected at Gate 6: redirects that land on a warning interstitial, are
limited to a same-site allowlist, or require an authenticated flow the program
excludes. Those are the control working.
