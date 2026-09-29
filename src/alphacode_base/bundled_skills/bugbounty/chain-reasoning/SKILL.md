---
name: chain-reasoning
description: Maps vulnerability primitives to what they unlock, so a small finding reprioritizes the rest of the hunt and the report describes real escalation accurately. Two uses — choosing the next test, and writing the impact section. Use when you find something that looks minor but implies the stack is exposed, or when writing an impact statement that needs to convey severity honestly.
---

# CHAIN REASONING — SMALL FINDINGS REPRIORITIZE THE HUNT

**Core principle:** a primitive is not a severity, it is **information about the
attacker position**. A stack trace is Low on its own. It tells you the internal
hostnames, the framework version, and that error handling is unhandled — and
each of those is worth more than the stack trace.

This skill exists because a serious hunter reads every finding as a *window*,
not a *dead end*. It is a **prioritization tool and a writing tool.** It is not
a licence to walk through the window.

---

## TWO USES, ONE GRAPH

```
USE 1 — PRIORITIZE.  "I found X. Given X, what do I test next?"
         This is where nearly all of the value is. Free. No risk.
USE 2 — WRITE.       "I found X. Why does this matter to the owner?"
         This is the analyst note in the report. Describes, does not claim.
```

**What this graph is not:** a ladder to climb. The graph never tells you to
execute the next link. It tells you what to *test*, and testing stops at the
first proven boundary (see IMPACT BOUNDARY below).

---

## THE PRIMITIVE GRAPH

| Primitive found | Unlocks (hypothesis, not fact) | Next test worth running |
|-----------------|-------------------------------|-------------------------|
| **Stack trace / error** | internal hostnames, framework + version, whether a debug mode is on | resolve the internal names; hunt `/debug`, `/_profiler`, `/actuator`; test whether errors are also verbose on unauthenticated routes |
| **Internal hostname** | the internal service surface exists and is not in your recon | test whether any internal route is *externally* reachable: `api-internal.x`, alternate ports, path-based routing (`/internal/api/...`) |
| **Verbose error (SQL)** | query shape, table names, parameter count | count injectable parameters; check whether errors differ between valid/invalid → boolean oracle |
| **Exposed `.git` / backup** | full source, `.env`, dependency manifests | the diff between the backup and production is the finding; look for config that differs |
| **Config / env disclosure** | DB and internal service credentials, feature flags | test whether a leaked credential is *still valid* and *still privileged* — stale config is the common case and the reportable one |
| **User enumeration** | valid accounts, roles, org structure | the account's privilege tier is the question — a leaked *admin* list is a different finding from a leaked *user* list |
| **Low-priv account access** | a real position inside the app | run the 3x3 matrix (control-verification) as that user — this is where the money is |
| **IDOR on one endpoint** | the *pattern* is likely repeated across sibling endpoints | test the **sibling endpoints of the same class**, not deeper on the same one |
| **SSRF (any)** | internal services, and cloud metadata if on a cloud provider | is the *response returned* (read) or blind? Confirm read, then **stop**. Metadata is explicitly the boundary. |
| **SSRF to metadata** | the platform's own credential service | one request to prove the reach. **Stop there.** Reading the credentials is out of bounds. |
| **Stored XSS** | executes in another user's browser on the app's origin | is it reflected to an *admin* view? Determine reachability from a public post only. **Do not** craft an admin-targeted payload. |
| **Open redirect** | token leakage if a redirect sits in an auth flow | is the redirect parameter present in a *login/callback* flow? Check by reading the flow, not by building an abuse chain. |
| **JWT weak secret / `alg:none`** | token forgery for arbitrary identity | forge **your own** token with a **non-privileged** claim. Proof of forgery, not of impersonation. |
| **GraphQL introspection** | the full object graph and resolver list | test **authz on one sensitive resolver**, not the whole graph. Introspection alone is not a finding. |
| **Public file / object store** | every object in that bucket | the finding is the **bucket policy**, not the objects. Do not enumerate further. |
| **Path traversal read** | source, config, possibly creds | read exactly one config file to confirm impact class. **Stop.** No credential reuse. |
| **Subdomain takeover** | control of a first-party origin | a single benign file is the standard safe proof. **Do not** host anything, do not set cookies, do not chain to session theft. |
| **Exposed API key** | whatever that key can reach | test the key's **capability**, not its volume. "This key reads all customer records" is the finding; pulling records is not. |
| **Mass assignment** | field-level write control | flip one *non-privileged* field (`display_name`). If that works, the class is proven. Escalating to `role` is a separate test — ask first. |
| **Race condition** | a check that is not atomic | re-run at low concurrency to confirm. **Stop at proof.** No real double-spend. |

