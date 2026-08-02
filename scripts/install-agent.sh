#!/usr/bin/env bash
# Convenience wrapper for the exact command emitted by the authenticated Hub
# UI. Secrets are accepted only through the environment, not command arguments.
set -euo pipefail

: "${PARADE_HUB_BASE:?set PARADE_HUB_BASE to the Hub HTTPS origin}"
: "${PARADE_ENROLL_TOKEN:?set PARADE_ENROLL_TOKEN from the enrollment command}"
: "${PARADE_MANIFEST_SHA256:?set PARADE_MANIFEST_SHA256 from the enrollment command}"
: "${PARADE_RELEASE_KEY_SHA256:?set PARADE_RELEASE_KEY_SHA256 from the enrollment command}"

[[ $PARADE_HUB_BASE == https://* ]] || {
  printf 'PARADE_HUB_BASE must use HTTPS\n' >&2
  exit 1
}
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  --connect-timeout 10 --max-time 120 "$PARADE_HUB_BASE/install.sh" \
  | sudo env \
      PARADE_ENROLL_TOKEN="$PARADE_ENROLL_TOKEN" \
      PARADE_MANIFEST_SHA256="$PARADE_MANIFEST_SHA256" \
      PARADE_RELEASE_KEY_SHA256="$PARADE_RELEASE_KEY_SHA256" \
      bash
