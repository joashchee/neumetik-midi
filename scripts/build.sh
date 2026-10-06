#!/usr/bin/env bash
# Builds Neumetik MIDI for release and its demo page:
#   dist/   the header, the native libraries, the WebAssembly module, the
#           bindings and the demo songs: what a release ships
#   demo/   the demo page, ready to serve:
#           python3 -m http.server -d demo 8000   (then open localhost:8000)
# Measures the sizes and runs the benchmark for the page as it goes.
#   --no-test   skip `cargo test`
set -euo pipefail
cd "$(dirname "$0")/.."
root=$PWD
test=1
[[ "${1:-}" == "--no-test" ]] && test=0

say() { printf '\n== %s\n' "$*"; }

if (( test )); then
  say "Tests"
  cargo test --quiet
fi

say "Native library (static and shared)"
cargo build --quiet --release --manifest-path capi/Cargo.toml
say "WebAssembly"
cargo build --quiet --release --manifest-path capi/Cargo.toml --target wasm32-unknown-unknown
say "The demo's SoundFont player (rustysynth, for the A/B comparison)"
cargo build --quiet --release --manifest-path demo/ab-soundfont/Cargo.toml --target wasm32-unknown-unknown

native=capi/target/release
wasm=capi/target/wasm32-unknown-unknown/release/neumetik_midi.wasm
case "$(uname -s)" in
  Darwin) shared=libneumetik_midi.dylib ;;
  MINGW*|MSYS*|CYGWIN*) shared=neumetik_midi.dll ;;
  *) shared=libneumetik_midi.so ;;
esac

say "Demo songs"
mkdir -p demo/songs
cargo run --quiet --release --example make_demos -- demo/songs

say "Benchmark"
cargo run --quiet --release --example bench -- demo/songs --json "${TMPDIR:-/tmp}/nm-bench.json"

say "Sizes"
# What linking the static library adds to a C program, stripped.
tmp=$(mktemp -d "${TMPDIR:-/tmp}/neumetik-midi.XXXXXX")
printf 'int main(void){return 0;}\n' > "$tmp/empty.c"
cc -O2 "$tmp/empty.c" -o "$tmp/empty"
cc -std=c99 -O2 -Icapi/include capi/examples/render_wav.c "$native/libneumetik_midi.a" -o "$tmp/render_wav" -lm
strip "$tmp/empty" "$tmp/render_wav" 2>/dev/null || true
python3 - "$wasm" "$native/$shared" "$tmp/empty" "$tmp/render_wav" "${TMPDIR:-/tmp}/nm-bench.json" demo/stats.json <<'PY'
import gzip, json, os, sys, datetime
wasm, shared, empty, linked, bench, out = sys.argv[1:]
stats = json.load(open(bench))
raw = open(wasm, 'rb').read()
stats["sizes"] = {
    "wasm": len(raw),
    "wasm_gzip": len(gzip.compress(raw, 9)),
    "shared_library": os.path.getsize(shared),
    "static_link_added": os.path.getsize(linked) - os.path.getsize(empty),
}
stats["measured"] = datetime.date.today().isoformat()
json.dump(stats, open(out, "w"), indent=1)
s = stats["sizes"]
print(f"wasm {s['wasm']/1024:.0f} KB ({s['wasm_gzip']/1024:.0f} KB gzipped), shared library {s['shared_library']/1024:.0f} KB, "
      f"static link adds {s['static_link_added']/1024:.0f} KB")
PY
cp "$tmp/render_wav" "$root/capi/target/render_wav" 2>/dev/null || true
rm -rf "$tmp"

say "dist/"
rm -rf dist
mkdir -p dist/include dist/lib dist/js dist/love dist/csharp dist/songs
cp capi/include/neumetik_midi.h dist/include/
cp "$native/libneumetik_midi.a" "$native/$shared" dist/lib/
cp "$wasm" bindings/js/neumetik-midi.js bindings/js/neumetik-midi-worklet.js dist/js/
cp bindings/love/neumetik_midi.lua bindings/love/main.lua dist/love/
cp bindings/csharp/NeumetikMidi.cs dist/csharp/
cp demo/songs/*.mid dist/songs/
cp LICENSE README.md dist/

say "demo/"
cp "$wasm" bindings/js/neumetik-midi.js bindings/js/neumetik-midi-worklet.js demo/
cp demo/ab-soundfont/target/wasm32-unknown-unknown/release/neumetik_midi_demo_soundfont.wasm demo/soundfont.wasm

say "Done"
echo "Serve the demo: python3 -m http.server -d demo 8000"
