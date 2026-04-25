# Outconceive UI → PST OS: Full Port Plan

## What Exists in Outconceive UI (Browser)

### Components (18 types)
| Char | Type | PST Status |
|------|------|------------|
| L | Label | ✅ Implemented |
| I | Text Input | ✅ Implemented |
| P | Password Input | ✅ Implemented |
| B | Button | ✅ Implemented |
| C | Checkbox | ✅ Implemented |
| R | Radio | ❌ Missing |
| S | Select (dropdown) | ❌ Missing |
| T | Textarea | ❌ Missing |
| G | Image | ❌ Missing |
| K | Link | ❌ Missing |
| D | Divider | ✅ Implemented |
| _ | Spacer | ✅ Just added |
| W | Pill | ❌ Missing |
| J | Badge | ❌ Missing |
| Q | Progress | ❌ Missing |
| Z | Sparkline | ❌ Missing |
| X | Custom | ❌ Missing |
| . | Continuation | ❌ Missing |

### Styles (11 types)
| Char | Style | PST Status |
|------|-------|------------|
| p | Primary | ✅ Partial (button only) |
| s | Secondary | ❌ Missing |
| d | Danger | ✅ Partial (button only) |
| w | Warning | ❌ Missing |
| i | Info | ❌ Missing |
| k | Dark | ❌ Missing |
| l | Light | ❌ Missing |
| o | Outline | ❌ Missing |
| g | Ghost | ✅ Partial |
| 1-9 | Size classes | ❌ Missing |

### Layout
| Feature | PST Status |
|---------|------------|
| 12-column grid (col-6) | ❌ Missing |
| Custom grid (col-3[5]) | ❌ Missing |
| Responsive breakpoints (sm: md: lg: xl:) | ❌ Missing |
| Container config (padding, width, height, gap, cols) | ❌ Missing |
| Parametric constraints | ✅ Implemented |

### Containers (14 types)
| Container | PST Status |
|-----------|------------|
| @card | ✅ Implemented |
| @nav | ❌ Renders as div |
| @header | ❌ Renders as div |
| @footer | ❌ Renders as div |
| @section | ❌ Renders as div |
| @article | ❌ Missing |
| @aside | ❌ Missing |
| @form | ❌ Renders as div |
| @parametric | ✅ Implemented |
| @editor | ❌ Missing |
| @each | ❌ Missing |
| @heading | ❌ Missing |
| @list / @ordered-list | ❌ Missing |
| @quote / @code-block | ❌ Missing |

### Component Properties
| Property | PST Status |
|----------|------------|
| Binding (state key) | ✅ Implemented |
| Label text | ✅ Implemented |
| Style (primary, danger, etc.) | ✅ Partial |
| href (links) | ❌ Missing |
| validate (required, email, min:N) | ❌ Missing |
| animate (fade, slide) | ❌ Missing |
| popover | ❌ Missing |
| logic_ref (event handler) | ❌ Missing |
| col-N / col-N[M] | ❌ Missing |
| Responsive (sm: md: lg: xl:) | ❌ Missing |

### State System
| Feature | PST Status |
|---------|------------|
| Text state | ❌ Not in renderer |
| Bool state | ❌ Not in renderer |
| Number state | ❌ Not in renderer |
| List state (@each) | ❌ Missing |
| Dirty tracking | ❌ Missing |
| Reactive re-render | ❌ Missing (pst-ui has basic version) |

### Rendering
| Feature | PST Status |
|---------|------------|
| VNode tree | ✅ Implemented |
| HTML output | ✅ Implemented |
| ANSI terminal output | ✅ Implemented |
| Pixel framebuffer output | ✅ Implemented |
| VDOM diffing + patching | ❌ Missing |
| Incremental re-render | ❌ Missing |
| SSR (server-side render) | ✅ Implemented |

### Editor (@editor)
| Feature | PST Status |
|---------|------------|
| Rich text editing | ❌ Missing |
| Bold / italic / underline | ❌ Missing |
| Headings | ❌ Missing |
| Lists / ordered lists | ❌ Missing |
| Code blocks | ❌ Missing |
| Links / images | ❌ Missing |
| Toolbar | ❌ Missing |
| bind:key state binding | ❌ Missing |

---

## Implementation Plan

### Phase 1: Complete the Component Set
**Port the missing components to pst-markout parser and all three renderers.**

