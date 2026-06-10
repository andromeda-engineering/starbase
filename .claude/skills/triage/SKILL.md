---
name: triage
description: >
  Decide what to work on next on NeonOS by reading the project's planning state in Linear
  (NeonWave / NEO team, NeonOS project) and the repo's recent PRs, then ranking and selecting
  the next tasks — optionally fanning several out in parallel. Use when the user says
  "/triage", "triage the project", "what's next", "what should I work on", "pick the next
  tasks", or wants to kick off a batch of parallel work. Accepts `--next=N` to choose how many
  tasks to run in parallel (default 3, hard cap 5); if omitted the coordinator proposes a count
  and confirms. This is a COORDINATOR workflow — it reads Linear directly, delegates each
  selected build to the `implementer` agent (and other global agents as suited), names branches
  via `branch-pattern` with the Linear identifier embedded so Linear auto-links the PR, and
  drives the Linear issue lifecycle (Todo/Backlog → In Progress). It does not write code itself.
---

# Triage Skill (NeonOS)

Answer "what next?" with evidence, not vibes. Read the **planning state** (Linear) and the
**delivery state** (recent PRs + worktrees), reconcile them so nothing already done or in-flight
gets re-proposed, rank the candidates, and — once the user picks `N` — fan the work out in
parallel using the global agent fleet, driving each issue through its Linear lifecycle.

This skill **coordinates**; it does not do the work itself:

```
gather status        ──▶  rank + select       ──▶  fan out N in parallel
(read Linear NEO +        (present table,           (worktree + branch-pattern
 gh PRs/worktrees)         user picks --next=N)       + Linear → In Progress
                                                      + implementer per task)
```

**Project coordinates (Linear):**
- Team **NeonWave** (key `NEO`) · Project **NeonOS** (`38e4d31c-021e-4825-9d21-fd0bdc42aa35`).
- Statuses (lifecycle): `Backlog` → `Todo` → `In Progress` → `In Review` → `Done`.
- Conventions live in memory [[neonwave-linear-conventions]]: estimates Fibonacci→21;
  every issue carries an **Area** label (`area:ci|docs|agent|core|cli`) **and** a **Type**
  (`Feature|Improvement|Bug`). `save_issue(labels=[...])` **replaces** the set — re-pass both.

---

## Step 0 — Parse args & confirm batch size

- `--next=N` → run the top `N` selected tasks in parallel. **Default 3, hard cap 5.**
- If `--next` is absent, **propose** a count (e.g. "I'd run the top 3 in parallel — ok?") and
  confirm before fanning out. Never blank-prompt; never exceed the cap (finite worktree/agent
  budget makes unbounded fan-out a footgun).
- Scope is the **NeonOS project on the NEO team**. The NEO team also holds other projects
  (Schema-Sync, Night Runner, Enclave, etc.); a bare `/triage` means **NeonOS only**. If the
  user names a different project, retarget the Linear queries to that `project` but keep the
  rest of the flow identical.

---

## Step 1 — Gather status (read Linear, then reconcile against delivery)

Read both sources and reconcile to a clean candidate list — conclusions, not dumps.

**Planning state — Linear (NeonOS project).** Pull the working set with the Linear MCP:

```
list_issues(team="NeonWave", project="NeonOS", limit=100)
```

Bucket by `status` / `statusType`:
- **Candidates** = `Backlog` (`backlog`) + `Todo` (`unstarted`) that are **leaves** (no sub-issues)
  and **unblocked** (no open parent/dependency, no unresolved decision).
- **In-flight** = `In Progress` (`started`) + `In Review` (`started`) → **never re-propose**.
- **Done** (`completed`) / archived → ignore.

Note for ranking: the active **cycle** (`cycleId` present) and **milestone**
(`projectMilestone`, e.g. "Phase 1 — neon CLI MVP + foundations") mark committed work; current
priority (`1`=Urgent…`4`=Low), `estimate`, `labels`, and `parentId` (epic membership) all feed
the rank. **Epics are parent issues** (e.g. NEO-27) — surface their open **sub-issues** as the
real candidates, not the parent.

**Delivery state — recent PRs + worktrees** (repo root `C:\Users\chris\projects\me\neonos`):

```powershell
gh pr list --state all --limit 20 --json number,title,state,headRefName,mergedAt,updatedAt
git worktree list                 # what's already checked out / in progress locally
```

Reconcile: drop any Linear candidate whose `gitBranchName` token (e.g. `neo-22`) appears in an
open PR/worktree even if Linear status lags. Linear status is the primary signal; PRs/worktrees
are the backstop. (For deeper investigation — "is this actually blocked?" — delegate a read-only
sweep to the **`researcher`** agent; it can reach Linear + `gh` via ToolSearch.)

---

## Step 2 — Rank & select

Reconcile planning vs delivery, then present a short ranked table:

```
Next candidates (NeonOS):
| # | Issue   | Task                                  | Why next                              | Est | Suggested agent   |
|---|---------|---------------------------------------|---------------------------------------|-----|-------------------|
| 1 | NEO-26  | SQLite schema v0 (SeaORM + SQLx)       | Todo, in cycle + Phase 1, unblocked    | 5   | implementer       |
| 2 | NEO-39  | neon doctor Windows .cmd shim          | Bug, small, unblocks doctor on Windows | —   | debugger→impl     |
| 3 | NEO-25  | tui-pantry component catalog           | Todo, in cycle, depends on doctor MVP  | 5   | implementer       |
```

