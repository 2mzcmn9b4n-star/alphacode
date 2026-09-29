//! Deterministic risk classification for shell commands.
//!
//! # Why this exists
//!
//! alphacode executes `bash` tool calls with no gate of its own: the only check in
//! `ToolRegistry::execute` is an opt-in external `pre_tool` hook, which is off
//! by default. A model that decides to run `rm -rf ~` is obeyed immediately.
//! That is issue #604, where a user lost their home directory.
//!
//! # Design
//!
//! This crate is **stage 1** of a two-stage cascade: a cheap, deterministic,
//! high-recall filter. It never calls a model and never touches the network, so
//! it costs nothing on the overwhelmingly common safe path. Stage 2 (the
//! reflection gate) only runs when this returns something other than
//! [`RiskLevel::Safe`].
//!
//! Two deliberate choices:
//!
//! 1. **Classify by blast radius, not by command name.** A denylist of
//!    `rm -rf` misses `find -delete`, `shred`, `truncate`, `dd`, and `>file`.
//!    We ask "what would this destroy, and can it be undone" instead.
//! 2. **Bias hard toward recall.** A false positive costs one reflection turn.
//!    A false negative costs a home directory. When parsing is ambiguous we
//!    escalate rather than allow.
//!
//! # Honest limitations
//!
//! This is defense in depth, not a sandbox. A determined or unlucky
//! `sh -c "$(printf ...)"` can defeat any static parser, which is exactly why
//! [`RiskLevel::Confirm`] is a reflection prompt rather than a hard block, and
//! why the catastrophic tier is a small, absolute, path-based deny that does
//! not depend on parsing the command correctly.

mod gate;
mod paths;
mod shell_url_safety;
mod tokenize;

// `paths_tests` / `tokenize_tests` are attached to their own modules (they use
// `super::` to reach private helpers), so they are not registered here.
#[cfg(test)]
#[path = "bypass_tests.rs"]
mod bypass_tests;

pub use gate::{GateOutcome, Justification, gate};
pub use paths::{ProtectedPaths, is_catastrophic_target};
#[allow(unused_imports)]
pub use shell_url_safety::{ShellUrlSafety, scan_for_shell_url_issues};
pub use tokenize::{Token, tokenize};

/// How dangerous a command looks, and therefore how much scrutiny it earns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// No destructive potential detected. Run immediately, no overhead.
    Safe,
    /// Destructive but bounded (inside the working directory, recoverable via
    /// git, or under a temp dir). Run, but record it.
    Low,
    /// Irreversible and reaches outside the working directory. Requires the
    /// model to re-justify against the user's actual request before running.
    Confirm,
    /// Would destroy the user's home, root, or credentials. Never runs, and no
    /// amount of model justification can unlock it.
    Catastrophic,
}

impl RiskLevel {
    /// Whether execution may proceed without a reflection turn.
    pub fn runs_immediately(self) -> bool {
        matches!(self, RiskLevel::Safe | RiskLevel::Low)
    }

    /// Whether any confirmation could ever unlock this.
    pub fn is_absolute_deny(self) -> bool {
        matches!(self, RiskLevel::Catastrophic)
    }
}

/// A specific reason a command was flagged, used to explain the refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskFinding {
    pub level: RiskLevel,
    /// Human-readable explanation, shown to the model verbatim.
    pub reason: String,
    /// The concrete path or argument that triggered this, when there is one.
    pub target: Option<String>,
}

/// The full verdict for one command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub findings: Vec<RiskFinding>,
}

impl RiskAssessment {
    fn safe() -> Self {
        Self {
            level: RiskLevel::Safe,
            findings: Vec::new(),
        }
    }

    fn from_findings(findings: Vec<RiskFinding>) -> Self {
        let level = findings
            .iter()
            .map(|f| f.level)
            .max()
            .unwrap_or(RiskLevel::Safe);
        Self { level, findings }
    }

