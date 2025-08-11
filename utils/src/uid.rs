use crate::concat_arrays;

use std::{ cell::Cell, marker::PhantomData, sync::atomic::{ AtomicU32, Ordering } };

#[derive_where::derive_where(Clone, Copy, PartialEq, Eq, Hash)]
// Byte array to keep alignment of 1
pub struct Uid<T = ()>([u8; 8], PhantomData<*const T>);

impl<T> Uid<T> {
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

            Self(concat_arrays(counter.to_ne_bytes(), thread_id.to_ne_bytes()), PhantomData)
        })
    }
}

impl<T> Default for Uid<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Debug for Uid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Uid")
            .field(&u64::from_ne_bytes(self.0))
            .finish()
    }
}
