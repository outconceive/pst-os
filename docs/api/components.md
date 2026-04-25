# Component Type Reference

| Char | Type | Tag | Syntax |
|------|------|-----|--------|
| L | Label | `span` | `{label:key "text"}` |
| I | Input | `input` | `{input:key}` |
| P | Password | `input` | `{password:key}` |
| B | Button | `button` | `{button:key "label" style}` |
| C | Checkbox | `input` | `{checkbox:key}` |
| R | Radio | `input` | `{radio:key "option"}` |
| S | Select | `select` | `{select:key "a,b,c"}` |
| T | Textarea | `textarea` | `{textarea:key}` |
| G | Image | `img` | `{image:key "file.bmp"}` |
| K | Link | `a` | `{link:key "text" href:/path}` |
| D | Divider | `hr` | `{divider:key}` |
| _ | Spacer | `span` | `{spacer:key}` |
| W | Pill | `span` | `{pill:key "tag"}` |
| J | Badge | `span` | `{badge:key "3"}` |
| Q | Progress | `div` | `{progress:key "75"}` |
| Z | Sparkline | `svg` | `{sparkline:key}` |

## Framebuffer Rendering

| Component | Appearance |
|-----------|------------|
| Input | Blue left tab, dark field, white cursor |
| Password | Red left tab, dark field, masked text |
| Checkbox | Green left tab, checkbox square |
| Radio | Purple left tab, circle |
| Select | Amber left tab, dropdown arrow |
| Textarea | Blue left tab, multi-line box |
| Button | Colored rectangle with 3D edges |
| Link | Blue underlined text |
| Pill | Dark rounded label |
| Badge | Red filled label |
| Progress | Blue filled bar |
| Sparkline | Mini chart with green line |
| Image | Gray placeholder |
| Divider | Gray horizontal line |
| Spacer | Empty gap |
