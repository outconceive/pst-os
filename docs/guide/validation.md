# Validation

Add `validate:rules` to any input component.

## Syntax

```
| {input:email validate:required,email}
| {password:pass validate:required,min:8}
| {input:name validate:max:50}
```

## Rules

| Rule | Description |
|------|-------------|
| `required` | Field must not be empty |
| `email` | Value must contain `@` |
| `min:N` | At least N characters |
| `max:N` | At most N characters |

## Multiple Rules

Separate with commas:

```
| {input:username validate:required,min:3,max:20}
```

## Validation Timing

Call `ui.validate()` to check all fields. Returns a list of `(key, error_message)` pairs. Empty list means all valid.

## Example

```
@card padding:24
| {input:email validate:required,email}  Email
| {password:pass validate:required,min:8}  Password
| {button:login "Sign In" primary}
@end card
```

On submit, validation checks run. If email is empty: `"email is required"`. If password is 3 characters: `"pass must be at least 8 characters"`.
