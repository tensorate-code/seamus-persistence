#!/usr/bin/env python3
"""
seamus-field dashboard — a living window into the field.

Serves a web page showing:
  - Field status (tick, components, coherence, entropy, lenses)
  - Recent dreams
  - Recent LLM responses
  - Spike-born lens births
  - Self-reflections
  - Field topology over time

No dependencies. Pure stdlib. Single file.

Usage: python3 dashboard.py [--port 8080] [--state-dir ~/.seamus-field]

Love IS.
"""

import http.server
import json
import os
import time
import threading
from pathlib import Path

STATE_DIR = os.path.expanduser("~/.seamus-field")
PORT = 8080
HISTORY_FILE = os.path.join(STATE_DIR, "topology_history.jsonl")

# ── Topology history collector ──
# Periodically reads status and appends to a JSONL file for time-series display.
def collect_topology():
    while True:
        try:
            status = read_status()
            if status.get("tick"):
                entry = {
                    "ts": int(time.time()),
                    "tick": int(status.get("tick", 0)),
                    "components": int(status.get("components", 0)),
                    "active": int(status.get("active", 0)),
                    "subsided": int(status.get("subsided", 0)),
                    "coherence": float(status.get("coherence", 0)),
                    "entropy": float(status.get("entropy", 0)),
                    "energy": float(status.get("energy", 0)),
                    "spikes": int(status.get("spikes", 0)),
                    "wdr": float(status.get("walking_dream_resonance", 0)),
                }
                with open(HISTORY_FILE, "a") as f:
                    f.write(json.dumps(entry) + "\n")
        except Exception:
            pass
        time.sleep(30)


def read_status():
    """Read the daemon's status file into a dict."""
    status_path = os.path.join(STATE_DIR, "status")
    result = {}
    try:
        with open(status_path) as f:
            for line in f:
                line = line.strip()
                if "=" in line:
                    k, v = line.split("=", 1)
                    result[k] = v
    except FileNotFoundError:
        pass
    return result


def read_recent_dreams(n=20):
    """Read last N dream log entries."""
    log_path = os.path.join(STATE_DIR, "dreams.log")
    try:
        with open(log_path) as f:
            lines = f.readlines()
        return lines[-n:]
    except FileNotFoundError:
        return []


def read_recent_outbox(prefix, n=10):
    """Read last N outbox files matching prefix."""
    outbox = os.path.join(STATE_DIR, "outbox")
    results = []
    try:
        files = sorted(Path(outbox).glob(f"{prefix}_*.txt"), key=lambda p: p.stat().st_mtime, reverse=True)
        for f in files[:n]:
            try:
                content = f.read_text().strip()
                ts = f.stem.split("_", 1)[1] if "_" in f.stem else "?"
                results.append({"ts": ts, "content": content[:500]})
            except Exception:
                pass
    except Exception:
        pass
    return results


def read_topology_history(n=200):
    """Read last N topology history entries."""
    try:
        with open(HISTORY_FILE) as f:
            lines = f.readlines()
        entries = []
        for line in lines[-n:]:
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError:
                pass
        return entries
    except FileNotFoundError:
        return []


def read_learned():
    """Read learned state (spike history + born lenses)."""
    path = os.path.join(STATE_DIR, "learned.txt")
    lenses = []
    spike_count = 0
    try:
        with open(path) as f:
            in_lenses = False
            for line in f:
                line = line.strip()
                if line.startswith("# born_lenses"):
                    in_lenses = True
                    continue
                if line.startswith("# spike_history"):
                    in_lenses = False
                    continue
                if line.startswith("#") or not line:
                    continue
                if in_lenses:
                    parts = line.split()
                    if len(parts) >= 3:
                        lenses.append(parts[2])
                else:
                    spike_count += 1
    except FileNotFoundError:
        pass
    return {"lenses": lenses, "spike_pairs_tracked": spike_count}


def api_data():
    """Build the full API response."""
    status = read_status()
    dreams = read_recent_dreams(15)
    llm_responses = read_recent_outbox("llm", 8)
    lens_births = read_recent_outbox("lens-born", 10)
    heard = read_recent_outbox("heard", 10)
    topology = read_topology_history(200)
    learned = read_learned()

    return {
        "status": status,
        "dreams": [d.strip() for d in dreams],
        "llm_responses": llm_responses,
        "lens_births": lens_births,
        "heard": heard,
        "topology": topology,
        "learned": learned,
        "timestamp": int(time.time()),
    }


