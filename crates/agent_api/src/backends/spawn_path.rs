use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub(super) fn resolve_binary_path_for_spawn(
    binary_path: PathBuf,
    effective_path_env: Option<&str>,
    ambient_path_env: Option<OsString>,
    invocation_cwd: Option<&Path>,
    effective_working_dir: Option<&Path>,
) -> Option<PathBuf> {
    if is_path_qualified(&binary_path) {
        return Some(resolve_path_qualified_binary(binary_path, invocation_cwd));
    }

    if let Some(path_env) = effective_path_env {
        return find_binary_on_path(
            &binary_path,
            Some(OsString::from(path_env)),
            effective_working_dir,
        );
    }

    find_binary_on_path(&binary_path, ambient_path_env, effective_working_dir)
}

fn resolve_path_qualified_binary(binary_path: PathBuf, invocation_cwd: Option<&Path>) -> PathBuf {
    if binary_path.is_absolute() {
        return binary_path;
    }

    let joined = effective_base_dir(invocation_cwd)
        .map(|cwd| cwd.join(&binary_path))
        .unwrap_or(binary_path);

    fs::canonicalize(&joined).unwrap_or(joined)
}

fn is_path_qualified(path: &Path) -> bool {
    path.is_absolute()
        || path
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty())
}

fn effective_base_dir(base_dir: Option<&Path>) -> Option<PathBuf> {
    match base_dir {
        Some(path) if path.is_absolute() => Some(path.to_path_buf()),
        Some(path) => env::current_dir().ok().map(|cwd| cwd.join(path)),
        None => env::current_dir().ok(),
    }
}

fn find_binary_on_path(
    binary_name: &Path,
    path_env: Option<OsString>,
    effective_working_dir: Option<&Path>,
) -> Option<PathBuf> {
    let path_env = path_env?;
    let effective_working_dir = effective_base_dir(effective_working_dir);
    env::split_paths(&path_env)
        .find_map(|directory| {
            let search_dir = if directory.is_absolute() {
                directory
            } else if let Some(cwd) = effective_working_dir.as_deref() {
                cwd.join(directory)
            } else {
                directory
            };
            candidate_binary_path(&search_dir, binary_name)
        })
        .map(|candidate| fs::canonicalize(&candidate).unwrap_or(candidate))
}

fn candidate_binary_path(directory: &Path, binary_name: &Path) -> Option<PathBuf> {
    let candidate = directory.join(binary_name);
    if is_runnable_path_candidate(&candidate) {
        return Some(candidate);
    }

    #[cfg(windows)]
    {
        if candidate.extension().is_some() {
            return None;
        }

        let pathext =
            env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
        for extension in pathext.to_string_lossy().split(';') {
            let extension = extension.trim();
            if extension.is_empty() {
                continue;
            }

            let suffixed = candidate.with_extension(extension.trim_start_matches('.'));
            if is_runnable_path_candidate(&suffixed) {
                return Some(suffixed);
            }
        }
    }

    None
}

