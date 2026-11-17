//! A mutex's guard, also after a thread panicked while it held the mutex: the data stays usable.

use std::sync::{Mutex, MutexGuard, PoisonError};

pub fn lock<T: ?Sized>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
