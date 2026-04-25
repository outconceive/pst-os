# State & Reactivity

Markout components bind to state keys. When state changes, the UI updates.

## State Types

| Type | Example | Description |
|------|---------|-------------|
| Text | `"Alice"` | String values |
| Number | `42.0` | Numeric values |
| Bool | `true/false` | Toggle values |
| Null | `""` | Empty/unset |

## Binding

```
| {input:username}  Username
```

The `username` key binds the input to state. Typing updates the state value.

## Checkboxes

```
| {checkbox:agree}  I agree to the terms
```

Space or click toggles between `true` and `false`.

## Dirty Tracking

The state system tracks which keys changed since the last render. Only dirty components re-render. Setting a key to its current value doesn't mark it dirty.

## Lists

```
@each:items
| {label:name}  {button:remove "x" danger}
@end each
```

List state uses scoped keys: `items.0.name`, `items.1.name`. Add items with `add_list_item`, remove with `remove_list_item`. The template repeats for each item.
