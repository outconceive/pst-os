# Your First Markout

Markout is the declarative UI language of PST OS. No JSX, no transpiler, no build step.

## Hello World

```
| Hello World
```

That's it. The `|` prefix marks a content line. The text renders as a label.

## Adding Components

```
| {input:name}  What's your name?
| {button:greet "Say Hello" primary}
```

Components use curly braces: `{type:key "label" style}`.

## A Card

```
@card
| Welcome
| {input:email validate:required,email}  Email
| {password:pass validate:min:8}  Password
| {checkbox:remember}  Remember me
| {button:login "Sign In" primary}
@end card
```

## A Form with Grid Layout

```
@card padding:24,max-width:500px
| {input:first col-6}  {input:last col-6}
| {input:email col-12 validate:required,email}
| {button:submit "Register" primary col-4}
@end card
```

`col-6` means span 6 of 12 columns. Components on the same line lay out horizontally.

## Parametric Layout

```
@parametric
| {label:title "Dashboard"}
| {input:search center-x:title gap-y:1rem}
| {button:go "Search" primary after:search gap-x:8px}
@end parametric
```

Components position themselves relative to each other using constraints. The solver computes absolute positions.

## Where It Renders

The same Markout renders on every target:

| Target | Renderer | Output |
|--------|----------|--------|
| VGA | pst-framebuffer | Pixels with colored tabs |
| Terminal | pst-terminal | ANSI escape sequences |
| Browser | Outconceive WASM | DOM elements |
| Serial | pst-terminal | Serial console |
| SSR | html::to_html | Static HTML |
