# Transitions: Property Animation for Markout

## Core Idea

A transition is a binding from time to a property value, with a curve.
Multiple transitions can run simultaneously on the same or different
components — they're parallel strings of animation state.

## Syntax

```
@transition:<name> property:<prop> from:<start> to:<end> duration:<ms> curve:<fn>
```

Multiple transitions on one component:

```
| {label:title "Hello" primary}
@transition:t1 target:title property:opacity from:0 to:1 duration:500 curve:linear
@transition:t2 target:title property:y from:20 to:0 duration:500 curve:ease-out
```

The label fades in (opacity 0→1) while sliding up (y 20→0) over 500ms.

## Curves

```
linear        — constant rate: t
ease-in       — accelerate:    t^2
ease-out      — decelerate:    1-(1-t)^2
ease-in-out   — both:          smooth S-curve
sqrt          — fast start:    sqrt(t)
cubic         — slow start:    t^3
bounce        — overshoot:     spring-like
step          — instant jump:  0 until end, then 1
```

All curves map normalized time t∈[0,1] to a value v∈[0,1].
The actual property value is: `from + (to - from) * curve(t)`

## Animatable Properties

```
x, y          — position offset (pixels)
w, h          — width, height
opacity       — 0.0 to 1.0
scale         — 1.0 = normal
rotation      — degrees
color-r/g/b   — individual color channels
progress      — progress bar fill
```

## Multiple Transitions

```
| {button:submit "Send" primary}
| {progress:bar}

@transition:fade target:submit property:opacity from:1 to:0.5 duration:300 curve:ease-in
@transition:slide target:submit property:x from:0 to:-20 duration:300 curve:ease-out
@transition:fill target:bar property:progress from:0 to:100 duration:2000 curve:linear
```

Three transitions, three targets, three curves, running in parallel.
Each is a row in the animation table.

## Chaining

```
@transition:t1 target:a property:opacity from:0 to:1 duration:500 curve:ease-out
@transition:t2 target:b property:opacity from:0 to:1 duration:500 curve:ease-out after:t1
@transition:t3 target:c property:opacity from:0 to:1 duration:500 curve:ease-out after:t2
```

`after:<name>` starts a transition when another finishes.
This creates a staggered reveal — a, then b, then c fade in
sequentially, 500ms apart.

## Looping

```
@transition:pulse target:dot property:scale from:1 to:1.3 duration:800 curve:ease-in-out loop:bounce
```

Loop modes:
- `loop:none` — play once (default)
- `loop:repeat` — restart from beginning
- `loop:bounce` — reverse direction each cycle (ping-pong)

## Trigger

By default, transitions start immediately on render.
Bind them to events:

```
@transition:hover target:btn property:scale from:1 to:1.05 duration:150 curve:ease-out trigger:hover
@transition:click target:btn property:y from:0 to:2 duration:100 curve:ease-in trigger:click
```

Triggers:
- `trigger:render` — on first render (default)
- `trigger:hover` — on mouse enter
- `trigger:click` — on click
- `trigger:focus` — on focus
- `trigger:scroll` — when scrolled into view
- `trigger:bind:<key>` — when a state key changes

## Parallel String Interpretation

```
Transition:  t1
Target:      title
Property:    opacity
From:        0
To:          1
Duration:    500
Curve:       ease-out
After:       (none)
Trigger:     render
Loop:        none
```

Each transition is a row. Each attribute is a column.
Multiple transitions are multiple rows — parallel strings
of animation state, all ticking against the same clock,
each with its own curve function mapping time to value.

Animation IS parallel strings with different curve functions
applied to the same time axis. Same input (elapsed ms),
different transforms (curves), different outputs (property values).

## Framebuffer Rendering

For PST OS (rdtsc-based timing):

1. On render, record start tick for each active transition
2. Each frame, compute elapsed = (rdtsc - start) / tsc_freq_ms
3. Compute normalized t = elapsed / duration, clamped to [0,1]
4. Apply curve: v = curve(t)
5. Compute property value: from + (to - from) * v
6. Apply to component before rendering
7. If t < 1.0, mark dirty for next frame

Curve functions are pure math — no allocations, no state:

```rust
fn ease_out(t: f64) -> f64 { 1.0 - (1.0 - t) * (1.0 - t) }
fn ease_in(t: f64) -> f64 { t * t }
fn sqrt_curve(t: f64) -> f64 { /* integer approx */ }
```

For no_std without f64, use fixed-point (16.16) arithmetic.

## Composability

Transitions compose with all other Markout features:

```
@parallax:hero
| @layer speed:0.3
|   {label:bg "Background" muted}
|   @transition:t1 target:bg property:opacity from:0 to:1 duration:1000 curve:sqrt
| @layer speed:1.0
|   @grid:content cols:1 rows:2
|   | (0,0) {label:title "PST OS" primary xl}
|   | (0,1) {button:go "Start" primary}
|   @end grid
|   @place:content anchor:center
|   @transition:t2 target:title property:y from:30 to:0 duration:600 curve:ease-out
|   @transition:t3 target:go property:opacity from:0 to:1 duration:400 curve:linear after:t2
@end parallax
```

Parallax layers + grids + placers + transitions.
All parallel strings. All composable. All declarative.
