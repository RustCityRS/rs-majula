//! Every trigger in `content/647` names a subject that exists.
//!
//! A trigger whose subject the compiler cannot resolve -- `[opheld2,black_axe]`
//! where 647 calls the obj `black_hatchet` -- is only a warning: the block
//! compiles, is packed, and can never dispatch, because nothing is keyed on a
//! name that is not there. Content ported from 289 carries 289's names, and
//! 2005-2011 renamed a good many items, so this is how a port goes quietly
//! dead. The level requirements on wielding were: 177 `[opheld2,..]` triggers,
//! each one an item anyone could wield whatever their level.
//!
//! The check is the compiler's own: its "did not resolve to an entity"
//! diagnostic, captured rather than printed.
//!
//! ```text
//! cargo test -p rs-pack --test trigger_subjects_647
//! ```

#![cfg(since_647)]

use std::fmt::Debug;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::subscriber::with_default;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};

/// Keeps every compiler event whose message says a subject did not resolve.
struct Unresolved(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> Layer<S> for Unresolved {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        struct Message(String);
        impl Visit for Message {
            fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}");
                }
            }
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        if message.0.contains("did not resolve to an entity") {
            self.0.lock().unwrap().push(message.0);
        }
    }
}

#[test]
fn every_trigger_subject_resolves() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(Unresolved(seen.clone()));
    with_default(subscriber, || {
        runec::compile_memory(
            &content.join("scripts/runescript"),
            Some(&content.join("pack")),
            false,
        )
        .expect("content/647 compiles")
    });

    let seen = seen.lock().unwrap();
    assert!(
        seen.is_empty(),
        "{} trigger(s) name a subject that does not exist, and can never run:\n  {}",
        seen.len(),
        seen.join("\n  ")
    );
}
