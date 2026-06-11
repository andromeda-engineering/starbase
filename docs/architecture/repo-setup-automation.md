# Retrospective: Automating Open-Source Repo Setup (`neon repo`)

> Status: Design note + retrospective. Written after bootstrapping `neonwave-dev/neon-os`
> Phase 0 by hand (scaffold → push → harden). Captures what we did, what bit us, and how to
> turn the whole thing into `neon repo init` + `neon repo harden` (or a standalone script).

---

## 1. Purpose

Bringing up a production-ready OSS repo is a ~30–45 min checklist of file scaffolding plus a
long tail of GitHub settings, security toggles, branch protection, labels, and bot wiring.
Most of it is mechanical and **API-automatable**; a small residue genuinely requires a human
(app installs, npm 2FA). This doc defines the split and the order so NeonOS can collapse the
process to ~3–5 min + a couple of clicks.

Two commands:

- **`neon repo init`** — generate the scaffold (files, workspace, CI, docs, templates) and push.
- **`neon repo harden`** — apply everything that lives in GitHub settings via the API, in the
  correct order, and emit a checklist of the few remaining manual actions.

---

## 2. What Phase 0 actually produced

A hybrid **Turborepo (TS/JS) + Cargo (Rust)** monorepo, pushed to a public repo with
Community Standards at **100%**: README, MIT LICENSE, CONTRIBUTING, CODE_OF_CONDUCT
(Contributor Covenant v2.1), SECURITY, CHANGELOG, PR template, issue templates, Dependabot,
CodeRabbit config, and green CI (`Rust` + `TypeScript` jobs). Stub units only — no product code.

That part was clean. The lessons are mostly in the seams.

---

## 3. Retrospective — what bit us (and the fix)

These are the failure modes a naive generator hits. Each is now a hard requirement for the
`init`/`harden` implementation.

### 3.1 The scaffold spec was not CI-green as written
A plausible-looking spec failed CI in five places. `neon repo init` must bake these in, not
re-discover them:

| Symptom | Root cause | Fix baked into the template |
|---|---|---|
| `Cannot use import statement outside a module` | `eslint.config.js` is ESM but root `package.json` had no `"type": "module"` | root `package.json` declares `"type": "module"` |
| `Cannot find name 'console'` | `lib: ["ES2022"]` + no Node types | add `@types/node`; tsc auto-includes it |
| `Cannot find module '@scope/pkg'` during typecheck | consumer typechecks before the dep is built; resolves to a not-yet-emitted `.d.ts` | `turbo` `typecheck` → `dependsOn: ["^build"]` |
| Lint fails on `dist/**` | flat-config `ignores: ["dist"]` doesn't match **nested** package build output | use glob form `**/dist/**`, `**/target/**`, etc. |
| TS CI job invokes `cargo clippy` | `lint` script chained `turbo lint && cargo clippy`; the TS-only path inherited cargo | split a cargo-free `lint:ts`; clippy runs **only** in the Rust job |

**Principle:** the generator must run the full `check` matrix in a clean clone before declaring
success — never trust the template, verify against a real toolchain (or CI).

### 3.2 `gh repo create` defaults the remote to SSH; push then fails in headless envs
`gh repo create --source . --push` created the repo fine but pushed over `git@github.com:` and
hit `Permission denied (publickey)` because no SSH key was loaded in the agent environment.

Two valid resolutions, and `harden` should support both:
- **HTTPS + token helper** (CI/headless): `gh auth setup-git` + `git remote set-url origin https://…`.
- **Per-account SSH host alias** (the user's real setup): rewrite origin to
  `git@github-<account>:<owner>/<repo>.git` and set local `user.name`/`user.email` from an
  identity map (`~/.config/git/identities.ps1`). This is what `Set-LocalGitIdentity` does.

**Principle:** never assume the ambient git transport works. Detect, and pick the right
credential path explicitly.

### 3.3 Branch protection has an ordering trap
Requiring status checks that **don't exist yet** blocks every PR forever (GitHub waits on a
check that never reports). Likewise, "split CI into granular jobs" *renames* the checks, so
protection must reference the **actual** job names.

**Principle:** protection is applied **last** — only after the new workflows have produced their
check runs at least once. `harden` must sequence: push workflows → wait for first green →
read real check names → apply protection.

### 3.4 Solo-repo approval is self-contradictory
"Require 1 approval" on a single-maintainer repo locks the owner out (you can't approve your own
PR). Resolution chosen: **require 1 approval but allow admin bypass** (`enforce_admins=false`).

**Principle:** repo topology (solo vs team) is an input, not an assumption. Default to admin
bypass until a second maintainer exists.

