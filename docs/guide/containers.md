# Containers

Containers group content with layout and styling. All use `@name config` / `@end name`.

## Card
```
@card padding:24,max-width:400px
| Content with border
@end card
```

## Semantic Containers
```
@nav
| {link:home "Home"}  {link:about "About"}
@end nav

@header
| {label:title "Page Title"}
@end header

@section
| Section content
@end section

@footer
| Copyright 2025
@end footer

@form
| {input:email}
| {button:submit "Go" primary}
@end form

@aside
| Sidebar content
@end aside
```

## Parametric
```
@parametric
| {label:title "Dashboard"}
| {input:search center-x:title gap-y:1rem}
@end parametric
```
Constraint-based positioning. See [Parametric Layout](/guide/parametric).

## Editor
```
@editor bold italic code heading bind:notes
| Initial content
@end editor
```
Rich text editor. See [Editor](/guide/editor).

## Each (Lists)
```
@each:items
| {label:name}  {button:remove "x" danger}
@end each
```
Repeats template for each item in state. See [Lists](/guide/lists).

## Container Config

All containers accept config after the tag name:

| Property | Example | Description |
|----------|---------|-------------|
| padding | `padding:24` | Inner spacing (px) |
| width | `width:400px` | Fixed width |
| max-width | `max-width:600px` | Maximum width |
| height | `height:200px` | Fixed height |
| gap | `gap:8` | Space between children |
