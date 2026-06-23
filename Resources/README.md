# The Menu Wall

**An AI hackathon challenge in deterministic layout — beautiful walls, live data, pennies to run**

## The story

Saffron Junction is a busy restaurant rolling out digital menu boards. Every location has a different TV wall: one store has a single landscape screen above the counter, another has four side by side, another runs a column of portrait totems. The screens are **unattended** — there is no employee with a mouse, no one to scroll, no one to fix a broken layout. Whatever your software renders is what customers see, all day.

And the day never sits still. The kitchen 86's the Chicken Dum Biryani at the lunch rush. The falooda machine comes back online at four. Breakfast categories vanish at 11:00; the dinner combos appear at 5. Every one of those changes needs a fresh wall — dozens of re-layouts a day, every day, at every location. And Saffron Junction isn't stopping at one store: the plan is hundreds of locations, thousands of screens. Whatever a menu change costs, multiply it by the fleet.

Your job: build the renderer. Given the full menu, a description of the wall, and the current state of the day, produce what each TV shows — every **available** item, spread across the screens, balanced, readable from across the room, nothing clipped, nothing missing. The same input must produce the **same wall, every time**. And because this happens a thousand times a month, it has to cost almost nothing per change.

This is an AI hackathon: we expect AI in your pipeline. The interesting question — the one this challenge is really about — is **where you put it**.

## What you get

```
menu.json        The full menu: 42 categories, 299 items, 38 with food photos,
                 7 categories with availability windows
configs/         Six wall configurations your submission must handle
  solo.json      1 landscape screen
  duo.json       2 landscape screens
  wall.json      4 landscape screens
  tower.json     1 portrait screen
  twins.json     2 portrait screens
  totem.json     3 portrait screens
states/          Three example day-states for demos
  weekday-morning.json      tue 08:30 — breakfast live, 2 items 86'd
  weekday-lunch-rush.json   wed 12:45 — 5 items 86'd, incl. the bestseller
  weekend-evening.json      sat 19:00 — weekend specials on, breakfast gone
samples/         Two reference renders (see "The bar" below)
```

**`menu.json`** — categories, each with items. Every item has an `id`, a `name`, and either a `price` or a `priceRange` (`{min, max}` — e.g. platters sold by size). A few items have a `description`; most don't. 38 items have an `image`: a stock food photo URL (hotlink-stable, from Wikimedia Commons under their respective free licenses; some items share a photo). Seven categories have an `availability` — a `{from, to}` time window and/or a `days` list; outside it the whole category disappears from the wall. That's real-world menu data.

**`configs/*.json`** — each config lists the screens on the wall. Every screen is `1920×1080` (landscape) or `1080×1920` (portrait), and **every wall is a single orientation** — all landscape or all portrait, never mixed. Screen count and orientation are the variables; resolution is not.

