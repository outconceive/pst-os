# Event Properties Reference

Component properties that trigger actions.

## Navigation

| Property | Syntax | Description |
|----------|--------|-------------|
| `href` | `href:/path` | Navigate to path |
| `route` | `route:name` | Internal route |
| `fetch` | `fetch:/api/data` | Fetch data from URL |

## Display

| Property | Syntax | Description |
|----------|--------|-------------|
| `popover` | `popover:"Help text"` | Tooltip on hover |
| `animate` | `animate:fade` | Entry animation |

## Animation Types

| Type | Description |
|------|-------------|
| `fade` | Fade in |
| `slide` | Slide in |

## Data Attributes

Events flow to VNodes as data attributes:

| Attribute | Source |
|-----------|--------|
| `data-href` | `href:`, `route:`, `fetch:` |
| `data-popover` | `popover:` |
| `data-animate` | `animate:` |

Renderers read these attributes to handle navigation, tooltips, and transitions.