Rank by: **in active cycle / Phase-1 milestone > priority (Urgent>High>…) > status readiness
(`Todo` > `Backlog`) > unblocked (no open parent/dep, decision resolved) > smaller `estimate`
first**. Exclude anything `In Progress`/`In Review`/`Done` or with an open PR/worktree.

Map each to the cheapest capable agent. The **doers** come from the global fleet; the NeonOS
**project-local governance agents** (committed in `.claude/agents/` per NEO-20) gate and plan
around them:
- code / feature build (`area:core|cli|agent|docs` + Type `Feature`/`Improvement`) → **`implementer`**
- bug (Type `Bug`) → **`debugger`** (root-cause) → hand fix list to `implementer`
- security-sensitive → **`appsec-auditor`** / **`threat-modeler`** first, then `implementer`
- dependency / supply-chain → **`dependency-auditor`**
- pure investigation / "is X feasible" → **`researcher`**

Governance agents you can lean on (don't use them to write code): **`engineering-manager`**
(fable; also the advisor) can turn a bigger slice into the per-unit breakdown that feeds Step 3,
**`product-owner`** (opus) checks a candidate actually traces to the phase plan / ADRs before you
build it, and **`project-manager`** (opus) / **`validator`** (sonnet) gate convention + lifecycle
correctness on the way to merge. For a routine `--next=3`, the coordinator's own ranking is
enough; pull in the EM/PO when a slice is large or its scope is uncertain.

Take the top `N` (`--next`). **Confirm the selection with the user before fanning out.**

---

## Step 3 — Fan out in parallel (coordinator pattern)

For **each** selected issue, set up an isolated lane and delegate:

1. **Branch + worktree** — create a worktree under `.claude/worktrees/` on a branch named per
   the **`branch-pattern`** skill **with the Linear identifier as the ticket segment** so Linear
   auto-links the PR to the issue. Map Type→branch type: `Feature`→`feat`, `Bug`→`fix`,
   `Improvement`→`refactor`/`chore`; `area:docs`→`docs`; `area:ci`→`ci`. Examples:
   `agent-feat/NEO-26/sqlite-schema-v0`, `agent-fix/NEO-39/doctor-windows-cmd-shim`.
   (Linear matches the `NEO-26` token anywhere in the branch name — this keeps the user's
   branch-pattern convention **and** Linear's PR↔issue auto-link. Don't use Linear's raw
   `chris/neo-…` default; it violates branch-pattern.)
2. **Move the Linear issue → In Progress** as the lane starts (lifecycle: `Backlog`/`Todo` →
   `In Progress`). Use `save_issue(id, state="In Progress")`. **Do not touch labels here** —
   `save_issue` replaces the full label set, so omit `labels` entirely (passing a partial set
   would drop the Area or Type label). See [[neonwave-linear-conventions]].
3. **Dev server (only for `apps/docs`, the Astro Starlight site).** Most NeonOS work is Rust/CLI
   (`cargo`) or library/config — **no server, skip this step**. Only when the lane touches the
   docs site and needs a live preview does the coordinator launch it in its OWN session as a
   background process: `Bash(command: "… pnpm --filter docs dev -- --port <port>",
   run_in_background: true)` so it survives across turns for the human to watch (a `runner`
   subagent's background process dies when it returns). Pick a free port by hand for the single
   docs server — there is no `port-registry` skill in this repo (that one is chriscoppola.me-local).
4. **Delegate the build** — spawn the suggested agent (usually **`implementer`**) with a concrete
   spec drawn from the Linear issue's **description + Scope + Done-when** (fetch full text with
   `get_issue(id)` when the list view truncated it), the branch/worktree, and the acceptance
   criteria. For bugs, run **`debugger`** first, then hand its fix list to `implementer`.
5. Agents report back to **this coordinator** — never agent-to-agent. The branch/PR + the Linear
   issue are the single sources of truth.

Keep the verify→ship pipeline in mind for each lane: when an implementation completes the default
next step is **`let-me-verify`** (checklist — for NeonOS usually `cargo test` / `pnpm check`, or a
docs-server smoke if applicable), then **`ship-it`** (opens a **draft** PR + Copilot triage).
Opening the PR auto-links the Linear issue (branch token) and Linear moves it to **In Review**;
marking the draft ready to trigger CI and merge is the human's **`ship-ready`** step, after which
the issue lands in **Done**.

---

## Step 4 — Report

Summarise the batch: each issue → branch · worktree · owning agent · new Linear status. Note any
candidates deferred (and why), and any **open questions / blockers** that need a human decision
before more work can start (e.g. an unresolved epic dependency, or an issue missing Done-when
criteria).

---

## Do-not rules

- Don't exceed the `--next` cap; don't fan out without confirming the count.
- Don't re-propose `In Progress`/`In Review`/`Done` work, or anything with an open PR/worktree —
  reconcile against Linear status **and** `gh pr list` first.
- Don't estimate or build epic **parents** — surface their sub-issues as candidates.
- Don't pass a partial `labels` set to `save_issue` — it replaces the whole set and drops Area/Type.
- Don't hand-pick branch names freeform — `branch-pattern` owns the format; the Linear `NEO-NN`
  identifier owns the ticket segment.
- Don't write code in the coordinator — delegate to `implementer`/`debugger`.
- Don't assume `port-registry` or `perf-auditor` exist here — they're chriscoppola.me-local.
