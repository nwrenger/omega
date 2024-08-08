use std::{
    fs,
    path::{Path, PathBuf},
};

use cursive::{
    view::Nameable,
    views::{EditView, LinearLayout, ScrollView, SelectView},
};

use crate::error::Result;

/// Creates a filepath input view
///
/// The name for the EditView is `name` + `"_edit"`, for the SelectView `name` + `"_select"`
pub fn new(path: &Path, name: String, files: bool) -> Result<LinearLayout> {
    let view_name = name.clone() + "_edit";
    let select_name = name + "_select";

    let mut select = SelectView::new();

    let view_name_clone = view_name.clone();
    let select_name_clone = select_name.clone();
    select.set_on_submit(move |siv, new_path: &String| {
        siv.call_on_name(&view_name_clone, |edit_view: &mut EditView| {
            edit_view.set_content(new_path);
        })
        .unwrap();
        siv.call_on_name(&select_name_clone, |select_view: &mut SelectView| {
            select_view.clear();
            select_view.add_all_str(get_paths(&PathBuf::from(new_path), files).unwrap_or_default());
        })
        .unwrap();
    });

    select.add_all_str(get_paths(path, files).unwrap_or_default());

    let mut edit_view = EditView::new().content(path.to_string_lossy());

    let select_name_clone = select_name.clone();
    edit_view.set_on_edit(move |siv, new_path, _| {
        let new_path = PathBuf::from(&new_path);
        siv.call_on_name(&select_name_clone, |view: &mut SelectView| {
            view.clear();
            view.add_all_str(get_paths(&new_path, files).unwrap_or_default());
        })
        .unwrap();
    });

    let view_name_clone = view_name.clone();
    let select_name_clone = select_name.clone();
    edit_view.set_on_submit(move |siv, _| {
        let selected = siv
            .call_on_name(&select_name_clone, |select: &mut SelectView| {
                select.selection()
            })
            .unwrap();

        if let Some(selected) = selected {
            siv.call_on_name(&view_name_clone, |edit_view: &mut EditView| {
                edit_view.set_content(selected.parse::<String>().unwrap_or_default());
            })
            .unwrap();
        }
    });

    Ok(LinearLayout::vertical()
        .child(edit_view.with_name(view_name))
        .child(ScrollView::new(select.with_name(select_name))))
}

/// Getting all paths by a path with search functionality for incomplete paths.
pub fn get_paths(path: &Path, include_files: bool) -> Result<Vec<String>> {
    let entries_result = fs::read_dir(path);

    let (dir_to_read, filter_prefix) = if entries_result.is_err() {
        let parent_dir = path.parent().unwrap_or_else(|| Path::new("/"));
        let filter_prefix = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();

        (parent_dir, Some(filter_prefix))
    } else {
        (path, None)
    };

    let mut entries = fs::read_dir(dir_to_read)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            if let Some(ref prefix) = filter_prefix {
                entry.file_name().to_string_lossy().starts_with(prefix)
            } else {
                true
            }
        })
        .filter_map(|entry| {
            let path = entry.path();
            match entry.file_type() {
                Ok(file_type) => {
                    if file_type.is_dir() || (include_files && file_type.is_file()) {
                        let name = path.to_string_lossy().into_owned();
                        Some((name, file_type.is_dir()))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
    });

    let paths = entries.into_iter().map(|(path, _)| path).collect();
    Ok(paths)
}
