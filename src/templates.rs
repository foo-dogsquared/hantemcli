use std::error::Error;
use std::path::{Component, Path, PathBuf};

use handlebars;

pub fn register_from_path(
    template_registry: &mut handlebars::Handlebars,
    paths: Vec<PathBuf>,
    extension: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    // Sanitizing the path naively.
    let extension = match extension.starts_with(".") {
        true => extension.to_string(),
        false => format!(".{}", extension),
    };

    let mut registered_files = vec![];

    for template in paths {
        if template.is_dir() {
            let walker = walkdir::WalkDir::new(&template).min_depth(1).into_iter();
            for entry in walker
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file() && has_file_extension(e.path(), &extension))
            {
                if register_file_to_template_registry(template_registry, &entry.path(), &template) {
                    registered_files.push(entry.path().to_path_buf());
                }
            }
        } else {
            if !has_file_extension(&template, &extension) {
                continue;
            }

            if register_file_to_template_registry(
                template_registry,
                &template,
                &template.parent().unwrap_or(Path::new("./")),
            ) {
                registered_files.push(template.to_path_buf());
            }
        }
    }

    Ok(registered_files)
}

// A closure to easily register a path into the template registry.
// It will return a boolean indicating the success of the registration.
pub fn register_file_to_template_registry(
    template_registry: &mut handlebars::Handlebars,
    template: &Path,
    base_dir: &Path,
) -> bool {
    let normalized_base_dir = naively_normalize_path(&base_dir);

    let name = match path_without_extension(&template) {
        Some(v) => naively_normalize_path(v),
        None => {
            eprintln!("{:?} have an error getting the file path.", template);
            return false;
        }
    };

    let relpath_from_base_dir = match relative_path_from(&name, &normalized_base_dir) {
        Some(v) => v,
        None => {
            eprintln!(
                "{:?} has an error getting the relative path of the template. How's that possible?",
                template
            );
            return false;
        }
    };

    match template_registry
        .register_template_file(relpath_from_base_dir.to_str().unwrap(), &template)
    {
        Ok(_v) => true,
        Err(e) => {
            eprintln!("Template file {:?} has an error.", &template);
            eprintln!("{}", e);
            false
        }
    }
}

pub fn path_without_extension<S>(path: S) -> Option<PathBuf>
where
    S: AsRef<Path>,
{
    let path = path.as_ref();

    let mut parent: PathBuf = match path.parent() {
        Some(v) => v.to_path_buf(),
        None => PathBuf::new(),
    };

    match path.file_stem() {
        Some(v) => {
            parent.push(v);
            Some(parent)
        }
        None => None,
    }
}

/// A filter function specifically used for the walking through the directory.
pub fn has_file_extension(
    entry: &Path,
    ext: &str,
) -> bool {
    entry
        .file_name()
        .and_then(|file| Some(file.to_str().unwrap().ends_with(ext)))
        .unwrap_or(false)
}

/// Get the relative path from two paths similar to Python `os.path.relpath`.
///
/// This does not check whether the path exists in the filesystem.
///
/// Furthermore, this code is adapted from the [`pathdiff`](https://github.com/Manishearth/pathdiff/blob/master/src/lib.rs) crate
/// which in turn adapted from the `rustc` code at
/// https://github.com/rust-lang/rust/blob/e1d0de82cc40b666b88d4a6d2c9dcbc81d7ed27f/src/librustc_back/rpath.rs .
pub fn relative_path_from<P: AsRef<Path>, Q: AsRef<Path>>(
    dst: P,
    base: Q,
) -> Option<PathBuf> {
    let base = base.as_ref();
    let dst = dst.as_ref();

    // checking if both of them are the same type of filepaths
    if base.is_absolute() != dst.is_absolute() {
        match dst.is_absolute() {
            true => Some(PathBuf::from(dst)),
            false => None,
        }
    } else {
        let mut dst_components = dst.components();
        let mut base_path_components = base.components();

        let mut common_components: Vec<Component> = vec![];

        // looping into each components
        loop {
            match (dst_components.next(), base_path_components.next()) {
                // if both path are now empty
                (None, None) => break,

                // if the dst path has more components
                (Some(c), None) => {
                    common_components.push(c);
                    common_components.extend(dst_components.by_ref());
                    break;
                }

                // if the base path has more components
                (None, _) => common_components.push(Component::ParentDir),
                (Some(a), Some(b)) if common_components.is_empty() && a == b => (),
                (Some(a), Some(b)) if b == Component::CurDir => common_components.push(a),
                (Some(_), Some(b)) if b == Component::ParentDir => return None,
                (Some(a), Some(_)) => {
                    common_components.push(Component::ParentDir);
                    for _ in base_path_components {
                        common_components.push(Component::ParentDir);
                    }
                    common_components.push(a);
                    common_components.extend(dst_components.by_ref());
                    break;
                }
            }
        }

        Some(common_components.iter().map(|c| c.as_os_str()).collect())
    }
}

fn is_parent_dir(component: Component) -> bool {
    match component {
        Component::ParentDir => true,
        _ => false,
    }
}

/// Normalize the given path.
/// Unlike the standard library `std::fs::canonicalize` function, it does not need the file to be in the filesystem.
///
/// That said, this leaves compromise the implementation to be very naive.
/// All resulting path will be based on the current directory.
///
/// If the resulting normalized path is empty, it will return `None`.
pub fn naively_normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();

    let mut normalized_components = vec![];

    for component in path.components() {
        match &component {
            Component::CurDir => continue,
            // The condition below can be safe to execute.
            // It will immediately continue to the if block if one of them is true which is why
            // the ordering of the conditions is important.
            // If the vector is empty, it will never reach the second condition.
            // That said, there has to be a better way than this.
            Component::ParentDir => match normalized_components.is_empty()
                || is_parent_dir(normalized_components[normalized_components.len() - 1])
            {
                true => normalized_components.push(component),
                false => {
                    normalized_components.pop();
                    ()
                }
            },
            _ => normalized_components.push(component),
        }
    }

    normalized_components.iter().collect()
}
