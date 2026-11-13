//! Whether a decoder should insist the data is ours.
//!
//! The config decoders assert: an opcode they do not recognise is a panic,
//! because in the server's own pipeline that means the cache and the code have
//! diverged, and failing loudly is the point. That assertion has caught real
//! bugs and must stay.
//!
//! Tools that read *other people's* caches need the opposite. `config-timeline`
//! decodes archived caches from across a decade, where an unknown opcode is not
//! a bug but a different revision's format, and there is nothing to fix. Left
//! strict, it takes the process down: a non-unwinding abort or an access
//! violation, neither of which `catch_unwind` can intercept.
//!
//! So strictness is a mode, defaulting to on. The server never touches it; a
//! tool reading foreign data turns it off and gets a partial record instead of
//! a crash.

use std::sync::atomic::{AtomicBool, Ordering};

static STRICT: AtomicBool = AtomicBool::new(true);

/// Decode foreign data without asserting on it. Off by default.
///
/// A decoder in lenient mode stops at the first thing it does not understand
/// and returns what it read up to that point, which is a truthful partial
/// answer rather than a wrong whole one.
pub fn set_lenient(lenient: bool) {
    STRICT.store(!lenient, Ordering::Relaxed);
}

/// True when an unrecognised opcode should panic. The default.
#[inline]
pub fn strict() -> bool {
    STRICT.load(Ordering::Relaxed)
}

/// Stop or panic, depending on the mode.
///
/// Written as a macro so the panic still reports the decoder at the call site,
/// which a shared function would flatten into one useless location.
///
/// The message is kept identical to the `panic!` this replaced, so a strict-mode
/// failure reads exactly as it always has and nothing that greps the logs
/// breaks. `obj.rs` alone spelled it "Unrecognised"; that one is normalised.
#[macro_export]
macro_rules! unknown_opcode {
    ($what:literal, $code:expr) => {{
        if $crate::cache::lenient::strict() {
            panic!("Unrecognized {} config code: {}", $what, $code);
        }
        return;
    }};
}
