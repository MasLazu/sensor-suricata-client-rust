use crate::pb::SensorEvent;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct SensorEventRecord {
    pub payload: SensorEvent,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct EventBatchQueue {
    // We use Mutex<HashMap> instead of DashMap because we want to swap the entire map
    // efficiently when processing batches.
    queue: Arc<Mutex<HashMap<String, SensorEventRecord>>>,
    delta: u64,
    // Metrics
    latest_event_per_sec: Arc<AtomicI64>,
    event_this_sec: Arc<AtomicI64>,
    latest_batch_per_sec: Arc<AtomicI64>,
    batch_this_sec: Arc<AtomicI64>,
    total_sent_events: Arc<AtomicI64>,
    total_processed_events: Arc<AtomicI64>,
}

impl EventBatchQueue {
    pub fn new(delta_seconds: u64) -> Self {
        Self {
            queue: Arc::new(Mutex::new(HashMap::new())),
            delta: delta_seconds,
            latest_event_per_sec: Arc::new(AtomicI64::new(0)),
            event_this_sec: Arc::new(AtomicI64::new(0)),
            latest_batch_per_sec: Arc::new(AtomicI64::new(0)),
            batch_this_sec: Arc::new(AtomicI64::new(0)),
            total_sent_events: Arc::new(AtomicI64::new(0)),
            total_processed_events: Arc::new(AtomicI64::new(0)),
        }
    }

    pub fn add(&self, mut event: SensorEvent) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Check if event already exists
        // We use the event hash as the key
        let key = event.event_hash_sha256.clone();

        {
            let mut queue = self.queue.lock().unwrap();
            queue
                .entry(key)
                .and_modify(|record| {
                    // Append metrics from new event to existing record
                    record.payload.metrics.append(&mut event.metrics);
                    record.updated_at = now;
                })
                .or_insert_with(|| SensorEventRecord {
                    payload: event,
                    created_at: now,
                    updated_at: now,
                });
        }

        self.event_this_sec.fetch_add(1, Ordering::Relaxed);
    }

    pub fn process_batch(&self) -> Vec<SensorEvent> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut batch = Vec::new();
        let mut total_metrics_count = 0;

        // Efficiently swap the entire map if delta is 0 (immediate processing)
        // or if we just want to process everything.
        // For non-zero delta, we still need to iterate, but we can do it efficiently.

        if self.delta == 0 {
            // O(1) swap strategy
            let mut queue = self.queue.lock().unwrap();
            if queue.is_empty() {
                return batch;
            }

            // Take the entire map content
            let old_queue = std::mem::take(&mut *queue);
            drop(queue); // Release lock immediately

            // Process outside the lock
            for (_, record) in old_queue {
                total_metrics_count += record.payload.metrics.len() as i64;
                batch.push(record.payload);
            }
        } else {
            // Standard iteration for time-based batching
            let mut queue = self.queue.lock().unwrap();
            let mut keys_to_remove = Vec::new();

            for (key, record) in queue.iter() {
                if now > record.updated_at + self.delta as i64 {
                    batch.push(record.payload.clone());
                    keys_to_remove.push(key.clone());
                    total_metrics_count += record.payload.metrics.len() as i64;
                }
            }

            for key in keys_to_remove {
                queue.remove(&key);
            }
        }

        if !batch.is_empty() {
            self.batch_this_sec.fetch_add(1, Ordering::Relaxed);
            self.total_sent_events
                .fetch_add(total_metrics_count, Ordering::Relaxed);
            self.total_processed_events.store(
                self.total_sent_events.load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
        }

        batch
    }

    pub fn update_metrics(&self) {
        let event_count = self.event_this_sec.swap(0, Ordering::Relaxed);
        self.latest_event_per_sec
            .store(event_count, Ordering::Relaxed);

        let batch_count = self.batch_this_sec.swap(0, Ordering::Relaxed);
        self.latest_batch_per_sec
            .store(batch_count, Ordering::Relaxed);
    }

    // Metrics Getters
    pub fn get_event_processed_per_second(&self) -> i64 {
        self.latest_event_per_sec.load(Ordering::Relaxed)
    }

    pub fn get_event_batch_sent_per_second(&self) -> i64 {
        self.latest_batch_per_sec.load(Ordering::Relaxed)
    }

    pub fn get_total_sent_events(&self) -> i64 {
        self.total_sent_events.load(Ordering::Relaxed)
    }

    pub fn get_total_processed_events(&self) -> i64 {
        self.total_processed_events.load(Ordering::Relaxed)
    }

    pub fn get_queue_size(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}
