#!/usr/bin/env bash
# =============================================================================
# Build parade-agent for every supported architecture, in BOTH flavours:
#
#   • static musl  — one binary, no libc version lock, runs on any Linux/arch
#   • dynamic gnu  — smaller, links the host glibc (build on an old base image
#                    if you need it to run on older systems)
#
# Output is staged into the hub's dist/ directory in the exact layout the
# one-click installer expects:
#
#     dist/<target-triple>/parade-agent
#
# The installer prefers the musl build and falls back to gnu, so shipping both
# means "it just works" on the widest possible range of machines.
#
# Usage:
#     scripts/build-agents.sh [--gnu-only|--musl-only] [--unsigned] [--dist DIR]
#
# Requires `cross` (recommended, handles all C toolchains via Docker/Podman):
#     cargo install cross --git https://github.com/cross-rs/cross
# Falls back to plain `cargo` for the host target if `cross` isn't present.
# =============================================================================
set -euo pipefail

command -v openssl >/dev/null 2>&1 || { echo "openssl is required" >&2; exit 1; }

cd "$(dirname "$0")/.."          # repo root
DIST="dist"
FLAVOURS=(musl gnu)
SIGNED=1

while [ $# -gt 0 ]; do
  case "$1" in
    --gnu-only)  FLAVOURS=(gnu) ;;
    --musl-only) FLAVOURS=(musl) ;;
    --unsigned)  SIGNED=0 ;;
    --dist)      DIST="$2"; shift ;;
    -h|--help)   sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 1 ;;
  esac
  shift
done

if [ "$SIGNED" -eq 1 ]; then
  : "${PARADE_RELEASE_SIGNING_KEY:?set PARADE_RELEASE_SIGNING_KEY to an offline Ed25519 private-key PEM}"
fi

# Architectures. Raw statfs(2) is implemented for x86_64/aarch64/riscv64;
# armv7 builds and runs (disk shown as 0 on that 32-bit arch — see docs).
MUSL_TARGETS=(
  x86_64-unknown-linux-musl
  aarch64-unknown-linux-musl
  armv7-unknown-linux-musleabihf
)
GNU_TARGETS=(
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  armv7-unknown-linux-gnueabihf
  riscv64gc-unknown-linux-gnu
)

if command -v cross >/dev/null 2>&1; then
  BUILDER=cross
else
  BUILDER=cargo
  HOST_TARGET=$(rustc -vV | sed -n 's/^host: //p')
  echo "note: 'cross' not found — building the host target only: ${HOST_TARGET}"
  echo "      install it for full cross-arch builds:"
  echo "      cargo install cross --git https://github.com/cross-rs/cross"
fi

build_one() {
  local target="$1"
  if [ "$BUILDER" = cargo ] && [ "$target" != "$HOST_TARGET" ]; then
    echo "──▶ skipping ${target} (cross is required)"
    return 1
  fi
  echo "──▶ building parade-agent for ${target} …"
  if [ "$BUILDER" = cross ]; then
    if ! cross build --release --locked -p parade-agent --all-features --target "$target"; then
      echo "   ✗ ${target} failed (toolchain/target missing?) — skipping"
      return 1
    fi
  else
    # Plain cargo is intentionally limited to the already installed host
    # target; cross-architecture failures are never silently ignored.
    if ! cargo build --release --locked -p parade-agent --all-features --target "$target"; then
      echo "   ✗ ${target} not buildable with plain cargo — skipping"
      return 1
    fi
  fi
  local src="target/${target}/release/parade-agent"
  local dst="${DIST}/${target}/parade-agent"
  mkdir -p "${DIST}/${target}"
  cp "$src" "$dst"
  # Stripping is optional and explicitly reported when the host tool cannot
  # understand a foreign binary.
  if command -v strip >/dev/null 2>&1 && ! strip "$dst" 2>/dev/null; then
    echo "   note: strip does not support ${target}; keeping symbols"
  fi
  local size
  size=$(du -h "$dst" | cut -f1)
  echo "   ✓ ${dst} (${size})"
}

built=0
for flavour in "${FLAVOURS[@]}"; do
  case "$flavour" in
    musl) targets=("${MUSL_TARGETS[@]}") ;;
    gnu)  targets=("${GNU_TARGETS[@]}") ;;
  esac
  for t in "${targets[@]}"; do
    if build_one "$t"; then built=$((built+1)); fi
  done
done

echo ""
if [ "$built" -eq 0 ]; then
  echo "✗ nothing built. Install 'cross' for cross-arch builds, or build the"
  echo "  host target with: cargo build --release --locked -p parade-agent"
  exit 1
fi
echo "✓ staged ${built} agent binaries under ${DIST}/"
(
  cd "$DIST"
  find . -mindepth 2 -maxdepth 2 -type f -name parade-agent -print0 \
    | sort -z \
    | xargs -0 sha256sum \
    | sed 's#  \./#  #' > SHA256SUMS
)
echo "✓ wrote ${DIST}/SHA256SUMS (its digest is pinned in each enrollment command)"
if [ "$SIGNED" -eq 1 ]; then
  openssl pkeyutl -sign -inkey "$PARADE_RELEASE_SIGNING_KEY" -rawin \
    -in "${DIST}/SHA256SUMS" -out "${DIST}/SHA256SUMS.sig"
  openssl pkey -in "$PARADE_RELEASE_SIGNING_KEY" -pubout \
    -out "${DIST}/release-public.pem"
  echo "✓ signed SHA256SUMS with the offline Ed25519 release key"
else
  echo "✓ staged unsigned manifest; signing must happen in an isolated release step"
fi
echo "  Point the hub at this directory (hub.toml → [hub].dist_dir = \"${DIST}\")"
echo "  and every new VPS can enroll with a single copy-paste command."
