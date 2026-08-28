#!/bin/sh
set -eu

dir=$(dirname "$0")

if [ -z "${GOOGLE_CLIENT_ID:-}" ]; then
    echo "inject-credentials.sh: \$GOOGLE_CLIENT_ID is not set" >&2
    exit 1
fi
if [ -z "${GOOGLE_CLIENT_SECRET:-}" ]; then
    echo "inject-credentials.sh: \$GOOGLE_CLIENT_SECRET is not set" >&2
    exit 1
fi
if [ -z "${MICROSOFT_CLIENT_ID:-}" ]; then
    echo "inject-credentials.sh: \$MICROSOFT_CLIENT_ID is not set" >&2
    exit 1
fi

# Escapes a value for safe use on the right-hand side of a sed s|||
# substitution (backslash, ampersand, and the | delimiter itself).
escape() {
    printf '%s' "$1" | sed -e 's/[\\&|]/\\&/g'
}

sed \
    -e "s|\${GOOGLE_CLIENT_ID}|$(escape "$GOOGLE_CLIENT_ID")|g" \
    -e "s|\${GOOGLE_CLIENT_SECRET}|$(escape "$GOOGLE_CLIENT_SECRET")|g" \
    "$dir/google.toml" > "$dir/google.toml.tmp"
mv "$dir/google.toml.tmp" "$dir/google.toml"

sed \
    -e "s|\${MICROSOFT_CLIENT_ID}|$(escape "$MICROSOFT_CLIENT_ID")|g" \
    "$dir/microsoft.toml" > "$dir/microsoft.toml.tmp"
mv "$dir/microsoft.toml.tmp" "$dir/microsoft.toml"
