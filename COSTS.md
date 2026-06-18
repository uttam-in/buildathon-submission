# COSTS

**Headline: a menu change costs $0.00 in AI. The per-change path makes zero
model calls. 1,000 changes/month ≈ $0 in AI spend, well under the $10 budget.**

This is by design, not by omission. The challenge's economics are a *placement*
problem: AI is allowed anywhere, but the recurring per-change path has to be
nearly free. We put all the intelligence *upstream of the running system*, at
build/design time, and made the per-change path a deterministic Rust function.

---

## 1. Pipeline map — what executes on a menu change

A "run" = one menu change (an item flips in/out of stock, or the clock crosses
an availability boundary) → a full re-layout of **every screen** in the config.

```
menu.json ──┐
config.json ─┤
state.json ──┴─▶ dmbr-convert (Rust)
                  │
                  ├─ adapt:      resolve availability (day+time), drop 86'd
                  │              items, flatten + normalize             [no AI]
                  ├─ pipeline:   filter, canonical sort                 [no AI]
                  ├─ layout:     capacity → partition → font → balance  [no AI]
                  ├─ paginate:   split dense screens into cycling pages [no AI]
                  ├─ render:     standalone HTML/CSS per screen          [no AI]
                  └─ hash:       SHA-256 over rendered HTML              [no AI]
                  ▼
            one HTML file per screen  (+ index.html)
```

**Every step is a pure, deterministic Rust function. None calls a model, makes
a network request, or loads an external resource.** The output is a set of
self-contained HTML5 documents (inline CSS, no JS, no fonts, no images fetched
at render time beyond the optional stock-photo URLs already in the menu data).

## 2. Cost per run

| Component | Per run |
|---|---|
| AI model invocations | **0** |
| AI tokens in / out | **0 / 0** |
| Marginal compute | a few ms of CPU on commodity hardware |
| **AI cost per run** | **$0.00** |

The arithmetic a judge can redo on a napkin: `0 invocations × any model price =
$0`. The renderer is a compiled binary; re-running it is the cost of a process
exec and some CPU cycles — fractions of a cent in compute even if you bill it,
and **nothing** in AI. There is no LLM in the per-change loop, so there are no
tokens to count.

## 3. Monthly total (largest config = `wall`, 4 screens)

| Line item | Frequency | Cost |
|---|---|---|
| Per-change renders (4 screens each) | 1,000 / month | **$0.00 AI** |
| Periodic AI re-compile (see below) | 0 / month (only on design change) | **$0.00** in a steady month |
| **Monthly AI total** | | **$0.00 — under $10** |

Even multiplied across a fleet — hundreds of locations, thousands of screens —
the AI cost stays at zero, because the multiplier is applied to a $0 per-change
path. That is the property that scales.

## 4. Where the AI actually goes (and why it's exempt)

Per the challenge rules, **build-time AI is unlimited and exempt from the $10
budget** — the budget governs the recurring per-change pipeline, not the
development process. Our AI spend lives entirely there:

- **Designing the layout rules** — the column counts per orientation, the
  density constants (item/header slot heights), the balance heuristic, the
  pagination strategy, the brand visual language. These were reasoned out once,
  at design time, and *compiled into the renderer as constants and code*. They
  do not re-run on a menu change.
- **Generating the renderer itself** — the Rust crates were written with AI
  coding assistance. That is a one-time (or rare) build cost.

If the *design* changes (new visual direction, new density tuning), you re-run
the expensive thinking once and ship a new binary. That is the only place a
periodic AI cost could appear, and in a steady month it is zero. This is the
"AI designs once, code renders for free" split the challenge asks for: the
expensive intelligence ran upstream and produced an artifact (the renderer) so
the thousand cheap re-renders never need it again.

## 5. Determinism (relevant to economics)

Because the per-change path is pure and AI-free, it is **byte-for-byte
deterministic**: same `menu + config + state` ⇒ identical HTML ⇒ identical
`render_hash`. Verified by re-running the same inputs and diffing the SHA-256 of
each screen's HTML (they match). This is what lets the renderer be cached
trivially and re-run by the judges without drift — and it is exactly the
property an LLM in the loop cannot cheaply guarantee.