---

## USE 1 — PRIORITIZATION, IN PRACTICE

When you find a primitive, immediately ask three questions and write the answers
down. This takes 30 seconds and is where finding-rate jumps come from.

```
1. What does this tell me about the stack that I did not know?
2. Given that, which UNTESTED control is now the most likely to be weak?
3. What is the cheapest test that would confirm or kill it?
```

Worked example:

```
FINDING: unauthenticated /api/debug/config returns the full feature-flag
         document, including internal service URLs and a "payments_v2"
         service name.

Q1: There is a second payments service. My recon only covered the host in
    the config.
Q2: If payments_v1 is legacy, its authZ is likely weaker. Legacy endpoints
    are the single most common place for an unauthenticated financial route.
Q3: Resolve payments_v2 hostnames, confirm the host is IN SCOPE, then hit
    /api/orders and /api/refund unauthenticated and record status codes.
    ~4 minutes.

NOTE: the flag document itself is a Low/Medium info disclosure. The reason
to report it well is that it maps the internal surface — that is what makes
it worth the owner's attention.
```

That is the whole skill. Thirty seconds of reasoning, four minutes of test, one
Low finding and possibly one High.

---

## USE 2 — WRITING THE IMPACT

The chain belongs in the report as an **analyst note**, structurally separated
from the demonstrated impact. Template:

```markdown
## Demonstrated impact (scored)
[Exactly what you did, with requests and responses. This is the base vector.]

## Analyst note — related exposure not tested
[Breadth, not depth. The point is to route the owner's attention, not to
claim severity.]

Reasoning: `/api/debug/config` is unauthenticated and enumerates internal
service names. Those services resolve publicly, which means the internal
surface is not actually internal. Legacy sibling endpoints commonly enforce
authorization less consistently than the current version. We did not test
`/api/orders` or `/api/refund` on the legacy host because they move money
and that is outside the scope file's rules — flagging it for your team to
check internally.
```

Three properties that make this work with triagers:
- **Breadth, not depth.** "This affects 14 sibling endpoints" is checkable and
  valuable. "This leads to RCE" is neither.
- **Reasoned, not asserted.** "Legacy endpoints commonly enforce less
  consistently" is domain knowledge the owner can evaluate.
- **Self-limited.** Stating what you *did not* do and why reads as rigour, and
  is often what gets a report prioritized over a noisier one.

---

## IMPACT BOUNDARY

The graph is a map. Walking the whole map is not the job.

```
Demonstrated one link  ->  STOP. Describe the rest in the analyst note.
Never execute link N+1 because link N was interesting.
Never "just confirm" a second boundary uninvited.
```

Escalate past one link only when **all three** hold:

1. The finding is fully verified.
2. The user explicitly asked for maximum reachable impact.
3. The next step is permitted by the scope file.

**"Reach the deepest point" is not a scope grant.** Depth beyond the first
proven boundary requires the *owner* to widen scope — which is a conversation,
not an inference. The correct move when a chain looks deep is to write the
excellent analyst note and let the owner open the next door. That is also
strictly better for your reputation than a report that broke their production
to prove a point.

---

## ANTI-PATTERNS

```
X  "This minor info leak is the first step toward RCE, so it's Critical."
   Critical is a score for demonstrated impact, not for a forecast.
   Score it for what it is. Explain the chain. Let triage decide.

X  Walking the graph to its end because each link is "just one more step".
   That is escalation, and the scope file is the only thing that bounds it.

X  Using a chain to make a small finding look substantial.
   Triagers see this constantly. It converts a valid report into a rejected one.

X  Describing a chain you have not reasoned about, as if you had.
   "Could potentially lead to further compromise" with no mechanism is noise.
   Name the specific endpoint and the specific reason.

O  "Unauthenticated flag document enumerates internal service names, and the
   legacy sibling host enforces no authorization on the orders endpoint
   (GET returns 200 unauthenticated). Base finding scores Medium; the sibling
   gap is noted but untested for write operations."
```

The pattern to internalise: **breadth you measured, depth you describe.**
Both raise the value of a report. Only one is safe to execute.