1. **Radio** — `{radio:choice "Option A"}` — circle + dot, group by key
2. **Select** — `{select:country "US,UK,CA"}` — dropdown box
3. **Textarea** — `{textarea:notes}` — multi-line input
4. **Link** — `{link:docs "Documentation" href:/pst/docs}` — clickable text
5. **Image** — `{image:logo "logo.bmp"}` — bitmap from disk
6. **Pill** — `{pill:tag "Rust"}` — rounded label
7. **Badge** — `{badge:count "3"}` — notification indicator
8. **Progress** — `{progress:loading "75"}` — progress bar
9. **Sparkline** — `{sparkline:cpu}` — inline chart (framebuffer only)
10. **Continuation** — `.` padding between components

### Phase 2: Complete the Style System
**Apply all style variants to all renderers.**

1. Map each style char to colors in pst-framebuffer (fill color, text color, border)
2. Map each style char to ANSI colors in pst-terminal
3. Implement size classes (1-9) as font scale or component width multipliers
4. Apply styles to ALL components, not just buttons

### Phase 3: Grid Layout
**Implement the 12-column grid system.**

1. Parse `col-N` and `col-N[M]` from Markout component syntax
2. In pst-framebuffer: divide container width by grid total (default 12), position components at `col * (width / total)`
3. In pst-terminal: divide terminal columns by grid total
4. Support row wrapping when cols exceed total

### Phase 4: Responsive Breakpoints
**Adapt layout to screen dimensions.**

1. Parse `sm:col-6 md:col-4 lg:col-3` syntax
2. Define breakpoints: sm=<640, md=<1024, lg=<1280, xl=>=1280
3. Select active breakpoint based on framebuffer width
4. Apply the matching col- value at render time

### Phase 5: Container Configs
**Support all container configuration options.**

1. Parse config string: `@card padding:24,max-width:400px`
2. Apply in framebuffer renderer: padding offsets, width clamping, height constraints
3. Support: `padding`, `width`, `max-width`, `height`, `max-height`, `gap`, `cols` (multi-column)

### Phase 6: State System
**Port Outconceive's StateStore to no_std.**

1. `StateStore` with text/bool/number/list values (use BTreeMap, no HashMap)
2. Dirty tracking — mark changed keys, re-render only affected rows
3. Wire state into the VNode render: `{input:name}` reads value from state
4. State-driven rendering: `{label:greeting "Hello {name}"}` interpolates state

### Phase 7: @each Lists
**Dynamic list rendering from state.**

1. Parse `@each:items` / `@end each`
2. Template lines between tags are repeated per list item
3. Scoped state: `items.0.name`, `items.1.name`
4. Add/remove items from state → re-render list
5. `{button:remove "×" danger}` with `logic_ref: "remove:items:0"`

### Phase 8: @editor Rich Text
**Rich text editor as a Markout directive.**

1. `@editor bold italic heading bind:notes`
2. Toolbar rendered from feature list
3. Content area with cursor, selection, text formatting
4. State binding: editor content syncs to state key
5. Framebuffer: render formatted text with bitmap font styles
6. Terminal: render with ANSI bold/italic

### Phase 9: Validation
**Input validation from Markout syntax.**

1. Parse `validate:required,email,min:8`
2. Validate on submit or on blur
3. Error display: red border, error message below field
4. Built-in validators: required, email, min, max, pattern

### Phase 10: Animations & Transitions
**Component animation from Markout syntax.**

1. Parse `animate:fade`, `animate:slide`
2. Framebuffer: interpolate opacity/position over frames
3. Terminal: instant (no animation support)

### Phase 11: Event System
**Component event handlers.**

1. Parse `logic:handler_fn` from component syntax
2. Click/submit events dispatch to named handlers
3. Handler registry maps names to callback functions
4. Popover: `popover:"Tooltip text"` — display on hover

### Phase 12: Disk-Driven Desktop
**Load all UI from Markout files on disk.**

1. `/pst/desktop.md` — desktop layout, window definitions, button bar
2. `/pst/theme.md` — color palette, font size, spacing
3. `/pst/form.md` — login form
4. `/pst/index.md` — page index (already exists)
5. Edit any file → desktop updates on next render

---

## Architecture After Port

```
Markout source (disk or inline)
        │
        ▼
    pst-markout parser (no_std)
    ├── All 18 component types
    ├── All 14 container types
    ├── Grid layout (col-N)
    ├── Responsive breakpoints
    └── Constraints (@parametric)
        │
        ▼
    pst-ui interaction layer (no_std)
    ├── State as parallel strings
    ├── Focus / hover / enabled columns
    ├── Tab order
    ├── Click → state update
    ├── Key → value update
    └── Dirty tracking → re-render
        │
        ▼
    Renderer (pluggable)
    ├── pst-framebuffer → VGA pixels
    ├── pst-terminal → ANSI sequences
    ├── html::to_html → HTML string
    └── (future) Outconceive WASM → DOM
```

Same Markout. Same parser. Same state model. Every surface.
