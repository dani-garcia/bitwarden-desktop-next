---
name: audit
description: Project-wide audit for duplication, complexity, stale comments, architectural drift, dead code, panic potential, theme parity, Iced anti-patterns, scattered TODOs, and oversized modules. Use when the user asks to "audit", "review the project", "find improvements", "look for cleanup", or wants a sweep of cleanup opportunities. Produces a deduplicated, impact-graded, verified findings list.
---

# Project audit

Multi-agent sweep over the codebase that produces an actionable, sorted findings list. Designed around lessons from a prior audit: partition by **directory slice**, not by topic; combine all lenses per agent; force impact grading; verify claims before presenting.

## Step 1 — Survey

Spawn ONE `Explore` agent (search breadth: quick) with the prompt:

> List the top-level directory layout under `crates/desktop/src/` (and any other source roots in `crates/`). For each directory at depth 1–2, report: file count, total LoC (rough), and a one-sentence summary of what lives there. Also list any oversized files (>500 LoC) with their path and line count. Reply with just the table — no preamble.

Use the result to choose **3–6 slices**. A slice is one directory subtree with cohesive responsibility (e.g. `views/vault/`, `views/login/`, `views/send/`, `components/`, `services/`, `theme/`+root). Aim for slices of comparable size — split big subtrees if needed.

## Step 2 — Slice agents (parallel)

Spawn one `feature-dev:code-reviewer` agent per slice, **all in a single message** for parallelism. Each agent gets the same prompt template, parameterized by slice path. Required template:

```
You are auditing the slice `<SLICE_PATH>` of a Rust + Iced 0.15 desktop app
(Bitwarden-lite). Work from the workspace root (the directory containing the
top-level `Cargo.toml`). Read CLAUDE.md and docs/architecture.md FIRST — they
define conventions you MUST check
adherence to (MVU compositional pattern, RenderCtx/UpdateCtx, modal::dialog_*,
components::buttons::*, dropdown helper rules, padding shorthand, theme access
rules, widget ID convention, Outcome::toast vs events, perform_with_*_client).

Apply ALL of the following lenses in one pass. Each finding belongs to exactly
one lens (pick the dominant one):

1. **DUP** — Duplicate code that should be extracted to a shared helper /
   component / context method. Skip ≤5-line duplications unless structurally
   important.
2. **CPX** — Code more complex than necessary: re-implementing existing
   helpers, premature flexibility (one-caller generics, unused enum variants),
   manual state machines, deep pattern-match guards, two-step message
   round-trips where one task chain works.
3. **CMT** — Comments that are verbose, obvious, historical (reference
   removed/old code, "TODO migrate to..."), or stale. Keep load-bearing WHY
   comments — match the bar set by CLAUDE.md "Iced Gotchas".
4. **ARCH** — Deviations from documented conventions: signature drift on
   view()/update(), naming (Message vs Event vs Action), missing pub on
   widget IDs, padding struct literals where shorthand applies, hand-rolled
   modal/header/button construction that bypasses standard helpers,
   per-view state held on App, etc.
5. **DEAD** — Functions / types / modules / variants with no callers
   reachable from main(). Check via grep before flagging.
6. **PANIC** — `unwrap()`, `expect()`, `panic!()`, `todo!()`, raw indexing
   that could panic. Each one is a desktop-app crash risk. Flag unless the
   invariant is locally provable.
7. **ICED** — Anti-patterns from CLAUDE.md "Iced Gotchas": `iced::time::every`
   for non-time event sources, uncached `image::Handle::from_bytes`, raw
   `pane_grid::State<T>` outside `CollapsiblePane`, `Stack` siblings without
   `ShellScope`, nested container backgrounds masking parent radius, hardcoded
   colors outside theme/dark.rs and theme/light.rs.
8. **THEME** — (skip if your slice doesn't include theme/dark.rs or
   theme/light.rs.) Color tokens defined in one theme but missing in the
   other; tokens defined but never referenced.
9. **TODO** — Scattered `// TODO`, `// FIXME`, `todo!()`. These belong in
   `docs/todo.md`, not source.
10. **SIZE** — Files in your slice >500 LoC where a clear extraction
    boundary exists.

For EACH finding output:
- **ID**: `<LENS>-<NN>` (e.g. `DUP-03`, `PANIC-01`)
- **Name**: ≤8 words
- **Impact**: one of `high` / `med` / `low` followed by a one-sentence
  justification. `high` = ≥30 LoC saved OR fixes a real bug OR removes a
  systemic inconsistency. `med` = 10–30 LoC OR notable consistency win.
  `low` = polish.
- **Payoff**: one phrase — concrete number when possible (LoC saved, bugs
  prevented, sites unified).
- **Detail**: 3–8 sentences. Include file paths with line numbers in
  markdown-link form `[file.rs:NN](path/file.rs#LNN)`. Quote offending
  comments verbatim (truncated). For DUP findings, sketch the proposed
  helper signature and where it should live.

**Cap: 15 findings max.** Force prioritization. If you have more candidates,
keep only the top 15 by impact.

Sort your output by impact (high → low). Return ONLY the findings list — no
preamble, no methodology section, no closing summary.
```

## Step 3 — Verifier (parallel-safe, runs after Step 2)

Spawn ONE `general-purpose` agent with the consolidated raw findings (concatenated outputs from Step 2) and this prompt:

> You are spot-checking claims in a code audit before they're presented to the
> user. For each finding that makes a falsifiable claim — "exactly N callers",
> "all call sites pass X", "byte-for-byte identical", "unused", "no
> references", "appears in M places" — verify it with grep/read against
> the current code in the workspace. Report ONLY findings
> where the claim is wrong or imprecise. For each, give: the finding ID,
> the disputed claim, what you actually found, and a corrected wording.
> Skip findings that don't make falsifiable claims (subjective ones like
> "this is overly complex" need no verification). Cap at 20 corrections.

## Step 4 — Synthesis (you, the parent agent)

1. **Deduplicate.** Cross-agent overlap is the strongest signal a finding is real, but only present it once. Merge by: same file:line refs, same proposed helper, same affected concept. When merging, list the original IDs in the merged item so the user sees the cross-agent confidence.
2. **Apply verifier corrections.** Edit affected findings; if a claim falls apart, downgrade or drop the finding.
3. **Re-sort by impact** across the whole list (each agent sorted within itself; you sort across). Use the agents' `high/med/low` grades as the primary axis.
4. **Renumber IDs** with a flat scheme (e.g. `R01`, `R02`, …) for readability. Keep the original lens prefix in parens after the name if useful.
5. **Group small polish items.** If you have ≥5 trivial findings (e.g. comment trims, single-token fixes), bundle them into one final "miscellaneous polish" item with a bullet list, rather than promoting each to top-level.

## Step 5 — Output to user

Present the consolidated list. Each item: ID, name, payoff, detailed body with file:line refs. No preamble — go straight to the list. After the list, one line noting how many slices ran and the total finding count before/after dedup.

## Notes

- **Why directory slices, not topic slices:** topic-based partitioning (one agent for "duplication", one for "complexity", etc.) caused massive cross-agent overlap in a prior audit (one finding flagged 4×). Directory slicing eliminates this — each region is owned by exactly one agent, with one combined lens.
- **Why a verifier:** agents make confident claims like "all 4 callers pass `Some`" or "exactly one caller" that are sometimes wrong. A cheap verification pass catches these before the user acts on them.
- **Don't expand the slice count past 6** — diminishing returns and the synthesis pass becomes harder.
- **If the user wants a focused sweep** ("audit just the views"), skip the survey and run a single slice agent on the requested subtree.
