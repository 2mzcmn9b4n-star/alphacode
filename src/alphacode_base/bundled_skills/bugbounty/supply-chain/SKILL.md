---
name: supply-chain
description: Source and dependency supply chain assessment — exposed VCS and environment files, leaked build artifacts, CI/CD configuration, infrastructure-as-code state, dependency confusion and typosquatting, and unpinned actions. Blackbox-first; scope-checked before any repository access.
---

# SUPPLY CHAIN ASSESSMENT

**Core:** the deployed application is the *output* of a build system. When you
cannot see source, the build system's own leaks are the highest-value
blackbox surface — they routinely contain credentials, internal hostnames, and
the dependency list.

---

## SCOPE GATE — READ FIRST

Testing a company **repository** is a different authorization surface from
testing a deployed application. Many programs scope only production hosts and
exclude source repositories, source-code review, and dependency analysis.

```
BEFORE any repository probe:
  - Is the repo/domain named in the scope file?
  - Does policy exclude "source code" / "repositories" / "open-source projects"?
  - Is the repo an OPEN-SOURCE project? If so it is public and in scope for
    everyone — but you are now reporting a vulnerability in a public project,
    which means a responsible disclosure path (SECURITY.md), not a bounty.

If the scope file is silent: report the exposure to the owner and let them
decide. Do not clone, build, or run anything.
```

**Open-source nuance:** a leak in a public repo is a real vulnerability, but it
belongs to that project's own disclosure process and its own bug bounty (if
any). Report it there, and mention it to the assessed program as context.

---

## PART 1 — EXPOSED SOURCE AND CONFIG (blackbox, highest yield)

```bash
for p in .git/HEAD .git/config .svn/entries .hg/requires .env .env.production \
         .env.local config.php config.yml settings.py app.config \
         .DS_Store Thumbs.db WEB-INF/web.xml .htaccess \
         backup.zip backup.tar.gz db.sql dump.sql site.tar.gz \
         composer.lock package-lock.json yarn.lock Pipfile.lock go.sum; do
  code=$(curl -so /dev/null -w "%{http_code}" "https://target.com/$p")
  [ "$code" = "200" ] && echo "200  $p"
done
```

```
.git/HEAD exposed -> full source, including .env and the full commit history.
                     This is a Critical finding on its own. Report the endpoint.
                     Recovering the repo is a separate engagement — ask first.

.env exposed       -> report WHICH variables are present and which look like
                     live credentials. Do not authenticate with them.
                     "Exposes DATABASE_URL, STRIPE_SECRET_KEY, JWT_SECRET in
                      plaintext at /.env" is a complete Critical report.
```

`.git/HEAD` returning 200 is a famous bug with a well-known fix, and it is
still routinely present. It is also the single highest-yield check in blackbox
bug bounty.

---

## PART 2 — CI/CD AND BUILD CONFIG

Exposed by the same class of checks, and often public by accident:

```
.github/workflows/*.yml     third-party actions pinned to a tag, not a SHA
.gitlab-ci.yml              secrets exposed in variable definitions, trigger rules
Jenkinsfile / .jenkins/     build steps, credential ids, deploy targets
Dockerfile / docker-compose.yml   base image versions, baked secrets, service map
package.json / build scripts       preinstall/postinstall, publish config
k8s manifests / helm charts        image tags, secret references, ingress rules
```

```
FINDING:  an action referenced as `uses: some/action@v3` rather than a pinned
          commit SHA — a tag is mutable, so whoever controls the upstream repo
          controls their CI. Medium, and genuinely useful.
FINDING:  a secret in a build config — Critical, report location + type.
FINDING:  a preinstall script that fetches a remote URL — supply-chain risk.
```

These are static-review findings on files that are *publicly served*. Reading
them is normal web reading. Running anything they contain is not.

---

## PART 3 — INFRASTRUCTURE AS CODE

```bash
terraform.tfstate     contains resource attributes, often in plaintext
*.tfplan              same
.terragrunt-cache/    same
Chart.lock / values.yaml
```

A `.tfstate` served publicly can contain database passwords, private IPs, and
the complete infrastructure graph. Report the endpoint and what classes of
data the state file holds. Do not enumerate the graph further than the report
requires.

---

## PART 4 — DEPENDENCY ECOSYSTEM

Only where a program explicitly permits dependency analysis. This is where
"don't scan third parties" matters most.

```
DEPENDENCY CONFUSION  a public package name matching an internal one; the
                       build resolves the public one. Requires the manifest to
                       be visible — usually only in a leaked .git.
TYPOSQUATTING         deliberate lookalike packages. This is *someone else's
                       registry*, not the assessed target. Out of scope unless
                       the program owns the package.
UNPINNED / ABANDONED   a dependency with no maintainer, or a version with a
                       known advisory and no upgrade path.
LOCKFILE DRIFT         lockfile present but not enforced, or differing between
                       environments.
```

**Registering a package under the target's namespace is publishing to
someone else's infrastructure.** Never do this. It is not authorized testing —
it is an intrusion into a third-party registry, and it can break their builds
for real users. Report the confusion condition; do not weaponize it.

---

## IMPACT BOUNDARY

```
.git/HEAD exposed   -> STOP. Do not clone the repo, do not read the history.
.env exposed        -> STOP. Do not authenticate with any credential in it.
Exposed .tfstate    -> STOP. Do not enumerate the infrastructure graph.
CI config exposed   -> read it as a file. Do not trigger builds, do not run
                      scripts, do not push to any referenced repo.
Dependency confusion-> report the condition. NEVER register the package.
```

The discipline is the same everywhere: **exposure is the finding, use is not
your engagement.** A credential in a report is a liability for the owner and a
crime for you.

---

## SEVERITY

```
.git or source repo exposed publicly                     -> Critical
.env / .tfstate / build config with live secrets         -> Critical
CI/CD able to deploy, exposed                            -> Critical
Dependency confusion on an internal-scoped package      -> High
Self-hosted registry with anonymous push                 -> High
Unpinned CI action (mutable tag)                         -> Medium
Leaked internal hostnames via config                     -> Low-Medium
Lockfile hygiene, stale dependency without CVE           -> Low
```

Use `severity-engine`. Supply chain findings often score High on
`AC:L`/`AT:P` because the "attack requirements" genuinely are a separate
compromise — be honest about that rather than scoring as though the leak alone
granted access. Honest nuance here reads as rigor, not weakness.
