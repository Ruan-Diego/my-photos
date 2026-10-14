//! The asynchronous queue (RF-057): worker threads decode and write the files; one
//! writer thread owns the database connection, records results in batches and keeps
//! the cache under its limit (RF-058).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rusqlite::Connection;

use super::{
    cache_bytes, evict, job_for, now, original_path, record_done, record_failure, render,
    thumb_path, write_thumb, Job, Priority, ThumbError, ThumbQueue, ThumbSize, ThumbSummary,
};

/// Results recorded per transaction at most; the writer commits sooner when idle.
const WRITE_BATCH: usize = 64;

enum Outcome {
    Done { width: u32, height: u32, bytes: u64 },
    Failed(ThumbError),
    WriteFailed,
}

enum Message {
    Request(i64, ThumbSize),
    Finished(Job, Outcome),
    Stop,
}

#[derive(Default)]
struct State {
    queue: ThumbQueue,
    /// Jobs taken by a worker plus requests sent to the writer, not yet recorded.
    busy: u64,
    closed: bool,
    cache_full: bool,
    summary: ThumbSummary,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub struct ThumbService {
    shared: Arc<Shared>,
    sender: Sender<Message>,
    workers: Vec<JoinHandle<()>>,
    writer: Option<JoinHandle<Connection>>,
}

impl ThumbService {
    /// `limit` is the cache size in bytes (`cache_limit`).
    pub fn start(connection: Connection, root: PathBuf, workers: usize, limit: u64) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            changed: Condvar::new(),
        });
        shared.lock().summary.cache_limit_bytes = limit;
        let (sender, receiver) = mpsc::channel();
        let workers = (0..workers.max(1))
            .map(|_| {
                let shared = Arc::clone(&shared);
                let sender = sender.clone();
                let root = root.clone();
                thread::spawn(move || work(&shared, &sender, &root))
            })
            .collect();
        let writer = {
            let shared = Arc::clone(&shared);
            thread::spawn(move || write(connection, &root, limit, &shared, &receiver))
        };
        Self {
            shared,
            sender,
            workers,
            writer: Some(writer),
        }
    }

    /// Background jobs (from `pending`). Ignored once the cache is full.
    pub(crate) fn enqueue(&self, jobs: Vec<Job>) {
        let mut state = self.shared.lock();
        if state.cache_full {
            return;
        }
        for job in jobs {
            if state.queue.push(job) {
                state.summary.queued += 1;
            }
        }
        drop(state);
        self.shared.changed.notify_all();
    }

    /// The UI needs this thumbnail now: it jumps the queue.
    pub fn request(&self, media_id: i64, size: ThumbSize) {
        self.shared.lock().busy += 1;
        if self.sender.send(Message::Request(media_id, size)).is_err() {
            self.shared.lock().busy -= 1;
        }
    }

    /// Blocks until nothing is queued or in progress. `progress(done, queued)`.
    pub fn wait_idle(&self, progress: &mut dyn FnMut(u64, u64)) {
        let mut reported = None;
        let mut state = self.shared.lock();
        loop {
            let now = (state.summary.done(), state.summary.queued);
            if reported != Some(now) {
                reported = Some(now);
                progress(now.0, now.1);
            }
            if state.queue.is_empty() && state.busy == 0 {
                return;
            }
            state = self
                .shared
                .changed
                .wait_timeout(state, Duration::from_millis(500))
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
    }

    pub fn summary(&self) -> ThumbSummary {
        self.shared.lock().summary.clone()
    }

    /// Stops after the jobs in progress (queued ones are dropped) and returns the
    /// connection for `db::close`.
    pub fn shutdown(mut self) -> (Connection, ThumbSummary) {
        self.shared.lock().closed = true;
        self.shared.changed.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        let _ = self.sender.send(Message::Stop);
        let connection = self
            .writer
            .take()
            .expect("writer")
            .join()
            .expect("thumbnail writer panicked");
        let summary = self.shared.lock().summary.clone();
        (connection, summary)
    }
}

