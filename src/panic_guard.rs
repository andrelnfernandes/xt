//! Contains panics from code that handles hostile input (decoders), so a bug
//! there cannot take down the TUI.
//!
//! Panic hooks run BEFORE `catch_unwind` returns, and the hooks installed by
//! ratatui and `Keyboard::detect` restore the terminal. While a guard is
//! active, the outermost hook (installed last in `main`) skips them.

use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// Restores the previous flag value on every path, including unwinding.
struct Reset(bool);

impl Drop for Reset {
    fn drop(&mut self) {
        ACTIVE.with(|a| a.set(self.0));
    }
}

// Used by `decompress`, which the viewer plugs in at Task 8.
#[allow(dead_code)]
/// Runs `f`; a panic becomes `None` and the panic hooks that would restore
/// the terminal stay quiet.
pub fn catch<T>(f: impl FnOnce() -> T) -> Option<T> {
    let _reset = Reset(ACTIVE.with(|a| a.replace(true)));
    catch_unwind(AssertUnwindSafe(f)).ok()
}

/// True while the current thread is inside [`catch`].
pub fn active() -> bool {
    ACTIVE.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_is_set_inside_and_cleared_after() {
        assert!(!active());
        assert_eq!(catch(active), Some(true));
        assert!(!active());
    }

    #[test]
    fn flag_is_cleared_after_a_panic() {
        assert_eq!(catch(|| -> u8 { panic!("boom") }), None);
        assert!(!active());
    }

    #[test]
    fn nested_catch_keeps_the_outer_flag() {
        assert_eq!(catch(|| catch(|| 1).map(|_| active())), Some(Some(true)));
        assert!(!active());
    }
}
