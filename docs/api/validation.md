# Validation Rules Reference

Add `validate:rules` to input components.

## Rules

| Rule | Description | Example |
|------|-------------|---------|
| `required` | Must not be empty | `validate:required` |
| `email` | Must contain `@` | `validate:email` |
| `min:N` | At least N characters | `validate:min:8` |
| `max:N` | At most N characters | `validate:max:100` |

## Combining Rules

Separate with commas:

```
{input:email validate:required,email}
{password:pw validate:required,min:8,max:64}
```

## Error Messages

| Rule | Error |
|------|-------|
| required | `"key is required"` |
| email | `"key must be a valid email"` |
| min:N | `"key must be at least N characters"` |
| max:N | `"key must be at most N characters"` |