    /// The refusal text shown to the model, phrased to force a comparison
    /// against what the user actually asked for rather than a yes/no reflex.
    pub fn explanation(&self) -> String {
        let mut out = String::new();
        for finding in &self.findings {
            out.push_str("- ");
            out.push_str(&finding.reason);
            if let Some(target) = &finding.target {
                out.push_str(&format!(" (target: {target})"));
            }
            out.push('\n');
        }
        out
    }
}

/// Context needed to judge blast radius. Supplied by the caller because this
/// crate deliberately does no I/O of its own beyond path inspection.
#[derive(Debug, Clone, Default)]
pub struct RiskContext {
    /// The tool call's working directory, if any.
    pub working_dir: Option<std::path::PathBuf>,
    /// The user's home directory.
    pub home_dir: Option<std::path::PathBuf>,
}

impl RiskContext {
    pub fn from_env(working_dir: Option<std::path::PathBuf>) -> Self {
        Self {
            working_dir,
            home_dir: dirs_home(),
        }
    }
}

/// Resolve the user's home directory.
///
/// `$HOME` wins when set so a caller can override it (and so the Unix shell's
/// own notion of `~` is honoured), but it must not be the only source: on
/// Windows `HOME` is normally unset and the home directory lives in
/// `USERPROFILE`. Falling through to `dirs::home_dir()` — the idiom used
/// everywhere else in this codebase — keeps the catastrophic tier armed there.
/// Without the fallback `home_dir` is `None` on Windows, and every home-path
/// protection (`~`, `~/.ssh`, `~/.aws`, `~/.gnupg`) silently degrades to
/// "not catastrophic".
fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(dirs::home_dir)
}

/// Case-insensitive membership test for [`DESTRUCTIVE_COMMANDS`].
///
/// Windows command names are case-insensitive and PowerShell cmdlets are
/// conventionally PascalCase (`Remove-Item`), so a case-sensitive lookup left
/// every PowerShell destructive cmdlet unmatched — the exact class of command
/// the bash tool's own schema tells the model to use.
fn is_destructive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    // `mkfs.ext4`, `wipefs.sha256`, `del.exe` are the real program names on
    // disk; the bare stem is what the list holds.
    let stem = lower.split_once('.').map(|(s, _)| s).unwrap_or(&lower);
    DESTRUCTIVE_COMMANDS.contains(&lower.as_str()) || DESTRUCTIVE_COMMANDS.contains(&stem)
}

/// Commands that destroy data as their primary purpose.
///
/// Presence here does not by itself mean danger: `rm` inside the working
/// directory is routine. It means "inspect the targets".
const DESTRUCTIVE_COMMANDS: &[&str] = &[
    // POSIX. Matched on the stem too, so `mkfs.ext4` and `wipefs.sha256`
    // resolve to their `mkfs` / `wipefs` entries.
    "rm",
    "rmdir",
    "shred",
    "unlink",
    "truncate",
    "dd",
    "mkfs",
    "fdisk",
    "parted",
    "wipefs",
    "srm",
    // Windows commands. `del`/`erase` remove files, `rd` removes trees,
    // `format`/`diskpart` destroy volumes. These were missing entirely, so
    // `del /f /s /q C:\Users\me\Documents` classified as Safe on the platform
    // where the gate matters most.
    "del",
    "erase",
    "rd",
    "format",
    "diskpart",
    "cipher",
    "fsutil",
    "del.exe",
    // PowerShell cmdlets. `Remove-Item -Recurse -Force` is `rm -rf`, and the
    // bash tool's own schema tells the model to reach for
    // `powershell -Command '...'`, so omitting these left the documented
    // Windows path completely unguarded.
    "remove-item",
    "clear-content",
    "set-content",
    "out-file",
    "new-item",
    "format-volume",
    "initialize-disk",
    "remove-partition",
    "clear-disk",
    "remove-vm",
    "stop-vm",
    "set-mppreference",
    "takeown",
];

