use cursive::{
    view::{Nameable, Scrollable},
    views::{NamedView, ScrollView},
    Cursive,
};
use cursive_tree_view::{Placement, TreeView};
use std::{fmt, fs, io, path::PathBuf};

use crate::{
    app::{EditorPanel, FileData, State},
    error::Error,
};

use super::edit_area::EditArea;

#[derive(Debug, Clone, Default)]
pub struct TreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub dir: Option<PathBuf>,
    pub row: Option<usize>,
    pub opened: bool,
}

impl fmt::Display for TreeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn collect_entries(dir: &PathBuf, entries: &mut Vec<TreeEntry>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                entries.push(TreeEntry {
                    name: entry
                        .file_name()
                        .into_string()
                        .unwrap_or_else(|_| "".to_string()),
                    path: entry.path(),
                    dir: Some(path),
                    row: None,
                    opened: false,
                });
            } else if path.is_file() {
                entries.push(TreeEntry {
                    name: entry
                        .file_name()
                        .into_string()
                        .unwrap_or_else(|_| "".to_string()),
                    path: entry.path(),
                    dir: None,
                    row: None,
                    opened: false,
                });
            }
        }
    }
    Ok(())
}

pub fn expand_tree(
    tree: &mut TreeView<TreeEntry>,
    parent_row: usize,
    dir: &PathBuf,
    placement: Placement,
) {
    let mut entries = Vec::new();
    if collect_entries(dir, &mut entries).is_ok() {
        // sort entries
        entries.sort_by(|a, b| {
            b.dir
                .is_some()
                .cmp(&a.dir.is_some())
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        // due to the nature of how the tree is being created, this has to be done
        let placement = match placement {
            Placement::LastChild => Placement::LastChild,
            Placement::Before => {
                entries.reverse();
                Placement::Before
            }
            _ => unimplemented!(),
        };

        for i in entries {
            if i.dir.is_some() {
                tree.insert_container_item(i.clone(), placement, parent_row);
            } else {
                tree.insert_item(i.clone(), placement, parent_row);
            }
        }
    }
}

pub fn load_parent(tree: &mut TreeView<TreeEntry>, dir: &PathBuf) {
    let items = tree.take_items();
    expand_tree(tree, 0, dir, Placement::Before);
    // Check if things were opened and if so load the content and give through the current state of the tree to the newly generated tree
    for item in items {
        if let Some(row) = item.row {
            tree.set_collapsed(row, !item.opened);
            if let Some(dir) = tree
                .borrow_item(row)
                .unwrap_or(&TreeEntry::default())
                .dir
                .clone()
            {
                expand_tree(tree, row, &dir, Placement::LastChild);
            }
            if let Some(new_item) = tree.borrow_item_mut(row) {
                new_item.opened = item.opened;
                new_item.row = item.row;
            }
        }
    }
}

pub fn new(parent: &PathBuf) -> ScrollView<NamedView<TreeView<TreeEntry>>> {
    let mut tree = TreeView::<TreeEntry>::new();

    load_parent(&mut tree, parent);

    // Stuff that should happen when interacted with a collapse
    tree.set_on_collapse(|siv: &mut Cursive, row, is_collapsed, children| {
        siv.call_on_name("tree", move |tree: &mut TreeView<TreeEntry>| {
            // Lazily insert directory listings for sub nodes if there weren't already opened
            if !is_collapsed && children == 0 {
                if let Some(dir) = tree
                    .borrow_item(row)
                    .unwrap_or(&TreeEntry::default())
                    .dir
                    .clone()
                {
                    let opened = tree
                        .borrow_item(row)
                        .unwrap_or(&TreeEntry::default())
                        .opened;
                    if !opened {
                        expand_tree(tree, row, &dir, Placement::LastChild);
                    }
                }
            }
            // Saving state in tree item
            if let Some(item) = tree.borrow_item_mut(row) {
                item.row = Some(row);
                item.opened = !is_collapsed;
            }
        });
    });

    tree.set_on_submit(move |siv: &mut Cursive, row| {
        if let Some(tree) = siv.find_name::<TreeView<TreeEntry>>("tree") {
            if let Some(item) = tree.borrow_item(row) {
                if item.dir.is_none() {
                    let mut state = siv
                        .with_user_data(|state: &mut State| state.clone())
                        .unwrap();
                    let path_clone = item.path.clone();
                    if state.get_file(&item.path).is_none() {
                        match fs::read_to_string(&item.path) {
                            Ok(content) => {
                                siv.call_on_name("editor", |edit_area: &mut EditArea| {
                                    edit_area.set_content(content.clone());
                                    edit_area.enable();
                                })
                                .unwrap();

                                siv.set_user_data(
                                    state.open_new_file(path_clone, FileData { str: content }),
                                );
                            }
                            Err(e) => {
                                Into::<Error>::into(e).to_dialog(siv);
                                return;
                            }
                        };
                    } else {
                        state = State {
                            current_file: Some(path_clone),
                            ..state
                        };

                        siv.call_on_name("editor", |edit_area: &mut EditArea| {
                            edit_area.set_content(&state.get_current_file().unwrap().str);
                            edit_area.enable();
                        })
                        .unwrap();

                        siv.set_user_data(state.clone());
                    }

                    // check if file has been added && update title accordingly
                    let title = if state.is_current_file_edited() {
                        format!("{} *", item.path.to_string_lossy())
                    } else {
                        item.path.to_string_lossy().to_string()
                    };

                    siv.call_on_name("editor_title", |view: &mut EditorPanel| {
                        view.set_title(title)
                    })
                    .unwrap();
                }
            }
        }
    });

    tree.with_name("tree").scrollable()
}
