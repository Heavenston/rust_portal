use crate::concat_arrays;

use std::{ cell::Cell, sync::atomic::{ AtomicU32, Ordering } };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// Byte array to keep alignment of 1
pub struct Uid([u8; 8]);

impl Uid {
    pub fn new() -> Self {
        // As it is not serializable this is enough
        // for serialization using an additional timestamp (and reducing the size
        // of the counters) should be enough.

        static THREAD_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

        struct ThreadLocalData {
            thread_id: u32,
            counter: Cell<u32>,
        }

        thread_local! {
            static THREAD_LOCAL_DATA: ThreadLocalData = ThreadLocalData {
                thread_id: THREAD_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
                counter: Cell::new(0),
            };
        };

        THREAD_LOCAL_DATA.with(|data| {
            let thread_id = data.thread_id;
            let counter = data.counter.replace(data.counter.get().checked_add(1).expect("No overflow"));

            Self(concat_arrays(thread_id.to_ne_bytes(), counter.to_ne_bytes()))
        })
    }
}

impl Default for Uid {
    fn default() -> Self {
        Self::new()
    }
}

