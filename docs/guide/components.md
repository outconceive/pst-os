# Components

Markout has 18 component types. All use the syntax `{type:key "label" style}`.

## Form Inputs

### Text Input
```
| {input:name}  Name
```
Blue left tab. Accepts text.

### Password
```
| {password:pass}  Password
```
Red left tab. Masks input as `***`.

### Textarea
```
| {textarea:notes}  Notes
```
Blue left tab. Multi-line text area.

### Checkbox
```
| {checkbox:agree}  I agree
```
Green left tab. Toggle with click or space.

### Radio
```
| {radio:color "Red"}  {radio:color "Blue"}
```
Purple left tab. Grouped by key.

### Select
```
| {select:country "US,UK,CA"}
```
Amber left tab. Dropdown.

## Actions

### Button
```
| {button:submit "Sign In" primary}
```
Styled button. Supports all style variants.

### Link
```
| {link:docs "Documentation" href:/pst/docs}
```
Blue underlined text. Navigates on click.

## Display

### Label
```
| {label:title "Dashboard"}
| Plain text is also a label
```

### Badge
```
| {badge:count "3"}
```
Red notification indicator.

### Pill
```
| {pill:tag "Rust"}
```
Rounded label.

### Progress
```
| {progress:loading "75"}
```
Blue filled bar. Value is percentage.

### Sparkline
```
| {sparkline:cpu}
```
Inline mini chart.

### Image
```
| {image:logo "logo.bmp"}
```
Bitmap placeholder.

## Layout

### Divider
```
| {divider:sep}
```
Horizontal rule.

### Spacer
```
| {button:a "Left"}  {spacer:gap}  {button:b "Right"}
```
Empty space between components.

## Component Properties

All components support these properties:

| Property | Example | Description |
|----------|---------|-------------|
| key | `input:name` | State binding key |
| label | `"Submit"` | Display text |
| style | `primary` | Visual style |
| col-N | `col-6` | Grid column span |
| validate | `validate:required,email` | Input validation |
| animate | `animate:fade` | Entry animation |
| href | `href:/path` | Navigation target |
| popover | `popover:"Help text"` | Hover tooltip |
