# Grid + Placer: Markout Layout Primitive

## Core Idea

Two independent concerns:
- **Grid**: defines content and bindings in an NxM matrix
- **Placer**: defines where the grid gets positioned on screen

What goes in the cells is arbitrary. Where it goes on screen is a separate declaration.
This is the parallel string principle applied to layout: content, position, and bindings
are parallel columns on the same component row.

## Syntax

```
@grid:<name> cols:<N> rows:<M>
| (<col>,<row>) {component}
| (<col>,<row>) {component}
@end grid
@place:<name> anchor:<position>
```

## D-pad Example

```
@grid:dpad cols:3 rows:3
| (0,0) {}  (1,0) {button:up "^" bind:nav.up}  (2,0) {}
| (0,1) {button:left "<" bind:nav.left}  (1,1) {label:pos "0,0"}  (2,1) {button:right ">" bind:nav.right}
| (0,2) {}  (1,2) {button:down "v" bind:nav.down}  (2,2) {}
@end grid
@place:dpad anchor:bottom-right
```

3x3 grid = 3 columns, 3 rows. Empty cells are valid (corners).
Cell size is derived from the largest child in each column/row.
Total grid size = sum of column widths x sum of row heights.

## Scalability

The grid scales from 1x1 (single button) to NxM (any layout).

```
@grid:toolbar cols:10 rows:1
| (0,0) {button:h1 "H1"}  (1,0) {button:h2 "H2"}  ...
@end grid
@place:toolbar anchor:top
```

A toolbar is a 1-row grid. A file card layout is a 4x3 grid.
A start menu is a 1x7 grid. Same primitive, different dimensions.

## Placer

The placer pins a named grid to a screen anchor:

| Anchor | Position |
|--------|----------|
| `top-left` | Pin to top-left corner |
| `top-right` | Pin to top-right corner |
| `bottom-left` | Pin to bottom-left corner |
| `bottom-right` | Pin to bottom-right corner |
| `top` | Centered at top edge |
| `bottom` | Centered at bottom edge |
| `center` | Centered on screen |

Optional offset: `@place:dpad anchor:bottom-right offset:8,8`

## Why Not Parametric

`@parametric` uses constraint references between components — each component
declares its position relative to other components. This creates a dependency
graph that must be topologically sorted.

The grid+placer is simpler:
- Grid cells have fixed positions within the grid (column, row)
- The placer pins the whole grid to a screen region
- No dependency graph, no solver, no constraint resolution

Parametric is for complex layouts with inter-component relationships.
Grid+placer is for structured, predictable layouts with absolute positioning.

## Parallel String Interpretation

```
Component:  dpad
Grid:       cols=3, rows=3
Cell(1,0):  {button:up "^" bind:nav.up}
Cell(0,1):  {button:left "<" bind:nav.left}
Cell(1,1):  {label:pos "0,0"}
Cell(2,1):  {button:right ">" bind:nav.right}
Cell(1,2):  {button:down "v" bind:nav.down}
Placement:  anchor=bottom-right
```

Each line is a parallel string. Change the placement without touching the grid.
Change a cell's content without touching the placement. Change a binding
without touching the content. Independent columns, same row.

## Desktop Application

The entire PST OS desktop can be expressed as grids + placers:

```
@grid:hero cols:1 rows:5
| (0,0) {label:title "PST OS" primary lg}
| (0,1) {label:sub "Parallel String Theory" muted}
| (0,2) {badge:k "seL4" success}  {badge:l "Rust" primary}
| (0,3) {pill:f1 "F1 Editor" primary}  {pill:f2 "F2 Markout" success}
| (0,4) {label:status "System ready" success}
@end grid
@place:hero anchor:center

@grid:start cols:1 rows:1
| (0,0) {button:fab "Start" primary bind:toggle.menu}
@end grid
@place:start anchor:bottom-left

@grid:nav cols:3 rows:3
| (1,0) {button:up "^" bind:grid.up}
| (0,1) {button:left "<" bind:grid.left}
| (1,1) {label:pos "0,0" bind:grid.pos}
| (2,1) {button:right ">" bind:grid.right}
| (1,2) {button:down "v" bind:grid.down}
@end grid
@place:nav anchor:bottom-right
```

Three grids, three placers, zero hardcoded pixel coordinates.
