#!/usr/bin/env bash
# Install a Swift toolchain into /opt/swift (isolated from the system).
# This is the piece xtool bundles; it gives us `swiftc` that can target
# arm64-apple-ios against the iPhoneOS SDK we already have.
set -euo pipefail

VERSION=${SWIFT_VERSION:-6.4.2}
# Directory segment and artifact suffix are spelled differently on purpose:
#   .../ubuntu2404/swift-6.4.2-RELEASE/swift-6.4.2-RELEASE-ubuntu24.04.tar.gz
DISTRO=${SWIFT_DISTRO:-ubuntu2404}
SUFFIX=${SWIFT_SUFFIX:-ubuntu24.04}
NAME="swift-${VERSION}-RELEASE"
TARBALL="${NAME}-${SUFFIX}.tar.gz"
URL="https://download.swift.org/swift-${VERSION}-release/${DISTRO}/${NAME}/${TARBALL}"

mkdir -p /opt/swift /tmp/swift-dl
cd /tmp/swift-dl

if [ ! -x "/opt/swift/${NAME}/usr/bin/swiftc" ]; then
  echo "downloading $URL"
  curl -fL --retry 3 -o "$TARBALL" "$URL"
  echo "extracting"
  tar xzf "$TARBALL" -C /opt/swift
fi

SWIFTC="/opt/swift/${NAME}/usr/bin/swiftc"
echo "--- swiftc ---"
"$SWIFTC" --version
