---
name: cloud-config
description: Cloud and platform configuration assessment — object storage public access, over-permissive IAM, instance metadata reachability, exposed Kubernetes and container registry APIs, managed identity trust, and secret material in the open. Read-only assessment methods; see IMPACT BOUNDARY.
---

# CLOUD AND PLATFORM CONFIGURATION

**Core:** most cloud findings are **read-only reachability tests**. You prove a
misconfiguration exists from outside. You do not use the access it grants.
The line between "assessing a cloud misconfiguration" and "using stolen cloud
credentials" is the entire legal and ethical boundary of this skill, and it is
not a grey area.

---

## PART 1 — WHAT IS IN SCOPE

A cloud misconfiguration is only in scope if the **cloud account is named in the
scope file**, or the program states that cloud infrastructure is in scope.
Many bug bounty programs scope only the web application and explicitly exclude
third-party SaaS and cloud accounts.

```
READ the program policy for:
  - "cloud infrastructure" / "hosted services" in scope?
  - explicit exclusions: "no testing of our cloud account",
    "AWS/GCP/Azure out of scope", "no credential testing"
  - safe-harbor wording that covers configuration review
```

**If the cloud account is not named, stop.** An S3 bucket found via a
subdomain is still the company's asset, but the authorization comes from the
program policy, not from the fact that you found it. Check before you probe.

---

## PART 2 — OBJECT STORAGE

```bash
# Anonymous listing — a 200 with keys is the finding
curl -s "https://<bucket>.s3.amazonaws.com/" | head -30
curl -s "https://storage.googleapis.com/storage/v1/b/<bucket>/o" | head -30
curl -s "https://<account>.blob.core.windows.net/<container>?restype=container&comp=list"

# Single known object — does the bucket deny by default?
curl -so /dev/null -w "%{http_code}\n" "https://<bucket>.s3.amazonaws.com/known-key"
```

```
READ:   403 Access Denied on list  -> correct. Not a finding.
READ:   200 with object keys       -> public listing. Report the policy.
READ:   403 list but 200 on a known key -> public objects, private listing.
        Still a finding; note the distinction precisely.
```

**What makes this a strong report:** the *policy*, stated as a fact, plus the
object classes exposed (`access_logs/`, `backups/`, `user-uploads/`, `*.csv`),
plus whether writes are permitted. Do not download the contents. The listing
you already have is the evidence.

---

## PART 3 — INSTANCE METADATA (read-only reachability)

If you find any SSRF, the first question is *which cloud*, because the metadata
endpoint is the highest-value target — and also the clearest boundary.

```bash
# AWS / GCP — a 200 with these keys means SSRF reaches credentials
curl -s http://169.254.169.254/latest/meta-data/
curl -s http://169.254.169.254/latest/meta-data/iam/security-credentials/
curl -s http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/
# Azure
curl -s -H "Metadata: true" "http://169.254.169.254/metadata/instance?api-version=2021-02-01"
```

```
ALLOWED  prove reachability with ONE request that returns the role name or the
         list of credential names.
         "SSRF returns the instance role name from the metadata service" is a
         complete, high-severity finding. It is the finding.
STOPPED  Do NOT fetch the access key id, secret, or session token.
         Do NOT use them against any API.
```

This is the clearest line in the skill. **A metadata `RoleName` in the report
is a critical finding. The `SecretAccessKey` is evidence of a crime.** There is
no scenario in an authorized engagement where you need the secret to make the
report valid.

---

## PART 4 — EXPOSED MANAGEMENT APIS

```bash
# Kubernetes API — often reachable and unauthenticated on a bad network path
curl -sk https://<k8s-host>/version
curl -sk https://<k8s-host>/api/v1/namespaces
curl -sk https://<k8s-host>/api/v1/secrets              # 403 = correct
# If 200 on /version and /api, the cluster is exposed — that is the finding.

# Container registry
curl -s https://<registry>/v2/                       # 401 = correct
curl -s https://<registry>/v2/_catalog               # 200 = enumerable

# Common admin panels — fingerprint only, no auth bypass attempts
/admin /actuator /actuator/env /actuator/health /debug/pprof /metrics
/jmx-console /solr /phpmyadmin /adminer
/swagger /openapi.json /graphql /_debug /server-status
```

```
/actuator/env exposing secrets, or /actuator/heapdump -> Critical, and the
   report is just the endpoint and the fact. Do not read the dump.
```

---

## PART 5 — IAM AND TRUST POLICY REVIEW

Where source code or a public config is available, the finding is in the
policy. Read the policy; test nothing.

```json
{ "Effect": "Allow", "Action": "s3:*", "Resource": "*" }
{ "Effect": "Allow", "Action": "iam:PassRole", "Resource": "*" }
{ "Effect": "Allow", "Action": "sts:AssumeRole", "Resource": "*" }
```

```
FINDING:  wildcard action on wildcard resource in a trust or access policy
SEVERITY: Critical if attached to a reachable principal; Medium if theoretical
EVIDENCE: the policy document. Nothing else is needed.
```

Wildcard-resource policies are routinely shipped to production and rarely
flagged by the team, so they are genuinely valuable findings — and reading a
policy is free and safe.

---

## IMPACT BOUNDARY

```
Metadata reachable     -> one request, role name only. STOP.
Bucket public          -> list keys, read nothing. STOP.
K8s API exposed        -> GET /version, GET /api. STOP before /api/v1/secrets.
Exposed admin panel    -> fingerprint and report. No auth bypass attempts.
IAM policy             -> read and report. Test nothing.
Exposed secret in repo -> report the location and type. Do not authenticate with it.
```

**Never use a credential to prove you could.** An owner who reads "your
instance role is reachable via SSRF and grants `s3:*` on `*`, here is the
request that demonstrates the reach" has everything they need, and no incident.
The report that includes stolen keys is the one that ends the relationship.

---

## SEVERITY

```
SSRF reaching instance metadata with a privileged role  -> Critical
Public bucket with PII, backups, or credentials            -> Critical
Exposed admin panel leaking secrets (actuator/env, heapdump)-> Critical
Unauthenticated Kubernetes API                            -> Critical
Wildcard IAM policy in a reachable context                 -> High (Critical if live)
Public bucket with only public content                     -> Low
Container registry anonymously enumerable                  -> Medium
Exposed instance type, region, public IP via metadata      -> Low
```

Use `severity-engine`. Cloud findings are scored on **what the misconfiguration
exposes**, and the report should lead with the policy or endpoint — a reviewer
can verify it in seconds, which is why these triage fast when reported honestly.
