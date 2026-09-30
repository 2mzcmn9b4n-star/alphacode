# Changelog

All notable changes to Alphacode are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.66] - 2026-09-30

### Performance

- **Streaming markdown no longer deep-clones the whole rendered line tree twice
  per frame.** `IncrementalMarkdownRenderer` kept a second `Vec<Line>` mirror
  alongside its `Arc`, populated by a full deep clone on every re-render — and
  the re-render runs on *every streaming frame*, which is exactly where the line
  tree is largest. The mirror existed only to serve `last_lines()`, which had no
  callers. The renderer is now the single owner of its lines, `last_lines()`
  borrows from the `Arc`, and one full copy of the tree per frame (plus a
  permanently retained duplicate) is gone. This was the clearest
  cache-miss/clone defect in the TUI render path.
- **Composer undo history is bounded by bytes, not just by count.** Each
  snapshot is a full copy of the composer, so `INPUT_UNDO_LIMIT = 128` never
  bounded memory: one 3 MB paste followed by 128 keystrokes retained ~384 MB.
  Eviction is now oldest-first under an 8 MB budget, which keeps ordinary
  typing at the full 128 levels while capping the pathological case. The stack
  also became a `VecDeque`, so the per-keystroke eviction is O(1) instead of an
  O(128) `Vec::remove(0)` shift — the case that happens constantly during
  sustained typing. The retained byte total is now maintained incrementally,
  so eviction and `debug-profile` no longer re-sum the stack.
- **Provider request fingerprinting serializes each payload once instead of
  twice.** `log_provider_canonical_input` asked for a hash and a character count
  of the same value, and each answer ran its own `serde_json::to_string`. That
  value is a whole provider request — message array plus every tool schema — so
  the request, system prompt, and tool definitions were each serialized twice,
  on every API call, for every provider, to fill diagnostic fields. Both answers
  now come from one serialization, with identical output and identical
  error-fallback behaviour.
- **`bash` captures bounded output instead of buffering the whole pipe.** The
  command's stdout and stderr were read with `read_to_string` and only truncated
  to 30 KB afterwards, so a chatty command (`cargo build` on a cold target dir,
  `rg` across `target/`, a verbose test run) held every byte it produced, in both
  streams simultaneously, before the cap applied — unbounded memory for output
  that could never reach the model. The reader now keeps a bounded prefix and
  keeps draining to EOF. Draining past the cap is required, not an
  optimisation: stopping the read would fill the pipe buffer and block the child
  forever. The cap sits above the truncation point so `smart_stream` still sees
  the same input it used to. The detached/unix path, which read an entire
  background output file, is bounded the same way.

### Fixed

- **The transcript could not be scrolled up while the agent was working — the
  reported "I cannot scroll up" bug.** Two independent causes, both in the
  scroll path:
  - `handle_prompt_history_navigation` claimed an *unmodified* `Up`/`Down`
    whenever the composer was empty and the session had any prior prompt, and
    returned `true`, so the scroll handler was never reached. The composer is
    empty for essentially the whole time a turn runs, which is exactly when a
    reader wants to scroll back: pressing `Up` silently recalled a prompt into
    the composer instead of moving the view. An unmodified arrow on an empty
    composer now scrolls when there is somewhere to scroll, and still recalls
    history when the transcript is not scrollable. Deliberately narrow: a
    non-empty draft keeps walking history with the arrows, and every explicit
    recall chord (`Ctrl`/`Alt`/`Cmd` + `Up`/`Down`) is unchanged.
  - `scroll_up` clamped the new offset to `ui::last_max_scroll()`, the
    renderer's extent *as of the previous frame*. While the agent is streaming
    that value trails text that has already been appended, so the first
    scroll-up from the bottom was clamped straight back down to the bottom of
    the visible frame and the keystroke moved nothing. `scroll_down` already
    had streaming-aware ceiling logic; `scroll_up` did not. Both now share
    `chat_scroll_ceiling`, which only trusts the renderer's extent once the
    transcript is quiescent.
