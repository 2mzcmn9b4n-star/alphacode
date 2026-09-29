---
name: severity-engine
description: CVSS v4.0 base scoring for triage acceptance, and the base-vs-chain separation that keeps a finding from being marked speculative. Use when assigning severity, writing an impact statement, or deciding whether a chained impact can be claimed as demonstrated.
---

# SEVERITY ENGINE — SCORE WHAT YOU PROVED

**Core principle:** severity describes **the vulnerability you demonstrated**.
It is not a measure of how interesting the researcher found it, and it is not a
lever. A report that scores honestly gets triaged fast; a report that inflates
gets marked "informative" and, on most programs, the reporter loses trust for
the rest of the engagement.

---

## WHY THE OLD MODEL WAS WRONG

The previous heuristic was `Impact × Exploitability × Confidence`, with
`>60 → test now`. Three problems:

1. **It isn't CVSS.** Programs and triagers map findings to CVSS v4.0. A score
   they cannot map is a score they re-do themselves, or discount.
2. **Confidence is not a severity metric.** CVSS has no confidence axis.
   Folding it in conflates *"how sure am I this is real"* with *"how bad is
   this"* — two independent questions. High confidence + low impact is a real
   Low finding, and multiplying them hides that.
3. **It rewards inflation.** Three factors you control means three ways to
   overshoot. CVSS has fixed enumerations per metric, which makes overclaiming
   visibly hard.

The old model survives in exactly one role: **triage during reconnaissance**, to
decide *what to spend time on*. Use it there. Never in a report.

```
Recon triage only:  Impact(1-5) x Exploitability(1-5) x Confidence(1-5)
  >60 test now | 30-59 test next | <30 skip
Report severity:    CVSS v4.0 base, demonstrable metrics only
```

---

## CVSS v4.0 BASE METRICS

Score the **base** vector: what an attacker can do, unauthenticated, against
the vulnerable system. Not your access level, not your tooling, not the chain.

### Attack (what it takes to get there)
| Metric | Values | Meaning |
|--------|--------|---------|
| **AV** Attack Vector | N / A / L / P | Network / Adjacent / Local / Physical |
| **AC** Attack Complexity | L / H | Low (no special conditions) / High (conditions beyond attacker control) |
| **AT** Attack Requirements | N / P | None / Present |
| **PR** Privileges Required | N / L / H | **v4:** None / Low / High — does *not* change with scope |
| **UI** User Interaction | N / P / A | None / Passive / Active |

> v4 change: PR is no longer scope-dependent. A PR:H finding against a lower
> scope scores higher than v3.1 would give it. Note this — many triagers still
> reflexively apply v3.1 math.

### Vulnerable System (what you get)
| Metric | Values | Meaning |
|--------|--------|---------|
| **VC** Confidentiality | H / L | High: total loss of system data confidentiality |
| **VI** Integrity | H / L | High: total loss of system integrity |
| **VA** Availability | H / L | High: total loss of system availability |

### Subsequent Systems (does it spread?)
| Metric | Values |
|--------|---------|
| **SC** Subsequent System Confidentiality | N / H / L |
| **SI** Subsequent System Integrity | N / H / L |
| **SA** Subsequent System Availability | N / H / L |

### At Risk (what is in the blast radius)
| Metric | Values | Meaning |
|--------|--------|---------|
| **AU** Subsequent System Availability (Auth) | N / Y | Is a system at risk? *(use E)* |
| **R** Recovery | A / U / C / T | Automatic / User / Irrecoverable / — *(see v4 vector form)* |
| **V** Value Density | D / C | Diffuse / Concentrated |
| **RE** Vulnerability Response Effort | L / M / H | |
| **U** Report Confidence *(v4: Report Confidence)* | Clear / Green / Amber / Red | |

Vector form: `CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:H/VI:H/VA:N/SC:N/SI:N/SA:N`

**Rating bands:** None 0.0 · Low 0.1-3.9 · Medium 4.0-6.9 · High 7.0-8.9 ·
Critical 9.0-10.0

Use a calculator for the number (`cvss` from `pipx install cvss`, or any
verified v4.0 calculator). Never do this arithmetic in your head — and never
let an LLM do it for you. **Compute it, then write it in.**

---

## THE METRICS MOST OFTEN SCORED WRONG

