# Working on this project with Claude

This project is built collaboratively with an AI assistant (Claude),
one iteration at a time, following the cadence in `CONTRIBUTING.md` and
the discipline in `LESSONS-LEARNED.md`. This document is the practical
playbook — especially the **resume prompt** for picking up in a fresh
session.

## The rhythm

1. **Pick the next iteration** from `_plans/iteration-plan.md` (e.g.
   "r02 — Chapter 7, kprobe + unlink").
2. **Claude drafts** the chapter prose and the `examples/NN-name/` Aya
   project, marking all new reconciliation rows `unverified`.
3. **Claude ships a tarball** `ebpf-with-aya-rNN.x.tar.gz`.
4. **You extract in place**, review `git diff --stat`, push, and
   `gh run watch` to confirm the site builds.
5. **You run `./demo.sh`** on real Fedora 44 (build on host → deploy to
   `ebpf-target` → drive load → check Grafana → cross-check with
   bpftool/bpftrace) and paste the output back.
6. **Claude debugs from the actual output**, ships a fix sub-iteration
   if needed (`rNNa`), and only then are rows promoted to
   `verified (Fedora 44)` — by you, not by Claude.
7. **Commit, repeat** with the next chapter.

The key discipline: **tested code first, then prose**, and **Claude
never self-promotes a claim to verified**.

## When to start a fresh session

Long sessions drift and slow down. Start fresh when you finish a phase,
when you've drafted several chapters, or when responses get recap-heavy.

## Resume prompt (paste into a fresh session)

> I'm continuing the **eBPF with Aya on Fedora** tutorial. Please read,
> in this order:
>
> 1. `onboarding/LESSONS-LEARNED.md` — the conventions to follow
> 2. `PRD.md` — what we're building and why
> 3. `CONTRIBUTING.md` — iteration cadence + the source/tooling
>    provenance policies + Liquid handling
> 4. `_plans/iteration-plan.md` — the roadmap
> 5. `_plans/reconciliation-plan.md` — current verified vs. unverified
> 6. The current `_docs/` and `examples/` for what's already drafted
>
> Status:
> - **Iterations shipped:** [e.g. r1.0 scaffold + Foundations]
> - **Verified on Fedora 44:** [list, or "none yet"]
> - **Currently working on:** [chapter/iteration]
> - **Next planned:** [from the roadmap]
> - **Open questions / stuck points:** [anything unresolved]
>
> Please summarize back: (1) what this tutorial is, (2) what's done vs.
> pending per the reconciliation plan, (3) the next reasonable piece of
> work. Then wait for me to confirm before writing.
>
> Follow throughout: deploy eBPF to the VM not the host; Podman +
> fully-qualified images + `:Z` + `127.0.0.1`; kernel tooling from
> Fedora repos only; tested code first then prose; new claims
> `unverified` and never self-promote; ship `ebpf-with-aya-rNN.x.tar.gz`
> tarballs; single-line pasted commands; no code from
> China/Russia/North-Korea/Iran repos (insight is fine).

## Anti-patterns to avoid

- **Skipping the reconciliation plan.** It's the difference between
  documentation and confident fiction.
- **Prose before working code.** Encourages plausible-but-untested
  claims.
- **Auto-promoting to verified.** A human runs the test; Claude marks
  `unverified` until then.
- **Long inline pastes.** Use tarballs; paste single-line commands.
- **Trusting AI-written Aya code without building it.** API churn is
  real — the first build is the test.
