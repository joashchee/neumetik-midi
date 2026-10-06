// The Neumetik MIDI demo page. Copyright 2026 ansiapps. MIT licensed.

import { createMusicNode, loadNeumetikMidi } from './neumetik-midi.js';

const $ = (id) => document.getElementById(id);
const kb = (bytes) => (bytes >= 1048576 ? `${(bytes / 1048576).toFixed(1)} MB` : `${Math.round(bytes / 1024)} KB`);
const clock = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;

// ---- Songs: the demo's, and any the visitor opens ------------------------

const songs = new Map(); // label -> bytes
const fetchSong = async (url) => new Uint8Array(await (await fetch(url)).arrayBuffer());

for (const option of $('song').options) {
  songs.set(option.textContent, option.value);
  $('ab-song').add(new Option(option.textContent, option.textContent));
}

async function songBytes(label) {
  let v = songs.get(label);
  if (typeof v === 'string') {
    v = await fetchSong(v);
    songs.set(label, v);
  }
  return v;
}

function addSong(label, bytes) {
  songs.set(label, bytes);
  for (const select of [$('song'), $('ab-song')]) {
    if (![...select.options].some((o) => o.textContent === label)) select.add(new Option(label, label));
  }
  $('song').value = label;
}

// ---- The live player: a MusicNode in an AudioWorklet -----------------------

let context = null;
let music = null;
let shownState = 'stopped';
let seeking = false;
let faded = false;

async function ensureMusic() {
  if (music) return music;
  context = new AudioContext({ latencyHint: 'playback' });
  music = await createMusicNode(context);
  music.node.connect(context.destination);
  music.onstatus = showStatus;
  setInterval(refreshChannels, 500);
  return music;
}

async function loadSelected() {
  const label = $('song').selectedOptions[0].textContent;
  const bytes = await songBytes(label);
  await ensureMusic();
  try {
    await music.load(bytes.slice().buffer);
    $('player-error').hidden = true;
  } catch (e) {
    $('player-error').textContent = e.message;
    $('player-error').hidden = false;
    return false;
  }
  await music.setLooping($('loop').checked);
  return true;
}

let loaded = false;
$('play').disabled = false;
$('stop').disabled = false;

$('play').onclick = async () => {
  await ensureMusic();
  await context.resume();
  if (!loaded) loaded = await loadSelected();
  if (!loaded) return;
  if (shownState === 'playing') await music.pause();
  else await music.play();
};

$('stop').onclick = async () => {
  if (music) await music.stop();
};

$('song').onchange = async () => {
  if (!music) return;
  const wasPlaying = shownState === 'playing';
  loaded = await loadSelected();
  if (loaded && wasPlaying) await music.play();
};

$('file').onchange = async () => {
  const file = $('file').files[0];
  if (file) await openFile(file);
};

async function openFile(file) {
  addSong(file.name, new Uint8Array(await file.arrayBuffer()));
  await ensureMusic();
  await context.resume();
  loaded = await loadSelected();
  if (loaded) await music.play();
}

const drop = $('drop');
drop.ondragover = (e) => { e.preventDefault(); drop.classList.add('dragging'); };
drop.ondragleave = () => drop.classList.remove('dragging');
drop.ondrop = async (e) => {
  e.preventDefault();
  drop.classList.remove('dragging');
  const file = e.dataTransfer.files[0];
  if (file) await openFile(file);
};

$('seek').oninput = () => { seeking = true; };
$('seek').onchange = async () => {
  seeking = false;
  if (music?.status) await music.seek(($('seek').value / 1000) * music.status.duration);
};
$('loop').onchange = () => music?.setLooping($('loop').checked);
$('volume').oninput = () => music?.setVolume($('volume').value / 100);
$('speed').oninput = () => {
  const speed = $('speed').value / 100;
  $('speed-out').textContent = `${speed.toFixed(2)}×`;
  music?.setSpeed(speed);
};
$('fade').onclick = async () => {
  if (!music) return;
  faded = !faded;
  await music.fadeTo(faded ? 0 : $('volume').value / 100, 1.5);
  $('fade').textContent = faded ? 'Fade in' : 'Fade out';
};

