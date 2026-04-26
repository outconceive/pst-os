# Spreadsheet Grid: Directional Computation for Markout

## Core Idea

Cells have values. Cells have behaviors. A behavior is a traversal:
an operation, a direction, and a termination condition. The formula
doesn't reference cell addresses — it walks the grid spatially.

This is not Excel. Excel says "=SUM(A1:A10)". Markout says
"sum upward until empty." The formula is a direction, not a coordinate.

## Syntax

```
@sheet:<name> cols:<N> rows:<M>
| (<col>,<row>) <value>
| (<col>,<row>) {compute <op>:<direction> until:<condition>}
@end sheet
```

## Operations

```
sum           — accumulate by addition
product       — accumulate by multiplication
count         — count cells traversed
min           — smallest value
max           — largest value
avg           — average (sum / count)
first         — first non-empty value found
last          — last non-empty value before termination
concat        — concatenate text values
and           — logical AND (all true)
or            — logical OR (any true)
```

## Directions

```
up            — walk upward (decreasing row)
down          — walk downward (increasing row)
left          — walk left (decreasing col)
right         — walk right (increasing col)
up-left       — diagonal
up-right      — diagonal
down-left     — diagonal
down-right    — diagonal
all-up        — all cells above (same as up, but reads better)
all-left      — all cells to the left
```

## Termination Conditions

```
until:empty       — stop at first empty cell
until:zero        — stop at first zero value
until:text        — stop at first non-numeric cell
until:edge        — stop at grid boundary
until:value:<v>   — stop at specific value
until:style:<s>   — stop at cell with specific style (e.g., header)
until:N           — stop after N cells
```

## Example: Financial Report

```
@sheet:report cols:5 rows:7
| (0,0) "" primary      (1,0) "Q1" primary  (2,0) "Q2" primary  (3,0) "Q3" primary  (4,0) "Q4" primary
| (0,1) "Revenue"       (1,1) 100           (2,1) 150           (3,1) 200           (4,1) 180
| (0,2) "COGS"          (1,2) 40            (2,2) 55            (3,2) 70            (4,2) 65
| (0,3) "OpEx"          (1,3) 25            (2,3) 30            (3,3) 35            (4,3) 40
| (0,4) "Profit"        (1,4) {compute sum:up until:text}
|                        (2,4) {compute sum:up until:text}
|                        (3,4) {compute sum:up until:text}
|                        (4,4) {compute sum:up until:text}
| (0,5) ""
| (0,6) "Yearly"        (1,6) {compute sum:right until:edge}
```

Cell (1,4): walks up from (1,3)=25, (1,2)=40, (1,1)=100. Hits (1,0)="Q1"
which is text → stops. Sum = 25+40+100 = 165. But wait — COGS and OpEx are
costs, so this needs sign handling. See "signed operations" below.

Cell (1,6): walks right from (2,6), (3,6), (4,6). But those are empty.
This means you'd put the Profit row values there, or reference them. See
"cross-reference" below.

## Signed Values / Subtraction

Prefix a value with `-` to indicate subtraction in a sum:

```
| (0,1) "Revenue"       (1,1) 100
| (0,2) "COGS"          (1,2) -40
| (0,3) "OpEx"          (1,3) -25
| (0,4) "Profit"        (1,4) {compute sum:up until:text}
```

Sum: 100 + (-40) + (-25) = 35. The sign is part of the value, not
the formula.

## Cross-Reference

Sometimes you need one cell to reference another specific cell:

```
| (1,6) {compute ref:(1,4)}
```

`ref:(col,row)` reads the computed value of another cell.
This is the escape hatch for when directional traversal isn't enough.
Use sparingly — the power of this system is that most formulas
DON'T need explicit cell references.

## Multi-Directional

A cell can combine directions:

```
| (2,2) {compute sum:up until:text + sum:left until:text}
```

Sum of everything above AND everything to the left. The `+` operator
combines the results of two traversals.

## Conditional Values

```
| (1,4) {compute sum:up until:text if:>0}
```

`if:>0` filters — only include cells where the value is > 0.
Other conditions: `if:>=100`, `if:<0`, `if:!=0`.

## Recursive / Cascading

When a compute cell references another compute cell in its traversal,
the referenced cell evaluates first. This creates a dependency graph —
same as the constraint solver in PST OS.

