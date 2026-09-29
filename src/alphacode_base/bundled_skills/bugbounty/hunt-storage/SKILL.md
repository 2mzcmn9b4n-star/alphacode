---
name: hunt-storage
description: File upload and object storage — extension and content-type bypass, server-side processing, filename and path handling, presigned URL scope and expiry, and bucket/download endpoint authorization. Combines upload flaws with the storage ACL flaws that make them permanent.
---

# FILE UPLOAD AND STORAGE HUNTING

**Core:** an upload is two systems. The **app** validates the file; the **store**
holds it. Most teams test the first and assume the second. Upload + weak bucket
ACL is one of the most reliable chains in bug bounty, and it is entirely
missed by generic scanners.

---

## PART 1 — THE UPLOAD

### The validation checklist
```bash
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
     -F "file=@test.svg;type=image/png"        "$B/api/upload"
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
     -F "file=@test.php;type=image/png"        "$B/api/upload"   # mismatch
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
     -F "file=@shell.phtml;type=image/png"     "$B/api/upload"   # alternate exec ext
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
     -F "file=@test.png.jpg"                   "$B/api/upload"   # double extension
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
     -F "file=@test.png%00.php"                "$B/api/upload"   # null byte (older stacks)
```

| Claim in the UI | Test |
|-----------------|------|
| "Images only" | upload a text file, a `.svg`, a `.html` |
| "Max 5 MB" | 5 MB + 1 byte; also a declared small size with a large body |
| "JPG/PNG only" | `.jpeg`, `.jfif`, `.webp`, `.avif`, `.svg` |
| "Safe filename" | non-ASCII, spaces, `..`, absolute path, `%00`, 255-char name |
| "Re-encoded on upload" | upload a file whose bytes you know; re-download and compare |

**SVG is the highest-yield bypass.** It is an image by MIME type, renders in a
browser, and can carry script. An SVG upload served from the same origin is a
stored-XSS finding, not an "SVG is an image" non-finding.

### Server-side processing is the real target
```bash
# ImageMagick / libvips / ffmpeg — the "harmless image" is the payload carrier
convert 'label:"https://x"' out.png          # older ImageMagick delegates
# SVG -> raster conversion executes embedded content on many toolchains
# PDF uploads: JS in a PDF viewer, XXE in the parser, SSRF via external refs
# Office docs: macros, external template refs (DDE), embedded OLE objects
```
Identify the processing library from headers, response format, or timing, then
check the version against known issues. Report *"a PDF is processed and the
parser is version X, which is affected by CVE-Y"* only if you can name the CVE
and the path.

---

## PART 2 — THE STORE (where the real findings live)

### Download endpoint authorization — run the 3x3 matrix
```
/api/files/{id}/download   with A's token and B's file id
/api/files/{name}          with a guessed or leaked name
/api/attachments/{id}      unauthenticated
```
A sequential or guessable file id plus a missing ownership check is a plain
IDOR over every user's attachments. Use `control-verification`.

### Presigned URL flaws
```bash
# Do the params actually bind the signature?
curl -s "$PRESIGNED" &expiry=<epoch+99999>          # extended expiry
curl -s "$PRESIGNED" &content-disposition=attachment
curl -s "${PRESIGNED%.jpg}.sql"                      # sibling object, same prefix
# Reuse: is the URL single-use, or a bearer credential good for a week?
# Does it work after the object is deleted or the account is disabled?
```
Finding: *presigned URL is valid for 7 days with no content-type binding* —
that is a real access-control finding, and it is a *design* finding, not a
filter bypass.

### Bucket ACL
```
public-read?          does listing work?  does the CDN expose it?
object-level ACL?     public READ but not WRITE is still a finding
block-public-access?  is it actually enforced, or just set?
```
Report the **policy**, not the objects. "The bucket permits anonymous GET on
all objects" is the finding; enumerating the contents is not your engagement.

---

## PART 3 — PATH HANDLING

```bash
-d '{"filename":"../../../etc/passwd"}'
-d '{"filename":"..%2f..%2f..%2fetc%2fpasswd"}'
-d '{"path":"uploads/../../../../app/config.py"}'
-d '{"url":"file:///etc/passwd"}'                    # if the app fetches by URL
```
For a traversal that reads, prove it with **one** non-sensitive target that
demonstrates the class — a config file, or a second user's own upload. Never a
system credential file, and never anything you then use.

---

## IMPACT BOUNDARY

Uploads and buckets are where testers most easily cause lasting damage. The
boundary is about **what you leave behind**.

```
Upload bypass proven   -> STOP. Do not upload a shell, webshell, or executable.
                          Do not leave any file on their server.
Weak ACL proven         -> STOP. Do not enumerate the bucket. Do not download
                          other users' files.
Presigned flaws proven  -> STOP. Do not mint URLs for objects you do not own.
Traversal proven        -> STOP. One read. No credential reuse.
SVG XSS proven          -> STOP. Do not build a cookie-stealing payload.
```

**Always upload a benign file, note its URL, and delete it if there is a delete
endpoint.** A report that says "I uploaded `proof.txt` and it was stored as
`/uploads/a3f9/proof.txt`" is complete. One that includes a working payload is
an incident you caused.

---

## SEVERITY

```
Upload → RCE via server-side processing              -> Critical
Arbitrary file write to a served path                 -> Critical
Public bucket exposing all user uploads/PII          -> Critical
Stored XSS via SVG on the app origin                  -> High
IDOR over another user's attachments                 -> High
Presigned URL valid indefinitely / no content bind   -> High
Path traversal reading source or config               -> Medium-High
Executable accepted, not proven reachable             -> Medium
Upload accepted, no demonstrated impact              -> Low
```

Use `severity-engine`, and be strict about the difference between *accepted* and
*reachable*. A `.php` upload stored in a bucket that is never executed by a web
server is a Medium, not a Critical — overstating it is the most common way
upload reports get closed.
