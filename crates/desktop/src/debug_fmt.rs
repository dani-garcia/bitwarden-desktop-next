//! Debug-formatting newtypes for `Message` variants whose payloads are
//! sensitive or large.
//!
//! iced debug-formats every update `Message` and warns if it takes >1 ms.
//! The load-test account's `ListLoaded(<20k items>)` walks measured ~31 ms
//! with `#[derive(Debug)]`; decrypted ciphers and generated passwords
//! shouldn't appear in debug logs at all. Both concerns are field-local,
//! so wrapping at the variant level lets the surrounding enum just derive.
//!
//! - [`NoDebug<T>`] — redacts to `<hidden>`. Use on cleartext secrets.
//! - [`Summary<Vec<T>>`] — prints `<{n} items>`. Use on bulk SDK loads.
//!
//! Both deref to the inner value, so read access is transparent;
//! pattern-destructure `NoDebug(x)` / `Summary(x)` when you need to move
//! the inner value out.

use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;

#[derive(Clone)]
pub struct NoDebug<T>(pub T);

impl<T> Debug for NoDebug<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("<hidden>")
    }
}

impl<T> Deref for NoDebug<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

#[derive(Clone)]
pub struct Summary<T>(pub T);

impl<T> Debug for Summary<Vec<T>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} items>", self.0.len())
    }
}

impl<T> Deref for Summary<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
