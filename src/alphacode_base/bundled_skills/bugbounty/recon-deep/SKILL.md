---
name: recon-deep
description: Deep reconnaissance beyond subdomain lists — certificate transparency mining, historical URL and JS archaeology, ASN and IP-range scoping, cloud asset discovery, dangling DNS records, and favicon and technology fingerprinting. Scope-gated at every expansion step.
---

# DEEP RECONNAISSANCE

**Core:** passive sources reveal hosts and endpoints that active enumeration
never finds — decommissioned staging, forgotten APIs, and names that resolve
to nobody. The discipline is knowing which of those are *in scope*, because
recon is where scope creep starts.

---

## SCOPE GATE — THE MOST IMPORTANT SECTION HERE

**Discovering a host does not authorize testing it.**

```
Discovery source          Default posture
------------------------  -----------------------------------------
Program's own scope page   IN SCOPE
subfinder on scoped domain IN SCOPE if it matches the scope pattern
CT log                    CANDIDATE — confirm against scope pattern
Wayback / historical      CANDIDATE — the host may be long dead
ASN / IP range            CANDIDATE — confirm the host is in the program scope
Third-party SaaS host     OUT OF SCOPE by default (see below)
Employee personal host    OUT OF SCOPE always
```

Rules:
1. **Test only what the scope file names or its pattern matches.** A wildcard
   `*.example.com` covers `staging.example.com` but not `example.co.uk` and
   not `example.com.attacker.net`.
2. **Never test a third-party's infrastructure** — a CDN, a Statuspage, a
   GitHub Pages site, a Heroku app. If a takeover is plausible there, *report
   it*; the hosting provider is not your client and the target does not control
   it. This is the most common scope violation in recon, and it is
   unambiguous.
3. **Re-read the scope file after every expansion.** See the `scope` skill.

---

## PART 1 — CERTIFICATE TRANSPARENCY

CT logs are public, passive, and contain names that never appear in a live
subdomain list.

```bash
# crt.sh — the workhorse
curl -s "https://crt.sh/?q=%25.example.com&output=json" \
  | jq -r '.[].name_value' | tr '\r' '\n' | sort -u > ct-names.txt

# Split on the first dot to find deeper subdomains
cut -d. -f1-3 ct-names.txt | sort -u

# Wildcard certs expose the wildcard itself; look for staging/dev/test
grep -E '^(dev|test|qa|staging|uat|sandbox|internal|admin|legacy|v[0-9]+)\.' ct-names.txt
```

High-value CT findings: `*.dev.`, `*.staging.`, `*.internal.`, old `v1`/`v2`
API hosts, and regional or per-tenant hostnames. Filter the result against the
scope pattern **before** probing anything.

---

## PART 2 — HISTORICAL URLS

Endpoints that were once live often still are, frequently without auth.

```bash
katana -u https://target.com -d 3 -jc
# Historical:
gau --subs --providers wayback,commoncrawl,otx,urlscan https://target.com
# urlscan.io search is excellent for auth-flow and API discovery
curl -s "https://urlscan.io/api/v1/search/?q=domain:target.com" | jq '.results[].page.url'
```

```
LOOK FOR:  /api/v1/... when only /api/v2/ is documented
           admin and internal paths
           query parameters that reveal the object model (?format=, ?callback=)
           old upload, export, and debug endpoints
```

Historical endpoints are a strong finding on their own merit, and they are
in-scope if the host matches the scope pattern — being old is not a
disqualification. Confirm they still respond before reporting.

---

## PART 3 — JAVASCRIPT ARCHEOLOGY

The most productive pass in manual blackbox, because it is the app's own map.

```bash
# Every chunk, not just the entry bundle
curl -s https://target.com/ | grep -oE 'src="[^"]+\.js"' | cut -d'"' -f2
curl -s https://target.com/main.js | grep -oE '"[^"]*/(api|graphql|v[0-9])/[^"]*"' | sort -u

# Endpoints
grep -oE '/api/[a-zA-Z0-9/_.-]+' *.js | sort -u
# Secrets and keys
grep -oiE '(api[_-]?key|secret|token|client[_-]?id|aws_access|bearer)[^,;]{0,60}' *.js
# Third-party services — reveals vendors, and therefore attack surface
grep -oE 'https://[a-z0-9.-]+\.(com|io|net|dev|cloudfront\.net)' *.js | sort -u
# Source maps — frequently shipped to production
curl -so /dev/null -w "%{http_code}\n" https://target.com/main.js.map
```

**A source map in production is a High finding on its own**: it is the complete
original source including server-side paths and stripped comments. Report it.
Reading it is a separate question — ask the user before you treat it as a
source-code review.

---

## PART 4 — ASSETS, FINGERPRINTING, IP SPACE

```bash
# Favicon hash — identifies the exact product/panel across the estate
curl -s https://target.com/favicon.ico | md5sum
# then search that hash in public fingerprint databases

# Technology: httpx -tech-detect, then check versions against advisories
httpx -l hosts.txt -sc -title -tech-detect -cdn

# IP space — only for hosts already confirmed in scope
# Certificate SANs and reverse DNS often reveal the hosting range

# IPv6 — a forgotten AAAA record is a common oversight
dig AAAA target.com +short
```

```
FINDING: a version banner with a known applicable CVE -> report with the CVE id
FINDING: an admin panel fingerprint on a non-standard port, in scope -> report
NOT A FINDING: the technology stack itself, without a vulnerable version
```

---

## PART 5 — DANGLING DNS

```bash
dig CNAME stale.example.com +short        # -> provider / S3 / heroku / azure
subzy run --targets subdomains.txt
```

```
FINDING:  a CNAME pointing to a deprovisioned service resource -> report it
          as a takeover risk, naming the provider and the dangling record.
EVIDENCE: the CNAME target and the provider's deprovisioned response.

STOP HERE. Claiming the resource is not the engagement — it takes the hostname
away from a third party and can disrupt real traffic. Report it. Use
hunt-subdomain-takeover for the full method and its explicit stop rule.
```

---

## RECON OUTPUT FORMAT

Recon is a deliverable, not a side effect. Record it so it survives
compaction and so the report can cite it:

```markdown
## Attack Surface — <target> (<date>)
| Host | In scope? | Ports | Tech | Endpoints | Notes |
|------|-----------|-------|------|-----------|-------|
| app.example.com | yes | 443 | nginx, Django 4.2 | 61 | /api/v2/ live, /api/v1/ still responds |
| dev.example.com | yes | 443 | nginx | 12 | source map present |
| help.example.com | NO | - | - | - | excluded, not tested |
| status.vendor.com | NO | - | - | - | third-party, not tested |

Discovery sources: CT log, subfinder, wayback, 3 JS bundles
Not tested (out of scope): 4 hosts
```

The "not tested" column is what proves you respected scope, and it is the first
thing a reviewer checks.

---

## PACING

```
CT log + wayback + JS:   20 minutes, highest yield, all passive
Fingerprinting:          10 minutes
Deep host enumeration:   only after the above, only in-scope hosts
```

Passive recon is cheap and safe. Do all of it before sending a single
active probe, because it determines what to probe.