- **A leftover `PROBE` debug print was writing to stderr on every animation
  frame.** `render_idle_animation` contained an unconditional
  `eprintln!("PROBE render_idle_animation ...")`, on both the full-frame path
  and the animation-only partial repaint. At the decorative animation cadence
  that is up to 30 unbuffered, locked, `{:?}`-formatted stderr writes per second
  for the lifetime of an idle session, interleaving with the alternate screen.
- **`debug_repaint_probe_temp` was a test that always failed.** It duplicated
  the real `partial_repaint_matches_a_full_frame_at_the_same_animation_time`
  test above it and ended in `panic!("probe done")`, so it could never pass.
  Removed; the real test immediately above it covers the same behaviour with
  proper assertions.
- Duplicated doc/line comments left by an earlier merge on `draw_idle_animation`.
- **`app::tests::swarm_plan_graph_inline` was a guaranteed-red block on every
  default build.** All 29 tests in that file assert on the mermaid
  `ACTIVE_DIAGRAMS` registry, so each needs a plan-graph message to actually
  render a diagram — but diagram rendering is `#[cfg(feature =
  "mermaid-renderer")]`, and that feature is deliberately not in `default`. On a
  default build the seed helper got 0 active diagrams instead of 1 and the file
  failed. CI runs `cargo test --lib` with default features, so this was red on
  every build. The include is now gated on the feature, matching the pattern
  already used in `alphacode_tui_markdown::markdown_tests::cases::placeholders`.
  Run the full mermaid suite with
  `cargo test --lib --features mermaid-renderer swarm_plan_graph`.

### Changed

- `tests/perf_gates.rs` is now run in CI, and rewritten around
  hardware-independent **complexity gates**: each operation is measured at 400
  and 800 rows and the cost must not grow more than 2.5x when the input doubles.
  These fail identically on a laptop and on a shared CI runner, and they catch
  the regressions that actually occur — a lost cache, a `partition_point` that
  became a linear scan, a reintroduced `O(n^2)`. The previous wall-clock budgets
  (4 ms paint, 2 ms facet toggle, …) were tuned on one dev machine and are now
  measured and printed on every run but only *enforced* under
  `ALPHACODE_PERF_ABSOLUTE=1`. They were never executed in CI at all, so they
  could not have protected anything; a perf gate that fails for reasons
  unrelated to the code is worse than none, because it teaches reviewers to
  re-run instead of read.

### Added

- `alphacode bugbounty {doctor,install,list}` — first-class, cross-platform
  management of the recon toolchain. `doctor` reports present/missing binaries
  with the exact install command for each; `install` resolves a structured
  install command per binary, verifies the required package manager, runs the
  install under a hard timeout with `kill_on_drop`, and re-probes to confirm.
  Installation is never implicit — the agent reports missing tools and the
  command, a human runs the install.
- The doctor and installer now resolve `~/go/bin` (`%USERPROFILE%\go\bin`),
  which is not on `PATH` by default on Windows and often not on Unix. A
  successful `go install` was previously reported as "still missing".
- The recon tools now surface a concrete `alphacode bugbounty install <tool>`
  hint when a binary cannot be spawned, instead of a bare "not found".

### Fixed

- **Command-risk gate bypasses** (each classified `Safe` and would have run):
  - `LANG=C rm -rf ~` — a leading `VAR=value` assignment was mistaken for the
    program name.
  - `powershell -Command "Remove-Item -Recurse -Force $env:USERPROFILE"` and
    `cmd.exe /C "del /f /q ..."` — the Windows shells were not recognised as
    opaque script carriers, and the tool schema actively instructs the model
    to use them.
  - `rm -rf /c/Users/<user>`, `rm -rf C:/Windows` — the protected-path set was
    POSIX-only, so every absolute Windows path was unprotected, including the
    Git-Bash `/c/...` drive mount.
  - `rm -rf ~/.ssh/id_*` — a glob whose parent is protected was only escalated
    to the `Confirm` tier, which is allowed to execute.
  - `eval "rm -rf ~"` — `eval` was treated as a transparent wrapper, so the
    quoted script became the "program name".
  - `chroot /tmp/jail rm -rf ~`, `xargs -I {} rm -rf ~`, `su - root -c "..."` —
    the wrapper-unwrapper stops at any option value it does not recognise and
    left a non-program in the first slot, hiding the destructive verb.
  - Windows `basename` only split on `/`, so `C:\...\cmd.exe` never resolved.
