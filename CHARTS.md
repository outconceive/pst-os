# Charts for Markout

## Pie Chart

### Core Idea

A pie is a sequence of slices. Each slice declares its share, color,
and optional label. Slices are laid out starting at 0° (12 o'clock)
and go clockwise. The total is computed from the slices — percentages
or raw values, the chart normalizes.

### Syntax

```
@pie:<name> radius:<px>
| {slice:<key> value:<n> <style> "Label"}
| {slice:<key> value:<n> <style> "Label"}
@end pie
```

### Example

```
@pie:usage radius:80
| {slice:rust value:45 primary "Rust 45%"}
| {slice:js value:25 warning "JS 25%"}
| {slice:py value:20 success "Python 20%"}
| {slice:other value:10 muted "Other 10%"}
@end pie
```

Four slices. Total = 100. Rust gets 45% of 360° = 162°, starting
at 0°. JS gets 90° starting at 162°. And so on.

### Auto-Normalization

Values don't have to sum to 100. The chart normalizes:

```
@pie:votes radius:60
| {slice:a value:340 primary "Alice"}
| {slice:b value:280 danger "Bob"}
| {slice:c value:180 success "Carol"}
@end pie
```

Total = 800. Alice gets 340/800 = 42.5% of the circle.

### Donut Variant

```
@pie:donut radius:80 inner:40
| {slice:used value:73 primary "Used"}
| {slice:free value:27 muted "Free"}
@end pie
```

`inner:<px>` makes it a donut (ring chart). The center is hollow.

### Exploded Slice

```
| {slice:highlight value:30 danger "Critical" explode:8}
```

`explode:<px>` pulls the slice outward from center by that many pixels.

### Start Angle

```
@pie:custom radius:60 start:90
```

`start:<degrees>` rotates the starting position. Default is 0 (top).

## Bar Chart

```
@bar:<name> height:<px>
| {bar:a value:85 primary "Rust"}
| {bar:b value:60 warning "JS"}
| {bar:c value:45 success "Python"}
@end bar
```

Vertical bars, auto-scaled to the max value. Each bar is a column.
Height sets the chart area height. Bar width is derived from count.

### Horizontal Variant

```
@bar:<name> height:<px> direction:horizontal
| {bar:a value:85 primary "Rust"}
@end bar
```

### Stacked

```
@bar:<name> height:<px> stacked:true
| {bar:q1 values:30,20,10 colors:primary,warning,danger "Q1"}
| {bar:q2 values:40,15,5 colors:primary,warning,danger "Q2"}
@end bar
```

Multiple values per bar, stacked vertically.

## Line Chart

```
@line:<name> width:<px> height:<px>
| {series:cpu points:20,45,30,60,55,70,65,80 primary "CPU"}
| {series:mem points:40,42,45,43,50,48,52,55 success "Memory"}
@end line
```

Multiple series on the same axes. Points are comma-separated values.
Axes auto-scale to min/max across all series.

### Area Fill

```
| {series:cpu points:20,45,30,60 primary fill "CPU"}
```

`fill` shades the area under the line.

## Sparkline (Already Exists)

The existing `{sparkline:key}` component is a mini inline line chart.
These full chart types are the block-level versions with labels,
axes, and legends.

## Parallel String Interpretation

```
Chart:     usage
Type:      pie
Radius:    80
Slice 0:   key=rust   value=45  style=primary  label="Rust 45%"
Slice 1:   key=js     value=25  style=warning  label="JS 25%"
Slice 2:   key=py     value=20  style=success  label="Python 20%"
Slice 3:   key=other  value=10  style=muted    label="Other 10%"
```

Each slice is a row. Value, style, label are columns.
The chart is a container of parallel strings where the
value column determines angular extent.

The pie chart is literally a circular parallel string table —
each slice is a segment of the circle, each segment's size
is proportional to its value column. Position is cumulative
sum of preceding values. Same principle as the process table:
positional identity, append-only, the order defines the layout.

## Framebuffer Rendering

### Pie Slices

For each slice, compute start and end angles:
```
start_angle = cumulative_sum_of_previous / total * 2π
end_angle = start_angle + value / total * 2π
```

Render using angle-based pixel iteration:
```rust
for y in -radius..radius {
    for x in -radius..radius {
        if x*x + y*y <= radius*radius {
            let angle = atan2(x, -y);  // 0 = top, clockwise
            if angle >= start && angle < end {
                set_pixel(cx + x, cy + y, color);
            }
        }
    }
}
```

For no_std without f64 trig: use integer atan2 lookup table
(256 entries, 8-bit angle). Accurate enough for 640x480.

### Bar Charts

Fill rectangles. Height proportional to value / max * chart_height.
Labels below each bar.

### Line Charts

Bresenham line drawing between consecutive points.
Area fill: fill vertically from line to baseline.

## Composability

Charts compose with cards, grids, and transitions:

```
@card
| {label:title "System Metrics" primary}
| {divider:d}
| @pie:cpu radius:50
| | {slice:user value:45 primary "User"}
| | {slice:sys value:25 danger "System"}
| | {slice:idle value:30 muted "Idle"}
| @end pie
| {sparkline:history}
@end card
@transition:spin target:cpu property:rotation from:0 to:360 duration:2000 curve:linear
```

A card containing a pie chart with a sparkline below it,
and the chart spins on load. All Markout. All parallel strings.