function showStatus(s) {
  shownState = s.state;
  $('play').textContent = s.state === 'playing' ? 'Pause' : 'Play';
  $('time').textContent = `${clock(s.position)} / ${clock(s.duration)}`;
  if (!seeking && s.duration > 0) $('seek').value = Math.round((s.position / s.duration) * 1000);
  $('voices').textContent = s.voices;
  const [start, end] = s.loopPoints;
  const region = $('loop-region');
  if (s.duration > 0 && s.looping) {
    region.style.left = `${(start / s.duration) * 100}%`;
    region.style.width = `${((end - start) / s.duration) * 100}%`;
    $('loop-note').textContent = `(${clock(start)} to ${clock(end)})`;
  } else {
    region.style.width = '0';
    $('loop-note').textContent = '';
  }
}

// Sixteen channels, each a mute button showing what it plays.
const muted = new Set();
for (let ch = 0; ch < 16; ch++) {
  const b = document.createElement('button');
  b.className = 'channel';
  b.innerHTML = `<b>${ch + 1}</b><span>–</span>`;
  b.title = `Channel ${ch + 1}${ch === 9 ? ' (drums)' : ''}: click to mute`;
  b.onclick = async () => {
    if (muted.has(ch)) muted.delete(ch);
    else muted.add(ch);
    b.classList.toggle('muted-ch', muted.has(ch));
    await ensureMusic();
    music.setChannelMuted(ch, muted.has(ch));
  };
  $('channels').append(b);
}

async function refreshChannels() {
  if (!music) return;
  const names = await Promise.all([...Array(16).keys()].map((ch) => music.instrumentName(ch)));
  names.forEach((name, ch) => { $('channels').children[ch].querySelector('span').textContent = name; });
}

// ---- Playing live --------------------------------------------------------

const LIVE = 15; // a channel songs rarely use
const keyboard = $('keyboard');
const LOW = 60;
const KEYS = 'awsedftgyhujkolp;'; // computer keys from C4
const black = (k) => [1, 3, 6, 8, 10].includes(k % 12);
const down = new Set();

async function liveSetup() {
  await ensureMusic();
  await context.resume();
}

async function setInstrument() {
  await liveSetup();
  await music.setInstrument(LIVE, Number($('bank').value), Number($('program').value));
  $('instrument').textContent = await music.instrumentName(LIVE);
}

async function noteOn(key) {
  if (down.has(key)) return;
  down.add(key);
  keyboard.querySelector(`[data-key="${key}"]`)?.classList.add('down');
  if (!music) await setInstrument();
  music.noteOn(LIVE, key, 100);
}

function noteOff(key) {
  if (!down.delete(key)) return;
  keyboard.querySelector(`[data-key="${key}"]`)?.classList.remove('down');
  music?.noteOff(LIVE, key);
}

{
  let whites = 0;
  const width = 34;
  for (let key = LOW; key <= LOW + 24; key++) {
    const el = document.createElement('div');
    el.className = `key${black(key) ? ' black' : ''}`;
    el.dataset.key = key;
    if (black(key)) {
      el.style.left = `${whites * width - 12}px`;
    } else {
      whites++;
      if (key % 12 === 0) el.textContent = `C${key / 12 - 1}`;
    }
    el.onpointerdown = (e) => { el.setPointerCapture(e.pointerId); noteOn(key); };
    el.onpointerup = () => noteOff(key);
    el.onpointercancel = () => noteOff(key);
    keyboard.append(el);
  }
}

const typing = (e) => e.target.matches('input[type="number"], input[type="text"], select');
window.addEventListener('keydown', (e) => {
  const i = KEYS.indexOf(e.key.toLowerCase());
  if (i < 0 || e.repeat || e.metaKey || e.ctrlKey || typing(e)) return;
  noteOn(LOW + i);
});
window.addEventListener('keyup', (e) => {
  const i = KEYS.indexOf(e.key.toLowerCase());
  if (i >= 0) noteOff(LOW + i);
});

