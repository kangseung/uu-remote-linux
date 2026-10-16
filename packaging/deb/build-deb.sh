#!/usr/bin/env bash
# Builds uuremote-server_<version>_amd64.deb with cargo + dpkg-deb (no cargo-deb).
# Run from anywhere; operates on the repository root two levels up.
set -euo pipefail

cd "$(dirname "$0")/../.."
root=$(pwd)
pkg=uuremote-server
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ -n "$version" ] || { echo "Could not read version from Cargo.toml" >&2; exit 1; }

command -v cargo >/dev/null 2>&1 || export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked --features wayland,portal

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

install -Dm755 target/release/uuremote-server "$stage/usr/bin/uuremote-server"
install -Dm644 packaging/deb/60-uuremote-uinput.rules "$stage/usr/lib/udev/rules.d/60-uuremote-uinput.rules"
install -Dm644 packaging/deb/uuremote-server.service "$stage/usr/lib/systemd/user/uuremote-server.service"
install -Dm644 packaging/deb/copyright "$stage/usr/share/doc/$pkg/copyright"

changelog="$stage/usr/share/doc/$pkg/changelog"
mkdir -p "$(dirname "$changelog")"
{
    echo "$pkg ($version) unstable; urgency=medium"
    echo
    echo "  * Release $version; see docs/release-notes in the source tree."
    echo
    echo " -- uu-remote-linux contributors <https://github.com/VallierDev/uu-remote-linux>  $(date -R)"
} > "$changelog"
gzip -9n "$changelog"

size=$(du -sk "$stage/usr" | cut -f1)
mkdir -p "$stage/DEBIAN"
sed -e "s/@VERSION@/$version/" -e "s/@SIZE@/$size/" packaging/deb/control > "$stage/DEBIAN/control"
for script in postinst prerm postrm; do
    install -Dm755 "packaging/deb/$script" "$stage/DEBIAN/$script"
done

out="packaging/deb/${pkg}_${version}_amd64.deb"
dpkg-deb --build --root-owner-group "$stage" "$root/$out"
echo "Built $root/$out"

# A local build links the build machine's glibc (e.g. GLIBC_2.38+ on Debian 13/LMDE7)
# and will NOT run on Ubuntu 22.04/Debian 12 (glibc 2.35/2.36). Redistribution builds
# come from the CI release job (ubuntu-22.04).
glibc=$(ldd --version | sed -n '1s/.* //p')
if [ "$(printf '2.35\n%s\n' "$glibc" | sort -V | tail -1)" = "$glibc" ] && [ "$glibc" != "2.35" ]; then
    echo "WARNING: built against glibc $glibc (> 2.35). This deb only runs on systems with glibc >= $glibc." >&2
    echo "         Do NOT share it with Ubuntu 22.04 / Debian 12 / Mint 21 machines;" >&2
    echo "         use the GitHub Releases deb (CI, ubuntu-22.04) for distribution." >&2
    echo "         警告：本机 glibc $glibc 过新，此 deb 不能发给 Ubuntu 22.04/Debian 12/Mint 21；分发请用 GitHub Releases 的 CI 构建。" >&2
fi