- **Recon tools invoked flags that do not exist**, so they exited 2 on every
  call: `subfinder -threads` (it is `-t`), `dnsx -target` (it is `-d`),
  `waybackurls -limit` / `-no-color` (neither is registered at all).
- `httpx -status-code` is a *boolean probe*, so `-status-code 200,404` applied
  no filter and made httpx probe a host literally named `200,404`. It now uses
  `-match-code`. Also replaced the non-existent `-response-size`, `-tls` and
  `-chains` with `-content-length`, `-tls-grab` and `-include-chain`.
- `katana`: `-threads`, `-robots`, `-no-remote`, `-no-store`,
  `-include-body` and `-include-params` are not katana flags, and the `d`
  parameter collided with `-depth` (an int) while silently overwriting it.
  Replaced with `-c`, `-kf all`, `-crawl-scope`, and the correct inverses
  `-ob` / `-iqp`. The meaningless `headers: bool` became a real
  `Name: value` list.
- `waybackurls` `limit` was only echoed into metadata and never enforced; it
  is now applied locally and the true total is always reported.
- Every recon tool ran `.output()` with no timeout and no `kill_on_drop`, so a
  network stall pinned the agent turn forever and left orphans. All are now
  bounded and killed on expiry. `scrapling` additionally blocked a tokio
  worker with a synchronous, unbounded `python --version` probe.
- Recon output was buffered without a cap; a large `gau`/`ffuf`/`katana` run
  materialised hundreds of MB. Output is now line- and byte-capped, and
  truncation is reported explicitly so a capped result is never mistaken for a
  complete one.
- Failure messages were empty for most tools because goflags writes flag errors
  to **stdout**; `subfinder`, `httpx`, `katana`, `dnsx`, `ffuf` and `gau` now
  report a real diagnostic.
- Domain/target inputs are validated: a value beginning with `-` (e.g. `-t`,
  `-dates`) was silently parsed as a flag by the target tool, changing what ran
  without error.
- `webfetch` SSRF guard only understood four-part dotted-decimal IPs, so
  `127.1`, `0x7f.0.0.1`, `2130706433`, `0177.0.0.1`, `[::1]` and
  `::ffff:127.0.0.1` all reached internal services. It now parses every
  spelling the resolver accepts, and additionally resolves the hostname and
  rejects any address in a non-public range.
- `scrapling` reported "anti-bot detected" for any page whose text merely
  mentioned Cloudflare or captcha, burning a headless browser and stamping a
  false "content may be partial" note on real content.
- `jwt forge` with `alg: none` emitted `header.payload.` (trailing dot), which
  every strict verifier — including this tool's own decoder — rejects. The
  JWS unsecured form has no trailing dot.
- `jwt crack` silently fell back to 10 built-in secrets when the wordlist was
  unreadable and reported the result as authoritative; the degraded path is now
  called out so a failed crack is not mistaken for a strong secret.
- `timeout: 0` in `httpflow` / `webfetch` / `scrapling` was passed through as
  `Duration::from_secs(0)`, failing every request instantly.
- The `shell_url_safety` detector existed but was never called, so the Windows
  `&`-in-URL mangling warning it was written for was never produced.
- `bugbounty_doctor` was dead code and skipped relative `PATH` entries, reported
  non-executable files as installed, and missed `.cmd`/`.bat` shims.

### Fixed (security reasoning core)