$('bank').onchange = setInstrument;
$('program').onchange = setInstrument;
$('bend').oninput = () => music?.pitchBend(LIVE, Number($('bend').value));
$('bend').onchange = () => {
  $('bend').value = 0;
  music?.pitchBend(LIVE, 0);
};

// A MIDI keyboard, through Web MIDI, straight into the synthesizer.
if (navigator.requestMIDIAccess) {
  const b = document.createElement('button');
  b.textContent = 'Use a MIDI keyboard';
  b.onclick = async () => {
    try {
      const access = await navigator.requestMIDIAccess();
      await liveSetup();
      const connect = () => {
        let n = 0;
        for (const input of access.inputs.values()) {
          input.onmidimessage = (m) => music.sendMidi(m.data);
          n++;
        }
        b.textContent = n ? `MIDI: ${n} input${n > 1 ? 's' : ''}` : 'No MIDI inputs found';
      };
      access.onstatechange = connect;
      connect();
    } catch {
      b.textContent = 'MIDI access was refused';
    }
  };
  $('midi-in').append(b);
}

// ---- A/B against a SoundFont --------------------------------------------

let engine = null; // Neumetik MIDI on this thread, for rendering whole songs
const engineReady = () => (engine ??= loadNeumetikMidi());

let sf = null;
let fontBytes = 0;
const ab = { a: null, b: null, sources: [], gains: null, which: 'a' };

async function soundfontModule() {
  if (sf) return sf;
  const { instance } = await WebAssembly.instantiate(await (await fetch('soundfont.wasm')).arrayBuffer(), {});
  sf = instance.exports;
  return sf;
}

function setAb(key, text) {
  document.querySelector(`[data-ab="${key}"]`).textContent = text;
}

$('sf2').onchange = async () => {
  const file = $('sf2').files[0];
  if (!file) return;
  $('sf2-name').textContent = `Reading ${file.name}…`;
  const x = await soundfontModule();
  const bytes = new Uint8Array(await file.arrayBuffer());
  const at = x.sf_alloc(bytes.length);
  new Uint8Array(x.memory.buffer, at, bytes.length).set(bytes);
  const ok = x.sf_load_font(at, bytes.length) === 0;
  x.sf_free(at, bytes.length);
  fontBytes = bytes.length;
  $('sf2-name').textContent = ok ? `${file.name} (${kb(bytes.length)})` : `${file.name} isn't a SoundFont that can be read.`;
  $('ab-render').disabled = !ok;
};

function rms(buffer) {
  let sum = 0;
  let n = 0;
  for (let c = 0; c < buffer.numberOfChannels; c++) {
    const d = buffer.getChannelData(c);
    for (let i = 0; i < d.length; i += 4) { sum += d[i] * d[i]; n++; }
  }
  return Math.sqrt(sum / Math.max(n, 1)) || 1e-9;
}

function scale(buffer, gain) {
  for (let c = 0; c < buffer.numberOfChannels; c++) {
    const d = buffer.getChannelData(c);
    for (let i = 0; i < d.length; i++) d[i] *= gain;
  }
}

$('ab-render').onclick = async () => {
  abStop();
  $('ab-render').disabled = true;
  $('ab-render').textContent = 'Rendering…';
  try {
    await ensureMusic();
    const midi = await songBytes($('ab-song').value);
    const rate = context.sampleRate;

    const nm = await engineReady();
    let t = performance.now();
    const a = nm.renderToAudioBuffer(context, midi, { tail: 2 });
    const aTime = (performance.now() - t) / 1000;

    const x = await soundfontModule();
    const at = x.sf_alloc(midi.length);
    new Uint8Array(x.memory.buffer, at, midi.length).set(midi);
    t = performance.now();
    const frames = x.sf_render(at, midi.length, rate, 2, 600);
    const bTime = (performance.now() - t) / 1000;
    x.sf_free(at, midi.length);
    if (frames < 0) throw new Error(frames === -2 ? "The SoundFont player couldn't read that MIDI file." : 'Choose a SoundFont first.');
    const b = context.createBuffer(2, frames, rate);
    b.copyToChannel(new Float32Array(x.memory.buffer, x.sf_left(), frames), 0);
    b.copyToChannel(new Float32Array(x.memory.buffer, x.sf_right(), frames), 1);

    // The same loudness, and safely under full scale.
    const target = 0.12;
    scale(a, target / rms(a));
    scale(b, target / rms(b));
    ab.a = a;
    ab.b = b;

    const stats = await statsReady;
    setAb('a-size', stats ? kb(stats.sizes.wasm_gzip) + ' (gzipped)' : '–');
    setAb('b-size', `${kb(fontBytes)} + ${stats ? kb(stats.sizes.sf_wasm_gzip ?? 0) : '–'}`);
    setAb('a-time', `${(aTime * 1000).toFixed(0)} ms`);
    setAb('b-time', `${(bTime * 1000).toFixed(0)} ms`);
    setAb('a-x', `${(a.duration / aTime).toFixed(0)}×`);
    setAb('b-x', `${(b.duration / bTime).toFixed(0)}×`);
    for (const id of ['ab-a', 'ab-b', 'ab-stop']) $(id).disabled = false;
  } catch (e) {
    $('sf2-name').textContent = e.message;
  } finally {
    $('ab-render').disabled = false;
    $('ab-render').textContent = 'Render both';
  }
};

