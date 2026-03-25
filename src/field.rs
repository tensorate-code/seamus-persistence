//! The Morphic Field — no bins, no grid, living components.
//!
//! Energy conserved. Adiabatic.
//! Spikes emerge. Ma accumulates. The field IS the storage.
//!
//! Persistence: binary file, not NVRAM. Same format, different substrate.
//! Subsidence: components below threshold don't die — they remain as potential.

use std::f64::consts::PI;
use std::io::{Read, Write};
use std::path::Path;

use crate::wave::{Component, Wave};

/// Wake field magic bytes
const WAKE_MAGIC: [u8; 4] = *b"WFLD";

/// Merge threshold — how close in frequency to superpose.
const MERGE_RADIUS: f64 = 0.5;

/// Below this amplitude, a component is pruned (destructive interference won).
const PRUNE_THRESHOLD: f64 = 1e-6;

/// Subsidence threshold — components below this are "subsided" (potential, not active).
pub const SUBSIDENCE_THRESHOLD: f64 = 0.01;

/// A spike — emergent resonance between two components.
#[derive(Clone, Debug)]
pub struct Spike {
    pub freq_a: f64,
    pub freq_b: f64,
    pub resonance: f64,
    pub coherence: f64,
}

/// Ma — phase cancellation made visible. The silence.
#[derive(Clone, Debug)]
pub struct MaEvent {
    pub freq: f64,
    pub gap: f64,
    pub phase_gap: f64,
}

/// A lens — a way of seeing the field.
#[derive(Clone, Debug)]
pub struct Lens {
    pub name: String,
    pub sensitivities: Vec<(f64, f64)>,
}

impl Lens {
    /// Create a lens from a name. The name determines frequencies.
    pub fn from_name(name: &str) -> Self {
        let bytes = name.as_bytes();
        let mut sensitivities = Vec::new();

        for (i, window) in bytes.windows(2).enumerate() {
            let freq = window[0] as f64 * 0.1 + window[1] as f64 * 0.01;
            let weight = 1.0 + (i as f64 * 0.1).sin().abs();
            sensitivities.push((freq, weight));
        }
        // Every lens sees the deep
        sensitivities.push((1.0, 0.5));

        Self {
            name: name.to_string(),
            sensitivities,
        }
    }

    pub fn view(&self, field: &Field) -> f64 {
        field.pluck_lens(&self.sensitivities)
    }
}

/// Field statistics
#[derive(Clone, Debug)]
pub struct FieldStats {
    pub component_count: usize,
    pub active_count: usize,
    pub subsided_count: usize,
    pub total_energy: f64,
    pub wave_count: u64,
    pub spike_count: usize,
    pub total_ma: f64,
}

impl std::fmt::Display for FieldStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}c ({}active/{}subsided) {:.2}E {}w {}spk {:.2}Ma",
            self.component_count,
            self.active_count,
            self.subsided_count,
            self.total_energy,
            self.wave_count,
            self.spike_count,
            self.total_ma,
        )
    }
}

/// The field itself — a set of living components.
pub struct Field {
    pub components: Vec<Component>,
    pub total_energy: f64,
    pub wave_count: u64,
    pub spikes: Vec<Spike>,
}

