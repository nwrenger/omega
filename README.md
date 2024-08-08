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

Navigating through your project is straightforward: selecting an entry from the left panel will close the currently open file and open the newly selected one. The editor efficiently manages your files by checking if a file is already open and retrieving its data from its current state or loading it from the filesystem to store in the state. All changes to files are temporarily cached in the state, ensuring that unsaved files can be reopened, edited further, and eventually saved, provided the editor remains open. Upon exiting the editor (using `Ctrl` + `p` -> typing `>q` and enter), it will prompt you to save any unsaved changes.

Files that are being edited will be marked with an asterisk `*` in the title bar; saving these files will remove the asterisk.

The editor provides a Quick Access view, accessible with the global shortcut `Ctrl` + `p`. This view displays your currently open files and, by entering command mode with `>`, allows you to perform file and directory management tasks. These tasks include opening a new project, saving the current file, adding, editing, and deleting files, and more, such as opening the info and debugger views. Please exercise caution when deleting files, as this action is irreversible and there is no intermediate trash bin for recovery.

Because you'll be opening many views, there is a global shortcut `Esc` to close the current one.

> Moving the cursor/selector via mouse input, arrow keys and `Tab` is also possible.

## Bindings

| Global             | Keybinding   |
| ------------------ | ------------ |
| Open Quick Access  | `Ctrl` + `p` |
| Close current View | `Esc`        |

| Quick Access                   | Command Name |
| ------------------------------ | ------------ |
| Open Debugger                  | `debug`      |
| Open Infos                     | `info`       |
| Opening a File/Project         | `open`       |
| Saving the current opened File | `save`       |
| Creating a new File/Directory  | `new`        |
| Renaming a File/Directory      | `rename`     |
| Deleting a File/Directory      | `delete`     |
| Quitting                       | `quit`       |

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

It's important to note that on macOS, `omega` relies on `ncurses` as its backend, which will need to be installed if not already pre-installed. For Linux-based systems and Windows, `omega` uses `crossterm` as its backend.

## Known Issues

There are currently some smaller known issues:

- Performance Issues on scrolling and editing files.
- If the content only needs a scrollbar on the x-Axis, this scrollbar won't be interactable.
- `Tabs` are currently not rendered/shown. So some files using `Tabs` could look not like their should.
- Fix macos coloring -> wait for `cursive-backend` to update

If you're encountering more Bugs please create an `Issue` and if you want to fix one create a `Pull Request` containing the fix.

## Contributing

I warmly welcome and thoroughly review all contributions submitted through `Pull Requests`.
