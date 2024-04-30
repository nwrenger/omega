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

Navigating through your project is straightforward: selecting an entry from the left panel will close the currently open file and open the newly selected one. The editor efficiently manages your files by checking if a file is already open and retrieving its data from its current state or loading it from the filesystem to store in the state. All changes to files are temporarily cached in the state, ensuring that unsaved files can be reopened, edited further, and eventually saved, provided the editor remains open. Upon exiting the editor (using `Ctrl` + `q`), it will prompt you to save any unsaved changes.

Files that are being edited will be marked with an asterisk `*` in the title bar; saving these files will remove the asterisk.

The editor also offers Global Keybindings for file and directory management tasks, such as adding, editing, and deleting. Please exercise caution when deleting files, as this action is irreversible, with no intermediate trash bin for recovery.

> Moving the cursor/selector via mouse input, arrow keys and `Tab` is also possible.

## Bindings

| Global                        | Keybinding   |
| ----------------------------- | ------------ |
| Infos                         | `Esc`        |
| Toggle debugger               | `Ctrl` + `p` |
| Quitting                      | `Ctrl` + `q` |
| Goto an already opened File   | `Ctrl` + `g` |
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

## Installation

To install `omega`, you can use Cargo by running the following command in your terminal:

```bash
cargo install omega
```
Alternatively, you can download the binary directly from the [releases page](https://github.com/nwrenger/omega/releases/latest).

### Additional Requirements

For Linux-based systems, it is necessary to have the `libx11-dev` (Debian/Ubuntu) or `libX11-devel` (Fedora/RHEL) packages installed.

It's important to note that on Unix-based systems, `omega` relies on `ncurses` as its backend, which will need to be installed. For Windows users, `omega` uses `crossterm` as its backend.

## Known Issues

There are currently some smaller known issues:

- Performance Issues on scrolling and editing files.
- If the content only needs a scrollbar on the x-Axis, this scrollbar won't be intractable.
- `Tabs` are currently not rendered/shown. So some files using `Tabs` could look not like their should.
- <kbd>&uarr;</kbd>/<kbd>&darr;</kbd>-Input inside the `Edit View` sometimes always moves the scroll, this should only happen if it needs to.

If you're encountering more Bugs please create an `Issue` and if you want to fix one create a `Pull Request` containing the fix.

## Contributing

I warmly welcome and thoroughly review all contributions submitted through `Pull Requests`.
