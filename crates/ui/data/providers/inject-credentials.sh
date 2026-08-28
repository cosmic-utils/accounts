#!/usr/bin/env bash
# Substitutes ${GOOGLE_CLIENT_ID}, ${GOOGLE_CLIENT_SECRET}, and
# ${MICROSOFT_CLIENT_ID} placeholders in the provider manifests with real
# values from the environment. Used by CI for release builds (from GitHub
# Actions secrets) so real OAuth credentials never get committed; for local
# development, just edit the placeholders directly instead of running this.
set -euo pipefail

dir="$(dirname "${BASH_SOURCE[0]}")"

for var in GOOGLE_CLIENT_ID GOOGLE_CLIENT_SECRET MICROSOFT_CLIENT_ID; do
    if [ -z "${!var:-}" ]; then
        echo "inject-credentials.sh: \$$var is not set" >&2
        exit 1
    fi
done

envsubst '${GOOGLE_CLIENT_ID} ${GOOGLE_CLIENT_SECRET}' \
    < "$dir/google.toml" > "$dir/google.toml.tmp"
mv "$dir/google.toml.tmp" "$dir/google.toml"

envsubst '${MICROSOFT_CLIENT_ID}' \
    < "$dir/microsoft.toml" > "$dir/microsoft.toml.tmp"
mv "$dir/microsoft.toml.tmp" "$dir/microsoft.toml"
