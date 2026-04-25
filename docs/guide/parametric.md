# Parametric Layout

The `@parametric` container positions components using constraints instead of a grid. Components declare relationships to each other, and the solver computes positions.

## Basic Example

```
@parametric
| {label:title "Dashboard"}
| {input:search center-x:title gap-y:1rem}
| {button:go "Search" primary after:search gap-x:8px center-y:search}
@end parametric
```

`center-x:title` means "my horizontal center aligns with title's horizontal center."

## Constraint Vocabulary

| Constraint | Meaning |
|------------|---------|
| `center-x:ref` | Horizontally centered on ref |
| `center-y:ref` | Vertically centered on ref |
| `left:ref` | Left edge aligns with ref's left |
| `right:ref` | Right edge aligns with ref's right |
| `top:ref` | Top edge aligns with ref's top |
| `bottom:ref` | Bottom edge aligns with ref's bottom |
| `gap-x:N` | N pixels of horizontal gap after previous |
| `gap-y:N` | N pixels of vertical gap after previous |
| `gap-x:N:ref` | N pixels right of ref |
| `gap-y:N:ref` | N pixels below ref |
| `width:ref` | Match ref's width |
| `height:ref` | Match ref's height |

## The Solver

The constraint solver is a topological sort. It resolves dependencies between components and computes absolute positions. This is the same algorithm that solves process scheduling — temporal constraints (process A after B) and spatial constraints (button below input) are both DAGs.

## Why Not CSS

CSS uses a box model with flow, flexbox, and grid — complex layout systems that require runtime measurement. Parametric constraints are declarative relationships resolved in one pass. No reflow, no measurement, no layout engine. Just a topological sort.
