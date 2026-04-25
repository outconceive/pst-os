# Lists (@each)

The `@each` container repeats a template for each item in a list.

## Syntax

```
@each:todos
| {checkbox:done}  {label:text}  {button:remove "x" danger}
@end each
```

## How It Works

1. The state stores list items as scoped keys: `todos.0.text`, `todos.1.text`
2. `@each:todos` reads `todos._count` to know how many items
3. The template lines repeat for each item
4. Each repetition accesses its scoped state

## Adding Items

```rust
state.add_list_item("todos", &[
    ("text", StateValue::Text("Buy milk")),
    ("done", StateValue::Bool(false)),
]);
```

## Removing Items

```rust
state.remove_list_item("todos", 0);
```

Items shift down automatically — item 1 becomes item 0.
