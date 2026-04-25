# Editor (@editor)

The `@editor` container creates a rich text editor with a configurable toolbar.

## Syntax

```
@editor bold italic code heading bind:notes
| Initial content goes here
@end editor
```

## Features

List the toolbar buttons after `@editor`:

| Feature | Button | Description |
|---------|--------|-------------|
| `bold` | **B** | Bold text |
| `italic` | *I* | Italic text |
| `underline` | U | Underlined text |
| `strikethrough` | S | Strikethrough |
| `code` | `<>` | Inline code |
| `heading` | H | Heading |
| `list` | • | Bulleted list |
| `ordered-list` | 1. | Numbered list |
| `quote` | " | Block quote |
| `code-block` | | Code block |
| `link` | | Insert link |
| `image` | | Insert image |
| `divider` | — | Horizontal rule |

## State Binding

`bind:notes` syncs the editor content to the `notes` state key.

## Rendering

- **Framebuffer**: Toolbar with clickable buttons, editable text area
- **Terminal**: Toolbar labels, underlined text area
- **Browser**: contenteditable div with JS toolbar (via Outconceive)
