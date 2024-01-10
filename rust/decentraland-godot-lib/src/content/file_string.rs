pub fn get_base_dir(file_path: &str) -> String {
    let last_slash = file_path.rfind('/');
    if let Some(last_slash) = last_slash {
        return file_path[0..last_slash].to_string();
    }
    "".to_string()
}

pub fn get_extension(file_path: &str) -> String {
    let last_dot = file_path.rfind('.');
    if let Some(last_dot) = last_dot {
        return file_path[last_dot..].to_string();
    }
    "".to_string()
}
