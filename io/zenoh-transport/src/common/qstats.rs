use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

const ALPHA: f64 = 0.1;

#[derive(Clone)]
pub struct QueueStatsReport {
    priority: usize,
    pub avg_qsize: f64,
    pub droprate: f64,
    pub avg_qdelay: f64,
    pub mcnt: usize,
}

pub struct QueueStats {
    queue_counter: AtomicUsize,
    priority: usize,
    pub qsize: Arc<Mutex<Vec<usize>>>,
    pub dropped: AtomicUsize,
    pub tried: AtomicUsize,
    pub queueing_delay: Vec<usize>,
}
#[allow(dead_code)]
impl QueueStats {
    pub fn new(priority: usize) -> Self {
        Self {
            queue_counter: AtomicUsize::new(0),
            priority,
            qsize: Arc::new(Mutex::new(Vec::new())),
            dropped: AtomicUsize::new(0),
            tried: AtomicUsize::new(0),
            queueing_delay: Vec::new(),
        }
    }
    pub fn record_qsize(&self) {
        let cur_qsize = self.queue_counter.load(Ordering::Relaxed);
        self.qsize.lock().unwrap().push(cur_qsize);
    }
    pub fn push_qdelay(&mut self, delay: usize) {
        self.queueing_delay.push(delay);
    }
    pub fn inc_qcnt(&self) {
        self.queue_counter.fetch_add(1, Ordering::Relaxed);
    }
    pub fn dec_qcnt(&self) {
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
        let avg_qsize = if !qsize.is_empty() {
            let avg_qsize = qsize[0] as f64;
            qsize.iter()
                .map(|&value| value as f64) // Convert input to f64
                .fold(avg_qsize, |state, value| {
                    // Use the exponential moving average formula
                    ALPHA * value + (1.0 - ALPHA) * state
                })
        } else {
            0.0
        };

        let qdelay = &self.queueing_delay;
        let avg_qdelay = if !qdelay.is_empty() {
            let avg_qdelay = qdelay[0] as f64;
            qdelay.iter()
                .map(|&value| value as f64) // Convert input to f64
                .fold(avg_qdelay, |state, value| {
                    // Use the exponential moving average formula
                    ALPHA * value + (1.0 - ALPHA) * state
                })
        } else {
            0.0
        };
        let dropped = self.dropped.load(Ordering::Relaxed) as f64;
        let tried = self.tried.load(Ordering::Relaxed) as f64;
        let droprate = if tried > 0.0 { dropped / tried } else { 0.0 };

        QueueStatsReport {
            priority: self.priority,
            avg_qsize,
            droprate,
            avg_qdelay,
            mcnt: self.tried.load(Ordering::Relaxed),
        }
    }
}

impl QueueStatsReport {
    pub fn openmetrics_text(&self) -> String {
        let mut string = String::new();
        string.push_str(&format!(
            "=== Zenoh Queue Stats for priority {} Report ===\n",
            self.priority
        ));
        string.push_str(&format!(
            "The average message queue size is {}\n",
            self.avg_qsize
        ));
        string.push_str(&format!("The total message count is {}\n", self.mcnt));
        string.push_str(&format!("The drop rate is {}%\n", self.droprate * 100.0));
        string.push_str(&format!(
            "The average queueing delay is {} us\n",
            self.avg_qdelay
        ));
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
