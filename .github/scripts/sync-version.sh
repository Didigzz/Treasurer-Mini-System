#!/usr/bin/env bash
# Rewrite every version field in the workspace from a release tag.
#
# The tag is the source of truth for a release: xbuild reads manifest.yaml, the
# Cargo metadata and Cargo.lock must agree with the crate version, and the app
# compiles VERSION_NAME/VERSION_CODE into the updater. Keeping all four in step
# stops a release from shipping a mixed version.
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: sync-version.sh <version_name> <version_code>" >&2
  exit 2
fi

name=$1
code=$2

if ! printf '%s' "$name" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "version_name must be three dot-separated numbers (got $name)" >&2
  exit 2
fi
if ! printf '%s' "$code" | grep -Eq '^[1-9][0-9]*$'; then
  echo "version_code must be a positive integer (got $code)" >&2
  exit 2
fi

cd "$(dirname "$0")/../.."

perl -pi -e "s/^(\s*version_name:).*\$/\$1 $name/" apps/mobile/manifest.yaml
perl -pi -e "s/^(\s*version_code:).*\$/\$1 $code/" apps/mobile/manifest.yaml

perl -pi -e "s/^version = \".*\"\$/version = \"$name\"/" apps/mobile/Cargo.toml
perl -pi -e "s/^version_code = .*\$/version_code = $code/" apps/mobile/Cargo.toml
perl -pi -e "s/^version_name = \".*\"\$/version_name = \"$name\"/" apps/mobile/Cargo.toml

perl -pi -e "s/^pub const VERSION_NAME: &str = \".*\";\$/pub const VERSION_NAME: &str = \"$name\";/" apps/mobile/src/version.rs
perl -pi -e "s/^pub const VERSION_CODE: i32 = .*;\$/pub const VERSION_CODE: i32 = $code;/" apps/mobile/src/version.rs

perl -0777 -pi -e "s/(name = \"treasurer-mobile\"\r?\nversion = \")[^\"]*(\")/\${1}$name\${2}/" Cargo.lock

# Fail loudly rather than build a release with a stale version field.
grep -q "version_name: $name" apps/mobile/manifest.yaml
grep -q "version_code: $code" apps/mobile/manifest.yaml
grep -q "version = \"$name\"" apps/mobile/Cargo.toml
grep -q "version_code = $code" apps/mobile/Cargo.toml
grep -q "version_name = \"$name\"" apps/mobile/Cargo.toml
grep -q "VERSION_NAME: &str = \"$name\";" apps/mobile/src/version.rs
grep -q "VERSION_CODE: i32 = $code;" apps/mobile/src/version.rs
awk -v name="$name" '
  /^name = "treasurer-mobile"$/ { found = 1; next }
  found && /^version = / { if ($0 == "version = \"" name "\"") ok = 1; found = 0 }
  END { exit ok ? 0 : 1 }
' Cargo.lock
