# Parallax Scrolling for Markout

## Core Idea

Parallax is layers moving at different speeds. In Markout, each layer
is a parallel string. The scroll position is a binding. Each layer
declares its speed relative to the scroll — that's it.

## Syntax

```
@parallax:<name>
| @layer speed:0.2 {content}
| @layer speed:0.5 {content}
| @layer speed:1.0 {content}
@end parallax
```

Speed is a multiplier on the scroll offset:
- `speed:0` — fixed (doesn't move)
- `speed:0.2` — slow background
- `speed:0.5` — midground
- `speed:1.0` — normal scroll (foreground)
- `speed:1.5` — faster than scroll (overlay effect)

## Example: Hero Section

```
@parallax:hero
| @layer speed:0.1
|   {image:stars "starfield.bmp"}
| @layer speed:0.3
|   {image:mountains "mountains.bmp"}
| @layer speed:0.6
|   {label:tagline "One primitive. Every subsystem." muted lg}
| @layer speed:1.0
|   {label:title "PST OS" primary xl}
|   {badge:k "seL4" success}  {badge:l "Rust" primary}
|   {button:start "Get Started" primary}
@end parallax
```

Four layers, four speeds. Scroll down and the stars barely move,
mountains drift slowly, tagline moves at half speed, title scrolls
normally.

## How It Maps to Parallel Strings

```
Layer 0:  speed=0.1  content=stars
Layer 1:  speed=0.3  content=mountains
Layer 2:  speed=0.6  content=tagline
Layer 3:  speed=1.0  content=title+badges+button
Scroll:   bind:scroll.y
```

Each layer is a row. Speed is a column. Content is a column.
Scroll position is a shared binding that each layer reads and
multiplies by its speed to compute its own offset.

The parallax effect emerges from parallel strings with different
speed multipliers on the same scroll value. That's literally
what "parallel" means — same direction, different rates.

## Grid + Placer Integration

Layers can use placers for positioning within the parallax container:

```
@parallax:hero
| @layer speed:0.2
|   @grid:bg cols:1 rows:1
|   | (0,0) {image:sky "sky.bmp"}
|   @end grid
|   @place:bg anchor:center
| @layer speed:1.0
|   @grid:content cols:1 rows:3
|   | (0,0) {label:title "PST OS" primary xl}
|   | (0,1) {label:sub "Parallel String Theory" muted}
|   | (0,2) {button:go "Launch" primary}
|   @end grid
|   @place:content anchor:center
@end parallax
```

Grids inside layers. Placers position grids within their layer.
Layers scroll at different speeds. Everything composes.

## Framebuffer Rendering

For PST OS (no GPU, raw VGA framebuffer):

1. Compute current scroll offset (from mouse wheel or key binding)
2. For each layer, compute `layer_offset = scroll * speed`
3. Render layer content at `y - layer_offset`
4. Composite layers back-to-front (painter's algorithm)
5. Blit to VGA

No hardware acceleration needed. Just offset math per layer.

For 640x480 with 3-4 layers, this is a few framebuffer blits per
scroll event — well within what a single-core QEMU/VirtualBox
VM can handle.

## Terminal Rendering

Terminal doesn't support parallax (single scroll context).
Degrade gracefully: render all layers at speed:1.0 in order.
The content is the same, just without the depth effect.

## Constraint

A parallax container cannot be nested inside another parallax.
Layers are flat. This keeps the scroll math simple and prevents
recursive compositing.

## Parallel String Theory Connection

Parallax IS parallel strings, literally:

- Multiple strings (layers) running in the same direction (scroll)
- At different rates (speed multiplier)
- With positional identity (layer order = z-index)
- Computed from the same input (scroll offset)

The name "parallel" in "Parallel String Theory" and "parallax"
share the same root. Parallel lines that never converge but move
at different speeds create the illusion of depth. That's parallax.
That's also what parallel strings do — multiple columns of data,
same rows, different rates of change, creating emergent structure.
