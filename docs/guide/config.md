# Configuration (/pst/)

PST OS loads its configuration from Markout files on disk. Edit the files to customize the desktop.

## System Files

| File | Purpose |
|------|---------|
| `/pst/desktop.md` | Window layout — one window name per line |
| `/pst/welcome.md` | Welcome screen content (Markout) |
| `/pst/theme.md` | Color palette |
| `/pst/index.md` | Browser page index |

## /pst/desktop.md

```
Terminal
Scratch
```

Each line creates a window. Edit this file to add or remove windows.

## /pst/welcome.md

```
@card
| Welcome to PST OS
| {label:sub "Parallel String Theory" primary}
| {link:docs "Documentation" href:/pst/docs}
@end card
```

Full Markout — renders in the first window on boot.

## /pst/theme.md

```
bg:30,30,30
fg:255,255,255
accent:59,130,246
danger:239,68,68
success:16,185,129
warning:245,158,11
```

Color values for the desktop renderer.

## Editing Config

Open the text editor (F1) and edit any `/pst/` file. Changes take effect on the next boot or desktop re-render.

## No Recompile

The entire desktop layout, welcome screen, and theme are Markout files on disk. Change them without rebuilding the OS.
