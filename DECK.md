# Deck: Presentations for Markout

## Core Idea

A presentation is a sequence of slides. Each slide is a Markout
document. Slide order is a parallel string. Transitions between
slides are another column. Speaker notes are another column.
Content is just Markout — components, containers, styles, grids,
charts, diagrams, anything.

No slide masters. No animation panes. No XML namespaces.
Just slides as rows with content and metadata as columns.

## Syntax

```
@deck:<name>
| @slide:<key> transition:<type> notes:"Speaker notes here"
|   {content}
| @slide:<key> transition:<type>
|   {content}
@end deck
```

## Example

```
@deck:talk
| @slide:intro transition:fade duration:500
|   {label:title "PST OS" primary xl}
|   {label:sub "Parallel String Theory" muted lg}
|   {spacer:s}
|   {badge:conf "RustConf 2026" primary}
|   {label:author "S. Seto" muted}
| @slide:problem transition:slide-left
|   {label:t "The Problem" primary lg}
|   {divider:d}
|   {label:p1 "Three data structures that must stay synchronized:"}
|   {pill:a "Adjacency Matrix" danger}
|   {pill:w "Weight Matrix" danger}
|   {pill:e "Embedding Table" danger}
|   {spacer:s}
|   {label:p2 "Every operation touches all three." muted}
| @slide:solution transition:zoom
|   {label:t "The Solution" primary lg}
|   {divider:d}
|   {label:s1 "One table. No synchronization."}
|   {spacer:s}
|   @table:psn
|   | @col:entity header:"Entity" type:text width:12
|   | @col:feat header:"Features" type:text width:16
|   | @col:topo header:"Topology" type:text width:16
|   | @row "cat", "0.92, 0.10", "near:dog"
|   | @row "dog", "0.89, 0.12", "near:cat"
|   @end table
| @slide:demo transition:slide-up notes:"Run the demo live here"
|   {label:t "Live Demo" primary lg}
|   {code:snippet language:rust "let table = ParallelTable::new();"}
|   {spacer:s}
|   {progress:demo}
|   {label:status "Running..." success}
| @slide:end transition:fade
|   {label:t "Thank You" primary xl}
|   {spacer:s}
|   {link:repo "github.com/outconceive/pst-os" primary}
|   {link:paper "doi.org/10.5281/zenodo.xxxxx" muted}
@end deck
```

## Slide Transitions

```
fade              — opacity crossfade
slide-left        — new slide enters from right
slide-right       — new slide enters from left
slide-up          — new slide enters from bottom
slide-down        — new slide enters from top
zoom              — new slide scales up from center
zoom-out          — old slide scales down, reveals new
flip              — 3D card flip (horizontal)
none              — instant switch (default)
```

Each transition uses the @transition system under the hood:
```
slide-left = @transition property:x from:640 to:0 curve:ease-out
fade = @transition property:opacity from:0 to:1 curve:ease-in-out
```

Optional duration override per slide:
```
| @slide:key transition:fade duration:800
```

Default duration: 400ms.

## Slide Properties

```
@slide:<key>
  transition:<type>       — entry transition
  duration:<ms>           — transition duration
  notes:"text"            — speaker notes (not rendered on slide)
  bg:<color>              — background color override
  layout:center           — vertically center all content
  layout:top              — top-align (default)
  auto:<ms>               — auto-advance after N milliseconds
```

## Fragments (Build Steps)

Content within a slide can appear incrementally:

```
| @slide:points transition:fade
|   {label:t "Key Points" primary lg}
|   {label:p1 "First point" fragment:1}
|   {label:p2 "Second point" fragment:2}
|   {label:p3 "Third point" fragment:3}
```

`fragment:N` means the element is hidden until build step N.
Pressing next within a slide reveals fragments in order before
advancing to the next slide.

Fragment transitions:
```
fragment:1                    — fade in (default)
fragment:2 fragment-type:fly-left   — fly in from left
fragment:3 fragment-type:zoom       — scale up
```

## Navigation

### Keyboard
```
Space / Right / Down / Enter  — next slide (or next fragment)
Left / Up / Backspace         — previous slide
Home                          — first slide
End                           — last slide
F                             — toggle fullscreen
N                             — toggle speaker notes
G                             — go to slide number (type number, press Enter)
```

### Mouse
```
Click                         — next slide
Right-click                   — previous slide
```

### Touch
```
Swipe left                    — next slide
Swipe right                   — previous slide
```

