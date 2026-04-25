# Parallel Strings

The core primitive of PST OS. Every resource is four equal-length strings.

## The Model

```
content:    "Username  ________  Login "
components: "LLLLLLLLLLIIIIIIIIIIBBBBBB"
state_keys: "__________username__submit"
styles:     "                    pppppp"
```

A component is a column slice. Its identity is its character offset. This never changes.

## Why Not Trees

Trees require:
- Pointer chasing for traversal
- Rebalancing on mutation
- Cascading updates on structural change
- Generated keys for identity

Parallel strings need none of this.

| Operation | Tree | Parallel Strings |
|-----------|------|-----------------|
| Insert | O(log n) + rebalance | O(1) append |
| Lookup | O(log n) | O(1) offset table |
| Delete | O(log n) + cascade | O(1) tombstone |
| Range query | O(log n + k) | O(n) scan |

## Applied Everywhere

| Domain | Parallel Strings |
|--------|-----------------|
| UI components | content, components, state_keys, styles |
| Process table | name, state, priority, affinity |
| Filesystem | name, content, owner, flags |
| IPC messages | sender, receiver, priority, payload |
| Scheduler | dependencies, deadlines, rates |
| Memory | start, length, owner, status |
| Time | tick, delta, retention, compaction |
