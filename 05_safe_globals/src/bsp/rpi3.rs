// SPDX-License-Identifier: MIT
//
// Copyright (c) 2018-2019 Andre Richter <andre.o.richter@gmail.com>

//! Board Support Package for the Raspberry Pi 3.

use crate::{arch::sync::NullLock, interface};
use core::fmt;

pub const BOOT_CORE_ID: u64 = 0;
pub const BOOT_CORE_STACK_START: u64 = 0x80_000;

/// A mystical, magical device for generating QEMU output out of the void.
///
/// The mutex protected part.
struct QEMUOutputInner {
    chars_written: usize,
}

impl QEMUOutputInner {
    const fn new() -> QEMUOutputInner {
        QEMUOutputInner { chars_written: 0 }
    }

    /// Send a character.
    fn write_char(&mut self, c: char) {
        unsafe {
            core::ptr::write_volatile(0x3F21_5040 as *mut u8, c as u8);
        }
    }
}

/// Implementing `core::fmt::Write` enables usage of the `format_args!` macros,
/// which in turn are used to implement the `kernel`'s `print!` and `println!`
/// macros. By implementing `write_str()`, we get `write_fmt()` automatically.
///
/// The function takes an `&mut self`, so it must be implemented for the inner
/// struct.
///
/// See [`src/print.rs`].
///
/// [`src/print.rs`]: ../../print/index.html
impl fmt::Write for QEMUOutputInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            // Convert newline to carrige return + newline.
            if c == '\n' {
                self.write_char('\r')
            }

            self.write_char(c);
        }

        self.chars_written += s.len();

        Ok(())
    }
}

////////////////////////////////////////////////////////////////////////////////
// OS interface implementations
////////////////////////////////////////////////////////////////////////////////

/// The main struct.
pub struct QEMUOutput {
    inner: NullLock<QEMUOutputInner>,
}

impl QEMUOutput {
    pub const fn new() -> QEMUOutput {
        QEMUOutput {
            inner: NullLock::new(QEMUOutputInner::new()),
        }
    }
}

/// Passthrough of `args` to the `core::fmt::Write` implementation, but guarded
/// by a Mutex to serialize access.
impl interface::console::Write for QEMUOutput {
    fn write_fmt(&self, args: core::fmt::Arguments) -> fmt::Result {
        use interface::sync::Mutex;

        // Fully qualified syntax for the call to
        // `core::fmt::Write::write:fmt()` to increase readability.
        let mut r = &self.inner;
        r.lock(|i| fmt::Write::write_fmt(i, args))
    }
}

impl interface::console::Read for QEMUOutput {}

impl interface::console::Statistics for QEMUOutput {
    fn chars_written(&self) -> usize {
        use interface::sync::Mutex;

        let mut r = &self.inner;
        r.lock(|i| i.chars_written)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Global instances
////////////////////////////////////////////////////////////////////////////////

static QEMU_OUTPUT: QEMUOutput = QEMUOutput::new();

////////////////////////////////////////////////////////////////////////////////
// Implementation of the kernel's BSP calls
////////////////////////////////////////////////////////////////////////////////

/// Return a reference to a `console::All` implementation.
pub fn console() -> &'static impl interface::console::All {
    &QEMU_OUTPUT
}