fn work(shared: &Shared, sender: &Sender<Message>, root: &Path) {
    loop {
        let job = {
            let mut state = shared.lock();
            loop {
                if state.closed {
                    return;
                }
                if let Some(job) = state.queue.pop() {
                    state.busy += 1;
                    break job;
                }
                state = shared
                    .changed
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
        };
        let outcome = make(root, &job);
        if sender.send(Message::Finished(job, outcome)).is_err() {
            return;
        }
    }
}

fn make(root: &Path, job: &Job) -> Outcome {
    match render(&original_path(root, job), job.size, job.orientation) {
        Ok(rendered) => match write_thumb(&thumb_path(root, job.media_id, job.size), &rendered.jpeg) {
            Ok(()) => Outcome::Done {
                width: rendered.width,
                height: rendered.height,
                bytes: rendered.jpeg.len() as u64,
            },
            Err(_) => Outcome::WriteFailed,
        },
        Err(error) => Outcome::Failed(error),
    }
}

fn write(
    mut connection: Connection,
    root: &Path,
    limit: u64,
    shared: &Shared,
    receiver: &Receiver<Message>,
) -> Connection {
    while let Ok(first) = receiver.recv() {
        let mut batch = vec![first];
        while batch.len() < WRITE_BATCH {
            match receiver.try_recv() {
                Ok(message) => batch.push(message),
                Err(_) => break,
            }
        }
        if record_batch(&mut connection, root, limit, shared, batch) {
            break;
        }
    }
    connection
}

/// True when told to stop.
fn record_batch(
    connection: &mut Connection,
    root: &Path,
    limit: u64,
    shared: &Shared,
    batch: Vec<Message>,
) -> bool {
    let now = now();
    let mut stop = false;
    let mut settled = 0u64;
    let mut requested = Vec::new();
    let mut finished = Vec::new();
    let mut background_done = false;
    let mut keep = HashSet::new();

    let saved = connection.transaction().and_then(|transaction| {
        for message in batch {
            match message {
                Message::Stop => stop = true,
                Message::Request(media_id, size) => {
                    settled += 1;
                    match job_for(&transaction, media_id, size)? {
                        Some(job) => requested.push(job),
                        None => {
                            transaction.execute(
                                "UPDATE thumbnails SET last_accessed_at = ?3 WHERE media_id = ?1 AND size = ?2",
                                rusqlite::params![media_id, size.as_column(), now],
                            )?;
                        }
                    }
                }
                Message::Finished(job, outcome) => {
                    settled += 1;
                    match &outcome {
                        Outcome::Done { width, height, bytes } => {
                            record_done(&transaction, &job, *width, *height, *bytes, &now)?;
                            if job.priority == Priority::Requested {
                                keep.insert((job.media_id, job.size));
                            } else {
                                background_done = true;
                            }
                        }
                        Outcome::Failed(error) => record_failure(&transaction, &job, *error, &now)?,
                        Outcome::WriteFailed => {}
                    }
                    finished.push(outcome);
                }
            }
        }
        transaction.commit()
    });

    let mut eviction = None;
    if saved.is_ok() && (background_done || !keep.is_empty()) {
        eviction = evict(connection, root, limit, !keep.is_empty(), &keep).ok();
    }
    let total = eviction
        .map(|eviction| eviction.cache_bytes)
        .or_else(|| cache_bytes(connection).ok());

    let mut state = shared.lock();
    for outcome in &finished {
        let summary = &mut state.summary;
        match (outcome, saved.is_ok()) {
            (_, false) | (Outcome::WriteFailed, _) => summary.write_failed += 1,
            (Outcome::Done { .. }, true) => summary.generated += 1,
            (Outcome::Failed(ThumbError::Unsupported), true) => summary.unsupported += 1,
            (Outcome::Failed(ThumbError::Corrupt), true) => summary.corrupt += 1,
            (Outcome::Failed(ThumbError::Unreadable), true) => summary.unreadable += 1,
        }
    }
    for job in requested {
        if state.queue.push(job) {
            state.summary.queued += 1;
        }
    }
    if let Some(eviction) = eviction {
        state.summary.evicted += eviction.count;
        state.summary.evicted_bytes += eviction.bytes;
    }
    if let Some(total) = total {
        state.summary.cache_bytes = total;
        // Only small thumbnails are left and they fill the cache: stop the background fill.
        if background_done && total > limit && !state.cache_full {
            state.cache_full = true;
            state.summary.stopped_cache_full = true;
            state.queue.drop_background();
        }
    }
    state.busy = state.busy.saturating_sub(settled);
    drop(state);
    shared.changed.notify_all();
    stop
}