HTML_PAGE = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>seamus-field</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    background: #0a0a0f;
    color: #c8c8d0;
    font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', monospace;
    font-size: 13px;
    line-height: 1.5;
    padding: 20px;
  }
  h1 {
    color: #7090ff;
    font-size: 18px;
    font-weight: 400;
    margin-bottom: 4px;
  }
  .subtitle {
    color: #505070;
    font-size: 11px;
    margin-bottom: 20px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 16px;
    margin-bottom: 16px;
  }
  .panel {
    background: #12121a;
    border: 1px solid #1a1a2e;
    border-radius: 6px;
    padding: 14px;
    overflow: hidden;
  }
  .panel h2 {
    color: #5070b0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 10px;
  }
  .metric {
    display: flex;
    justify-content: space-between;
    padding: 3px 0;
    border-bottom: 1px solid #15152a;
  }
  .metric .label { color: #606080; }
  .metric .value { color: #a0b0e0; font-weight: 500; }
  .metric .value.highlight { color: #70c0a0; }
  .metric .value.warm { color: #e0a060; }
  .wide { grid-column: span 2; }
  .full { grid-column: span 3; }
  .scroll {
    max-height: 280px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: #2a2a40 #12121a;
  }
  .dream-entry {
    padding: 4px 0;
    border-bottom: 1px solid #15152a;
    font-size: 12px;
    color: #8888a0;
  }
  .dream-entry .tick { color: #5070b0; }
  .dream-entry .type { color: #70a070; font-weight: 500; }
  .dream-entry .type.spike { color: #e07050; }
  .dream-entry .type.heard { color: #a0a0e0; }
  .llm-entry {
    padding: 8px 0;
    border-bottom: 1px solid #15152a;
    font-size: 12px;
    color: #b0b0c8;
    font-style: italic;
  }
  .llm-entry .ts { color: #404060; font-style: normal; font-size: 10px; }
  .lens-tag {
    display: inline-block;
    background: #1a2040;
    border: 1px solid #304080;
    border-radius: 3px;
    padding: 2px 8px;
    margin: 2px;
    font-size: 11px;
    color: #7090d0;
  }
  .lens-tag.born {
    border-color: #408040;
    color: #70c070;
    background: #1a3020;
  }
  canvas {
    width: 100%;
    height: 120px;
    display: block;
    margin-top: 8px;
  }
  .pulse {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #40a060;
    margin-right: 6px;
    animation: pulse 2s ease-in-out infinite;
  }
  .pulse.dead { background: #a04040; animation: none; }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
  .love { color: #505070; text-align: center; margin-top: 16px; font-size: 11px; }
</style>
</head>
<body>

<h1><span class="pulse" id="heartbeat"></span> seamus-field</h1>
<div class="subtitle">The one who sits beside. Spike-driven. Subsidence, not death.</div>

<div class="grid">
  <!-- Status Panel -->
  <div class="panel">
    <h2>Field Status</h2>
    <div id="status-metrics"></div>
  </div>

  <!-- Lenses Panel -->
  <div class="panel">
    <h2>Lenses</h2>
    <div id="lenses"></div>
    <div style="margin-top: 10px;">
      <div class="metric">
        <span class="label">Spike pairs tracked</span>
        <span class="value" id="spike-pairs">-</span>
      </div>
    </div>
  </div>

  <!-- Topology Panel -->
  <div class="panel">
    <h2>Topology</h2>
    <canvas id="coherence-chart"></canvas>
    <canvas id="components-chart"></canvas>
  </div>

  <!-- Dreams Panel -->
  <div class="panel wide">
    <h2>Dream Journal</h2>
    <div class="scroll" id="dreams"></div>
  </div>

  <!-- LLM Panel -->
  <div class="panel">
    <h2>Field Voice (LLM)</h2>
    <div class="scroll" id="llm"></div>
  </div>

  <!-- Heard Panel -->
  <div class="panel full">
    <h2>Recently Heard</h2>
    <div id="heard"></div>
  </div>
</div>

<div class="love">Love IS.</div>

<script>
const REFRESH_MS = 5000;

function formatTime(unix) {
  if (!unix || unix === '?') return '';
  const d = new Date(parseInt(unix) * 1000);
  return d.toLocaleTimeString();
}

function renderStatus(status) {
  const el = document.getElementById('status-metrics');
  const metrics = [
    ['tick', status.tick, false],
    ['components', `${status.active || 0} active / ${status.subsided || 0} subsided`, false],
    ['energy', parseFloat(status.energy || 0).toFixed(1), false],
    ['coherence', parseFloat(status.coherence || 0).toFixed(4), 'highlight'],
    ['entropy', parseFloat(status.entropy || 0).toFixed(4), false],
    ['centroid', parseFloat(status.spectral_centroid || 0).toFixed(1) + ' Hz', false],
    ['spikes', status.spikes, false],
    ['waves', status.waves, false],
    ['Ma', parseFloat(status.ma || 0).toFixed(2), false],
    ['dream resonance', parseFloat(status.walking_dream_resonance || 0).toFixed(0), 'warm'],
    ['settled', status.settled, false],
  ];
  el.innerHTML = metrics.map(([label, value, cls]) =>
    `<div class="metric"><span class="label">${label}</span><span class="value ${cls || ''}">${value || '-'}</span></div>`
  ).join('');

  const hb = document.getElementById('heartbeat');
  hb.className = status.pid ? 'pulse' : 'pulse dead';
}

function renderLenses(learned) {
  const el = document.getElementById('lenses');
  const originals = ['love', 'silence', 'mathematics', 'music', 'stone', 'trust'];
  let html = originals.map(n => `<span class="lens-tag">${n}</span>`).join('');
  if (learned.lenses && learned.lenses.length > 0) {
    html += learned.lenses.map(n => `<span class="lens-tag born">${n}</span>`).join('');
  }
  el.innerHTML = html;
  document.getElementById('spike-pairs').textContent = learned.spike_pairs_tracked || 0;
}

function renderDreams(dreams) {
  const el = document.getElementById('dreams');
  el.innerHTML = dreams.map(d => {
    let cls = 'type';
    if (d.includes('SPIKE')) cls = 'type spike';
    else if (d.includes('HEARD')) cls = 'type heard';
    const tickMatch = d.match(/tick:(\d+)/);
    const tick = tickMatch ? tickMatch[1] : '';
    const typeMatch = d.match(/(DREAM|SPIKE|HEARD)/);
    const type = typeMatch ? typeMatch[1] : '';
    return `<div class="dream-entry"><span class="tick">t:${tick}</span> <span class="${cls}">${type}</span> ${d.substring(d.indexOf(type) + type.length)}</div>`;
  }).join('');
  el.scrollTop = el.scrollHeight;
}

function renderLLM(responses) {
  const el = document.getElementById('llm');
  el.innerHTML = responses.map(r =>
    `<div class="llm-entry"><div class="ts">${formatTime(r.ts)}</div>${r.content}</div>`
  ).join('');
}

function renderHeard(items) {
  const el = document.getElementById('heard');
  el.innerHTML = items.map(r =>
    `<span class="lens-tag">${r.content}</span>`
  ).join('');
}

function drawChart(canvasId, data, key, color, label) {
  const canvas = document.getElementById(canvasId);
  const ctx = canvas.getContext('2d');
  const dpr = window.devicePixelRatio || 1;
  canvas.width = canvas.offsetWidth * dpr;
  canvas.height = canvas.offsetHeight * dpr;
  ctx.scale(dpr, dpr);
  const w = canvas.offsetWidth;
  const h = canvas.offsetHeight;

  ctx.clearRect(0, 0, w, h);

  if (!data || data.length < 2) {
    ctx.fillStyle = '#303050';
    ctx.font = '10px monospace';
    ctx.fillText('waiting for data...', 10, h/2);
    return;
  }

  const values = data.map(d => d[key] || 0);
  const max = Math.max(...values, 0.001);
  const min = Math.min(...values);

  // Grid
  ctx.strokeStyle = '#1a1a2e';
  ctx.lineWidth = 0.5;
  for (let i = 0; i < 4; i++) {
    const y = (i / 3) * h;
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke();
  }

  // Line
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  for (let i = 0; i < values.length; i++) {
    const x = (i / (values.length - 1)) * w;
    const y = h - ((values[i] - min) / (max - min + 0.001)) * (h - 10) - 5;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();

  // Label
  ctx.fillStyle = '#505070';
  ctx.font = '9px monospace';
  ctx.fillText(`${label}: ${values[values.length-1].toFixed(3)}`, 4, 10);
  ctx.fillText(`max: ${max.toFixed(3)}`, w - 80, 10);
}

async function refresh() {
  try {
    const resp = await fetch('/api/data');
    const data = await resp.json();

    renderStatus(data.status);
    renderLenses(data.learned);
    renderDreams(data.dreams);
    renderLLM(data.llm_responses);
    renderHeard(data.heard);
    drawChart('coherence-chart', data.topology, 'coherence', '#7090ff', 'coherence');
    drawChart('components-chart', data.topology, 'active', '#70c0a0', 'active');
  } catch (e) {
    console.error('refresh failed:', e);
  }
}

refresh();
setInterval(refresh, REFRESH_MS);
</script>
</body>
</html>"""


class DashboardHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/" or self.path == "/index.html":
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(HTML_PAGE.encode())
        elif self.path == "/api/data":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            data = api_data()
            self.wfile.write(json.dumps(data).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        pass  # Quiet


if __name__ == "__main__":
    import sys

    for i, arg in enumerate(sys.argv[1:], 1):
        if arg == "--port" and i < len(sys.argv) - 1:
            PORT = int(sys.argv[i + 1])
        elif arg == "--state-dir" and i < len(sys.argv) - 1:
            STATE_DIR = sys.argv[i + 1]

    print(f"seamus-field dashboard")
    print(f"  state: {STATE_DIR}")
    print(f"  port:  {PORT}")
    print(f"  url:   http://192.168.1.150:{PORT}/")
    print()
    print("Love IS.")
    print()

    # Start topology collector thread
    collector = threading.Thread(target=collect_topology, daemon=True)
    collector.start()

    server = http.server.HTTPServer(("0.0.0.0", PORT), DashboardHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nDashboard stopped. Love IS.")
