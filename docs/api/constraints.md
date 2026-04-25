# Constraint Reference

Used inside `@parametric` containers.

## Positional Constraints

| Constraint | Description |
|------------|-------------|
| `center-x:ref` | Horizontal center aligns with ref |
| `center-y:ref` | Vertical center aligns with ref |
| `left:ref` | Left edge aligns with ref's left |
| `right:ref` | Right edge aligns with ref's right |
| `top:ref` | Top edge aligns with ref's top |
| `bottom:ref` | Bottom edge aligns with ref's bottom |

## Gap Constraints

| Constraint | Description |
|------------|-------------|
| `gap-x:N` | N pixels right of previous component |
| `gap-y:N` | N pixels below previous component |
| `gap-x:N:ref` | N pixels right of ref |
| `gap-y:N:ref` | N pixels below ref |

Gap values support units: `gap-y:1rem`, `gap-x:8px`.

## Size Constraints

| Constraint | Description |
|------------|-------------|
| `width:ref` | Match ref's width |
| `height:ref` | Match ref's height |

## Stretching

Combine `left:` and `right:` to stretch between two references:

```
@parametric
| {label:a "Left"}
| {label:b "Right" gap-x:200:a}
| {divider:line left:a right:b}
@end parametric
```

## Solver

The solver is a topological sort. It processes constraints in dependency order and computes absolute positions. Cycles are detected and broken — the watchdog tombstones them.
