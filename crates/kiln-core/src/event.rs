//! Event bus and editor events for `kiln-core`.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::buffer::BufferId;
use crate::transaction::Transaction;

/// Unique subscription token for unsubscribing or identifying handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(usize);

static NEXT_SUB_ID: AtomicUsize = AtomicUsize::new(1);

/// Strongly-typed events representing editor state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    /// A buffer was mutated by a transaction.
    BufferChanged {
        /// The buffer that was modified.
        buffer_id: BufferId,
        /// Version of the buffer before the edit.
        old_version: u64,
        /// Version of the buffer after the edit.
        new_version: u64,
        /// The transaction that was applied.
        transaction: Transaction,
    },
    /// A buffer's saved state was confirmed.
    BufferSaved {
        /// The buffer that was saved.
        buffer_id: BufferId,
    },
    /// The editor mode changed (e.g. Normal to Insert).
    ModeChanged {
        /// Previous mode name.
        old_mode: String,
        /// New mode name.
        new_mode: String,
    },
}

/// Callback type for event bus subscribers.
pub type EventHandler = Box<dyn FnMut(&EditorEvent)>;

struct Subscription {
    id: SubscriptionId,
    handler: EventHandler,
}

/// Synchronous, single-threaded event bus for publishing and subscribing to editor events.
#[derive(Default)]
pub struct EventBus {
    subscriptions: Vec<Subscription>,
    queued_events: Vec<EditorEvent>,
}

impl EventBus {
    /// Creates a new, empty event bus.
    #[must_use]
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
            queued_events: Vec::new(),
        }
    }

    /// Subscribes a handler to receive editor events.
    ///
    /// Returns a `SubscriptionId` that can be used to unsubscribe later.
    pub fn subscribe<F>(&mut self, handler: F) -> SubscriptionId
    where
        F: FnMut(&EditorEvent) + 'static,
    {
        let id = SubscriptionId(NEXT_SUB_ID.fetch_add(1, Ordering::Relaxed));
        self.subscriptions.push(Subscription {
            id,
            handler: Box::new(handler),
        });
        id
    }

    /// Unsubscribes a previously registered handler by its `SubscriptionId`.
    ///
    /// Returns `true` if a subscription was found and removed.
    pub fn unsubscribe(&mut self, id: SubscriptionId) -> bool {
        if let Some(pos) = self.subscriptions.iter().position(|s| s.id == id) {
            self.subscriptions.remove(pos);
            true
        } else {
            false
        }
    }

    /// Publishes an event to all subscribers and records it in the internal queue.
    pub fn publish(&mut self, event: EditorEvent) {
        for sub in &mut self.subscriptions {
            (sub.handler)(&event);
        }
        self.queued_events.push(event);
    }

    /// Returns a slice of all events published since creation or the last `drain_events`.
    #[must_use]
    pub fn events(&self) -> &[EditorEvent] {
        &self.queued_events
    }

    /// Drains and returns all queued events, clearing the internal log.
    pub fn drain_events(&mut self) -> Vec<EditorEvent> {
        std::mem::take(&mut self.queued_events)
    }

    /// Clears the queued events log without returning them.
    pub fn clear_events(&mut self) {
        self.queued_events.clear();
    }
}
