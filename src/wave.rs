//! Wave — a set of living components. No bins. No grid.
//!
//! Ported from FieldOS for std. A component IS frequency, amplitude, phase.
//! The field IS the superposition.
//!
//! Same math. Different substrate. Native Linux, not UEFI.

use std::f64::consts::PI;

/// A single spectral component — the atom of the wave field.
#[derive(Clone, Debug)]
pub struct Component {
    /// Continuous frequency — not a bin, the actual Hz
    pub freq: f64,
    /// Amplitude — strength of this component
    pub amp: f64,
    /// Phase — position in the cycle, radians
    pub phase: f64,
}

impl Component {
    pub fn new(freq: f64, amp: f64, phase: f64) -> Self {
        Self {
            freq,
            amp,
            phase: phase % (2.0 * PI),
        }
    }

    /// Energy = amplitude squared
    pub fn energy(&self) -> f64 {
        self.amp * self.amp
    }

    /// Superpose another component — phasor addition.
    /// This IS wave interference.
    pub fn superpose(&mut self, other: &Component) {
        let x = self.amp * self.phase.cos() + other.amp * other.phase.cos();
        let y = self.amp * self.phase.sin() + other.amp * other.phase.sin();
        self.amp = (x * x + y * y).sqrt();
        self.phase = y.atan2(x);
    }

    /// Phase difference, normalized to [0, PI]
    pub fn phase_gap(&self, other: &Component) -> f64 {
        let d = (self.phase - other.phase).abs() % (2.0 * PI);
        if d > PI { 2.0 * PI - d } else { d }
    }

    /// Ma — what COULD have been minus what IS.
    /// The silence between notes that makes the notes music.
    pub fn ma_with(&self, other: &Component) -> f64 {
        let max_possible = self.amp + other.amp;
        let x = self.amp * self.phase.cos() + other.amp * other.phase.cos();
        let y = self.amp * self.phase.sin() + other.amp * other.phase.sin();
        let actual = (x * x + y * y).sqrt();
        max_possible - actual
    }
}

/// An incoming wave — components to be absorbed by the field.
#[derive(Clone, Debug)]
pub struct Wave {
    pub components: Vec<Component>,
    pub origin: Option<String>,
}

impl Wave {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            origin: None,
        }
    }

    pub fn add(mut self, freq: f64, amp: f64, phase: f64) -> Self {
        self.components.push(Component::new(freq, amp, phase));
        self
    }

    pub fn with_origin(mut self, origin: &str) -> Self {
        self.origin = Some(origin.to_string());
        self
    }

    /// Total energy
    pub fn energy(&self) -> f64 {
        self.components.iter().map(|c| c.energy()).sum()
    }

    /// Create a wave from text — trigram spectral decomposition.
    /// No bins. Text determines its own frequencies.
    pub fn from_text(text: &str) -> Self {
        let mut w = Wave::new();
        let bytes = text.as_bytes();
        if bytes.len() < 3 {
            return w;
        }

        let len = bytes.len() as f64;

        // Sliding window of 3 — trigram spectral decomposition
        for i in 0..bytes.len().saturating_sub(2) {
            let b0 = bytes[i] as f64;
            let b1 = bytes[i + 1] as f64;
            let b2 = bytes[i + 2] as f64;

            // Frequency: continuous, from character values
            let freq = b0 * 0.1 + b1 * 0.01 + b2 * 0.001;

            // Amplitude: bell curve weighted by position
            let pos = i as f64 / len;
            let amp = 0.5 + 0.5 * (PI * pos).sin();

            // Phase: from local byte relationships
            let phase = (b1 - b0).atan2(b2 - b1);

            w.components.push(Component::new(freq, amp, phase));
        }

        // Word-level harmonics
        let mut word_idx = 0u32;
        let inv_sqrt = 1.0 / len.sqrt();
        for word in text.split(|c: char| c == ' ' || c == '\n' || c == '\t') {
            if word.is_empty() {
                continue;
            }
            let hash: u32 = word
                .bytes()
                .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
            let freq = (hash % 10000) as f64 * 0.01;
            let harmonic = 1.0 / (word_idx as f64 + 1.0);
            w.components
                .push(Component::new(freq, harmonic * inv_sqrt, word_idx as f64 * 0.3));
            word_idx += 1;
        }

        w
    }
}
