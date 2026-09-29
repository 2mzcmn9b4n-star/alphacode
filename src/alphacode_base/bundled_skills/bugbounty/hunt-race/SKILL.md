---
name: hunt-race
description: Race condition and concurrency flaws — TOCTOU, double-spend, double-redeem, limit bypass, and session races. Includes a controlled test harness and the reproducibility discipline needed to separate a real race from an app being slow. See hunt-business-logic for the state rules these break.
---

# RACE CONDITION HUNTING

**Core:** a check and the action it guards are not one operation. Anything the
application validates in one query and acts on in another is exploitable by
reaching both in the same instant.

**The hard part is not finding races. It is proving one is a race and not just
a slow endpoint** — that distinction is the whole difference between a
Critical finding and a rejected report.

---

## WHERE RACES LIVE

```
CHECK-THEN-ACT     read balance → write debit        (wallet, wallet balance)
VALIDATE-THEN-CONSUME  check coupon valid → mark used   (coupons, vouchers)
LIMIT-THEN-INSERT  read "9 of 10 seats" → insert        (seats, quotas, uploads)
STATE-THEN-TRANSITION  read PENDING → process refund    (workflow transitions)
EXIST-THEN-CREATE  check email unused → create user     (registration)
FILE-THEN-MOVE     validate extension → write to disk  (upload)
```

Every one of these is a business rule, not a vulnerability class that lives in
the framework. Map the app's rules first — see `hunt-business-logic`.

---

## CONTROLLED TEST HARNESS

Fire-and-forget loops do not prove races. They prove you sent N requests.
Control the concurrency and observe per-request outcomes.

```bash
# N parallel requests against your OWN test object, printing each status
seq 1 20 | xargs -P 20 -I{} sh -c \
  'curl -s -o /dev/null -w "%{http_code} " -X POST \
     -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
     -d "{\"code\":\"TESTPROMO\"}" \
     "https://target.com/api/redeem"'
echo

# Baseline: the same request 20 times SEQUENTIALLY must succeed exactly once
seq 1 20 | xargs -P 1 -I{} sh -c \
  'curl -s -o /dev/null -w "%{http_code} " -X POST \
     -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
     -d "{\"code\":\"TESTPROMO\"}" \
     "https://target.com/api/redeem"'
echo
```

**A race is proven when the parallel run produces a different multiset of
outcomes than the sequential run.** Parallel: 20 × `200`. Sequential: 1 × `200`,
19 × `409`. That difference is the evidence, and it is what the report needs.

A tighter test narrows the window instead of just adding load:

```bash
# Pipelined on one connection — the two requests leave microseconds apart
printf 'POST /api/redeem HTTP/1.1\r\nHost: target.com\r\nContent-Type: application/json\r\nContent-Length: 18\r\n\r\n{"code":"TESTPROMO"}POST /api/redeem HTTP/1.1\r\nHost: target.com\r\nContent-Type: application/json\r\nContent-Length: 18\r\n\r\n{"code":"TESTPROMO"}' \
  | openssl s_client -quiet -connect target.com:443 2>/dev/null
```

---

## REPRODUCIBILITY DISCIPLINE — THE ANTI-FALSE-POSITIVE PART

Almost every rejected race report fails one of these. Check all of them.

```
1. SEQUENTIAL BASELINE   Does the operation also succeed twice when run one at
                         a time? If yes, it is not a race — it is a missing
                         uniqueness constraint, which is a different (often
                         still valid) finding. Say which one it is.

2. REPRODUCIBILITY      Run the parallel test 5+ times. A "race" that hits 1 in
                         20 attempts is a report the reviewer cannot verify, and
                         unverifiable findings are closed. Report the hit rate
                         honestly: "reproduced in 3 of 5 runs at concurrency 20".
                         If it only reproduces at concurrency 200, the window is
                         too small to be practically exploitable — that is a
                         Low or an Informational, and honesty here is the only
                         way it gets accepted at all.

3. IS IT THE APP BEING SLOW?
                         A 200 under load that would have been a 409 given time
                         is a timeout, not a race. Distinguish by checking the
                         response BODY, not the status code.

4. DID YOU OWN THE OBJECT?
                         Races on objects the test itself created may reflect the
                         test's setup, not a production-reachable state. Use
                         objects that exist through the normal user flow.

5. AM I CREATING REAL HARM?
                         This is a boundary, not a check. See below.
```

The reviewer must be able to run your exact command and see the result. If they
cannot reproduce it, they will close it — and they will be right to.

---

## CLASSES

```
DOUBLE SPEND        parallel debit/transfer → balance goes negative
DOUBLE REDEEM       parallel coupon redeem → one coupon, N discounts
DUPLICATE ENTITY    parallel create with the same unique key → two accounts,
                    two orders, two subscriptions
LIMIT BYPASS        parallel invite/upload → seat/quota/size limit exceeded
STATE TRANSITION    parallel approve+reject → both applied, or invalid state
SESSION RACE        parallel "create session" + "use session" → fixation onto
                    a session an attacker chose
OTP / 2FA RACE      parallel verify with a guessed code → second attempt
                    checked against a different record
COUPON BOUNDARY     redeem at T and apply at T across two endpoints
```

---

## IMPACT BOUNDARY

Race conditions are the easiest bug class to accidentally weaponize, because
the requests are legitimate and the effect is real. The boundary is absolute.

```
Race proven  ->  STOP at proof.
                No real double-spend. No drained balance. No real money moved.
                No real duplicate order fulfilled. No real duplicate account
                enrolled in a paid plan.

PROVE IT WITH:  your own object, your own balance, a test coupon you created,
                a test account you own, and a bounded concurrency (10-20).
NEVER WITH:     someone else's data, a real payment, a production coupon, or
                concurrency high enough to degrade the service.
```

**Quantify by analysis, not by damage.** "The debit path reads the balance and
writes it in two statements, so concurrency N yields up to N-1 extra debits; the
maximum loss is bounded by the account balance" is a complete impact statement
and requires no money to move. A report containing an actual negative balance
is a *worse* report — it is an incident you created and it hands the program
grounds to ban you.

If a race is genuine but exploitation would require production conditions, say
so: "requires sustained concurrency against a live balance; the window is
approximately 1-3 ms based on response timing. I did not attempt a real
double-spend." Reviewers respect that and it is the truth.

---

## SEVERITY

```
Double-spend on a stored-value balance          -> Critical
Duplicate fulfillment of a paid order           -> Critical
Free subscription via duplicate enrollment     -> High
Coupon / credit over-redemption                -> High
Quota or seat limit bypass                    -> Medium
Duplicate low-value entity (e.g. profile)      -> Low
```
Use `severity-engine`. Report **base severity for what you proved**, and note
the theoretical ceiling as an analyst note — the same base-vs-chain separation
described in `chain-reasoning`.
