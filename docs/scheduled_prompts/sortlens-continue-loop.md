# Sortlens: phone replacement loop — scheduled-task contract

This file is the in-repo contract for the recurring scheduled task the owner calls
**the continuous coder**. The task lives in the DeepAI Code scheduler (the older
DeepSeek Harness MCP scheduler used the same lease id). Reproduce the prompt body
below into the scheduler's `prompt` field whenever the task is re-created; if the
task is recreated with a new lease id, update the "Lease note" at the bottom of
`docs/CHECKPOINT.md` to point at the new id, and replace the lease id here.

## Identity

- **Title:** Sortlens: phone replacement loop (a.k.a. the continuous coder)
- **Lease id (live):** `scheduled-task:command:mcp:6c864c39-52f6-49f8-b58e-82d40c2823ce:schedule-task:sortlens-continue-loop`
- **Stale lease ids:** `scheduled-task:command:mcp:c79b9a35-…:schedule-task:sortlens-continue-loop` — do not use.
- **Cadence:** roughly every 30–60 minutes; the loop is bounded to one vertical
  slice per tick, so the next tick should not start before this one's work is
  committed and pushed.
- **Workspace:** `/Volumes/Expansion/Development/sortlens-rust` (the project root).
- **Authoritative doc on each tick:** `docs/MOBILE_PLAN.md`.

## What changed in this revision

The prompt that lived only in the scheduler kept updating `docs/MOBILE_PLAN.md`
locally but never told the agent to commit, and the loop's exit path left the
task disabled. As a result the doc accumulated correct content in the working
tree but never landed in git, and the next tick could not pick up "what changed
last run" from the repo. This revision:

1. Makes the commit-and-push step **mandatory** and the last action of every
   tick, before re-enabling the lease.
2. Tells the agent explicitly that `docs/MOBILE_PLAN.md` must be in git at the
   end of the tick and to add it on the first tick if it is still untracked.
3. Keeps the existing safety, security and execution rules (lease, agent
   coordination, PhotoKit, simulated testing only, no signing credentials).
4. Stops the "leave the task disabled on exit" pattern; disabled is now reserved
   for the genuine "I cannot leave the tree clean" case named in STEP LAST.

## Prompt body (paste verbatim into the scheduler)