## Speaker Notes

```
| @slide:data transition:fade notes:"Explain the three-structure problem. Emphasize that GNNs maintain adjacency, weights, and embeddings separately."
```

Notes are a parallel string column — same row as the slide,
different column. Rendered in a separate panel (presenter view)
or toggled with N key.

Presenter view shows:
- Current slide (large)
- Next slide (small preview)
- Speaker notes (text)
- Elapsed time
- Slide number / total

## Slide Layouts

### Default (top-aligned)
Content flows from top to bottom, left-aligned.

### Centered
```
| @slide:title layout:center
|   {label:t "Big Title" primary xl}
```
All content vertically and horizontally centered.

### Split
```
| @slide:comparison layout:split
|   @left
|     {label:t "Before" danger}
|     {code:old "HashMap + Vec"}
|   @right
|     {label:t "After" success}
|     {code:new "ParallelTable"}
@end split
```
Two-column layout with `@left` and `@right`.

### Grid
```
| @slide:gallery layout:grid cols:3
|   {image:a "screenshot1.bmp"}
|   {image:b "screenshot2.bmp"}
|   {image:c "screenshot3.bmp"}
```
Content arranged in a grid.

### Parametric
Any slide can use `@parametric` for constraint-solved positioning:
```
| @slide:arch
|   @parametric
|   | {label:kernel "seL4" primary}
|   | {label:init "init" gap-y:2rem center-x:kernel}
|   | {label:vfs "vfs" gap-y:1rem left:kernel}
|   | {label:net "netd" gap-y:1rem right:kernel}
|   @end parametric
```

## Themes

```
@deck:talk theme:dark
```

Themes:
```
dark          — dark background, light text (default)
light         — white background, dark text
terminal      — green on black, monospace
paper         — sepia, serif feel
```

Or custom:
```
@deck:talk bg:15,15,22 fg:210,210,215 accent:59,130,246
```

## Export Targets

The same @deck renders to:

### Framebuffer (PST OS)
Full-screen slides rendered to VGA. Arrow keys navigate.
Transitions use rdtsc timing. Fragments build step by step.

### Terminal (pst-terminal)
Text-mode slides. Transitions degrade to instant switch.
Content renders as ANSI. Navigate with arrow keys.

### HTML
Each slide is a `<section>`. CSS handles transitions.
Similar to reveal.js but generated from Markout.
Presenter mode in a separate window.

### PDF
Each slide is a page. Fragments expand to one page per
build step. Transitions don't apply. Static export.

## Parallel String Interpretation

```
Deck:        talk
Slide 0:     key=intro     transition=fade      content=[title,sub,badge]  notes=""
Slide 1:     key=problem   transition=slide-left content=[t,d,p1,pills]   notes=""
Slide 2:     key=solution  transition=zoom       content=[t,d,s1,table]   notes=""
Slide 3:     key=demo      transition=slide-up   content=[t,code,progress] notes="Run demo"
Slide 4:     key=end       transition=fade       content=[t,links]        notes=""
```

Five rows. Each row is a slide. Columns:
- Key (identity)
- Transition (how it enters)
- Content (Markout components)
- Notes (speaker text)
- Duration, background, layout (optional)

Slides are parallel strings. Reorder slides by reordering rows.
Change a transition without touching content. Change content
without touching transitions. Add notes without touching either.
Each concern is an independent column.

## Why Not PowerPoint

PowerPoint has:
- Slide masters, layouts, placeholders
- Animation pane with entrance/exit/emphasis/motion paths
- SmartArt with 200+ diagram types
- Theme XML with 50+ configurable elements
- Embedded OLE objects
- VBA macros
- 30 years of backwards compatibility

Markout has:
- Slides are rows
- Content is Markout
- Transitions are a column
- Notes are a column
- Fragments are numbered attributes

Same expressive power for 95% of presentations.
Zero accidental complexity.

## Composability

Everything composes. A slide can contain:
- Components (labels, buttons, badges, pills)
- Containers (cards, grids, parametric)
- Charts (pie, bar, line, sparkline)
- Diagrams (diagram grid with shapes and connectors)
- Tables (columnar with compute cells)
- Code blocks (with syntax highlighting)
- Parallax (background effects)
- Transitions (per-element animation)

A presentation about PST OS can contain a live process table
that reads from state, a pie chart of memory usage, and a
diagram of the architecture — all in Markout, all in the
same deck, all as parallel strings.
