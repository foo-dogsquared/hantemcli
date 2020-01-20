use std::path::{Path, PathBuf};

pub fn path_without_extension<S>(path: S) -> Option<PathBuf>
where
    S: AsRef<Path>,
{
    let path = path.as_ref();

    let mut parent: PathBuf = match path.parent() {
        Some(v) => v.to_path_buf(),
        None => return None,
    };

    match path.file_stem() {
        Some(v) => {
            parent.push(v);
            Some(parent)
        }
        None => None,
    }
}