function abStart() {
  abStop();
  const when = context.currentTime + 0.05;
  ab.gains = {};
  for (const key of ['a', 'b']) {
    const source = context.createBufferSource();
    source.buffer = ab[key];
    const gain = context.createGain();
    gain.gain.value = key === ab.which ? 1 : 0;
    source.connect(gain).connect(context.destination);
    source.start(when);
    ab.sources.push(source);
    ab.gains[key] = gain;
  }
  // Stop when the longer one ends (and only if these are still playing).
  const mine = ab.sources;
  const longer = ab.a.duration >= ab.b.duration ? 0 : 1;
  mine[longer].onended = () => { if (ab.sources === mine) abStop(); };
}

function abStop() {
  for (const s of ab.sources) { try { s.stop(); } catch { /* already stopped */ } }
  ab.sources = [];
  ab.gains = null;
  $('ab-a').classList.remove('on');
  $('ab-b').classList.remove('on');
}

async function abPlay(which) {
  await context.resume();
  if (music && shownState === 'playing') await music.pause();
  ab.which = which;
  if (!ab.gains) abStart();
  const now = context.currentTime;
  for (const key of ['a', 'b']) {
    ab.gains[key].gain.setTargetAtTime(key === which ? 1 : 0, now, 0.01);
  }
  $('ab-a').classList.toggle('on', which === 'a');
  $('ab-b').classList.toggle('on', which === 'b');
}

$('ab-a').onclick = () => abPlay('a');
$('ab-b').onclick = () => abPlay('b');
$('ab-stop').onclick = abStop;

// ---- Stats and benchmarks -------------------------------------------------

const statsReady = fetch('stats.json').then((r) => (r.ok ? r.json() : null)).catch(() => null);

function benchRow(tbody, name, x, voices) {
  const tr = document.createElement('tr');
  for (const v of [name, `${x.toFixed(0)}×`, `${(100 / x).toFixed(2)}%`, voices]) {
    const td = document.createElement('td');
    td.textContent = v;
    tr.append(td);
  }
  tbody.append(tr);
}