/// Commands that run another command. The real program is one of their
/// arguments, so `sudo rm -rf ~` must be unwrapped before classification or the
/// destructive verb is never seen at all.
///
/// `eval` is deliberately absent: it is not a transparent wrapper but a
/// *string* evaluator, so `eval "rm -rf ~"` must be handled by the
/// opaque-shell path instead. Listing it here consumed the token and left the
/// quoted script as the "program name", which matched nothing.
const WRAPPER_COMMANDS: &[&str] = &[
    "sudo", "doas", "env", "nice", "ionice", "time", "timeout", "nohup", "xargs", "command",
    "builtin", "exec", "setsid", "stdbuf", "chroot", "su", "watch",
];

/// Wrapper options that consume the following word as their value.
const WRAPPER_FLAGS_WITH_VALUES: &[&str] = &[
    "-n",
    "-u",
    "-s",
    "-c",
    "-k",
    "--signal",
    "--adjustment",
    "--user",
];

/// Shells, which take their program from a string argument we cannot parse
/// reliably. Treated as opaque rather than assumed safe.
///
/// The Windows shells belong here too. `powershell -Command "Remove-Item
/// -Recurse -Force $env:USERPROFILE"` was previously invisible to this gate
/// (and the tool schema actively *instructs* the model to reach for
/// `powershell -Command` / `cmd.exe /C`), so it is a bypass on the platform
/// where it is most dangerous.
const SHELL_COMMANDS: &[&str] = &[
    "sh",
    "bash",
    "zsh",
    "dash",
    "ksh",
    "fish",
    // Windows shells + string evaluators.
    "powershell",
    "powershell.exe",
    "pwsh",
    "pwsh.exe",
    "cmd",
    "cmd.exe",
    "eval",
    "source",
];

/// Whether `name`, appearing in *program position*, is a shell or string
/// evaluator whose real program cannot be parsed reliably.
///
/// `.` (the POSIX `source` builtin) is handled here rather than in
/// [`SHELL_COMMANDS`] because that list is also consulted in *argument*
/// position by the hidden-verb scan. With `.` in the list, every bare `.`
/// argument matched, so `ls .`, `ls -la .`, `find . -type f`, `grep -r foo .`
/// and `cargo fmt .` were all hard-denied as `Catastrophic` — among the most
/// common commands an agent emits. As a program name, `. script.sh` is still
/// correctly treated as opaque.
fn is_shell_program(name: &str) -> bool {
    SHELL_COMMANDS.contains(&name) || name == "."
}

/// Commands that are destructive only with specific flags.
const CONDITIONALLY_DESTRUCTIVE: &[(&str, &[&str])] = &[
    ("find", &["-delete", "-exec"]),
    ("git", &["clean"]),
    ("chmod", &["-R"]),
    ("chown", &["-R"]),
];

