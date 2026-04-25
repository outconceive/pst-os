# Interaction Model

UI interaction in PST OS is parallel strings — not an engine.

## The Table

```
component:  input    password  checkbox  button
state_key:  username pass      remember  login
value:      "alice"  "***"     "true"    ""
focus:      "true"   ""        ""        ""
tab_order:  "1"      "2"       "3"       "4"
hover:      ""       ""        ""        "true"
enabled:    "true"   "true"    "true"    "true"
validate:   "req"    "min:8"   ""        ""
```

Each interactive state is a column. Click sets the focus column. Tab advances it. Keyboard appends to the value column. Hover updates on mouse move.

## No Engine

There is no interaction engine. The table IS the state. The solver reads it. The renderer paints it.

- Click → update focus column
- Key → update value column of focused row
- Tab → advance focus to next tab_order
- Hover → update hover column
- Submit → read value columns, run validation

## pst-ui Crate

```rust
let mut ui = UiState::from_markout(markout);
ui.handle_key(b'a');          // types 'a' into focused field
ui.tab_next();                // moves focus
ui.handle_click(100, 200);    // clicks at pixel position
let errors = ui.validate();   // checks validation rules
```

Built from Markout — parses the VNode tree, extracts interactive components, assigns tab order, tracks state. All flat. All parallel strings.