| Mistake | Correct |
|---------|---------|
| "AC:H because the payload must be crafted" | AC:H is for conditions *outside* the attacker's control. Crafting a payload is not a special condition. → **AC:L** |
| "UI:A because the victim clicks a link" | UI:Passive. Clicking is passive. Active means the attacker does something beyond the link. |
| "AT:N when a race needs timing" | Timing is attacker-controlled. → **AT:N** |
| "PR:N when a token is needed" | If you must hold any valid account, → **PR:L**. An auth-bypass to guest is still PR:N. |
| "VC:H for one user's email" | VC is H only for **total** loss of the system's confidentiality. One record is **L**, plus V:Diffuse. |
| "S:Unchanged omitted" | Required for a valid vector. Don't drop metrics. |

The VC mistake is the single biggest source of inflation. A proven IDOR
reading **one other user's profile** is `VC:L`, not `VC:H`.

---

## BASE vs CHAIN — THE SEPARATION THAT GETS REPORTS ACCEPTED

A finding has two parts, and they must never be blended:

```
BASE   = what you demonstrated, end to end, with evidence
CHAIN  = what that plausibly enables if an attacker also has X
```

Report them separately. Score only the base.

```markdown
## Severity
CVSS v4.0: 6.5 (CVSS:4.0/AV:N/AC:L/AT:N/PR:L/UI:N/VC:L/VI:N/VA:N/SC:N/SI:N/SA:N)
Bands: Medium
This scores the demonstrated behavior only: an authenticated user reads one
other user's order record.

## Demonstrated
GET /api/orders/{id} with User A's token and User B's order id returns
User B's order including address and line items. Two records retrieved, no
modification.

## Analyst note — not demonstrated, not scored
`/api/orders/{id}` sits behind the same missing-ownership check as
`/api/orders/{id}/refund` and the admin route `/api/admin/orders/{id}`, which
returned 404 to an unauthenticated request in testing. If the same check is
absent on the refund route the impact is materially higher; we did not test
that route because it moves money.
```

Why this works: the triager sees a correctly scored, fully evidenced base, plus
a precise map of where the risk likely continues. They can raise the severity
themselves — with program knowledge you do not have. You have not claimed
something you cannot prove, and you have not hidden anything.

**Never** write the chain into the impact paragraph as fact. The moment it
becomes "an attacker can refund arbitrary orders", the whole report is a
hypothesis and the proven half gets discounted with it.

---

## WHAT PROGRAMS ACTUALLY PAY ON

CVSS is the *language*, not the *decision*. Most programs publish their own
severity matrix, and it is frequently not CVSS-derived. Before scoring:

1. **Read the program's severity matrix**, not just the scope page.
2. Score in their terms if they publish terms. If they publish CVSS, use CVSS.
3. Note where your impact sits in their bands, and mention the discrepancy
   once if it matters: *"CVSS 6.5 (Medium); per your matrix, unauthorized
   access to another user's order is High — flagging in case that maps
   differently."* One sentence, no argument.
4. **Payout is per-accepted-unique-bug, not per-severity-point.** A duplicate
   or "informative" is worth zero regardless of the number in the report.
   Avoiding FPs is worth more than any scoring trick.

### Impact-scaling, not severity-inflating

When impact is genuinely large, make it *verifiable* rather than asserted. This
raises the score honestly and it is what a triager wants:

```
Weak:   "This IDOR exposes all user data."            (unquantified)
Strong: "This IDOR applies to all 14 list endpoints. At 41k users the
         reachable record set is every order, invoice and saved card in the
         system. Retrieved 2 records to confirm; estimate from endpoint count
         and row counts, not from retrieval."

Weak:   "Rate limiting is missing, so accounts can be brute-forced."
Strong: "No rate limit observed across 20 requests in 4s. Combined with the
         4-character minimum length, this is a full account-takeover path on
         accounts without MFA. Estimated 8,140 such accounts exist."
```

The second version wins because the reviewer can act on it. The first is a
claim; the second is a measurement with a stated basis and a stated limit.

---

## CHECKLIST BEFORE SUBMITTING

```
[ ] CVSS v4.0 base, computed by a calculator, vector string in the report
[ ] Every metric defensible from the evidence, not from the worst case
[ ] Confidence NOT folded into the score
[ ] Demonstrated impact is the scored impact
[ ] Chain, if any, is in a separate analyst-note section
[ ] Quantification uses counts you actually measured, with the method stated
[ ] Program's own severity matrix checked and referenced
[ ] Nothing in the impact paragraph is counterfactual
```

A report that passes this list is accepted. A report that fails it is not
fixable by editing the number.
