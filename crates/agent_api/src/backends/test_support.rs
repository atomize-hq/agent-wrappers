use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tokio::sync::Mutex;

pub(crate) fn test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) struct CurrentDirGuard {
    previous: PathBuf,
}

impl CurrentDirGuard {
    pub(crate) fn set(path: &Path) -> Self {
        let previous = env::current_dir().unwrap_or_else(|_| env::temp_dir());
        env::set_current_dir(path).expect("current dir should be set");
        Self { previous }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        if env::set_current_dir(&self.previous).is_err() {
            env::set_current_dir(env::temp_dir()).expect("fallback current dir should be restored");
        }
    }
}