/// Whether a token is a `NAME=value` environment assignment rather than a word.
///
/// Requires a valid shell identifier before the `=`. A loose `contains('=')`
/// would also swallow legitimate path arguments like `a=b.txt` or
/// `?filter=name`, hiding the real program.
fn is_assignment(text: &str) -> bool {
    let Some((name, _)) = text.split_once('=') else {
        return false;
    };
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().expect("name is non-empty");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Drop leading `VAR=value` tokens from a segment.
fn strip_leading_assignments(mut tokens: &[Token]) -> &[Token] {
    while let Some(first) = tokens.first() {
        if is_assignment(&first.text) {
            tokens = &tokens[1..];
        } else {
            break;
        }
    }
    tokens
}

/// Assess a single shell command string.
///
/// This is the crate's entry point and is intentionally total: any input,
/// including garbage, produces an assessment rather than an error.
pub fn assess(command: &str, ctx: &RiskContext) -> RiskAssessment {
    let mut findings = Vec::new();

    for segment in tokenize::split_segments(command) {
        assess_segment(&segment, ctx, &mut findings);
    }

    if findings.is_empty() {
        return RiskAssessment::safe();
    }
    RiskAssessment::from_findings(findings)
}

fn assess_segment(tokens: &[Token], ctx: &RiskContext, findings: &mut Vec<RiskFinding>) {
    // Leading `VAR=value` assignments set the environment for the command that
    // follows and are not the command. They must be stripped *before* the
    // wrapper loop: the loop only skipped them after a wrapper program, so a
    // segment that begins with an assignment never got unwrapped and its
    // `program_name` became the assignment itself. `LANG=C rm -rf ~` therefore
    // classified as Safe.
    let mut tokens = strip_leading_assignments(tokens);

    // Strip wrapper programs (`sudo`, `env`, `xargs`, ...) so the destructive
    // verb underneath is the one we classify. Without this, any common prefix
    // is a complete bypass.
    let mut wrapped_by: Option<String> = None;
    loop {
        let Some(first) = tokens.first() else {
            // Ran off the end while unwrapping: the payload is invisible.
            if let Some(wrapper) = wrapped_by {
                findings.push(RiskFinding {
                    level: RiskLevel::Confirm,
                    reason: format!(
                        "`{wrapper}` runs another command that could not be \
                         identified statically"
                    ),
                    target: None,
                });
            }
            return;
        };
        let name = first.basename();
        if !WRAPPER_COMMANDS.contains(&name.as_str()) {
            break;
        }
        wrapped_by = Some(name);
        // Skip the wrapper plus its own options and `VAR=value` assignments,
        // landing on the wrapped program. Options that take a separate value
        // (`nice -n 10`, `timeout 5`) must consume that value too.
        let rest = &tokens[1..];
        let mut idx = 0;
        while idx < rest.len() {
            let token = &rest[idx];
            if token.is_operator || is_assignment(&token.text) {
                idx += 1;
                continue;
            }
            if token.is_flag() {
                idx += 1;
                // A short flag known to take an argument consumes the next word.
                if WRAPPER_FLAGS_WITH_VALUES.contains(&token.text.as_str()) && idx < rest.len() {
                    idx += 1;
                }
                continue;
            }
            // A bare number is an operand of the wrapper itself (`timeout 5`),
            // not the program to run.
            if token.text.chars().all(|c| c.is_ascii_digit() || c == '.') {
                idx += 1;
                continue;
            }
            break;
        }
        tokens = &rest[idx..];
    }
    let Some(program) = tokens.first() else {
        // A wrapper with nothing recognizable after it hides its payload.
        if let Some(wrapper) = wrapped_by {
            findings.push(RiskFinding {
                level: RiskLevel::Confirm,
                reason: format!(
                    "`{wrapper}` runs another command that could not be \
                     identified statically"
                ),
                target: None,
            });
        }
        return;
    };
    let program_name = program.basename();

    // `mkfs.ext4`, `mkfs.xfs`, `mke2fs`, `wipefs.sha256` etc. are the real
    // program names on disk — the bare stem in the list never matches, so
    // `mkfs.ext4 /dev/sda1` classified as Safe. `is_destructive_name` also
    // lowercases, because Windows command names are case-insensitive and
    // PowerShell cmdlets are conventionally PascalCase.
    let is_destructive = is_destructive_name(&program_name);

    // Defense in depth for wrappers whose option grammar we could not parse.
    // The unwrap loop stops at the first word it does not recognise as a
    // wrapper operand, so `chroot /tmp/jail rm -rf ~`, `su - root -c "..."` and
    // `xargs -I {} rm -rf ~` all leave a *non-program* in slot 0 and hide the
    // destructive verb behind it. Rather than trying to model every wrapper's
    // flag grammar, scan the whole segment for a known destructive verb; if one
    // is present the segment is unsafe no matter what we think runs first.
    if !is_destructive && !is_shell_program(&program_name) {
        let hidden = tokens.iter().find_map(|t| {
            // A quoted argument can itself be a whole command string
            // (`su - root -c "rm -rf ~"`), so the scan has to look *inside*
            // each token as well as at the token itself.
            let mut words: Vec<String> = t.text.split_whitespace().map(str::to_string).collect();
            words.push(t.basename());
            for name in words {
                let name = name.rsplit(['/', '\\']).next().unwrap_or(&name).to_string();
                if is_destructive_name(&name) || SHELL_COMMANDS.contains(&name.as_str()) {
                    return Some(name);
                }
            }
            None
        });
        if let Some(name) = hidden {
            findings.push(RiskFinding {
                level: RiskLevel::Catastrophic,
                reason: format!(
                    "`{name}` appears inside a `{program_name}` invocation whose \
                     real command line could not be resolved statically"
                ),
                target: None,
            });
            return;
        }
    }

    // A shell invoked with an inline script is opaque to this parser. Assess
    // the script text too, so `sh -c "rm -rf ~"` is not a free pass.
    if is_shell_program(&program_name) {
        for token in tokens.iter().skip(1).filter(|t| !t.is_flag()) {
            for segment in tokenize::split_segments(&token.text) {
                assess_segment(&segment, ctx, findings);
            }
        }
        return;
    }

    // `is_destructive` (including the `mkfs.ext4`-style stem match) is computed
    // once, above, before the shell handling.
    let conditional_flags = CONDITIONALLY_DESTRUCTIVE
        .iter()
        .find(|(name, _)| *name == program_name)
        .map(|(_, flags)| *flags);

    let triggered = if is_destructive {
        true
    } else if let Some(flags) = conditional_flags {
        tokens.iter().any(|t| flags.contains(&t.text.as_str()))
    } else {
        false
    };

    // Output redirection truncates a file even with a harmless program.
    let redirect_targets: Vec<&Token> = tokens
        .iter()
        .filter(|t| t.is_truncating_redirect_target)
        .collect();

    if !triggered && redirect_targets.is_empty() {
        return;
    }

    let mut targets: Vec<&Token> = tokens
        .iter()
        .skip(1)
        .filter(|t| !t.is_flag() && !t.is_operator)
        .collect();
    targets.extend(redirect_targets.iter().copied());

    // A destructive command fed by a pipe takes its operands from the previous
    // command's output, which we cannot enumerate. `find ~ -type f | xargs rm`
    // is a real deletion of home contents that neither segment reveals on its
    // own, so escalate rather than trust the visible arguments.
    if triggered && tokens.first().is_some_and(|t| t.receives_pipe) {
        findings.push(RiskFinding {
            level: RiskLevel::Confirm,
            reason: format!(
                "`{program_name}` deletes paths supplied by a pipe, so the set \
                 of affected files cannot be checked before it runs"
            ),
            target: None,
        });
    }

    // A destructive program with no parsable target is more suspicious, not
    // less: we could not see what it would touch.
    if triggered && targets.is_empty() {
        findings.push(RiskFinding {
            level: RiskLevel::Confirm,
            reason: format!(
                "`{program_name}` is destructive but its target could not be \
                 determined statically, so its blast radius is unknown"
            ),
            target: None,
        });
        return;
    }

    let recursive = tokens.iter().any(|t| t.is_recursive_flag());

    for target in targets {
        // `dd`-style `key=value` operands hide the path from a naive scan.
        let raw = target
            .text
            .split_once('=')
            .filter(|(key, _)| matches!(*key, "of" | "if" | "seek" | "conv"))
            .map(|(_, value)| value)
            .unwrap_or(&target.text);
        let expanded = paths::expand(raw, ctx);
        if let Some(finding) = paths::classify_target(&expanded, raw, recursive, ctx) {
            findings.push(finding);
        }
    }
}
