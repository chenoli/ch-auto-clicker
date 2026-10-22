use std::path::PathBuf;

pub fn local_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("Chenoli")
        .join("CH Auto Clicker")
}

pub fn log_dir() -> PathBuf {
    local_data_dir().join("logs")
}

pub fn cache_dir() -> PathBuf {
    local_data_dir().join("cache")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_lives_under_the_local_data_dir() {
        let base = local_data_dir();
        assert!(
            base.ends_with("Chenoli\\CH Auto Clicker") || base.ends_with("Chenoli/CH Auto Clicker")
        );
        assert!(log_dir().starts_with(&base));
        assert!(cache_dir().starts_with(&base));
    }
}
