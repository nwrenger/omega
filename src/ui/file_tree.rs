use cursive::{
    view::{Nameable, Scrollable},
    views::{NamedView, ScrollView},
    Cursive,
};
use cursive_tree_view::{Placement, TreeView};
use std::{fmt, fs, io, path::PathBuf};

use crate::error::ResultExt;

use super::open_file;

#[derive(Debug, Clone, Default)]
pub struct TreeEntry {
    pub name: String,
    pub path: PathBuf,
    pub dir: Option<PathBuf>,
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
                        .unwrap_or_else(|_| String::new()),
                    path: entry.path(),
                    dir: Some(path),
                });
            } else if path.is_file() {
                entries.push(TreeEntry {
                    name: entry
                        .file_name()
                        .into_string()
                        .unwrap_or_else(|_| String::new()),
                    path: entry.path(),
                    dir: None,
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
    tree.clear();
    expand_tree(tree, 0, dir, Placement::Before);
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
                    expand_tree(tree, row, &dir, Placement::LastChild);
                }
            }
        });
    });

    tree.set_on_submit(move |siv: &mut Cursive, row| {
        if let Some(tree) = siv.find_name::<TreeView<TreeEntry>>("tree") {
            if let Some(item) = tree.borrow_item(row) {
                if item.dir.is_none() {
                    open_file(siv, &item.path).handle(siv);
                }
            }
        }
    });

    tree.with_name("tree").scrollable()
}
