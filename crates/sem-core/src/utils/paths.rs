pub fn normalize_file_exts(exts: &[String]) -> Vec<String> {
    exts.iter().map(|ext| {
        if ext.starts_with('.') {
            ext.clone()
        } else {
            format!(".{}", ext)
        }
    }).collect()
}
