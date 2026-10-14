//! The work order: what the UI asked for first, then the rest newest first (RF-057).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use super::ThumbSize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Filling in the library in the background.
    Background,
    /// The UI is showing this photo now.
    Requested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub media_id: i64,
    pub size: ThumbSize,
    pub relative_path: String,
    pub orientation: Option<u16>,
    pub fingerprint: String,
    /// UTC ISO text, so it sorts as time.
    pub taken_at: Option<String>,
    pub priority: Priority,
}

impl Job {
    fn key(&self) -> (i64, ThumbSize) {
        (self.media_id, self.size)
    }
}

struct Entry(Job);

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Max-heap: higher priority, then newer (`None` oldest), then higher id.
        self.0
            .priority
            .cmp(&other.0.priority)
            .then_with(|| self.0.taken_at.cmp(&other.0.taken_at))
            .then_with(|| self.0.media_id.cmp(&other.0.media_id))
            .then_with(|| other.0.size.cmp(&self.0.size))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Entry {}

/// One entry per (photo, size). Asking again at a higher priority moves it up; the
/// lower copy left in the heap is skipped when it comes out.
#[derive(Default)]
pub struct ThumbQueue {
    heap: BinaryHeap<Entry>,
    queued: HashMap<(i64, ThumbSize), Priority>,
}

impl ThumbQueue {
    /// False when the same job is already queued at this priority or higher.
    pub fn push(&mut self, job: Job) -> bool {
        if self
            .queued
            .get(&job.key())
            .is_some_and(|&queued| queued >= job.priority)
        {
            return false;
        }
        self.queued.insert(job.key(), job.priority);
        self.heap.push(Entry(job));
        true
    }

    pub fn pop(&mut self) -> Option<Job> {
        while let Some(Entry(job)) = self.heap.pop() {
            if self.queued.get(&job.key()) == Some(&job.priority) {
                self.queued.remove(&job.key());
                return Some(job);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }

    /// Drops every background job (the cache is full); returns how many.
    pub fn drop_background(&mut self) -> usize {
        let before = self.queued.len();
        self.queued
            .retain(|_, priority| *priority == Priority::Requested);
        self.heap
            .retain(|Entry(job)| job.priority == Priority::Requested);
        before - self.queued.len()
    }
}
