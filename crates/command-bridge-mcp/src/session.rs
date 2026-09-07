use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Session {
    pub cwd: String,
    pub shell: String,
}

static STORE: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();
static COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn store() -> &'static Mutex<HashMap<String, Session>> {
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn generate_id() -> String {
    format!("s{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