- `mul_div_floor` divided by `d` twice, giving a 10,000× error in the LTV
  invariant. A fully-collateralised lending pool was reported as
  `DebtLteCollateralTimesLtv` violated — a permanent false positive.
- `evaluate_gates` compared only the *count* of gates, never that they were
  distinct, so twelve copies of one easy gate passed the entire 12-gate web3
  barrier.
- `should_stop` used `&&` where `||` belonged, so cost could never
  independently trigger a stop; any action with gain ≥ 0.25 continued
  regardless of cost. `ResourceLedger::pressure` also ignored the wall-clock
  budget that `exhausted()` enforces.
- `gate_security_relevance` failed *every* `VulnerabilityClass::Custom`, but
  web3 findings arrive exclusively as `Custom` — so no web3 finding could ever
  reach `Certain` confidence.
- `gate_reportability` never consulted the live scope verdict, so a finding on
  an explicitly excluded host passed all seven gates and landed at `Certain`.
- `confirmed_findings()` silently dropped every finding once it advanced to
  the `Report` stage, emptying the chain-analysis and report paths.
- `FalsePositiveDefense::is_still_viable` ignored whether any negative test had
  been run, so an untested finding read as maximally viable and an
  unexercised defense was promoted straight to `VerifiedReportable`.
- `LiveScope::set_verdict(InScope)` could not clear a sticky `out_of_scope`
  entry, permanently blocking a host re-confirmed as in scope.
- `CoverageTracker` left a stale `vuln_classes_with_signal` after a re-test
  refuted a finding, permanently recommending the endpoint for deeper testing.
- `SkillRouter::route` ignored host- and subdomain-level technologies, so the
  entire web3 routing path was skipped for exactly the targets most likely to
  need it.
- `Evidence::redacted()` did not redact `EvidenceData::Text` or `Binary`, so
  live credentials in raw HTTP transcripts shipped in exported reports. URL
  query redaction was also case-sensitive (`?Token=` slipped through).
- `VerifierVerdict::decide` made `InsufficientEvidence` unreachable and mapped
  "could not reproduce" to `Contradicted`, a hard refutation.
- `Action::ranked` and `HypothesisSet::most_urgent` iterated a `HashMap` with
  no tie-break, so they returned different results run-to-run despite
  documenting themselves as deterministic.

## [1.0.64] - 2026-09-26

### Fixed

- Prevented the health reporter from aborting AlphaCode on freshly booted systems when it constructed a 30-minute `Instant` cutoff.
- Hardened related resource-history and OpenRouter version-cache time handling against monotonic-clock underflow.
- Stopped the OpenRouter client version cache from leaking a new static string on every request and preserved the last valid version after a failed refresh.
- Fixed Firefox browser-bridge evaluation so `return <expr>`, top-level `await`, and legacy bridge calls are supported; upgraded the bundled extension to 1.6.0, moved installation to a versioned XPI path to avoid Windows file locks, and strengthened readiness diagnostics.
- Hardened reconnect bootstrap so a successful socket is not treated as a successful session attach until `SessionId`, `History`, and `Done` are correlated; added bounded timeouts, delayed/missing-session fault coverage, and terminal handling for invalid session targets.
- Fixed Windows named-pipe protocol flushing and added a detached server-reload handoff that waits for the predecessor before binding, preserving reload markers and recovering from dead predecessors.
- Added conservative tool execution classes and bounded concurrent execution for consecutive read-only batch calls while keeping mutations ordered.
- Added gateway bind/accept retry supervision and live runtime state reporting.
- Synchronized the committed lockfile with AlphaCode 1.0.63 so locked builds start reliably.

### Changed

- Pinned CI test/lint jobs to the repository's Rust 1.94.1 toolchain and enabled locked dependency resolution.
- Made release artifacts use the committed lockfile instead of mutating dependencies during publication.
- Added a product-wide 100× reliability, performance, accuracy, UX, tools, skills, security, and quality execution plan.

## [1.0.60] - 2026-09-23

- Initial release.
