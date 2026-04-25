# Styles

Append a style name to any component to change its appearance.

## Style Variants

```
| {button:a "Primary" primary}
| {button:b "Secondary" secondary}
| {button:c "Danger" danger}
| {button:d "Warning" warning}
| {button:e "Info" info}
| {button:f "Dark" dark}
| {button:g "Light" light}
| {button:h "Outline" outline}
| {button:i "Ghost" ghost}
```

| Style | Color | Use |
|-------|-------|-----|
| `primary` | Blue (#3B82F6) | Main actions |
| `secondary` | Gray (#6B7280) | Secondary actions |
| `danger` | Red (#EF4444) | Destructive actions |
| `warning` | Amber (#F59E0B) | Warnings |
| `info` | Cyan (#06B6D4) | Information |
| `dark` | Near-black (#1E1E1E) | Dark theme elements |
| `light` | Light gray (#E5E7EB) | Light theme elements |
| `outline` | Gray border | Outlined style |
| `ghost` | Transparent (#374151) | Minimal style |

## Size Classes

```
| {label:small "Small text" 2}
| {label:normal "Normal text" 5}
| {label:large "Large text" 8}
```

Size `1` through `9`. Default is `5`.

## Styles Apply to All Components

Styles aren't just for buttons — they work on labels, pills, badges, inputs, and any component:

```
| {label:status "Online" info}
| {pill:tag "Rust" primary}
| {badge:alerts "5" danger}
```