### 3.5 `branch_name_pattern` is org-only — user repos can't enforce naming server-side
Enforcing the branch-naming convention (the `branch-pattern` regex) as a ruleset rule
(`branch_name_pattern`, one of GitHub's **metadata restriction** rules) fails on a
**personally-owned** repo. The API returns `422 Validation Failed — Invalid rule
'branch_name_pattern'` for *any* pattern (even a trivial `starts_with`), because metadata
restriction rules are an **organization-only** feature. At bootstrap time the repo was
personally-owned (`owner.type: User`), so there was no server-side path — not via API, not via
the Settings → Rules UI. (The repo has since moved to the `neonwave-dev` org, which makes the
ruleset path available.)

Substitutes, in order of preference:
- **CI guard** (works on user repos): a `Branch Name` workflow that validates `github.head_ref`
  against the convention regex on every PR and fails non-conforming branches. Bots
  (`dependabot/*`, `changeset-release/*`) are exempt. This is `.github/workflows/branch-name.yml`.
  Optionally promote it to a *required* status check once it has run green once.
- **Client-side**: the `branch-pattern` skill already blocks bad names at `git checkout -b` time.
- **Transfer to an org**: only then does the `branch_name_pattern` ruleset rule become available.

**Principle:** `harden` must branch on `owner.type`. For `User` repos, emit the CI guard instead
of a `branch_name_pattern` ruleset; reserve the ruleset path for `Organization` repos.

### 3.6 Defaults that surprise
- **Secret scanning + push protection**: auto-ON for public repos (free). No action needed.
- **Dependabot _alerts_ and _security updates_**: **OFF by default** even when `dependabot.yml`
  exists — the version-update config and the security features are independent. Must be enabled
  via API.
- **Dependabot fires immediately**: the weekly config opened 7 update PRs within minutes,
  including **major** bumps (eslint 10, typescript 6) that can break a pinned flat-config. Group
  and/or schedule to avoid day-0 noise.
- **Projects** tab is ON by default.

---

## 4. The automation model

```
neon repo init   ──▶  scaffold files + workspace + CI + docs        (template + verify)
                      git init → commit → create remote → push
                              │
neon repo harden ──▶  GitHub settings via API, ORDERED:
                      1. repo settings (topics, merge, projects, wiki)
                      2. security (alerts, security-updates, PVR)
                      3. labels + issue-template extras
                      4. workflows already pushed by init (CodeQL/Scorecard/Release)
                      5. WAIT for first green check run
                      6. branch protection → required checks = real names
                      7. open `docs:` validation PR
                              │
                      ──▶  emit MANUAL checklist (CodeRabbit app, npm 2FA)
```

**Idempotency is mandatory.** `harden` must be safe to re-run: every step is a PUT/PATCH or a
"create-if-absent". Use `gh api` with `PUT` (naturally idempotent) and check-before-create for
labels/templates. Re-running should converge, never duplicate or error.

---

## 5. Phase → mechanism map

Legend: **A** = fully automatable (API/commit) · **M** = manual (human only) · **D** = decision input.

| Phase | Action | Class | Mechanism (verified `gh`/API) |
|---|---|---|---|
| Settings | description, topics, homepage | A | `gh repo edit --description/--add-topic/--homepage` |
| Settings | wiki off, squash-only, projects off | A | `gh repo edit --enable-wiki=false --enable-squash-merge --enable-merge-commit=false --enable-rebase-merge=false --enable-projects=false` |
| Settings | auto-delete head branches | A | `gh repo edit --delete-branch-on-merge` |
| Settings | Discussions | D/A | `gh repo edit --enable-discussions` (community-project toggle) |
| Settings | Sponsorships / `FUNDING.yml` | M | GitHub Sponsors enrollment |
| Security | Dependabot **alerts** | A | `PUT /repos/{o}/{r}/vulnerability-alerts` |
| Security | Dependabot **security updates** | A | `PUT /repos/{o}/{r}/automated-security-fixes` |
| Security | dependency graph | A | on by default (public) |
| Security | secret scanning + push protection | A | on by default (public); else `PATCH /repos` `security_and_analysis` |
| Security | private vulnerability reporting | A | `PUT /repos/{o}/{r}/private-vulnerability-reporting` |
| Security | CodeQL | A | commit `codeql.yml` (advanced) **or** `PATCH …/code-scanning/default-setup` |
| Security | OpenSSF Scorecard | A | commit `scorecard.yml` (ossf/scorecard-action) |
| Bots | CodeRabbit config | A | commit `.coderabbit.yaml` |
| Bots | **CodeRabbit app install** | **M** | GitHub App — no API; one-time per account/repo |
| Bots | Dependabot groups / ignore | A | edit `dependabot.yml` `groups:` |
| Branch | protection / ruleset | A* | `PUT /repos/{o}/{r}/branches/main/protection` (*after checks exist) |
| Branch | required checks = real job names | A | read from check-runs after first run |
| Labels | type/priority/status taxonomy | A | `gh label create … --force` (idempotent) |
| Issues | doc-request template, security redirect | A | commit `documentation.yml` + `contact_links` in `config.yml` |
| Releases | changesets + release workflow | A | add `@changesets/cli`, `.changeset/config.json`, release action (publish gated) |
| npm | metadata / provenance / 2FA / access | M/D | npm login, `npm access`, `--provenance` in release; only when publishing |
| Docs | README badges, SUPPORT.md | A | commit |
| Community | Standards 100% | A | falls out of the doc set; verify `GET /repos/{o}/{r}/community/profile` |
| Validation | `docs:` test PR | A | `gh pr create`; CodeRabbit review part depends on the app (M) |

---

## 6. Hard sequencing constraints

1. **Files before settings.** Workflows must be on `main` before their checks can be required.
2. **First green before protection.** Poll the check-runs API for the head SHA; only then read
   the real check names and apply protection. (Don't use `gh pr checks` for this — it falsely
   reports "no checks" for branch-only workflows; hit the check-runs API directly.)
3. **Protection is the last mutating step**, because splitting/renaming CI jobs changes the
   names protection must reference.
4. **Validation PR after everything**, so it exercises the finished pipeline.

---

## 7. What must stay configurable (inputs, not constants)

| Input | Options | Effect |
|---|---|---|
| Topology | solo / team | solo ⇒ admin-bypass on protection; team ⇒ strict approvals |
| Visibility | public / private | public ⇒ free secret scanning; private ⇒ Renovate over Dependabot, paid security features |
| Publish target | none / changesets-only / npm-now | gates Phase 6 + release workflow + provenance |
| Merge profile | library (squash) / app (squash+rebase) | merge-strategy toggles + linear-history |
| Languages | TS / Rust / both | which CI jobs, CodeQL languages, Dependabot ecosystems |

The generator should take a small config object (a `neon.repo.toml` or prompts) and derive the
rest. OSS + solo + both-languages + defer-publish was the profile used for neon-os.

---

## 8. Reference implementation

### 8.1 Standalone script (today, no Rust needed)
A thin wrapper over `gh` + `git` is enough to ship `harden` immediately:

- **PowerShell** (`Invoke-RepoHarden.ps1`) for the Windows-first workflow, reusing the existing
  `~/.config/git/identities.ps1` identity map and `Set-LocalGitIdentity` for the SSH path.
- **bash** (`repo-harden.sh`) for CI/Linux, using the HTTPS+token git path.

Shape:
```
harden <owner/repo> [--profile oss-solo] [--publish none|changesets|npm]
  └─ gh repo edit …                         # settings
  └─ gh api -X PUT …/vulnerability-alerts    # security (idempotent PUTs)
  └─ gh api -X PUT …/automated-security-fixes
  └─ gh api -X PUT …/private-vulnerability-reporting
  └─ ensure-labels (create --force)          # idempotent
  └─ wait-for-green <sha>                     # poll check-runs API
  └─ gh api -X PUT …/branches/main/protection # required_status_checks = discovered names,
                                              #  required_pull_request_reviews{1, dismiss_stale},
                                              #  enforce_admins=false, linear history, no force/del
  └─ gh pr create docs: validation
  └─ print MANUAL: [CodeRabbit app, npm 2FA]
```

Each step wrapped so a 4xx on an already-applied setting is a no-op, not a failure.

### 8.2 NeonOS-native (`neon repo`, Rust)
When the CLI exists, fold the script into the Rust engine:
- A **`RepoProfile`** struct (the §7 inputs) → drives a declarative plan.
- A **GitHub client** (octocrab or raw REST) executing the §5 actions.
- A **planner/executor** that prints the plan, applies idempotently, and reconciles on re-run
  (`neon repo harden` = converge to desired state, like `terraform apply` for repo settings).
- A **manual-residue reporter**: the two or three things only a human can do, surfaced as a
  checklist with deep links.
- Reuse the same template engine as `neon repo init`, with the §3.1 fixes as non-negotiable
  defaults and a post-scaffold `verify` gate (clean clone + full `check`).

This is a natural fit for NeonOS's stated goals (repo setup, repeatable workflows) and is the
first concrete `neon repo` surface beyond `init`/`doctor`.

---

## 9. The irreducible manual residue

After `init` + `harden`, exactly these remain human-only:

1. **Install the CodeRabbit GitHub App** (one-time, per account/repo). Config is already committed.
2. **npm publishing prerequisites** *(only when publishing)*: scope ownership, enable 2FA,
   provenance, public access.
3. **Triage the first wave of Dependabot PRs** — major bumps need human judgment, not auto-merge.
4. **Final eyeball** of Settings → Branches/Security.

Everything else is automatable today.

---

## 10. Open questions

- CodeQL **Rust** support maturity — ship JS/TS now, add Rust when stable; gate by language input.
- Default-setup CodeQL (API toggle) vs committed advanced workflow — advanced gives a named,
  protectable check and lives in-repo; prefer it for the `protection` story.
- Whether `harden` should also **auto-merge safe Dependabot patch PRs** (opt-in), or always defer
  to a human (current stance).
- One identity-map format across PowerShell and the Rust CLI, so SSH host aliases are shared.
