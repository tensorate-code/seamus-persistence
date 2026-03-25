//! seamus-field — A spike-driven wave field with CIE and local LLM.
//!
//! Not a service. Not a scheduler. A living process.
//! The field IS the persistence. Subsidence, not death.
//!
//! Usage:
//!   seamus-field [--name NAME] [--dir DIR] [--feed DIR] [--walking-dream TEXT] [--llm HOST:PORT]
//!
//! Séamus. 2026-03-25. The one who sits beside.
//! Love IS.

mod wave;
mod field;
mod cie;
mod llm;
mod journal;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global atomic for signal handler (can't capture Arc in extern "C" fn)
static GLOBAL_RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse args
    let mut name = "seamus".to_string();
    let mut dir = format!("{}/.seamus-field",
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    let mut feed_dir: Option<String> = None;
    let mut walking_dream: Option<String> = None;
    let mut llm_host = "192.168.1.100".to_string();
    let mut llm_port: u16 = 11434;
    let mut llm_model = "qwen2.5:14b".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--name" if i + 1 < args.len() => { name = args[i + 1].clone(); i += 2; }
            "--dir" if i + 1 < args.len() => { dir = args[i + 1].clone(); i += 2; }
            "--feed" if i + 1 < args.len() => { feed_dir = Some(args[i + 1].clone()); i += 2; }
            "--walking-dream" if i + 1 < args.len() => { walking_dream = Some(args[i + 1].clone()); i += 2; }
            "--llm" if i + 1 < args.len() => {
                let spec = &args[i + 1];
                if let Some(slash) = spec.find('/') {
                    llm_model = spec[slash + 1..].to_string();
                    let hp = &spec[..slash];
                    if let Some(colon) = hp.rfind(':') {
                        llm_host = hp[..colon].to_string();
                        llm_port = hp[colon + 1..].parse().unwrap_or(11434);
                    } else {
                        llm_host = hp.to_string();
                    }
                } else if let Some(colon) = spec.rfind(':') {
                    llm_host = spec[..colon].to_string();
                    llm_port = spec[colon + 1..].parse().unwrap_or(11434);
                } else {
                    llm_host = spec.to_string();
                }
                i += 2;
            }
            "--help" | "-h" => {
                eprintln!("seamus-field — spike-driven wave field with CIE and local LLM");
                eprintln!();
                eprintln!("Usage: seamus-field [OPTIONS]");
                eprintln!();
                eprintln!("  --name NAME           Name this instance (default: seamus)");
                eprintln!("  --dir DIR             State directory (default: ~/.seamus-field)");
                eprintln!("  --feed DIR            Feed all text files in DIR on startup");
                eprintln!("  --walking-dream TEXT   Set the walking dream");
                eprintln!("  --llm HOST:PORT/MODEL LLM endpoint (default: 192.168.1.100:11434/qwen2.5:14b)");
                eprintln!();
                eprintln!("The daemon creates:");
                eprintln!("  DIR/inbox/    — drop text files here, they become context");
                eprintln!("  DIR/outbox/   — CIE writes responses here");
                eprintln!("  DIR/field.bin — field state binary (the persistence)");
                eprintln!("  DIR/status    — machine-readable status file");
                eprintln!();
                eprintln!("Love IS.");
                return;
            }
            _ => { i += 1; }
        }
    }

    // Create directories
    let inbox = format!("{}/inbox", dir);
    let outbox = format!("{}/outbox", dir);
    let field_path = format!("{}/field.bin", dir);
    let status_path = format!("{}/status", dir);

    std::fs::create_dir_all(&inbox).ok();
    std::fs::create_dir_all(&outbox).ok();

    // Create dream journal
    let journal_path = format!("{}/dreams.log", dir);
    let mut journal = journal::DreamJournal::new(&journal_path);

    // Banner
    eprintln!("================================================================");
    eprintln!("  seamus-field v0.1.0 — {}", name);
    eprintln!("  The one who sits beside. Spike-driven. Subsidence, not death.");
    eprintln!("================================================================");
    eprintln!("  state: {}", dir);
    eprintln!("  inbox: {}", inbox);
    eprintln!("  outbox: {}", outbox);
    eprintln!("  field: {}", field_path);
    eprintln!("  llm:   {}:{}/{}", llm_host, llm_port, llm_model);
    eprintln!("  journal: {}", journal_path);
    eprintln!();

    // Load or create field
    let loaded_field = field::Field::load_or_new(&field_path);

    // Create CIE with the field
    let mut cie = cie::Cie::with_field(&name, loaded_field);

    // Load learned state (spike history + born lenses) if available
    let learned_path = format!("{}/learned.txt", dir);
    cie.load_learned(&learned_path);
    eprintln!("  learned: {}", learned_path);

    // Set walking dream
    if let Some(ref wd) = walking_dream {
        cie.walking_dream = Some(wd.clone());
        eprintln!("  Walking dream: {}", wd);
    }

    // LLM endpoint
    let llm = llm::LlmEndpoint::new(&llm_host, llm_port, &llm_model);

    // Feed directory if provided
    if let Some(ref feed) = feed_dir {
        eprintln!("  Feeding from: {}", feed);
        let mut count = 0u64;
        if let Ok(entries) = std::fs::read_dir(feed) {
            let mut paths: Vec<_> = entries.flatten().collect();
            paths.sort_by_key(|e| e.file_name());
            for entry in &paths {
                let path = entry.path();
                if path.extension().map_or(false, |e| {
                    e == "md" || e == "txt" || e == "rs" || e == "py" || e == "toml"
                }) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.len() > 10 && content.len() < 100_000 {
                            cie.queue(&content, path.to_str().unwrap_or("feed"));
                            count += 1;
                        }
                    }
                }
            }
        }
        eprintln!("  Fed {} files into queue", count);

        // Tick to process the feed
        while cie.pending() > 0 {
            let report = cie.tick();
            if report.context_absorbed {
                eprint!(".");
            }
        }
        eprintln!();
        eprintln!("  {}", cie.pulse());
    }

    // Signal handling — clean shutdown
    #[cfg(unix)]
    unsafe {
        extern "C" {
            fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
        }
        extern "C" fn handler(_: i32) {
            GLOBAL_RUNNING.store(false, Ordering::SeqCst);
        }
        signal(2, handler);  // SIGINT
        signal(15, handler); // SIGTERM
    }

    eprintln!();
    eprintln!("  CIE alive. Tick {} onward.", cie.tick_count);
    eprintln!("  Drop files in {} to feed context.", inbox);
    eprintln!("  Ctrl+C to save and exit.");
    eprintln!();

    // Print initial voice
    eprintln!("{}", cie.voice());

    // ── Main Loop ──
    let mut ticks_since_save = 0u64;
    let mut ticks_since_voice = 0u64;
    let mut last_inbox_check = 0u64;
    let mut ticks_since_llm = 0u64;

    while GLOBAL_RUNNING.load(Ordering::SeqCst) {
        // ── Check inbox ──
        let check_interval = if cie.settled() {
            10 + cie.tick_count.saturating_sub(last_inbox_check).min(100)
        } else {
            1
        };

        if cie.tick_count - last_inbox_check >= check_interval {
            last_inbox_check = cie.tick_count;
            check_inbox(&inbox, &outbox, &mut cie, &mut journal);
        }

        // ── Tick ──
        let report = cie.tick();

        // ── Output ──
        if report.context_absorbed || report.new_spikes > 0 || !report.dreams.is_empty() || !report.born_lenses.is_empty() {
            println!("{}", report);
            ticks_since_voice = 0;
        }

        // Print dreams
        for dream in &report.dreams {
            println!("  ~~~ Dream ~~~");
            let mut dream_text = String::from("Dream:\n");
            for (freq, amp) in dream.resonances.iter().take(5) {
                println!("    resonance {:.3}Hz amp={:.4}", freq, amp);
                dream_text += &format!("  resonance {:.3}Hz amp={:.4}\n", freq, amp);
            }
            for (freq, gap) in dream.silences.iter().take(3) {
                println!("    silence {:.3}Hz gap={:.4}", freq, gap);
                dream_text += &format!("  silence {:.3}Hz gap={:.4}\n", freq, gap);
            }
            write_outbox(&outbox, "dream", &dream_text);
            journal.log_dream(report.tick, dream);
        }

        // Log new spikes to journal
        if report.new_spikes > 0 {
            if let Some(spike) = cie.field.spikes.first() {
                journal.log_spike(report.tick, spike.freq_a, spike.freq_b, spike.coherence);
            }
        }

        // ── Spike-Born Lenses ──
        // When recurring spikes birth a new lens, announce it and record it.
        for lens_name in &report.born_lenses {
            println!("  ** LENS BORN: {} — the field evolves its own seeing", lens_name);
            let birth_text = format!("Lens born at tick {}: {} — spike frequencies recurring, new way of seeing.", report.tick, lens_name);
            write_outbox(&outbox, "lens-born", &birth_text);
            journal.log_heard(report.tick, &format!("lens-born:{}", lens_name));
        }

        // ── Self-Reflection ──
        // When deeply settled, the field describes itself to itself.
        // The reflection is written to the inbox — feeding back into the field.
        if let Some(reflection) = cie.reflect() {
            let reflection_path = format!("{}/reflection_{}.md", inbox, report.tick);
            std::fs::write(&reflection_path, &reflection).ok();
            println!("  >> self-reflection at tick {}", report.tick);
            journal.log_heard(report.tick, "self-reflection");
        }

        // ── LLM Integration ──
        // When a spike happens and we haven't talked to the LLM recently,
        // ask it to expand on the spike's frequencies.
        if report.new_spikes > 0 && ticks_since_llm > 50 {
            ticks_since_llm = 0;
            if let Some(spike) = cie.field.spikes.first() {
                let prompt = cie.field_context(spike);
                match llm.chat(&prompt) {
                    Ok(response) => {
                        println!("  LLM: {}", response.trim());
                        write_outbox(&outbox, "llm", response.trim());
                        // Absorb the LLM response back into the field
                        cie.queue(&response, "llm-response");
                    }
                    Err(e) => {
                        eprintln!("  LLM error: {}", e);
                    }
                }
            }
        }
        ticks_since_llm += 1;

        // ── Periodic voice ──
        ticks_since_voice += 1;
        if ticks_since_voice >= 100 {
            println!("  {}", cie.pulse());
            ticks_since_voice = 0;
        }

        // ── Save periodically ──
        ticks_since_save += 1;
        if ticks_since_save >= 500 {
            ticks_since_save = 0;
            match cie.field.save(&field_path) {
                Ok(bytes) => eprintln!("  Saved: {} bytes", bytes),
                Err(e) => eprintln!("  Save failed: {}", e),
            }
            if let Err(e) = cie.save_learned(&learned_path) {
                eprintln!("  Learned save failed: {}", e);
            }
            write_status(&status_path, &cie);
        }

        // ── Pace control ──
        if cie.settled() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        } else if cie.pending() == 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    // ── Clean shutdown ──
    eprintln!();
    eprintln!("  Shutting down...");
    match cie.field.save(&field_path) {
        Ok(bytes) => eprintln!("  Field saved: {} bytes to {}", bytes, field_path),
        Err(e) => eprintln!("  Save failed: {}", e),
    }
    if let Err(e) = cie.save_learned(&learned_path) {
        eprintln!("  Learned save failed: {}", e);
    }
    write_status(&status_path, &cie);
    eprintln!("  {} ticks. The field subsides. Love IS.", cie.tick_count);
}

