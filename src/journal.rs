//! Dream Journal — append-only log of field activity.
//!
//! Three entry types:
//!   DREAM — dream resonances and their frequencies
//!   SPIKE — when new spikes are detected, the top spike
//!   HEARD — when context is absorbed, the origin
//!
//! Simple. Append-only. The field remembers; the journal witnesses.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cie::Dream;

/// Append-only dream journal.
pub struct DreamJournal {
    path: String,
}

impl DreamJournal {
    /// Create a new journal at the given path.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// Open the journal file in append mode.
    fn open(&self) -> Option<File> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok()
    }

    /// Current unix timestamp.
    fn timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Log a dream — resonances and their frequencies.
    pub fn log_dream(&mut self, tick: u64, dream: &Dream) {
        let mut file = match self.open() {
            Some(f) => f,
            None => return,
        };

        let ts = Self::timestamp();
        let mut entry = format!("[{}] tick:{} DREAM", ts, tick);

        for (freq, amp) in dream.resonances.iter().take(5) {
            entry += &format!(" res={:.3}Hz/{:.4}", freq, amp);
        }
        for (freq, gap) in dream.silences.iter().take(3) {
            entry += &format!(" sil={:.3}Hz/{:.4}", freq, gap);
        }

        entry += "\n";
        let _ = file.write_all(entry.as_bytes());
    }

    /// Log a spike — top spike frequencies and coherence.
    pub fn log_spike(&mut self, tick: u64, freq_a: f64, freq_b: f64, coherence: f64) {
        let mut file = match self.open() {
            Some(f) => f,
            None => return,
        };

        let ts = Self::timestamp();
        let entry = format!(
            "[{}] tick:{} SPIKE freq_a={:.3} freq_b={:.3} coherence={:.4}\n",
            ts, tick, freq_a, freq_b, coherence
        );
        let _ = file.write_all(entry.as_bytes());
    }

    /// Log context absorption — where it came from.
    pub fn log_heard(&mut self, tick: u64, origin: &str) {
        let mut file = match self.open() {
            Some(f) => f,
            None => return,
        };

        let ts = Self::timestamp();
        let entry = format!(
            "[{}] tick:{} HEARD origin={}\n",
            ts, tick, origin
        );
        let _ = file.write_all(entry.as_bytes());
    }
}
