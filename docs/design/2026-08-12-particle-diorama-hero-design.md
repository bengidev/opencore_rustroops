# Particle Diorama Hero — Design

**Date:** 2026-08-12  
**Status:** Spec approved; implementation plan written (`docs/design/2026-08-12-particle-diorama-hero-plan.md`)  
**Path note:** Canonical copy lives under `docs/design/` (`docs/superpowers/` is gitignored in this repo).  
**Stack:** Rust + GPUI (`canvas`; light mist — no shader requirement for v1)

## Goal

Create an **original** onboarding/desktop hero: a **three-quarter edge-only cube cage** that acts as a barrier, with a **light mist** of particles living inside it (diorama / vitrine). Soft underglow seats the object. Do **not** copy either reference’s silhouette, palette, or particle look — reuse compositional principles only.

## Decisions (locked)

| Topic | Choice |
|--------|--------|
| Shell | Edge-only cube cage (faces invisible) |
| Interior | Contained breathing orb (slow equilibrium churn) |
| Stage light | Soft underglow at ground contact |
| Barrier feel | Soft press along inner faces, then drift inward |
| Interaction | Subtle hover/focus field or density bias |
| Density | Light mist (hundreds–low thousands) |
| Approach | Projected wireframe + 2D particle field (Approach 1); optional back/front edge split if depth reads flat |

## Non-goals (v1)

- Solid or glass face fills / refraction
- Heavy swarm particle counts
- Strong pointer-driven spectacle
- Runtime shader path (unless later density demands it)
- Copying reference forms, text, or colors
- Replacing unrelated home chrome outside the hero slot

---

## Section 1 — Composition

- **Hero:** one three-quarter cube as an **edge-only cage** (no face fills).
- **Interior:** light mist that **breathes** — slow churn, soft equilibrium, no directed jet.
- **Barrier read:** near the six inner faces, density **soft-presses** (slight thicken), then drifts inward; particles never leave the box.
- **Stage:** soft **underglow** under the cube’s ground footprint; generous empty margin around the object.
- **React:** on hover/focus, a **small** field bias or density shift; idle motion stays primary.
- **Principles borrowed (not copied):** hard primitive as container; mass vs emission via underglow; form-from-density; curl-driven churn; internal negative space via sparse mist; soft press for “locked in.”

---

## Section 2 — Motion & simulation

- **Space:** particles in a unit (or aspect-matched) cube; position (+ optional velocity).
- **Core motion:** slow **curl / noise field** advection; field evolves gently over time.
- **Containment:** soft walls — near a face, **inward bias** + **density boost** (soft press); hard clamp so nothing crosses.
- **Center hold:** weak attractor toward box center so mist stays a coherent volume.
- **Lifecycle:** fixed pool; continuous loop; no spawn-burst fireworks.
- **Hover/focus:** small field/density bias; ease in/out ~150–250ms.
- **Render map:** same camera projection for cage and particles; farther points slightly smaller / softer opacity.
- **Perf:** CPU step + GPUI `canvas` points/quads; fixed particle cap; skip step when hero hidden/off-screen.

---

## Section 3 — Architecture

### Placement

- **v1 home:** onboarding hero slot (`hero_block` / current ASCII galaxy).
- New diorama is a **separate module**. Keep ASCII hero until an explicit swap or flag chooses the diorama.
- Do not silently delete `ascii_galaxy` in the first implementation PR unless the plan says so.

### Module sketch

Suggested under `src/app/onboarding/` (or a sibling `particle_diorama/` if preferred at plan time):

- `sim` — particle buffer, `step(dt)`, containment, soft press, hover bias
- `cage` — 3D cube edges, projection, underglow footprint geometry
- `view` — GPUI element: `canvas` paint + hover/focus wiring

### Data flow

1. View/`Entity` owns: particle buffer, time, hover/focus flag, (optional) seed.
2. Animation frame / timer → `step(dt)`.
3. Paint → project edges + underglow + points.
4. No network/IO.

### Theming

- Cage, glow, and particle colors come from **app theme tokens**, not hardcoded reference hues.

### Optional depth upgrade

If the first paint feels flat: split cage into **back edges → particles → front edges** (Approach 2 paint order) without changing the simulation model.

---

## Section 4 — Testing & acceptance

### Automated

- **Containment:** after many steps, all particles remain inside the box (epsilon).
- **Soft press:** near-face density higher than center under a deterministic seeded idle (or equivalent measurable bias).
- **Projection:** cage edge endpoints project to finite 2D points; no NaNs.
- **Smoke:** onboarding still mounts with the new hero view (follow existing test style where present).

### Manual acceptance

- Cage reads edge-only; faces not filled.
- Mist breathes; no directed jet; no escape through edges.
- Soft thicken near walls, then drift inward.
- Underglow under footprint, calm.
- Hover/focus: small bias; release eases back.
- Feels light on a normal laptop idle.

### Done when

Checks above pass, and the diorama can replace or sit behind the ASCII hero via a clear flag/swap in onboarding. Implementation plan is written only after this spec is reviewed.

---

## Open points for the implementation plan (not blockers)

- Exact particle count cap and frame timing API (`request_animation_frame` vs existing onboarding pattern).
- Whether v1 ships behind a feature flag or direct swap of `hero_block` content.
- File path finalization (`onboarding/particle_diorama` vs top-level module).

## References (study only)

- Particle screen recording (composition principles).
- Cube award screenshot (composition principles: edge mass, underglow, empty stage).
- Explicit: do not copy.
