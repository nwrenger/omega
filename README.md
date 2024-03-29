[![crates.io](https://img.shields.io/crates/v/omega.svg)](https://crates.io/crates/omega)
[![crates.io](https://img.shields.io/crates/d/omega.svg)](https://crates.io/crates/omega)

# `omega`

A performant and extensive terminal-based project editor with a wide variety of modern shortcuts.

<img src="images/screenshot.png" width="650"/>

## How to use

```bash
omega [path]
```
That will open the editor. If the `path` is a file, the directory around it will be the project directory. Otherwise the `path` directly will be project directory.

On the left side is a panel with the project directory where you can easily traverse your project.

Each time you click on one of the entries in the left side panel the current file will be closed and the new selected file will be opened. Here will the Editor check if the current state has this file already loaded and get that data or load the data from the file (from the filesystem) and save that in the state. Changes will be cached temporary in the state (via a Hashmap). Therefore you will always be able to open unsaved files again, edit them further and save them (as long as the editor wasn't quitted!). On quit (`Ctrl` + `q`) the editor detects files which hasn't been saved and will prompt you if you want to save these changes. On force quitting this check won't be done!

Opened files which are edited will be indicated in the Title via a `*`. If you safe them the `*` will disappear.

Using the Global Keybindings you can add, edit, delete Files and Directories. Be careful with deleting, the files are getting directly deleted with no trash bin or something in between!

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
