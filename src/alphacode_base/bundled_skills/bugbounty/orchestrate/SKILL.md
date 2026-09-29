---
name: orchestrate
description: "Automated bug bounty orchestration — runs the complete workflow from reconnaissance to reporting. Use when the user wants a fully automated bug bounty assessment."
---

# Bug Bounty Orchestrator

The orchestrator provides automated workflow management for bug bounty assessments.

## Usage

```
alphacode bugbounty orchestrate TARGET [OPTIONS]
```

## Options

- `--output <DIR>` — Output directory (default: bugbounty_output)
- `--silent` — Silent mode (less output)
- `--json` — JSON output
- `--dry-run` — Preview mode
- `--resume` — Resume previous run

## Workflow

The orchestrator runs 5 phases automatically:

### Phase 1: Reconnaissance
- Subdomain enumeration (subfinder, amass, assetfinder)
- DNS enumeration (dnsx)
- HTTP probing (httpx)

### Phase 2: Attack Surface Mapping
- Web crawling (katana)
- URL discovery (gau, waybackurls)
- Content discovery (feroxbuster)
- URL parsing (unfurl)

### Phase 3: Vulnerability Discovery
- Template scanning (nuclei)
- Web server scanning (nikto)
- XSS scanning (dalfox)
- CORS scanning (corsy)
- CRLF scanning (crlfuzz)

### Phase 4: Exploitation
- SQL injection (sqlmap)
- Directory brute force (gobuster)

### Phase 5: Reporting
- Comprehensive vulnerability documentation
- Severity classification
- Remediation recommendations

## Tool Chaining

The orchestrator chains tools intelligently:

```
subfinder → httpx → katana → nuclei
amass → httpx → feroxbuster → nuclei
gau → waybackurls → unfurl → qsreplace → dalfox
```

## Never-Miss Guarantee

Every task is tracked and completed:
- Each phase has explicit entry and exit criteria
- Tool results are validated before proceeding
- Failed tasks are retried with alternative approaches
- Progress is tracked and reported

## Output

The orchestrator produces:
- Structured JSON report
- Markdown summary
- Raw tool output
- Vulnerability details
- Remediation recommendations