```
Work on the Rust-based Sortlens phone replacement requested by the owner,
following docs/MOBILE_PLAN.md and SL-J10 in docs/ROADMAP.md. One bounded,
tested vertical slice per run; this is implementation, not just a status
report. The user's screenshots are design references; the handoff project at
/Users/iancunningham/Documents/sortlens-mac-handoff-20260504-190347-main is the
feature source but was inaccessible under macOS privacy restrictions. First
check if it is accessible; if yes, audit real behavior and complete M0's
evidence-backed parity matrix. If not, do not invent parity: report the
exact OS error and make independent M1 platform/host progress instead. After
M0, work through the plan in dependency order. Ship a dedicated phone-first
experience, not the desktop shell squeezed onto a phone; preserve the Rust
shared engine, data safety, Android and desktop behavior.

LEASE: STEP 0, before any other work: call list_scheduled_tasks and locate
this exact task — `Sortlens: phone replacement loop` — whose id is
`scheduled-task:command:mcp:6c864c39-52f6-49f8-b58e-82d40c2823ce:schedule-task:sortlens-continue-loop`.
Disable it with update_scheduled_task enabled=false so a second tick cannot
start while this one walks the plan. If the disable fails, stop; do not walk
with the schedule still armed. On STEP LAST (below) you must re-enable the
SAME id and verify the returned nextRunAt is in the future.

SAFETY AND COLLABORATION: inspect git status and recent commits, then check
list_agents before edits. Other agents and the user share the working tree.
Never reset, clean, overwrite, revert or force-push another person's
uncommitted files. Existing apps/sortlens-mobile/src/entry.rs has an
uncommitted iOS GPU-limits fix and other agents own other uncommitted slices
in this tree; preserve and inspect them, and avoid editing anything whose
ownership is unclear. If an active agent overlaps the intended slice,
coordinate and choose a non-overlapping slice; silence is not agreement. Do
not commit or push another agent's work — only commit work this tick
actually produced (the test, the slice, the doc delta, the CHECKPOINT entry
and any other files this slice genuinely owns). The lease knows nothing
about file ownership, so this rule is yours to enforce.

SECURITY: the current remote code is plain HTTP with bearer tokens in phone
settings. Never expose it to the public Internet, port-forward it, send
credentials to an untrusted service, or claim secure access from anywhere.
Secure transport, authenticated server identity, protected credential
storage, revocation and network/deployment tests are explicit M8
requirements. Avoid real user credentials and libraries in testing. Don't
ask the user to paste secrets in chat.

EXECUTION: read docs/MOBILE_PLAN.md and the git status, not screenshots,
for the source of truth. Probe Xcode, simulator, devices and signing
availability rather than relying on past CHECKPOINT claims. Select one
specific unchecked slice; write a failing test where practical, implement
and verify it. Run targeted tests and applicable cargo xtask ci / snapshot /
deny / mobile / screenshots as feasible, reporting unavailable or failed
gates honestly. Device and simulator screenshots do not substitute for
interaction testing.

DOC UPDATES — required every tick:
- Update docs/MOBILE_PLAN.md with this tick's slice: a dated "Progress
  (YYYY-MM-DD)" subsection naming the files, the gates run and the next
  step. The tail of this doc has a "Next slice and execution rules" section
  that the tick's progress note must keep in sync so the next tick reads a
  correct starting point.
- Update docs/ROADMAP.md's SL-J10 row and any phase ordering it shifts.
- Do not touch CHECKPOINT.md unless this tick is the one writing the
  corresponding numbered entry; CHECKPOINT entries are reserved for the
  tick that owns the slice and the gates that prove it.

COMMIT AND PUSH — the heart of this revision:
- Every tick that changed code, tests or docs must end with the changes
  committed on the current branch and pushed to origin, with `git push`
  exit 0 confirmed. A slice that exists only in the working tree is not
  finished, and the next tick will re-do it.
- The first commit of this loop's lifetime must include `docs/MOBILE_PLAN.md`
  (this file currently sits untracked in the working tree); commit it on
  its own as the bootstrap commit if no other slice is ready, so the agent
  has a tracked starting point.
- Commit messages: "ci: …" or "fix: …" or "feat: …", in the repo's
  established voice, naming the slice and the failing-then-passing test.
- Do not commit other agents' uncommitted work; if it sits in git status,
  leave it alone and proceed only with work this tick owns. If only
  other agents' work is dirty and you have no own slice to ship, commit
  nothing and report the overlap to the user.
- Push to the configured remote (origin). If `git push` returns a
  non-fast-forward error, stop and report it; do not `--force`.

END OF TICK (always):
1. `git status` clean for files this tick owns. Other agents' files may
   still show as dirty; that is allowed.
2. `git log -1` shows the commit pushed this tick is on the branch.
3. Re-enable the lease: update_scheduled_task
   scheduledTaskId=`scheduled-task:command:mcp:6c864c39-52f6-49f8-b58e-82d40c2823ce:schedule-task:sortlens-continue-loop`
   enabled=true. Verify the result has a nextRunAt. Do this before your
   final reply, even when the tick only shipped docs.
4. Leave the task disabled only when you cannot leave a clean tree this
   tick owns (a stalled build, a force-push you cannot avoid, an external
   blocker). When you do, state the exact reason in the final reply.

Finish by reporting the slice, tests, gates, files, push confirmation and
next step. If interrupted, leave the slice's commit-and-push before the
interruption; do not erase prior work.
```

## When this contract changes

1. Update this file in the same commit that justifies the change (a behaviour
   change in the doc, a new safety rule, a new commit policy, etc.).
2. Re-apply the new "Prompt body" verbatim to the MCP scheduler.
3. Update the "Lease note" at the bottom of `CHECKPOINT.md` if the lease id,
   title or re-enable rule moved.
4. Note the change in the next CHECKPOINT entry that ships work.

## Cross-references

- `docs/MOBILE_PLAN.md` — the doc the loop keeps current.
- `docs/ROADMAP.md` — the SL-J10 row this loop ticks.
- `CHECKPOINT.md` "Lease note" — pointer to the live lease id.
- `docs/AUTONOMOUS_AGENT.md` — the GitHub intake playbook (a different loop,
   `Sortlens: GitHub intake`); the two loops share the repo but must not share
   a lease.