```
| (1,1) 10
| (1,2) 20
| (1,3) {compute sum:up until:edge}       // = 10+20 = 30
| (1,4) 5
| (1,5) {compute sum:up until:edge}       // = 5+30+20+10 = 65
```

Cell (1,5) walks up, hits (1,4)=5, then (1,3) which is a compute cell.
(1,3) evaluates first → 30. Then (1,5) continues: 5+30+20+10 = 65.

The compute walks up, evaluating dependencies as it encounters them.
This is topological sort on the cell dependency graph — the same
algorithm that solves process scheduling in PST OS.

## Formatting

Cells inherit Markout styles and can specify format:

```
| (1,4) {compute sum:up until:text format:currency}
| (2,4) {compute sum:up until:text format:percent}
```

Formats:
```
currency      — $1,234.56
percent       — 45.2%
decimal:N     — N decimal places
int           — integer
bytes         — auto B/KB/MB
comma         — 1,234,567
```

## Cell Styles

```
| (0,0) "Header" primary bold
| (1,4) {compute sum:up until:text} success bold
```

Cells with negative computed values can auto-style:

```
| (1,4) {compute sum:up until:text neg:danger}
```

`neg:danger` applies the danger style if the result is negative.

## Binding to State

```
@sheet:budget cols:3 rows:3 bind:budget
| @col:label type:text
| @col:planned type:number
| @col:actual type:number
```

When bound to state, the sheet reads from and writes to the state store.
Editing a cell updates the state key. Compute cells react to changes.
Same reactive model as Markout components.

## Editable Cells

```
| (1,1) {input:revenue type:number value:100}
```

A cell can be an input field. Changing the value triggers recomputation
of all dependent compute cells. The spreadsheet is live.

## Parallel String Interpretation

```
Sheet:     report
Col 0:     ["", "Revenue", "COGS", "OpEx", "Profit"]
Col 1:     [header, 100, -40, -25, {sum:up:text}]
Col 2:     [header, 150, -55, -30, {sum:up:text}]
Col 3:     [header, 200, -70, -35, {sum:up:text}]
Col 4:     [header, 180, -65, -40, {sum:up:text}]
```

Each column is a parallel string. The compute cells are formulas
embedded in the string — they evaluate by walking the same column
(or crossing to another) and accumulating.

The spreadsheet IS parallel strings where some cells contain
values and other cells contain traversal instructions over the
same strings. The data and the computation live in the same
table. No separate formula layer. No separate cell reference
namespace. Just strings with values and strings with walks.

## Dependency Resolution

When the sheet renders:

1. Scan all cells for compute instructions
2. Build dependency graph (which cells reference which)
3. Topological sort (same algorithm as process scheduler)
4. Evaluate in dependency order
5. Render results

If a cycle is detected (A depends on B depends on A), mark both
cells as `#CYCLE` — same as the scheduler's CycleAction::Break.

## Composability

```
@card
| {label:title "Q4 Budget" primary}
| {divider:d}
| @sheet:q4 cols:3 rows:5
| | (0,0) "" primary       (1,0) "Planned" primary  (2,0) "Actual" primary
| | (0,1) "Revenue"        (1,1) 500                (2,1) 480
| | (0,2) "Costs"          (1,2) -200               (2,2) -220
| | (0,3) "Profit"         (1,3) {compute sum:up until:text}
| |                        (2,3) {compute sum:up until:text}
| | (0,4) "Variance"       (1,4) {compute ref:(2,3) - ref:(1,3)}
| @end sheet
| {divider:d2}
| @pie:breakdown radius:40
| | {slice:rev value:{ref:q4:(2,1)} success "Revenue"}
| | {slice:cost value:{ref:q4:(2,2)} danger "Costs"}
| @end pie
@end card
```

A card containing a spreadsheet AND a pie chart that reads
from the spreadsheet cells. All Markout. All parallel strings.
The pie chart's slice values are cross-references to the sheet.

## Why Not Excel

Excel addresses cells: `=SUM(B2:B4)`. You must know that B is
column 2 and rows 2-4 contain costs. The formula encodes position.

Markout describes traversal: `sum:up until:text`. You declare
intent — "add everything above me until you hit a header."
The formula encodes direction and termination, not position.

This means:
- Insert a row above → formula still works (it walks, not references)
- Move the total row → formula still works (it walks from wherever it is)
- Add a column → other columns' formulas still work (they walk their own column)

Position-independent formulas. The same principle as PST OS:
identity is position, but behavior is relative, not absolute.