fn is_runnable_path_candidate(candidate: &Path) -> bool {
    let Ok(metadata) = fs::metadata(candidate) else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };

    use tempfile::TempDir;

    use super::resolve_binary_path_for_spawn;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    struct CurrentDirGuard {
        previous: PathBuf,
    }

    impl CurrentDirGuard {
        fn set(path: &Path) -> Self {
            let previous = env::current_dir().expect("current dir");
            env::set_current_dir(path).expect("set current dir");
            Self { previous }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.previous).expect("restore current dir");
        }
    }

    fn write_test_binary(path: &Path) {
        fs::write(path, "binary").expect("write fake binary");

        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path).expect("binary metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("binary should be executable");
        }
    }

    #[test]
    fn resolves_relative_path_qualified_binary_against_supplied_current_dir() {
        let temp_dir = TempDir::new().expect("temp dir");
        let binary_dir = temp_dir.path().join("bin");
        let binary_path = binary_dir.join("codex");
        std::fs::create_dir_all(&binary_dir).expect("create binary dir");
        write_test_binary(&binary_path);

        let resolved = resolve_binary_path_for_spawn(
            PathBuf::from("bin/codex"),
            None,
            None,
            Some(temp_dir.path()),
            None,
        )
        .expect("relative qualified path should resolve");

        assert_eq!(
            resolved,
            std::fs::canonicalize(&binary_path).expect("canonicalize binary")
        );
        assert!(resolved.is_absolute());
    }

    #[test]
    fn resolves_bare_binary_from_path_before_current_dir_joining() {
        let path_dir = TempDir::new().expect("path dir");
        let cwd_dir = TempDir::new().expect("cwd dir");
        let binary_path = path_dir.path().join("codex");
        write_test_binary(&binary_path);

        let resolved = resolve_binary_path_for_spawn(
            PathBuf::from("codex"),
            Some(path_dir.path().to_string_lossy().as_ref()),
            env::var_os("PATH"),
            None,
            Some(cwd_dir.path()),
        )
        .expect("PATH lookup should resolve");

        assert_eq!(
            resolved,
            std::fs::canonicalize(&binary_path).expect("canonicalize binary")
        );
    }

    #[test]
    fn resolves_relative_path_entry_against_absolute_current_dir() {
        let cwd_dir = TempDir::new().expect("cwd dir");
        let binary_dir = cwd_dir.path().join("bin");
        let binary_path = binary_dir.join("codex");
        std::fs::create_dir_all(&binary_dir).expect("create binary dir");
        write_test_binary(&binary_path);

        let resolved = resolve_binary_path_for_spawn(
            PathBuf::from("codex"),
            Some("bin"),
            env::var_os("PATH"),
            None,
            Some(cwd_dir.path()),
        )
        .expect("PATH lookup should resolve");

        assert_eq!(
            resolved,
            std::fs::canonicalize(&binary_path).expect("canonicalize binary")
        );
    }

    #[test]
    fn resolves_relative_path_entry_against_relative_current_dir() {
        let parent_dir = TempDir::new().expect("parent dir");
        let wrapper_dir = parent_dir.path().join("wrapper");
        let working_dir = parent_dir.path().join("repo");
        let binary_dir = working_dir.join("bin");
        let binary_path = binary_dir.join("codex");
        std::fs::create_dir_all(&wrapper_dir).expect("create wrapper dir");
        std::fs::create_dir_all(&binary_dir).expect("create binary dir");
        write_test_binary(&binary_path);
        let _cwd_guard = CurrentDirGuard::set(parent_dir.path());

        let resolved = resolve_binary_path_for_spawn(
            PathBuf::from("codex"),
            Some("bin"),
            env::var_os("PATH"),
            None,
            Some(Path::new("repo")),
        )
        .expect("PATH lookup should resolve");

        assert_eq!(
            resolved,
            std::fs::canonicalize(&binary_path).expect("canonicalize binary")
        );

        let wrapper_candidate = wrapper_dir.join("bin").join("codex");
        assert!(
            !wrapper_candidate.exists(),
            "resolution must not use an unrelated wrapper cwd"
        );
    }

    #[cfg(unix)]
    #[test]
    fn skips_non_executable_shadow_file_during_path_lookup() {
        let shadow_dir = TempDir::new().expect("shadow dir");
        let executable_dir = TempDir::new().expect("executable dir");
        let shadow_path = shadow_dir.path().join("codex");
        let executable_path = executable_dir.path().join("codex");
        fs::write(&shadow_path, "shadow").expect("write shadow file");
        write_test_binary(&executable_path);

        let joined_path =
            env::join_paths([shadow_dir.path(), executable_dir.path()]).expect("join PATH entries");
        let resolved = resolve_binary_path_for_spawn(
            PathBuf::from("codex"),
            Some(joined_path.to_string_lossy().as_ref()),
            None,
            None,
            None,
        )
        .expect("PATH lookup should skip non-executable shadow");

        assert_eq!(
            resolved,
            fs::canonicalize(&executable_path).expect("canonicalize executable")
        );
    }
}
