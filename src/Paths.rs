pub mod paths{
    use std::path::{Path, PathBuf};
    pub fn str_to_path (str : &str)-> &Path {
        let path = Path::new(str);
        path
    }
}
