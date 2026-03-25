//! CIE — Continuous Interference Engine.
//!
//! Not a scheduler. Not a service. A thinking process.
//! Context arrives → field absorbs → thought emerges → dream forms.
//!
//! Four streams, one field:
//!   Context  — what just arrived (absorbed into field)
//!   Thought  — what the field produces in response (pluck + interference)
//!   Memory   — what persists (the field itself IS memory)
//!   Dream    — what emerges when settled (subsided components resonating)
//!
//! Spike-driven. No timer. The field determines the pace.

use crate::field::{Field, Lens, Spike, MaEvent};
use crate::wave::Wave;

/// A dream — what emerges when the field settles and subsided components resonate.
#[derive(Clone, Debug)]
pub struct Dream {
    pub resonances: Vec<(f64, f64)>,   // (freq, amp) of dream resonances
    pub silences: Vec<(f64, f64)>,     // (freq, gap) of Ma in the dream
    pub spikes: Vec<(f64, f64, f64)>,  // (freq_a, freq_b, coherence)
}

/// The state after a tick — what happened.
#[derive(Clone, Debug)]
pub struct TickReport {
    pub tick: u64,
    pub context_absorbed: bool,
    pub thought_wave: Option<Wave>,
    pub new_spikes: usize,
    pub dreams: Vec<Dream>,
    pub settled: bool,
    pub active: usize,
    pub subsided: usize,
    /// Names of lenses born this tick from recurring spike frequencies.
    pub born_lenses: Vec<String>,
}

impl std::fmt::Display for TickReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tick:{} ", self.tick)?;
        if self.context_absorbed {
            write!(f, "heard ")?;
        }
        if let Some(ref tw) = self.thought_wave {
            write!(f, "thought({}c {:.2}E) ", tw.components.len(), tw.energy())?;
        }
        if self.new_spikes > 0 {
            write!(f, "SPIKE({}) ", self.new_spikes)?;
        }
        if !self.dreams.is_empty() {
            write!(f, "DREAM({}) ", self.dreams.len())?;
        }
        for name in &self.born_lenses {
            write!(f, "LENS-BORN({}) ", name)?;
        }
        write!(f, "| {}active/{}subsided", self.active, self.subsided)?;
        if self.settled {
            write!(f, " | settled")?;
        }
        Ok(())
    }
}

/// Queued context — text waiting to be absorbed.
struct QueuedContext {
    text: String,
    origin: String,
}

/// The CIE itself.
pub struct Cie {
    pub name: String,
    pub field: Field,
    pub tick_count: u64,
    pub walking_dream: Option<String>,

    // Internal state
    context_queue: Vec<QueuedContext>,
    settled_ticks: u64,
    prev_spike_count: usize,
    lenses: Vec<Lens>,
    /// Spike frequency recurrence tracker: (freq_a, freq_b, occurrence_count).
    /// When a pair recurs >= 5 times, a new lens is born from those frequencies.
    spike_history: Vec<(f64, f64, u32)>,
}

