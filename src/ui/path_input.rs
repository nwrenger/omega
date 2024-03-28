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
    let mut select = SelectView::<String>::new();
    select.add_all_str(&get_paths(path, files).unwrap_or_default());

    let view_name = name.clone() + "_edit";
    let select_name = name.clone() + "_select";
    let select_name2 = name.clone() + "_select";

    Ok(LinearLayout::vertical()
        .child(
            EditView::new()
                .content(path.to_string_lossy())
                .on_edit(move |siv, new_path, _| {
                    let new_path = PathBuf::from(&new_path);
                    siv.call_on_name(&select_name, |view: &mut SelectView| {
                        view.clear();
                        view.add_all_str(&get_paths(&new_path, files).unwrap_or_default());
                    })
                    .unwrap();
                })
                .with_name(name.to_string() + "_edit"),
        )
        .child(ScrollView::new(
            select
                .on_submit(move |siv, new_path: &String| {
                    siv.call_on_name(&view_name, |siv: &mut EditView| {
                        siv.set_content(new_path);
                    })
                    .unwrap();
                    siv.call_on_name(&select_name2, |view: &mut SelectView| {
                        view.clear();
                        view.add_all_str(
                            &get_paths(&PathBuf::from(new_path), files).unwrap_or_default(),
                        );
                    })
                    .unwrap();
                })
                .with_name(name.to_string() + "_select"),
        )))
}

/// Getting all paths by a path
fn get_paths(path: &Path, include_files: bool) -> Result<Vec<String>> {
    let mut entries: Vec<(String, bool)> = fs::read_dir(path)?
        .filter_map(|entry_result| match entry_result {
            Ok(entry) => {
                let path = entry.path();
                match entry.file_type() {
                    Ok(file_type) => {
                        if file_type.is_dir() || (include_files && file_type.is_file()) {
                            let name = path.to_string_lossy().into_owned();
                            Some(Ok((name, file_type.is_dir())))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e.into())),
                }
            }
            Err(e) => Some(Err(e.into())),
        })
        .collect::<Result<Vec<_>>>()?;

    entries.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
    });

    let paths = entries.into_iter().map(|(path, _)| path).collect();

    Ok(paths)
}