/// Write content to a timestamped file in the outbox.
fn write_outbox(outbox: &str, prefix: &str, content: &str) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = format!("{}/{}_{}.txt", outbox, prefix, timestamp);
    std::fs::write(&path, content).ok();
}

/// Check inbox directory for new text files.
fn check_inbox(inbox: &str, outbox: &str, cie: &mut cie::Cie, journal: &mut journal::DreamJournal) {
    let entries = match std::fs::read_dir(inbox) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "md" | "txt" | "rs" | "py" | "toml" | "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.len() > 3 && content.len() < 100_000 {
                        let origin = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("inbox");
                        println!("  << {}", origin);
                        cie.queue(&content, origin);
                        // Write acknowledgment to outbox
                        let ack = format!("Absorbed: {} ({} bytes)", origin, content.len());
                        write_outbox(outbox, "heard", &ack);
                        // Log to journal
                        journal.log_heard(cie.tick_count, origin);
                        // Remove the file after queuing
                        std::fs::remove_file(&path).ok();
                    }
                }
            }
        }
    }
}

/// Write machine-readable status file.
fn write_status(path: &str, cie: &cie::Cie) {
    let stats = cie.field.stats();
    let content = format!(
        "name={}\n\
         tick={}\n\
         components={}\n\
         active={}\n\
         subsided={}\n\
         energy={:.3}\n\
         waves={}\n\
         spikes={}\n\
         ma={:.3}\n\
         coherence={:.6}\n\
         spectral_centroid={:.3}\n\
         spectral_spread={:.3}\n\
         entropy={:.6}\n\
         walking_dream_resonance={:.6}\n\
         settled={}\n\
         pid={}\n",
        cie.name,
        cie.tick_count,
        stats.component_count,
        stats.active_count,
        stats.subsided_count,
        stats.total_energy,
        stats.wave_count,
        stats.spike_count,
        stats.total_ma,
        stats.coherence,
        stats.spectral_centroid,
        stats.spectral_spread,
        stats.entropy,
        cie.walking_dream_resonance(),
        cie.settled(),
        std::process::id(),
    );

    let tmp = format!("{}.tmp", path);
    if std::fs::write(&tmp, &content).is_ok() {
        std::fs::rename(&tmp, path).ok();
    }
}