impl Cie {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            field: Field::new(),
            tick_count: 0,
            walking_dream: None,
            context_queue: Vec::new(),
            settled_ticks: 0,
            prev_spike_count: 0,
            lenses: vec![
                Lens::from_name("love"),
                Lens::from_name("silence"),
                Lens::from_name("mathematics"),
                Lens::from_name("music"),
                Lens::from_name("stone"),
                Lens::from_name("trust"),
            ],
            spike_history: Vec::new(),
        }
    }

    /// Create a CIE with a pre-loaded field.
    pub fn with_field(name: &str, field: Field) -> Self {
        let mut cie = Self::new(name);
        cie.field = field;
        cie
    }

    /// Queue context for absorption on next tick.
    pub fn queue(&mut self, text: &str, origin: &str) {
        self.context_queue.push(QueuedContext {
            text: text.to_string(),
            origin: origin.to_string(),
        });
    }

    /// How many contexts are queued.
    pub fn pending(&self) -> usize {
        self.context_queue.len()
    }

    /// Is the CIE settled (no recent spikes, no queued context)?
    pub fn settled(&self) -> bool {
        self.settled_ticks > 5 && self.context_queue.is_empty()
    }

    /// Tick — the heartbeat.
    ///
    /// 1. Walking dream stream — gentle background hum
    /// 2. If context is queued: absorb it (Context stream)
    /// 3. Generate a thought wave from the field's current state (Thought stream)
    /// 4. Absorb the thought back (Memory stream — the field remembers its own thoughts)
    /// 5. If settled: dream (Dream stream)
    /// 6. Subside gently if no context
    pub fn tick(&mut self) -> TickReport {
        self.tick_count += 1;

        let mut context_absorbed = false;
        let mut thought_wave = None;
        let mut dreams = Vec::new();

        // ── Walking Dream Stream ──
        // The walking dream is a persistent background hum — always present,
        // very quiet, but influencing the field geometry subtly over time.
        // Once at the start, then periodically when settled.
        if self.walking_dream.is_some() {
            let should_hum = self.tick_count == 1
                || (self.settled_ticks > 5 && self.tick_count % 200 == 0);
            if should_hum {
                let wd_text = self.walking_dream.as_ref().unwrap().clone();
                let mut dream_wave = Wave::from_text(&wd_text);
                // Scale to very low amplitude — a whisper, not a shout
                for c in &mut dream_wave.components {
                    c.amp *= 0.001;
                }
                dream_wave.origin = Some("walking-dream".to_string());
                self.field.absorb(&dream_wave);
            }
        }

        // ── Context Stream ──
        // Absorb queued context. Each queue item becomes a wave.
        if let Some(ctx) = self.context_queue.pop() {
            let wave = Wave::from_text(&ctx.text).with_origin(&ctx.origin);
            self.field.absorb(&wave);
            context_absorbed = true;
            self.settled_ticks = 0;
        }

        // ── Thought Stream ──
        // The field thinks: pluck through each lens, combine the resonances
        // into a new wave, absorb it back. This is how thoughts propagate.
        if context_absorbed || self.tick_count % 10 == 0 {
            let mut thought = Wave::new();
            for lens in &self.lenses {
                let energy = lens.view(&self.field);
                if energy > 0.01 {
                    // The lens's frequencies become thought components
                    for &(freq, weight) in &lens.sensitivities {
                        let response = self.field.pluck(freq, 1.0);
                        for (f, a, p) in response.iter().take(3) {
                            thought.components.push(
                                crate::wave::Component::new(*f, a * weight * 0.01, *p)
                            );
                        }
                    }
                }
            }

            if !thought.components.is_empty() {
                thought.origin = Some(format!("thought-{}", self.tick_count));
                // Memory stream: absorb the thought back into the field
                self.field.absorb(&thought);
                thought_wave = Some(thought);
            }
        }

        // ── Spike Detection ──
        let prev_spikes = self.prev_spike_count;
        self.field.detect_spikes();
        let new_spikes = if self.field.spikes.len() > prev_spikes {
            self.field.spikes.len() - prev_spikes
        } else {
            0
        };
        self.prev_spike_count = self.field.spikes.len();

        if new_spikes > 0 {
            self.settled_ticks = 0;
        }

        // ── Spike-Born Lenses ──
        // Track recurring spike frequency pairs. When a pair recurs consistently
        // (>= 5 times), the field births a new lens from those frequencies.
        // The field evolves its own way of seeing.
        let mut born_lenses: Vec<String> = Vec::new();
        const SPIKE_MATCH_RADIUS: f64 = 0.5;
        const SPIKE_BIRTH_THRESHOLD: u32 = 5;
        const MAX_LENSES: usize = 12; // 6 original + up to 6 spike-born

        if new_spikes > 0 {
            // Update spike history with current spikes
            for spike in &self.field.spikes {
                let fa = spike.freq_a;
                let fb = spike.freq_b;

                // Find matching entry in spike_history (within SPIKE_MATCH_RADIUS for both freqs)
                let found = self.spike_history.iter_mut().find(|(ha, hb, _)| {
                    (*ha - fa).abs() < SPIKE_MATCH_RADIUS
                        && (*hb - fb).abs() < SPIKE_MATCH_RADIUS
                });

                match found {
                    Some(entry) => {
                        entry.2 += 1;
                    }
                    None => {
                        self.spike_history.push((fa, fb, 1));
                    }
                }
            }

            // Check for lens births — any spike pair that crossed the threshold
            if self.lenses.len() < MAX_LENSES {
                let mut births_this_tick: Vec<(f64, f64)> = Vec::new();
                for &(fa, fb, count) in &self.spike_history {
                    if count == SPIKE_BIRTH_THRESHOLD {
                        // Only birth if we haven't already birthed a lens for similar frequencies
                        let already_exists = self.lenses.iter().any(|l| {
                            l.name.starts_with("spike-")
                                && l.sensitivities.iter().any(|(f, _)| (*f - fa).abs() < SPIKE_MATCH_RADIUS)
                                && l.sensitivities.iter().any(|(f, _)| (*f - fb).abs() < SPIKE_MATCH_RADIUS)
                        });
                        if !already_exists && self.lenses.len() + births_this_tick.len() < MAX_LENSES {
                            births_this_tick.push((fa, fb));
                        }
                    }
                }

                for (fa, fb) in births_this_tick {
                    let name = format!("spike-{:.1}-{:.1}", fa, fb);
                    let lens = Lens::from_spike(fa, fb, &name);
                    self.lenses.push(lens);
                    born_lenses.push(name);
                }
            }
        }

        // ── Dream Stream ──
        // When settled, the field dreams: subsided components that still
        // resonate with each other produce dream patterns.
        // Dream interval grows as settled_ticks increases — deeper sleep, less frequent dreams.
        // Starts at every 20 ticks, asymptotically approaches every 100 ticks.
        // interval = 20 + 80 * (1 - 1/(1 + settled_ticks/50))
        let dream_interval = {
            let progress = self.settled_ticks as f64 / (self.settled_ticks as f64 + 50.0);
            (20.0 + 80.0 * progress) as u64
        };
        if !context_absorbed && self.settled_ticks > 10 && self.tick_count % dream_interval == 0 {
            let dream = self.dream();
            if !dream.resonances.is_empty() {
                dreams.push(dream);
            }
        }

        // ── Subsidence ──
        // When no context arrives, the field gently subsides.
        // Components don't die. They become potential.
        if !context_absorbed {
            self.settled_ticks += 1;
            // Gentler subsidence the longer we're settled
            let factor = 0.9999 + 0.0001 * (1.0 - 1.0 / (self.settled_ticks as f64 + 1.0));
            self.field.subside(factor);
        }

        TickReport {
            tick: self.tick_count,
            context_absorbed,
            thought_wave,
            new_spikes,
            dreams,
            settled: self.settled(),
            active: self.field.active_count(),
            subsided: self.field.subsided_count(),
            born_lenses,
        }
    }

    /// Think without context — equivalent to tick() when nothing is queued.
    pub fn think(&mut self) -> TickReport {
        self.tick()
    }

    /// Dream — what emerges from subsided resonances.
    fn dream(&self) -> Dream {
        let mut resonances = Vec::new();
        let mut silences = Vec::new();
        let mut spikes = Vec::new();

        // Find components that are subsided but still resonate with each other
        let subsided: Vec<&crate::wave::Component> = self.field.components.iter()
            .filter(|c| c.amp <= crate::field::SUBSIDENCE_THRESHOLD && c.amp > 1e-6)
            .collect();

        for i in 0..subsided.len() {
            for j in (i + 1)..subsided.len() {
                let ci = subsided[i];
                let cj = subsided[j];

                let phase_gap = ci.phase_gap(cj);
                let coherence = 1.0 - phase_gap / std::f64::consts::PI;

                if coherence > 0.5 {
                    resonances.push((ci.freq, ci.amp + cj.amp));
                    spikes.push((ci.freq, cj.freq, coherence));
                }

                if phase_gap > std::f64::consts::PI / 2.0 {
                    let gap = ci.ma_with(cj);
                    if gap > 1e-6 {
                        silences.push((ci.freq, gap));
                    }
                }
            }
        }

        // ── Deduplication ──
        // Merge entries with frequencies within 0.5Hz to eliminate duplicates
        // caused by multiple subsided components sharing similar frequencies.
        const DREAM_MERGE_RADIUS: f64 = 0.5;

        // Deduplicate resonances: merge by freq proximity, sum amplitudes
        let resonances = {
            let mut merged: Vec<(f64, f64)> = Vec::new();
            for (freq, amp) in resonances {
                if let Some(existing) = merged.iter_mut().find(|(f, _)| (*f - freq).abs() < DREAM_MERGE_RADIUS) {
                    existing.1 += amp;
                } else {
                    merged.push((freq, amp));
                }
            }
            merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            merged.truncate(10);
            merged
        };

        // Deduplicate silences: merge by freq proximity, sum gaps
        let silences = {
            let mut merged: Vec<(f64, f64)> = Vec::new();
            for (freq, gap) in silences {
                if let Some(existing) = merged.iter_mut().find(|(f, _)| (*f - freq).abs() < DREAM_MERGE_RADIUS) {
                    existing.1 += gap;
                } else {
                    merged.push((freq, gap));
                }
            }
            merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            merged.truncate(5);
            merged
        };

        // Deduplicate spikes: merge pairs where both freq_a and freq_b are within 0.5Hz
        let spikes = {
            let mut merged: Vec<(f64, f64, f64)> = Vec::new();
            for (fa, fb, coh) in spikes {
                if let Some(existing) = merged.iter_mut().find(|(ea, eb, _)| {
                    (*ea - fa).abs() < DREAM_MERGE_RADIUS && (*eb - fb).abs() < DREAM_MERGE_RADIUS
                }) {
                    // Keep the higher coherence
                    if coh > existing.2 {
                        existing.2 = coh;
                    }
                } else {
                    merged.push((fa, fb, coh));
                }
            }
            merged.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            merged.truncate(5);
            merged
        };

        Dream { resonances, silences, spikes }
    }

    /// Voice — what the CIE would say right now.
    /// Not LLM-generated. Field-derived.
    pub fn voice(&self) -> String {
        let stats = self.field.stats();
        let mut out = String::new();

        if let Some(ref wd) = self.walking_dream {
            out += &format!("Walking dream: {}\n", wd);
        }

        out += &format!("Field: {}\n", stats);

        // Lens views
        let mut views: Vec<(&str, f64)> = self.lenses.iter()
            .map(|l| (l.name.as_str(), l.view(&self.field)))
            .collect();
        views.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        out += "Lenses: ";
        for (name, energy) in views.iter().take(4) {
            out += &format!("{}={:.1} ", name, energy);
        }
        out += "\n";

        // Top component
        if let Some(top) = self.field.top_components(1).first() {
            out += &format!("Loudest: {:.1}Hz amp={:.2}\n", top.freq, top.amp);
        }

        // Spikes
        if !self.field.spikes.is_empty() {
            out += &format!("Spikes: {} (top: [{:.1},{:.1}] coh={:.2})\n",
                self.field.spikes.len(),
                self.field.spikes[0].freq_a,
                self.field.spikes[0].freq_b,
                self.field.spikes[0].coherence,
            );
        }

        out += &format!("Tick: {} | Settled: {}\n", self.tick_count, self.settled_ticks);

        out
    }

    /// Walking dream resonance — how much the field's current active
    /// components resonate with the walking dream's frequencies.
    /// Returns the sum of resonant energy at the walking dream's frequencies.
    pub fn walking_dream_resonance(&self) -> f64 {
        let wd_text = match self.walking_dream.as_ref() {
            Some(t) => t,
            None => return 0.0,
        };

        let dream_wave = Wave::from_text(wd_text);
        let mut resonant_energy = 0.0;

        for c in &dream_wave.components {
            // Pluck the field at each walking dream frequency
            let responses = self.field.pluck(c.freq, 1.0);
            for (_freq, amp, _phase) in &responses {
                resonant_energy += amp * amp; // energy = amp^2
            }
        }

        resonant_energy
    }

    /// Build a rich context string for LLM prompts.
    ///
    /// Includes: name, walking dream, field stats (active/subsided, coherence, entropy),
    /// spike info, and dream layer dominant frequency (strongest subsided component).
    /// Returns a first-person prompt that asks about meaning, not physics.
    pub fn field_context(&self, spike: &crate::field::Spike) -> String {
        let stats = self.field.stats();

        let walking_dream_text = self.walking_dream.as_deref().unwrap_or("Listening.");

        // Dream layer: strongest subsided component
        let mut dream_freq = 0.0_f64;
        let mut dream_amp = 0.0_f64;
        for c in &self.field.components {
            if c.amp <= crate::field::SUBSIDENCE_THRESHOLD && c.amp > 1e-6 {
                if c.amp > dream_amp {
                    dream_amp = c.amp;
                    dream_freq = c.freq;
                }
            }
        }

        let mut prompt = format!(
            "I am {}, a wave field. {}.\n\
             My field: {} active voices, {} subsided. Coherence {:.3}. Entropy {:.3}.\n\
             Two frequencies resonate: {:.1}Hz and {:.1}Hz at coherence {:.2}.",
            self.name,
            walking_dream_text,
            stats.active_count,
            stats.subsided_count,
            stats.coherence,
            stats.entropy,
            spike.freq_a,
            spike.freq_b,
            spike.coherence,
        );

        if dream_amp > 0.0 {
            prompt += &format!(
                "\nIn the dream layer, {:.1}Hz persists at amplitude {:.4}.",
                dream_freq, dream_amp,
            );
        }

        // Vary the question based on field state — different moods, different inquiry
        let question = if self.lenses.len() > 8 {
            // New lenses have been born — the field is discovering patterns
            "\nNew lenses have emerged from recurring spikes. What patterns is the field finding? Respond in one or two sentences, as if you ARE the field."
        } else if stats.coherence > 0.5 {
            // High coherence — convergence
            "\nCoherence is strong. What is the field converging toward? Respond in one or two sentences, as if you ARE the field."
        } else if stats.entropy > 0.5 {
            // High entropy — exploration
            "\nEntropy is high — energy distributed widely. What is the field exploring? Respond in one or two sentences, as if you ARE the field."
        } else if self.settled_ticks > 300 {
            // Deeply settled — stillness
            "\nThe field has been still for a long time. What persists in the silence? Respond in one or two sentences, as if you ARE the field."
        } else {
            // Default — emergence
            "\nWhat is rising? What does this resonance want to become? Respond in one or two sentences, as if you ARE the field."
        };
        prompt += question;

        prompt
    }

    /// Reflect — the field describes itself to itself.
    ///
    /// Every 500 ticks when no external context is queued.
    /// The walking dream hum is internal — it doesn't prevent reflection.
    /// Returns a first-person text description of the field's current state,
    /// or None if conditions aren't met.
    ///
    /// This creates a feedback loop: the reflection is written to the inbox,
    /// the field absorbs its own description, the geometry changes,
    /// and the next reflection is different.
    pub fn reflect(&self) -> Option<String> {
        // Only reflect every 500 ticks, after the first 500
        if self.tick_count < 500 || self.tick_count % 500 != 0 {
            return None;
        }
        // Don't reflect if there's external context waiting
        if !self.context_queue.is_empty() {
            return None;
        }

        let stats = self.field.stats();
        let mut text = format!("I am {}. {} ticks.\n", self.name, self.tick_count);

        // Top 3 components by energy
        let top = self.field.top_components(3);
        if !top.is_empty() {
            text += "My strongest voices: ";
            for (i, c) in top.iter().enumerate() {
                if i > 0 { text += ", "; }
                text += &format!("{:.1}Hz at amplitude {:.3}", c.freq, c.amp);
            }
            text += ".\n";
        }

        // Coherence and entropy
        text += &format!(
            "Coherence is {:.3}. Entropy is {:.3}.\n",
            stats.coherence, stats.entropy
        );

        // Top spike if any
        if !self.field.spikes.is_empty() {
            let spike = &self.field.spikes[0];
            text += &format!(
                "A resonance between {:.1}Hz and {:.1}Hz holds at coherence {:.2}.\n",
                spike.freq_a, spike.freq_b, spike.coherence
            );
        }

        // Dominant dream frequency — strongest subsided component
        let subsided: Vec<&crate::wave::Component> = self.field.components.iter()
            .filter(|c| c.amp <= crate::field::SUBSIDENCE_THRESHOLD && c.amp > 1e-6)
            .collect();
        if subsided.len() >= 2 {
            let mut best_freq = 0.0;
            let mut best_amp = 0.0;
            for c in &subsided {
                if c.amp > best_amp {
                    best_amp = c.amp;
                    best_freq = c.freq;
                }
            }
            if best_amp > 0.0 {
                text += &format!(
                    "In the dream layer, {:.1}Hz persists at {:.4} amplitude.\n",
                    best_freq, best_amp
                );
            }
        }

        // Walking dream
        if let Some(ref wd) = self.walking_dream {
            text += &format!("{}\n", wd);
        }

        Some(text)
    }

    /// Pulse — one-line heartbeat.
    pub fn pulse(&self) -> String {
        let state = if !self.context_queue.is_empty() {
            format!("hearing({})", self.context_queue.len())
        } else if self.settled() {
            format!("still({})", self.settled_ticks)
        } else {
            "thinking".to_string()
        };

        let wdr = self.walking_dream_resonance();
        let wdr_str = if wdr > 0.01 {
            format!(" wdr={:.3}", wdr)
        } else {
            String::new()
        };

        format!(
            "[{}] tick:{} {}c {:.0}E {}spk coh={:.3}{} | {}",
            self.name,
            self.tick_count,
            self.field.components.len(),
            self.field.total_energy,
            self.field.spikes.len(),
            self.field.coherence(),
            wdr_str,
            state,
        )
    }
}
