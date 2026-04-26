# Diagram Grid: Structured Diagramming for Markout

## Core Constraint

Every other cell is a shape. Cells between shapes are connectors or empty.
Two shapes cannot be directly adjacent — they must be separated by a
connector or None. The grid enforces this structurally.

```
Even cells (0,2,4...): shapes
Odd cells (1,3,5...):  connectors or empty
```

This applies to both columns and rows. The result is a checkerboard
pattern where shapes never touch.

## Syntax

```
@diagram:<name> cols:<N> rows:<M>
| (<col>,<row>) [Shape Label]
| (<col>,<row>) -->          // right arrow
| (<col>,<row>) <--          // left arrow
| (<col>,<row>) |            // vertical connector (down)
| (<col>,<row>) ^            // vertical connector (up)
| (<col>,<row>) <->          // bidirectional
| (<col>,<row>) ..>          // dashed arrow
@end diagram
```

Cols and rows define the full grid including connector cells.
A 5x3 grid has 3 shape columns (0,2,4) and 2 connector columns (1,3).

## Shape Types

```
[Label]           // rectangle (default)
(Label)           // rounded rectangle
<Label>           // diamond (decision)
((Label))         // circle
[/Label/]         // parallelogram (I/O)
[[Label]]         // double-border (subprocess)
```

## Connector Types

```
-->     // solid arrow right
<--     // solid arrow left
<->     // bidirectional arrow
--      // solid line (no arrow)
..>     // dashed arrow right
<..     // dashed arrow left
|       // vertical down
^       // vertical up
|^|     // vertical bidirectional
```

## Example: Flowchart

```
@diagram:flow cols:5 rows:5
| (0,0) [Start]    (1,0) -->   (2,0) <Input?>   (3,0) -->   (4,0) [Process]
|                               (2,1) |
| (0,2) [Log]      (1,2) <--   (2,2) <Error?>   (3,2) -->   (4,2) [Output]
|                               (2,3) |
|                               (2,4) [End]
@end diagram
```

Renders as:

```
┌───────┐       ◇─────────◇       ┌─────────┐
│ Start │──────▶│ Input?  │──────▶│ Process │
└───────┘       ◇─────────◇       └─────────┘
                     │
┌───────┐       ◇─────────◇       ┌─────────┐
│  Log  │◀──────│ Error?  │──────▶│ Output  │
└───────┘       ◇─────────◇       └─────────┘
                     │
                ┌─────────┐
                │   End   │
                └─────────┘
```

## Example: Architecture

```
@diagram:arch cols:7 rows:3
| (0,0) [[UI]]     (1,0) -->  (2,0) [[API]]    (3,0) -->  (4,0) [[Auth]]   (5,0) -->  (6,0) ((DB))
|                              (2,1) |
|                              (2,2) [[Cache]]  (3,2) ..>  (4,2) [[Logs]]
@end diagram
```

## Example: Simple Pipeline

```
@diagram:pipe cols:5 rows:1
| (0,0) [Parse]  (1,0) -->  (2,0) [Transform]  (3,0) -->  (4,0) [Render]
@end diagram
```

## Validation Rules

1. Even-position cells (0,2,4...) may contain shapes or be empty
2. Odd-position cells (1,3,5...) may contain connectors or be empty
3. A shape in an odd cell is a parse error
4. A connector in an even cell is a parse error
5. A connector must have at least one shape neighbor in its direction
6. Grid dimensions must be odd (so edges are shape cells)

## Styling

Shapes inherit Markout styles:

```
| (0,0) [Start primary]
| (2,0) <Error? danger>
| (4,0) [Done success]
```

Connectors can be labeled:

```
| (1,0) --"yes"->
| (3,0) --"no"->
```

## Rendering

### Framebuffer (pst-framebuffer)
- Shapes: filled rectangles with borders, text centered
- Diamonds: rotated square with clipped corners
- Connectors: lines with arrowheads drawn pixel-by-pixel
- Cell size: derived from largest shape in each column/row

### Terminal (pst-terminal)
- Shapes: Unicode box drawing (┌─┐│└─┘)
- Diamonds: ASCII art (◇)
- Connectors: ──▶ ◀── │ ▲ ▼

### HTML (Markout web)
- SVG elements positioned by grid coordinates
- CSS grid for layout, SVG for connectors

## Parallel String Interpretation

```
Component:   flow
Grid:        cols=5, rows=5
Shape(0,0):  [Start]
Conn(1,0):   -->
Shape(2,0):  <Input?>
Conn(3,0):   -->
Shape(4,0):  [Process]
Conn(2,1):   |
Shape(2,2):  <Error?>
...
```

Each cell is a row in the parallel string table.
Column 1: position (col,row).
Column 2: type (shape/connector/empty).
Column 3: content (label/direction).
Column 4: style.

Same primitive. Diagram structure IS parallel strings with a
checkerboard constraint.
