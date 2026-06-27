#!/usr/bin/env bash
# Build eyegentic to a zellij-loadable .wasm.
#
# Newer Rust toolchains renamed `wasm32-wasi` to `wasm32-wasip1`; the two are
# ABI-compatible for zellij's purposes. We pick whichever is installed, build,
# and copy the result to a stable `eyegentic.wasm` next to this script so the
# layout file doesn't have to know the target triple.
set -euo pipefail
cd "$(dirname "$0")"

TARGET=""
for t in wasm32-wasip1 wasm32-wasi; do
  if rustup target list --installed 2>/dev/null | grep -q "$t"; then
    TARGET="$t"
    break
  fi
done

if [ -z "$TARGET" ]; then
  echo ">> installing wasm32-wasip1 target (one-time)…"
  rustup target add wasm32-wasip1
  TARGET="wasm32-wasip1"
fi

PROFILE="${PROFILE:-release}"
echo ">> cargo build --$PROFILE --target $TARGET"
if [ "$PROFILE" = "release" ]; then
  cargo build --release --target "$TARGET"
else
  cargo build --target "$TARGET"
fi

WASM="target/$TARGET/$PROFILE/eyegentic.wasm"
cp -f "$WASM" eyegentic.wasm
echo ">> built $WASM"
echo ">> copied to $(pwd)/eyegentic.wasm"

# Seed zellij's permission cache so the plugin is granted on first load without
# an interactive prompt. eyegentic is meant to live in a 1-row borderless pane,
# where zellij's permission prompt can't be displayed or answered
# (zellij-org/zellij#4749) — so without this the plugin loads and then sits
# frozen (no Timer events, no status bar, no auto-installed hook).
#
# Opt out with SEED_PERMISSIONS=0 (e.g. for release/CI builds where the install
# target path isn't known yet).
if [ "${SEED_PERMISSIONS:-1}" != "0" ]; then
  echo ">> seeding zellij permission cache…"
  ./scripts/seed-permissions.sh || echo "!! permission seeding failed (non-fatal); run scripts/seed-permissions.sh manually"
fi

echo "   load with:  zellij -l zellij.kdl   (from this folder)"
