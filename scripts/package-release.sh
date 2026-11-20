#!/usr/bin/env bash
# Builds the release archive, SHA256SUMS and the release notes.
#
# Usage: scripts/package-release.sh VERSION [BINARY] [OUT_DIR]
#
# Set CHECK_LINKS=1 to also require that glibc and libasound are linked dynamically.
set -euo pipefail

# The one place that names the app in the release.
readonly APP_NAME=pikutracker

version=${1:?usage: package-release.sh VERSION [BINARY] [OUT_DIR]}
binary=$(realpath "${2:-target/release/$APP_NAME}")
out_dir=$(realpath --canonicalize-missing "${3:-dist}")
cd "$(dirname "${BASH_SOURCE[0]}")/.."

reported=$("$binary" --version)
if [[ $reported != "$APP_NAME $version" ]]; then
    echo "error: expected '$APP_NAME $version' but the binary reports '$reported'" >&2
    exit 1
fi

if [[ ${CHECK_LINKS:-0} == 1 ]]; then
    needed=$(readelf --dynamic "$binary" | grep NEEDED)
    for library in libc.so.6 libasound.so.2; do
        if ! grep --quiet --fixed-strings "[$library]" <<<"$needed"; then
            echo "error: $library is not linked dynamically" >&2
            exit 1
        fi
    done
fi

notes=$(awk -v version="$version" '
    /^## / { if (found) exit; if ($2 == version) { found = 1; next } }
    found
' CHANGELOG.md)
if [[ -z ${notes//[[:space:]]/} ]]; then
    echo "error: CHANGELOG.md has no '## $version' section" >&2
    exit 1
fi

name="$APP_NAME-$version-x86_64-linux"
mkdir -p "$out_dir/$name"
install -m 755 "$binary" "$out_dir/$name/$APP_NAME"
install -m 644 README.md CHANGELOG.md LICENSE-MIT LICENSE-APACHE THIRD_PARTY_LICENSES \
    config.example.ron "$out_dir/$name/"

tar --create --gzip --sort=name --owner=0 --group=0 --numeric-owner \
    --file "$out_dir/$name.tar.gz" --directory "$out_dir" "$name"
rm -r "${out_dir:?}/$name"

(cd "$out_dir" && sha256sum "$name.tar.gz" >SHA256SUMS)
printf '%s\n' "$notes" >"$out_dir/RELEASE_NOTES.md"
