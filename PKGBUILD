# Maintainer: Skuld Mod Manager contributors
# Contributor: Claude <noreply@anthropic.com>

pkgname=skuld-mod-manager
pkgver=0.1.0
pkgrel=1
pkgdesc="Tauri 2 desktop app for managing game mods on Linux"
arch=('x86_64')
url="https://github.com/skuld/skuld-mod-manager"
license=('MIT')
depends=('webkit2gtk-4.1' 'p7zip' 'openssl' 'glib2' 'gtk3' 'libsoup3')
makedepends=('cargo' 'nodejs' 'npm' 'base-devel' 'git')
source=("${pkgname}-${pkgver}.tar.gz::https://github.com/skuld/skuld-mod-manager/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('SKIP')

build() {
  cd "${srcdir}/${pkgname}-${pkgver}"
  npm install
  npm run build
  cd src-tauri
  cargo build --release --locked
}

package() {
  cd "${srcdir}/${pkgname}-${pkgver}"

  install -Dm755 "src-tauri/target/release/${pkgname}" \
    "${pkgdir}/usr/bin/${pkgname}"

  install -Dm644 "src-tauri/icons/128x128.png" \
    "${pkgdir}/usr/share/icons/hicolor/128x128/apps/${pkgname}.png"

  install -Dm644 /dev/stdin "${pkgdir}/usr/share/applications/${pkgname}.desktop" <<EOF
[Desktop Entry]
Name=Skuld Mod Manager
Comment=Manage game mods with symlinks
Exec=${pkgname}
Icon=${pkgname}
Terminal=false
Type=Application
Categories=Utility;Game;
EOF
}
