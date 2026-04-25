# Markout Syntax Reference

## Lines

```
| Content line — parsed as components
@container config
@end container
@each:listkey
@end each
```

## Components

```
{type:key "label" style prop1 prop2}
```

| Part | Required | Example |
|------|----------|---------|
| type | Yes | `input`, `button`, `label` |
| key | No | `:username`, `:submit` |
| label | No | `"Sign In"` |
| style | No | `primary`, `danger`, `ghost` |
| col | No | `col-6`, `col-3[5]` |
| responsive | No | `sm:col-12 lg:col-6` |
| validate | No | `validate:required,email` |
| animate | No | `animate:fade` |
| href | No | `href:/path` |
| route | No | `route:home` |
| fetch | No | `fetch:/api/data` |
| popover | No | `popover:"Help"` |
| constraints | No | `center-x:ref gap-y:16` |

## Component Types

`input` `password` `button` `checkbox` `radio` `select` `textarea` `image` `link` `label` `divider` `spacer` `pill` `badge` `progress` `sparkline`

## Styles

`primary` `secondary` `danger` `warning` `info` `dark` `light` `outline` `ghost` `1`-`9`

## Containers

`@card` `@nav` `@header` `@footer` `@section` `@aside` `@form` `@parametric` `@editor` `@each`

## Container Config

`padding:N` `width:N` `max-width:N` `height:N` `gap:N`

## Constraints (@parametric)

`center-x:ref` `center-y:ref` `left:ref` `right:ref` `top:ref` `bottom:ref` `gap-x:N` `gap-y:N` `gap-x:N:ref` `gap-y:N:ref` `width:ref` `height:ref`
