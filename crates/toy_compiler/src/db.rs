use std::sync::Arc;

use lasso::ThreadedRodeo;

/// Shared string interner for all compilation stages.
///
/// Uses `ThreadedRodeo` for lock-free reads - once strings are interned,
/// resolution is wait-free across all threads.
#[salsa::input]
pub struct SharedInterner {
    #[returns(ref)]
    pub rodeo: Arc<ThreadedRodeo>,
}

#[salsa::db]
pub trait Db: salsa::Database {
    /// Get access to the shared interner.
    fn interner(&self) -> &ThreadedRodeo;
}

#[salsa::db]
#[derive(Clone)]
pub struct Database {
    storage: salsa::Storage<Self>,
    interner: Arc<ThreadedRodeo>,
}

impl Database {
    /// Create a new database with the given interner.
    pub fn with_interner(interner: Arc<ThreadedRodeo>) -> Self {
        Self {
            storage: salsa::Storage::new(None),
            interner,
        }
    }

    /// Get access to the shared interner.
    pub fn interner(&self) -> &ThreadedRodeo {
        &self.interner
    }

    /// Get a SharedInterner salsa input for use in tracked functions.
    pub fn shared_interner(&self) -> SharedInterner {
        SharedInterner::new(self, Arc::clone(&self.interner))
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::with_interner(Arc::new(ThreadedRodeo::default()))
    }
}

#[salsa::db]
impl salsa::Database for Database {}

#[salsa::db]
impl Db for Database {
    fn interner(&self) -> &ThreadedRodeo {
        &self.interner
    }
}

/// Test utilities for tracking salsa memoization behavior.
#[cfg(test)]
pub mod test_utils {
    use super::*;
    use salsa::{Event, EventKind};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counters for tracking query execution events.
    #[derive(Debug, Default)]
    pub struct EventCounters {
        pub will_execute: AtomicUsize,
        #[allow(dead_code)]
        pub did_validate_memoized: AtomicUsize,
    }

    impl EventCounters {
        pub fn will_execute(&self) -> usize {
            self.will_execute.load(Ordering::SeqCst)
        }

        #[allow(dead_code)]
        pub fn did_validate_memoized(&self) -> usize {
            self.did_validate_memoized.load(Ordering::SeqCst)
        }

        pub fn reset(&self) {
            self.will_execute.store(0, Ordering::SeqCst);
            self.did_validate_memoized.store(0, Ordering::SeqCst);
        }
    }

    /// Database that tracks salsa events for testing memoization.
    #[salsa::db]
    #[derive(Clone)]
    pub struct TrackedDatabase {
        storage: salsa::Storage<Self>,
        #[allow(dead_code)]
        interner: Arc<ThreadedRodeo>,
        pub counters: Arc<EventCounters>,
    }

    impl TrackedDatabase {
        pub fn new() -> Self {
            let counters = Arc::new(EventCounters::default());
            let counters_clone = Arc::clone(&counters);

            Self {
                storage: salsa::Storage::new(Some(Box::new(move |event: Event| {
                    match event.kind {
                        EventKind::WillExecute { .. } => {
                            counters_clone.will_execute.fetch_add(1, Ordering::SeqCst);
                        }
                        EventKind::DidValidateMemoizedValue { .. } => {
                            counters_clone
                                .did_validate_memoized
                                .fetch_add(1, Ordering::SeqCst);
                        }
                        _ => {}
                    }
                }))),
                interner: Arc::new(ThreadedRodeo::default()),
                counters,
            }
        }

        #[allow(dead_code)]
        pub fn interner(&self) -> &ThreadedRodeo {
            &self.interner
        }
    }

    impl Default for TrackedDatabase {
        fn default() -> Self {
            Self::new()
        }
    }

    #[salsa::db]
    impl salsa::Database for TrackedDatabase {}

    #[salsa::db]
    impl Db for TrackedDatabase {
        fn interner(&self) -> &ThreadedRodeo {
            &self.interner
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_resolve_roundtrip() {
        let db = Database::default();
        let interner = db.interner();

        // Intern some strings
        let key_hello = interner.get_or_intern("hello");
        let key_world = interner.get_or_intern("world");
        let key_hello2 = interner.get_or_intern("hello");

        // Same string → same key
        assert_eq!(key_hello, key_hello2);
        // Different strings → different keys
        assert_ne!(key_hello, key_world);

        // Resolve back to original strings
        assert_eq!(interner.resolve(&key_hello), "hello");
        assert_eq!(interner.resolve(&key_world), "world");
    }

    #[test]
    fn shared_interner_salsa_input() {
        let db = Database::default();

        // Get SharedInterner as salsa input
        let shared = db.shared_interner();
        let rodeo = shared.rodeo(&db);

        // Intern via shared interner
        let key = rodeo.get_or_intern("test");
        assert_eq!(rodeo.resolve(&key), "test");

        // Same Arc, same interner
        assert!(Arc::ptr_eq(rodeo, &db.interner));
    }

    #[test]
    fn interner_persists_across_clones() {
        let db = Database::default();
        let key = db.interner().get_or_intern("persistent");

        // Clone the database
        let db2 = db.clone();

        // Both share the same interner
        assert_eq!(db2.interner().resolve(&key), "persistent");
        assert!(Arc::ptr_eq(&db.interner, &db2.interner));
    }

    #[test]
    fn interner_threaded_access() {
        use std::thread;

        let db = Database::default();
        let interner = Arc::clone(&db.interner);

        // Pre-intern in main thread
        let key_main = interner.get_or_intern("main");

        // Spawn thread to intern and resolve
        let handle = thread::spawn(move || {
            let key_thread = interner.get_or_intern("thread");
            let key_main_from_thread = interner.get_or_intern("main");

            // Same key for "main" across threads
            assert_eq!(key_main, key_main_from_thread);

            (key_thread, interner.resolve(&key_main).to_string())
        });

        let (key_thread, resolved_main) = handle.join().unwrap();
        assert_eq!(resolved_main, "main");

        // Can resolve thread's key from main thread
        assert_eq!(db.interner().resolve(&key_thread), "thread");
    }
}
