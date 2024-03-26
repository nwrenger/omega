[![crates.io](https://img.shields.io/crates/v/omega.svg)](https://crates.io/crates/omega)
[![crates.io](https://img.shields.io/crates/d/omega.svg)](https://crates.io/crates/omega)

# `omega`

A performant and extensive terminal-based file editor with a wide variety of modern shortcuts.

<img src="images/screenshot.png" width="650"/>

## How to use

```bash
omega [path]
```
That will open a file and the editor itself.

`path` can only be a file, not a directory.

When specifying a not existing/invalid path the editor displays that in the top right via a `*` after the path (if no path is specified only the `*` will be visible).

## Bindings

| Global          | Keybinding   |
| --------------- | ------------ |
| Infos           | `Ctrl` + `z` |
| Toggle debugger | `Ctrl` + `d` |
| Quitting        | `Ctrl` + `q` |
| Force Quitting  | `Ctrl` + `f` |
| Saving File     | `Ctrl` + `s` |

| Editor             | Keybinding                                    |
| ------------------ | --------------------------------------------- |
| Copying Line       | `Ctrl` + `c`                                  |
| Paste Clipboard    | `Ctrl` + `v`                                  |
| Cut Line           | `Ctrl` + `x`                                  |
| Move Line          | `Shift` + <kbd>&uarr;</kbd>/<kbd>&darr;</kbd> |
| Move Cursor to EoL | `Shift` + <kbd>&larr;</kbd>/<kbd>&rarr;</kbd> |
| Ident              | `Tab`                                         |
| Remove Ident       | `Shift` + `Tab`                               |

> Moving the cursor via mouse input is also possible.

## Installation

You can currently install `omega` using cargo:
```bash
cargo install omega
```
Or download the binary directly from the [releases page](https://github.com/nwrenger/omega/releases/latest).
