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

        for mut i in entries {
            if i.dir.is_some() {
                i.row = tree.insert_container_item(i.clone(), placement, parent_row);
            } else {
                i.row = tree.insert_item(i.clone(), placement, parent_row);
            }
        }
    }
}

pub fn new(parent: &PathBuf) -> ScrollView<NamedView<TreeView<TreeEntry>>> {
    let mut tree = TreeView::<TreeEntry>::new();

    expand_tree(&mut tree, 0, parent, Placement::Before);

    // Lazily insert directory listings for sub nodes
    tree.set_on_collapse(|siv: &mut Cursive, row, is_collapsed, children| {
        if !is_collapsed && children == 0 {
            siv.call_on_name("tree", move |tree: &mut TreeView<TreeEntry>| {
                if let Some(dir) = tree.borrow_item(row).unwrap().dir.clone() {
                    expand_tree(tree, row, &dir, Placement::LastChild);
                }
            });
        }
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
                                siv.call_on_name("editor", |text_area: &mut EditArea| {
                                    text_area.set_content(content.clone());
                                    text_area.enable();
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

                        siv.call_on_name("editor", |text_area: &mut EditArea| {
                            text_area.set_content(&state.get_current_file().unwrap().str);
                            text_area.enable();
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
