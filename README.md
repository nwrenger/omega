[![crates.io](https://img.shields.io/crates/v/omega.svg)](https://crates.io/crates/omega)
[![crates.io](https://img.shields.io/crates/d/omega.svg)](https://crates.io/crates/omega)

# `omega`

A performant and extensive terminal-based project editor with a wide variety of modern shortcuts.

<img src="images/screenshot.png" width="650"/>

## How to use

```bash
omega [path]
```
This section will guide you through the initial steps of opening the editor. When specifying a `path`, if it points to a file, the editor will set the surrounding directory as the project directory. If the `path` points directly to a directory, that directory will become the project directory.

Within the editor, you'll find a panel on the left side that displays your project's directory structure, allowing for easy navigation through your project files.

Navigating through your project is straightforward: selecting an entry from the left panel will close the currently open file and open the newly selected one. The editor efficiently manages your files by checking if a file is already open and retrieving its data from its current state or loading it from the filesystem to store in the state. All changes to files are temporarily cached in the state using a Hashmap, ensuring that unsaved files can be reopened, edited further, and eventually saved, provided the editor remains open. Upon exiting the editor (using `Ctrl` + `q`), it will prompt you to save any unsaved changes. It is important to note that this prompt will not appear if you force quit the editor.

Files that are being edited will be marked with an asterisk `*` in the title bar; saving these files will remove the asterisk.

The editor also offers Global Keybindings for file and directory management tasks, such as adding, editing, and deleting. Please exercise caution when deleting files, as this action is irreversible, with no intermediate trash bin for recovery.

## Bindings

| Global                        | Keybinding   |
| ----------------------------- | ------------ |
| Infos                         | `Esc`        |
| Toggle debugger               | `Ctrl` + `p` |
| Quitting                      | `Ctrl` + `q` |
| Force Quitting                | `Ctrl` + `f` |
| Opening a File/Project        | `Ctrl` + `o` |
| Creating a new File/Directory | `Ctrl` + `n` |
| Renaming a File/Directory     | `Ctrl` + `r` |
| Deleting a File/Directory     | `Ctrl` + `d` |
| Saving File                   | `Ctrl` + `s` |

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
On linux based systems you'll need the `libx11-dev`/`libX11-devel` packages to be installed.