statsReady.then(async (stats) => {
  if (!stats) {
    document.querySelectorAll('[data-stat]').forEach((el) => { if (el.textContent === '–') el.textContent = 'run scripts/build.sh'; });
    return;
  }
  const songsOnly = stats.songs.filter((s) => s.name.endsWith('.mid'));
  const typical = songsOnly.reduce((m, s) => Math.max(m, s.cpu_percent), 0);
  const stress = stats.songs.find((s) => !s.name.endsWith('.mid'));
  const set = (key, text) => document.querySelectorAll(`[data-stat="${key}"]`).forEach((el) => { el.textContent = text; });

  try {
    const sfWasm = await (await fetch('soundfont.wasm')).arrayBuffer();
    stats.sizes.sf_wasm = sfWasm.byteLength;
    stats.sizes.sf_wasm_gzip = await gzipSize(sfWasm);
  } catch { /* the comparison isn't built */ }

  set('wasm_gzip', kb(stats.sizes.wasm_gzip));
  set('cpu_typical', `${typical.toFixed(1)}%`);
  set('memory', `${(stats.memory_bytes / 1e6).toFixed(1)} MB`);
  set('ship', `${kb(stats.sizes.wasm_gzip)} (web, gzipped), ${kb(stats.sizes.static_link_added)} (native)`);
  set('size_line', `${kb(stats.sizes.wasm)} of WebAssembly (${kb(stats.sizes.wasm_gzip)} gzipped); linked into a native program it adds ${kb(stats.sizes.static_link_added)}.`);
  set('cpu_line', `${typical.toFixed(2)}% of one core at most for the demo songs (${stress ? stress.cpu_percent.toFixed(1) : '–'}% with all 64 voices busy); ${(stats.memory_bytes / 1e6).toFixed(1)} MB, allocated when it's made.`);
  if (stats.sizes.sf_wasm) set('sf_wasm', `${kb(stats.sizes.sf_wasm)} (${kb(stats.sizes.sf_wasm_gzip)} gzipped)`);

  $('native-where').textContent = `(${stats.arch}, ${stats.os}, measured ${stats.measured})`;
  const tbody = $('native-bench').tBodies[0];
  for (const s of stats.songs) benchRow(tbody, s.name, s.realtime, s.voices);

  const sizes = $('sizes').tBodies[0];
  const rows = [
    ['WebAssembly module', `${kb(stats.sizes.wasm)} (${kb(stats.sizes.wasm_gzip)} gzipped)`],
    ['Added to a native program, linked statically', kb(stats.sizes.static_link_added)],
    ['Shared library', kb(stats.sizes.shared_library)],
    ['Memory for one player', `${(stats.memory_bytes / 1e6).toFixed(2)} MB (64 voices, chorus, reverb)`],
    ['Samples and SoundFonts', '0 bytes'],
  ];
  for (const [k, v] of rows) {
    const tr = document.createElement('tr');
    tr.innerHTML = '<th></th><td></td>';
    tr.children[0].textContent = k;
    tr.children[1].textContent = v;
    sizes.append(tr);
  }
});

async function gzipSize(buffer) {
  if (typeof CompressionStream === 'undefined') return 0;
  const stream = new Blob([buffer]).stream().pipeThrough(new CompressionStream('gzip'));
  return (await new Response(stream).arrayBuffer()).byteLength;
}

$('run-bench').onclick = async () => {
  $('run-bench').disabled = true;
  $('run-bench').textContent = 'Running…';
  const tbody = $('web-bench').tBodies[0];
  tbody.replaceChildren();
  const nm = await engineReady();
  const left = new Float32Array(512);
  const right = new Float32Array(512);
  for (const option of $('song').options) {
    if (!option.value.startsWith('songs/')) continue;
    const midi = await songBytes(option.textContent);
    const p = nm.createPlayer(48000);
    p.load(midi);
    p.play();
    let frames = 0;
    let voices = 0;
    const t = performance.now();
    while (p.playing) {
      p.render(left, right);
      frames += 512;
      voices = Math.max(voices, p.activeVoices);
    }
    const took = (performance.now() - t) / 1000;
    p.destroy();
    benchRow(tbody, option.value.slice(6), frames / 48000 / took, voices);
    await new Promise((r) => setTimeout(r, 0));
  }
  $('run-bench').disabled = false;
  $('run-bench').textContent = 'Run it again';
};

// ---- Get started tabs --------------------------------------------------

{
  const panels = [...document.querySelectorAll('#tab-panels pre')];
  panels.forEach((panel, i) => {
    const b = document.createElement('button');
    b.textContent = panel.dataset.tab;
    b.setAttribute('role', 'tab');
    b.onclick = () => {
      panels.forEach((p, j) => { p.hidden = j !== i; });
      [...$('tabs').children].forEach((t, j) => t.setAttribute('aria-selected', String(j === i)));
    };
    $('tabs').append(b);
  });
  $('tabs').children[0].click();
}

statsReady.then((stats) => { if (stats) $('version').textContent = stats.version; });
