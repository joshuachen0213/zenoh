use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::{Duration, Instant},
};
use zenoh_protocol::transport::TransportSn;
// use tracing::info;

const ALPHA: f64 = 0.1;

#[derive(Clone)]
pub struct QueueStatsReport {
    pub avg_qsize: f64,
    pub droprate: f64,
    pub avg_qdelay: f64,
}

pub struct QueueStats {
    queue_counter: AtomicUsize,
    timemap: Arc<Mutex<HashMap<TransportSn, Instant>>>,
    pub qsize: Arc<Mutex<Vec<usize>>>,
    pub dropped: AtomicUsize,
    pub tried: AtomicUsize,
    pub queueing_delay: Arc<Mutex<Vec<usize>>>,
}
#[allow(dead_code)]
impl QueueStats {
    pub fn new() -> Self {
        Self {
            queue_counter: AtomicUsize::new(0),
            timemap: Arc::new(Mutex::new(HashMap::new())),
            qsize: Arc::new(Mutex::new(Vec::new())),
            dropped: AtomicUsize::new(0),
            tried: AtomicUsize::new(0),
            queueing_delay: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn record_qsize(&self) {
        let cur_qsize = self.queue_counter.load(Ordering::Relaxed);
        self.qsize.lock().unwrap().push(cur_qsize);
    }
    pub fn push_q_delay(&self, delay: usize) {
        self.queueing_delay.lock().unwrap().push(delay);
    }
    pub fn push_time(&self, sn: TransportSn, time: Instant) {
        self.timemap.lock().unwrap().insert(sn, time);
    }
    pub fn inc_q_cnt(&self) {
        self.queue_counter.fetch_add(1, Ordering::Relaxed);
    }
    pub fn dec_q_cnt(&self) {
        self.queue_counter.fetch_sub(1, Ordering::Relaxed);
    }
    pub fn inc_dropped(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_tried(&self) {
        self.tried.fetch_add(1, Ordering::Relaxed);
    }
    pub fn report(&self) -> QueueStatsReport {
        let qsize = self.qsize.lock().unwrap();
        if qsize.is_empty() {
            return QueueStatsReport {
                avg_qsize: 0.0,
                droprate: 0.0,
                avg_qdelay: 0.0,
            };
        }
        let mut avg_qsize = qsize[0] as f64;
        avg_qsize = qsize.iter()
            .map(|&value| value as f64) // Convert input to f64
            .fold(avg_qsize, |state, value| {
                // Use the exponential moving average formula
                ALPHA * value + (1.0 - ALPHA) * state
            });
        drop(qsize); // release the lock
                     /*
                         let qdelay = self.queueing_delay.lock().unwrap();
                         let mut avg_qdelay = qdelay[0] as f64;
                         avg_qdelay = qdelay.iter()
                             .map(|&value| value as f64) // Convert input to f64
                             .fold(avg_qdelay, |state, value| {
                                 // Use the exponential moving average formula
                                 ALPHA * value + (1.0 - ALPHA) * state
                             });
                         drop(qdelay); // release the lock
                     */
        let droprate =
            self.dropped.load(Ordering::Relaxed) as f64 / self.tried.load(Ordering::Relaxed) as f64;
        QueueStatsReport {
            avg_qsize,
            droprate,
            avg_qdelay: 0.0,
        }
    }
}

impl QueueStatsReport {
    pub fn openmetrics_text(&self) -> String {
        let mut string = String::new();
        string.push_str("=== Zenoh Queue Stats Report ===\n");
        string.push_str("The average queue size is ");
        string.push_str(&self.avg_qsize.to_string());
        string.push_str("\n");
        string.push_str("The drop rate is ");
        string.push_str(&self.droprate.to_string());
        string.push_str("\n");
        string.push_str("The average queueing delay is ");
        string.push_str(&self.avg_qdelay.to_string());
        string.push_str("\n");
        string.push_str("=== End of Zenoh Queue Stats Report ===\n");
        string
    }
    pub fn print_report(&self) {
        println!("{}", self.openmetrics_text());
    }
}

impl Drop for QueueStats {
    fn drop(&mut self) {
        self.report().print_report();
    }
}