**`states/*.json`** — the state of the day: `day`, `time`, and `outOfStock` (a list of item `id`s currently 86'd). Your renderer takes one of these alongside the menu and config. Out-of-stock items and out-of-window categories are **omitted** from the wall — no "sold out" badges; the layout reflows as if they weren't on the menu.

## The bar

`samples/screen-1.png` and `screen-2.png` show roughly 15% of the menu rendered on a 2-screen landscape wall — featured-item rail with photos, balanced columns, prices aligned, legible from across a room. This is the visual quality we expect, **but for the whole menu, on every config, in every state**. The samples are a floor for craft, not a template — bring your own design language.

## What you build

A program that takes a config (any of the six) and a state, and renders **one browser page per screen**, each viewed at exactly that screen's resolution. Any language, any stack, any framework — server-rendered, static files, a dev server, whatever. The only requirement is that the final output opens in a browser.

### Hard requirements

1. **All six configs work.** Submissions are demoed against every config — both orientations, every screen count.
2. **State drives the wall.** Change the state — flip an item out of stock, move the clock past 11 — and re-render: the new wall shows exactly the available items, rebalanced. No gaps where items used to be, no ghost categories.
3. **Every available item appears exactly once** across the wall (not once per screen — once per wall).
4. **Nothing clipped, nothing overflowing.** Every item legible on screen. No manual scrolling — there is no one to scroll.
5. **Deterministic.** Same menu + same config + same state ⇒ the same wall. During judging your demo is run twice on the same inputs; the layouts must match.
6. **Unattended.** No interaction of any kind — the screens are TVs, not kiosks.

### The economics — read this twice

The budget: **1,000 menu changes per month must cost under $10** — that is, **about one cent per change, all-in**. That number is not arbitrary. A change's cost is the *unit economics* of the whole product: a restaurant fleet means hundreds of locations and thousands of displays, and whatever one re-layout costs gets multiplied by all of them, every month, forever. A penny per change scales to a fleet; a dollar per change is a business that dies at ten stores.

Now do the math on the naive AI architecture. "An LLM regenerates the HTML for the wall on every change" means emitting, say, 30–100k tokens of HTML per screen × 4 screens (the `wall` config) × 1,000 changes — easily a hundred million output tokens a month. At typical frontier-model output pricing (~$10–25 per million tokens), that's **thousands of dollars per location per month**, two to three orders of magnitude over budget. One cent buys you roughly a thousand output tokens — not even one screen's `<head>`.

So the constraint is the design problem: **AI is allowed anywhere, but the per-change path has to be nearly free.** Where does the expensive intelligence run — once? Rarely? What does it produce so that the thousand cheap re-renders never need it again? That split is what we're judging.

Submissions must include a **`COSTS.md`** with, at minimum:

1. **Pipeline map** — every step that runs when the menu changes, and which steps (if any) call an AI model.
2. **Cost per run** — the headline number. One *run* = one menu change → **every screen of the wall regenerated**. State it in cents, with the arithmetic behind it: tokens in/out per AI invocation × invocations per run × cited model list price. If a run costs nothing because AI only runs upstream, say so and show what the upstream step costs and how often it actually re-runs.
3. **Monthly total** — cost per run × 1,000 runs, plus any periodic AI costs (e.g. a weekly re-compile), for the largest config (`wall`, 4 screens). Must land under $10.

Honest estimates, public pricing, arithmetic a judge can re-do on a napkin. A beautiful wall with hand-wavy economics loses to a plain one with a defensible penny.

### What IS allowed (and encouraged)

- **Ambient motion.** Auto-scrolling or cycling *within* a category panel, gentle transitions, marquee effects — welcome, as long as every item still gets shown legibly during a cycle and the behavior is deterministic (same input ⇒ same sequence).
- **Photos and visual flair.** 38 items ship with stock photo URLs — use them, swap them, or add your own imagery. Visual design carries 20 points; food photography used well is the easiest way to earn them.

### Bonus

A dedicated **featured / specials zone** — a place on the wall that showcases selected items with photos and extra visual treatment, without violating the every-item-exactly-once rule for the regular listing. The reference renders show one way to do this. Extra credit if your featured picks adapt sensibly to the state (don't feature the thing that's 86'd).

## Judging

Human judges, live demo. Protocol:

1. Judges walk your submission through all six configs at a baseline state.
2. Judges switch states (the provided `states/` files) and watch the wall reflow.
3. Judges flip one more item out of stock by hand and re-render — the small change that forces a full re-layout.
4. One config + state is re-run from scratch — the layout must match the first run (the determinism check).
5. Judges read your `COSTS.md` and ask you to defend the cost-per-run arithmetic — expect "the menu just changed: walk me through exactly what executes, what it costs, and why that holds at 1,000 changes a month."

| Criterion | Weight |
|---|---|
| **Correctness** — all 6 configs, state-accurate, nothing clipped, every available item exactly once | 25 |
| **Visual design** — craft, taste, brand feel, use of imagery; does it look like a restaurant you'd eat at? | 20 |
| **AI architecture & economics** — AI placed where it earns its cost; credible COSTS.md within budget | 15 |
| **Readability** — legible from 10 feet; sensible hierarchy of category → item → price | 15 |
| **Balance** — screens carry comparable visual load; no half-empty TV next to a crammed one | 10 |
| **Determinism** — the re-run check; hesitation or layout drift costs points here | 10 |
| **Bonus: featured zone** — a specials showcase done well | 5 |

## FAQ

**Where are we supposed to use AI?**
That's the challenge — but here's the shape of it: LLMs are good at judgment (what layout flatters this menu? how should categories group? what's worth featuring?) and bad at being invoked 1,000 times a month on a budget of pennies. Architectures that spend AI once — to design, to compile rules or templates, to generate the renderer itself — and then re-render deterministically for free tend to fit the budget. Architectures that put an LLM in the per-change loop tend not to. We're describing the trade-off, not prescribing the answer.

**Can I call an LLM on every stock change?**
If you can show the arithmetic keeping 1,000 changes under $10 AND pass the determinism re-run, go ahead. Both at once is the hard part — LLM output isn't naturally deterministic, and a cent per change doesn't buy many tokens.

**Can I skip AI entirely?**
A pure hand-written renderer is legal, but this is an AI hackathon and 15 points sit on AI architecture & economics. A thoughtful "AI designs, code renders" pipeline beats both "no AI" and "AI does everything."

**What model pricing do I assume in COSTS.md?**
The public list price of whatever model you actually use, cited. If you use caching/batching discounts, show them.

**What exactly counts as one "run"?**
One menu change — an item flipping in or out of stock, or the clock crossing an availability boundary — triggering a full re-layout of **all screens** in the config. The menu changes many times a day; every change is a run, and every run produces a complete, fresh wall. Cost per run is the number we hold you to: it's what scales (or doesn't) across a fleet.

**Can I hand-tune layouts for each of the six configs?**
Tuning constants (font scale, column counts, panel sizing) per orientation or screen count is fine — that's engineering. Hand-placing individual items for a specific config is against the spirit: judges change the state and re-render, so a layout that only works for one frozen menu will break in front of them.

**Can a category be split across columns or screens?**
Yes — large categories sometimes have to be. Make the split legible (e.g. a "continued" marker). A category scattered randomly across the wall will cost you on readability.

**Does category order matter?**
No required global order, but items must stay grouped under their category, and the arrangement should feel intentional (e.g. mains before desserts reads better than the reverse).

**How do I display `priceRange` items?**
Your call: `$19.99–79.99`, `from $19.99` — anything honest and legible.

**Can I truncate long item names?**
No. Full names, legible. Wrapping is fine; ellipsis is clipping.

**Do I have to use the provided photos?**
No. They're there so you don't burn hackathon hours hunting stock images. Any imagery you have rights to is fine — visual design is judged on the result, not the source.

**What does "deterministic motion" mean?**
If a panel cycles or scrolls, the sequence and timing must be a pure function of the input — same menu + config + state ⇒ same animation from t=0. Seeded by input, not by `Math.random()` or the clock.

**What's the demo setup?**
A modern Chromium-based browser. Each screen is shown in a viewport (or window) at exactly its configured resolution. Be ready to launch from a clean clone in a couple of minutes.

**Can I use AI coding assistants to build my submission?**
Yes, freely. Build-time AI is unlimited and exempt from the $10 budget — the budget governs the recurring per-change pipeline, not your development process.

## Rules of engagement

- Solo or any team size — bring whoever you want.
- All code written during the event. Open source libraries are fine. Pre-built menu-board products are not.

Good luck. Make it beautiful, make it live, make it boring — in the best way: the same wall for the same moment, every single time, for a penny.
