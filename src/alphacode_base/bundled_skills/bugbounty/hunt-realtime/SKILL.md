---
name: hunt-realtime
description: WebSocket, SSE, gRPC and GraphQL subscription authorization — handshake auth, per-channel and per-topic subscription authz, message replay, gRPC reflection and metadata trust, and the fact that these protocols bypass REST middleware entirely.
---

# REALTIME PROTOCOL HUNTING

**Core:** WebSocket, gRPC, SSE and GraphQL subscriptions each have their **own**
authentication and authorization path. REST middleware does not apply to them.
A perfectly hardened REST API with an unauthenticated WebSocket is a normal
real-world configuration, and it is a critical finding.

---

## WEBSOCKET

### 1. The handshake is unauthenticated by default
The `GET` upgrade request is just an HTTP request — test it as one.

```bash
# No upgrade, no cookie, no token: what does the endpoint say?
curl -si "https://target.com/ws" | head -20
curl -si "https://target.com/socket.io/?EIO=4&transport=websocket" | head -20

# Valid upgrade, no credentials
curl -si -H "Connection: Upgrade" -H "Upgrade: websocket" \
     -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==" \
     "https://target.com/ws" | head -20
```

```
A handshake that reaches 101 with no credentials is the finding.
A socket.io transport that upgrades and then only gates on an event
("auth" / "subscribe") is normal — the test continues at the event layer.
```

### 2. Auth is often in the query string — and that is logged
```
wss://target.com/ws?token=JWT      token in a URL ends up in proxy/analytics logs
Sec-WebSocket-Protocol: bearer, JWT   legitimate; test that the app validates it
```
Flag token-in-URL as a finding on its own (credential exposure via logs), then
continue testing the socket.

### 3. Subscription authorization — the real surface
Connection auth does **not** imply message auth. Once connected as A, try to
join B's topics.

```
subscribe  {"type":"subscribe","channel":"orders:12345"}   channel for B
publish    {"type":"message","channel":"orders:12345","body":"..."} as A
```

```
CHECK: does the server validate that the actor may join that channel?
CHECK: can channel names be guessed or enumerated (sequential ids, usernames)?
CHECK: does history/replay on join expose prior messages of a channel?
CHECK: is authorization re-checked per message, or only at subscribe time?
CHECK: does a role change (demotion) revoke an active subscription?
```

**History replay is the highest-value WebSocket bug** and is almost never
tested: connect to a channel you are entitled to, then to one you are not, and
compare the buffered history.

---

## GRAPHQL SUBSCRIPTIONS

```graphql
subscription { messageAdded(roomId: "9999") { body sender } }
```

Identical authorization problem, and the `roomId` is a plain argument — a
`roomId` you do not own is a direct BOLA over a live channel. Test with two
accounts exactly as you would a REST IDOR.

---

## SERVER-SENT EVENTS

```bash
curl -sN -H "Authorization: Bearer $TOKEN_A" "https://target.com/api/events/stream"
```
SSE inherits HTTP auth, so it is usually *better* protected than WebSocket.
The bug is nearly always the **identifier in the URL**:
```
/api/users/$A_ID/events/stream   ->  does the server check ownership of $A_ID?
```

---

## gRPC

```bash
# Reflection is often enabled in production — it enumerates the whole schema
grpcurl -plaintext target.com:443 list
grpcurl -plaintext target.com:443 list <package>
grpcurl -plaintext -d '{"id":"9999"}' target.com:443 pkg.Service/Get

# Auth usually rides in metadata, not the path — easy to forget on a call
grpcurl -H "authorization: Bearer $TOKEN_A" -d '{"id":"9999"}' \
  target.com:443 pkg.Service/Get
grpcurl -H "authorization: "          -d '{"id":"9999"}' target.com:443 pkg.Service/Get
grpcurl -H "x-api-key: leaked-from-js"    -d '{}' target.com:443 pkg.Service/Admin
```

```
CHECK:  is reflection enabled outside dev? (Info, Low on its own)
CHECK:  does every RPC enforce auth, or only the gateway-visible ones?
CHECK:  is any RPC missing auth entirely? (the highest-value check)
CHECK:  is TLS enforced, or does plaintext still work?
CHECK:  are internal-only RPCs exposed on the same listener?
```

**gRPC + reflection is the best of both worlds for a reviewer**: the schema is
free, and an unauthenticated `Admin*` RPC is an unambiguous, copy-pasteable
critical.

---

## IMPACT BOUNDARY

Realtime channels are live and shared. Do not send messages, do not inject
content into a channel other testers are using, and do not subscribe to
anything you cannot justify in the scope file.

```
Channel authz proven  -> STOP. Do not read more history. Do not publish.
gRPC unauth'd RPC      -> STOP. Prove the one call, do not enumerate methods.
Reflection enabled     -> Low/Info. Do not treat it as a gateway to everything.
```

Proof is one unauthorized read of one message. That is the whole finding.

---

## SEVERITY

```
Unauthenticated subscribe to another user's private channel  -> Critical
Unauthenticated gRPC RPC that reads bulk data                -> Critical
Per-message authz missing, subscribe-time present            -> High
Token in WebSocket query string (log exposure)               -> Medium
gRPC reflection in production                                -> Low
SSE identifier with no ownership check, low-sensitivity data  -> Medium
```

Use `severity-engine`. These findings are unusually easy to score correctly
because the boundary crossed is unambiguous — say so in the report.
