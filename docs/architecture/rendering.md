# Rendering Pipeline

One Markout document produces output on every surface.

## Pipeline

```
Markout → parse → VNode tree → renderer → output
```

1. **Parse**: `pst_markout::parse::parse(markout)` → `Vec<Line>`
2. **Render**: `pst_markout::render::render(&lines)` → `VNode`
3. **Output**: renderer walks the VNode tree

## Renderers

### pst-framebuffer (VGA pixels)

Renders to a 640x480 BGRA framebuffer. Components render as GUI elements with colored tabs, 3D buttons, progress bars, checkboxes.

### pst-terminal (ANSI)

Renders to ANSI escape sequences. Box-drawing characters for cards, colored text for buttons, brackets for inputs.

### html::to_html (HTML)

Produces static HTML with CSS classes. Same VNodes, different output format.

### VGA Console (vgacon)

Real-time character rendering to the framebuffer. Interprets ANSI cursor positioning, handles UTF-8, scrolls on overflow.

## VNode Structure

```rust
enum VNode {
    Element(VElement),  // tag, attrs, children
    Text(VText),        // content string
}
```

Attributes carry component data:
- `class` — component type + style (`mc-button mc-primary`)
- `data-bind` — state key
- `data-col` — grid column (`6,12`)
- `data-responsive` — breakpoints (`sm:12,12;lg:6,12`)
- `data-validate` — validation rules
- `data-animate` — animation type
- `data-href` — navigation target
- `data-popover` — tooltip text
- `data-config` — container configuration
- `data-editor` — editor marker
- `data-features` — editor toolbar features
