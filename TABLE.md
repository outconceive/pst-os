# Columnar Table for Markout

## Core Idea

Define columns, not rows. Each column declares its header, data type,
format, width, and alignment. Data fills in row by row but the schema
is column-first. This is the parallel string principle applied to
tabular data — each column IS a parallel string.

## Syntax

```
@table:<name>
| @col:<key> header:"Label" type:<type> format:<fmt> width:<n> align:<align>
| @row <val>, <val>, <val>
| @row <val>, <val>, <val>
@end table
```

## Example

```
@table:processes
| @col:pid header:"PID" type:number format:int width:6 align:right
| @col:name header:"Name" type:text width:16 align:left
| @col:state header:"State" type:badge width:12
| @col:cpu header:"CPU" type:progress width:20
| @col:mem header:"Memory" type:text format:bytes width:10 align:right
| @row 0, "init", "running" success, 12, 4096
| @row 1, "cryptod", "running" success, 8, 2048
| @row 2, "vfs", "blocked" warning, 0, 8192
| @row 3, "netd", "ready" primary, 3, 1024
| @row 4, "driverd", "tombstoned" danger, 0, 512
@end table
```

## Column Types

```
text          — plain string
number        — numeric, respects format
badge         — rendered as colored badge, value = "text" style
pill          — rendered as pill
progress      — rendered as progress bar, value = 0-100
sparkline     — rendered as inline sparkline, value = comma-separated points
checkbox      — rendered as checkbox, value = true/false
icon          — rendered as icon, value = icon name
link          — rendered as clickable link
```

## Formats (for number type)

```
int           — integer, no decimals (default)
decimal:N     — N decimal places
percent       — append %
bytes         — auto-format as B/KB/MB/GB
duration      — auto-format as ms/s/m/h
hex           — hexadecimal with 0x prefix
```

## Column Properties

```
header:"Label"     — column header text
type:<type>        — data type (determines renderer)
format:<fmt>       — display format
width:<n>          — width in characters (or auto)
align:left/center/right  — text alignment
sort:asc/desc/none — default sort direction
style:<style>      — default Markout style for cells
hide:true          — column exists in data but not rendered
pin:left/right     — sticky column on scroll
```

## Features

### Sorting

```
@table:data sortable:true
| @col:name header:"Name" type:text sort:asc
| @col:age header:"Age" type:number sort:none
```

Clicking a column header toggles sort. The `sort:` property
sets the initial sort. Bind to state for reactive sorting.

### Filtering

```
@table:data filterable:true
| @col:name header:"Name" type:text filter:text
| @col:state header:"State" type:badge filter:select
```

`filter:text` adds a text search box below the header.
`filter:select` adds a dropdown of unique values.

### Row Binding

```
@table:tasks bind:todos
| @col:done header:"" type:checkbox bind:done
| @col:text header:"Task" type:text bind:text
| @col:remove header:"" type:button label:"x" danger bind:remove
@end table
```

When bound to a list state key, rows auto-populate from
`todos.0.done`, `todos.0.text`, `todos.1.done`, etc.
Same as `@each` but structured as a table.

### Pagination

```
@table:logs page-size:20 paginate:true
```

Shows 20 rows at a time with prev/next controls.

### Striping

```
@table:data striped:true
```

Alternating row background colors.

### Selection

```
@table:files selectable:true
| @col:check header:"" type:checkbox width:4
| @col:name header:"File" type:text
```

Clicking a row selects it. Multiple selection with checkboxes.

## Computed Columns

```
| @col:total header:"Total" type:number format:decimal:2 compute:"price * qty"
```

A column whose value is derived from other columns in the same row.
Expression evaluates left to right with basic arithmetic.

## Footer Aggregates

```
| @col:amount header:"Amount" type:number format:decimal:2 footer:sum
| @col:count header:"Count" type:number footer:count
| @col:avg header:"Score" type:number format:decimal:1 footer:avg
```

Footer functions: `sum`, `count`, `avg`, `min`, `max`.
Rendered as a summary row at the bottom.

## Parallel String Interpretation

This is the most literal application of parallel strings.
Each column IS a parallel string:

```
Table:     processes
Col 0:     pid    = [0, 1, 2, 3, 4]
Col 1:     name   = ["init", "cryptod", "vfs", "netd", "driverd"]
Col 2:     state  = ["running", "running", "blocked", "ready", "tombstoned"]
Col 3:     cpu    = [12, 8, 0, 3, 0]
Col 4:     mem    = [4096, 2048, 8192, 1024, 512]
```

Five parallel strings. Same length. Positional identity —
row 0 across all columns refers to the same entity.
This IS the PST parallel table. The columnar table Markout
component is a visual renderer for ParallelTable.

Sorting reorders the position mapping (like OffsetTable).
Filtering tombstones rows (like ParallelTable.tombstone).
Pagination is a window over the offset range.

The table component doesn't just USE parallel strings —
it IS a parallel string table rendered to pixels.

## Framebuffer Rendering

```
┌──────┬──────────────────┬──────────────┬──────────────────────┬────────────┐
│  PID │ Name             │ State        │ CPU                  │    Memory  │
├──────┼──────────────────┼──────────────┼──────────────────────┼────────────┤
│    0 │ init             │ running      │ ████████░░░░░░░░░░░░ │    4.0 KB  │
│    1 │ cryptod          │ running      │ ██████░░░░░░░░░░░░░░ │    2.0 KB  │
│    2 │ vfs              │ blocked      │ ░░░░░░░░░░░░░░░░░░░░ │    8.0 KB  │
│    3 │ netd             │ ready        │ ██░░░░░░░░░░░░░░░░░░ │    1.0 KB  │
│    4 │ driverd          │ tombstoned   │ ░░░░░░░░░░░░░░░░░░░░ │      512 B │
├──────┴──────────────────┴──────────────┴──────────────────────┴────────────┤
│                                                          Total: 15.5 KB   │
└───────────────────────────────────────────────────────────────────────────┘
```

Header row: bold text, bottom border.
Data rows: type-specific renderers per cell.
Badge cells: colored pill inline.
Progress cells: mini progress bar inline.
Footer: aggregate row with top border.
Striping: alternating row background.

Column widths: specified in chars, or auto-fit to content.
Total table width: sum of column widths + borders.
Overflow: horizontal scroll or truncate with ellipsis.

## Composability

```
@card
| {label:title "System Monitor" primary}
| {divider:d}
| @table:procs striped:true sortable:true
| | @col:pid header:"PID" type:number width:4 align:right
| | @col:name header:"Process" type:text width:14
| | @col:state header:"State" type:badge width:10
| | @col:cpu header:"CPU" type:progress width:16
| | @row 0, "init", "running" success, 45
| | @row 1, "cryptod", "running" success, 22
| @end table
@end card
@place:monitor anchor:center
```

Table inside a card, placed at center. Sortable, striped.
All Markout. All parallel strings. All the way down.