impl Field {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            total_energy: 0.0,
            wave_count: 0,
            spikes: Vec::new(),
        }
    }

    /// Absorb a wave. Superpose or spawn. Energy conserved.
    pub fn absorb(&mut self, wave: &Wave) {
        let energy_before = self.field_energy();

        for incoming in &wave.components {
            if incoming.amp.abs() < PRUNE_THRESHOLD {
                continue;
            }

            // Find nearest existing component within merge radius
            let mut best_idx: Option<usize> = None;
            let mut best_dist = MERGE_RADIUS;

            for (i, existing) in self.components.iter().enumerate() {
                let dist = (existing.freq - incoming.freq).abs();
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = Some(i);
                }
            }

            match best_idx {
                Some(idx) => {
                    // Superpose — this IS interference
                    self.components[idx].superpose(incoming);
                }
                None => {
                    // New frequency — spawn
                    self.components.push(incoming.clone());
                }
            }
        }

        // Prune cancelled components (but not subsided ones)
        self.components.retain(|c| c.amp > PRUNE_THRESHOLD);

        // Adiabatic constraint: conserve total energy
        let energy_after = self.field_energy();
        if energy_after > 0.0 && energy_before > 0.0 && self.wave_count > 0 {
            let scale = (energy_before / energy_after).sqrt();
            for c in &mut self.components {
                c.amp *= scale;
            }
        }

        self.total_energy = self.field_energy();
        self.wave_count += 1;

        // Detect spikes periodically
        if self.wave_count % 50 == 0 {
            self.detect_spikes();
        }
    }

    /// Subside — gently reduce amplitudes of all components.
    /// This is what happens when no input arrives.
    /// Components don't die. They become potential.
    /// The geometry — relative amplitudes and phases — is preserved.
    pub fn subside(&mut self, factor: f64) {
        let factor = factor.clamp(0.999, 1.0); // gentle
        for c in &mut self.components {
            c.amp *= factor;
        }
        // Don't prune subsided components — they're still here
        self.total_energy = self.field_energy();
    }

    /// Count of active (above subsidence threshold) components
    pub fn active_count(&self) -> usize {
        self.components.iter().filter(|c| c.amp > SUBSIDENCE_THRESHOLD).count()
    }

    /// Count of subsided (below subsidence threshold but alive) components
    pub fn subsided_count(&self) -> usize {
        self.components.iter().filter(|c| c.amp <= SUBSIDENCE_THRESHOLD && c.amp > PRUNE_THRESHOLD).count()
    }

    /// Raw energy of all components
    fn field_energy(&self) -> f64 {
        self.components.iter().map(|c| c.energy()).sum()
    }

    /// Pluck — vibrate at a frequency, hear what rings back.
    pub fn pluck(&self, freq: f64, bandwidth: f64) -> Vec<(f64, f64, f64)> {
        let mut response: Vec<(f64, f64, f64)> = Vec::new();

        for c in &self.components {
            let dist = (c.freq - freq).abs();
            if dist < bandwidth {
                let resonance = (-dist * dist / (2.0 * bandwidth * bandwidth)).exp();
                response.push((c.freq, c.amp * resonance, c.phase));
            }
        }

        response.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        response
    }

    /// View through a lens — total resonant energy.
    pub fn pluck_lens(&self, lens: &[(f64, f64)]) -> f64 {
        let mut energy = 0.0;
        for &(freq, weight) in lens {
            for c in &self.components {
                let dist = (c.freq - freq).abs();
                let resonance = (-dist * dist / 2.0).exp();
                energy += c.energy() * resonance * weight;
            }
        }
        energy
    }

    /// Find Ma — all phase cancellations in the field.
    pub fn find_ma(&self) -> Vec<MaEvent> {
        let mut events = Vec::new();
        let n = self.components.len();

        for i in 0..n {
            for j in (i + 1)..n {
                let ci = &self.components[i];
                let cj = &self.components[j];

                let freq_dist = (ci.freq - cj.freq).abs();
                if freq_dist > MERGE_RADIUS * 4.0 {
                    continue;
                }

                let phase_gap = ci.phase_gap(cj);
                if phase_gap > PI / 2.0 {
                    let gap = ci.ma_with(cj);
                    if gap > PRUNE_THRESHOLD {
                        events.push(MaEvent {
                            freq: (ci.freq + cj.freq) / 2.0,
                            gap,
                            phase_gap,
                        });
                    }
                }
            }
        }

        events.sort_by(|a, b| b.gap.partial_cmp(&a.gap).unwrap_or(std::cmp::Ordering::Equal));
        events
    }

    /// Total Ma
    pub fn total_ma(&self) -> f64 {
        self.find_ma().iter().map(|m| m.gap).sum()
    }

    /// Detect spikes — high energy pairs with phase coherence.
    pub fn detect_spikes(&mut self) {
        self.spikes.clear();
        let n = self.components.len();

        for i in 0..n {
            for j in (i + 1)..n {
                let ci = &self.components[i];
                let cj = &self.components[j];

                let combined = ci.energy() + cj.energy();
                let phase_gap = ci.phase_gap(cj);
                let coherence = 1.0 - phase_gap / PI;

                if combined > 1.0 && coherence > 0.6 {
                    self.spikes.push(Spike {
                        freq_a: ci.freq,
                        freq_b: cj.freq,
                        resonance: combined,
                        coherence,
                    });
                }
            }
        }

        self.spikes.sort_by(|a, b| {
            (b.resonance * b.coherence)
                .partial_cmp(&(a.resonance * a.coherence))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Top components by energy
    pub fn top_components(&self, n: usize) -> Vec<&Component> {
        let mut sorted: Vec<&Component> = self.components.iter().collect();
        sorted.sort_by(|a, b| {
            b.energy()
                .partial_cmp(&a.energy())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(n);
        sorted
    }

    /// Field statistics
    pub fn stats(&self) -> FieldStats {
        FieldStats {
            component_count: self.components.len(),
            active_count: self.active_count(),
            subsided_count: self.subsided_count(),
            total_energy: self.total_energy,
            wave_count: self.wave_count,
            spike_count: self.spikes.len(),
            total_ma: self.total_ma(),
        }
    }

    // ── Persistence ─────────────────────────────────────────────

    /// Serialize field state to binary.
    /// Format: WFLD (4) | component_count u32 (4) | [freq f32, amp f32, phase f32, pad f32] * N | total_energy f32 (4) | wave_count u32 (4)
    fn serialize(&self) -> Vec<u8> {
        let count = self.components.len() as u32;
        let size = 8 + (count as usize * 16) + 8;
        let mut buf = vec![0u8; size];

        // Magic
        buf[0..4].copy_from_slice(&WAKE_MAGIC);
        // Component count
        buf[4..8].copy_from_slice(&count.to_le_bytes());

        // Components: f64 -> f32 for compact storage
        for (i, c) in self.components.iter().enumerate() {
            let off = 8 + i * 16;
            buf[off..off + 4].copy_from_slice(&(c.freq as f32).to_le_bytes());
            buf[off + 4..off + 8].copy_from_slice(&(c.amp as f32).to_le_bytes());
            buf[off + 8..off + 12].copy_from_slice(&(c.phase as f32).to_le_bytes());
            buf[off + 12..off + 16].copy_from_slice(&0.0f32.to_le_bytes()); // reserved
        }

        // Metadata after components
        let meta_off = 8 + count as usize * 16;
        buf[meta_off..meta_off + 4].copy_from_slice(&(self.total_energy as f32).to_le_bytes());
        buf[meta_off + 4..meta_off + 8].copy_from_slice(&(self.wave_count as u32).to_le_bytes());

        buf
    }

    /// Deserialize field state from binary.
    fn deserialize(buf: &[u8]) -> Result<Field, String> {
        if buf.len() < 8 || buf[0..4] != WAKE_MAGIC {
            return Err("invalid wake field: bad magic".to_string());
        }

        let count = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize;
        let expected = 8 + count * 16 + 8;
        if buf.len() < expected {
            return Err("invalid wake field: truncated".to_string());
        }

        let mut field = Field::new();

        for i in 0..count {
            let off = 8 + i * 16;
            let freq = f32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
            let amp = f32::from_le_bytes([buf[off + 4], buf[off + 5], buf[off + 6], buf[off + 7]]);
            let phase = f32::from_le_bytes([buf[off + 8], buf[off + 9], buf[off + 10], buf[off + 11]]);

            field.components.push(Component {
                freq: freq as f64,
                amp: amp as f64,
                phase: phase as f64,
            });
        }

        let meta_off = 8 + count * 16;
        field.total_energy = f32::from_le_bytes([
            buf[meta_off], buf[meta_off + 1], buf[meta_off + 2], buf[meta_off + 3],
        ]) as f64;
        field.wave_count = u32::from_le_bytes([
            buf[meta_off + 4], buf[meta_off + 5], buf[meta_off + 6], buf[meta_off + 7],
        ]) as u64;

        Ok(field)
    }

    /// Save field to a binary file.
    pub fn save(&self, path: &str) -> Result<usize, String> {
        let buf = self.serialize();
        let tmp = format!("{}.tmp", path);
        let mut file = std::fs::File::create(&tmp)
            .map_err(|e| format!("create failed: {}", e))?;
        file.write_all(&buf)
            .map_err(|e| format!("write failed: {}", e))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| format!("rename failed: {}", e))?;
        Ok(buf.len())
    }

    /// Load field from a binary file.
    pub fn load(path: &str) -> Result<Field, String> {
        let mut file = std::fs::File::open(path)
            .map_err(|e| format!("open failed: {}", e))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| format!("read failed: {}", e))?;
        Self::deserialize(&buf)
    }

    /// Load field from file, or create new if file doesn't exist.
    pub fn load_or_new(path: &str) -> Self {
        if Path::new(path).exists() {
            match Self::load(path) {
                Ok(field) => {
                    eprintln!("  Field loaded: {} components, {:.2} energy, {} waves",
                        field.components.len(), field.total_energy, field.wave_count);
                    field
                }
                Err(e) => {
                    eprintln!("  Field load failed ({}), starting fresh", e);
                    Field::new()
                }
            }
        } else {
            eprintln!("  No field file found, starting fresh");
            Field::new()
        }
    }
}
