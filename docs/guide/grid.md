# Grid Layout

Markout uses a 12-column grid for horizontal layout.

## Basic Grid

```
| {input:first col-6}  {input:last col-6}
```

`col-6` = span 6 of 12 columns = 50% width.

## Common Patterns

```
| {input:name col-12}
| {input:email col-8}  {button:go "Go" col-4}
| {input:a col-4}  {input:b col-4}  {input:c col-4}
```

## Custom Grid

Not limited to 12 columns:

```
| {input:name col-3[5]}
```

`col-3[5]` = span 3 of 5 columns = 60% width.

## Responsive Breakpoints

Adapt layout to screen size:

```
| {input:name sm:col-12 md:col-6 lg:col-4}
```

| Breakpoint | Width | Typical use |
|------------|-------|-------------|
| `sm:` | < 640px | Mobile — stack vertically |
| `md:` | < 1024px | Tablet — two columns |
| `lg:` | < 1280px | Desktop — three columns |
| `xl:` | >= 1280px | Wide — four columns |

The renderer selects the matching breakpoint based on the display width. On a 640x480 VGA framebuffer, `sm:` rules apply.

## Grid + Styles

Grid and styles combine naturally:

```
| {button:cancel "Cancel" ghost col-6}  {button:submit "Submit" primary col-6}
```
