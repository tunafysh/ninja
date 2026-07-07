# shellcheck disable=SC2148

pkgname=shurikenctl
pkgver=1.15.3
pkgrel=1
depends=(
    'cairo' 'desktop-file-utils' 'gdk-pixbuf2' 'glib2' 'gtk3'
    'hicolor-icon-theme' 'libsoup' 'pango' 'webkit2gtk-4.1'
)
makedepends=("binutils")
arch=('x86_64' 'aarch64')
source_x86_64=(
    "Ninja_${pkgver}_amd64.deb::https://github.com/tunafysh/ninja/releases/download/v${pkgver}/Ninja_${pkgver}_amd64.deb"
    "Ninja_${pkgver}_amd64.deb.sig::https://github.com/tunafysh/ninja/releases/download/v${pkgver}/Ninja_${pkgver}_amd64.deb.sig"
)

source_x86_64=(
    "Ninja_${pkgver}_aarch64.deb::https://github.com/tunafysh/ninja/releases/download/v${pkgver}/Ninja_${pkgver}_aarch64.deb"
    "Ninja_${pkgver}_aarch64.deb.sig::https://github.com/tunafysh/ninja/releases/download/v${pkgver}/Ninja_${pkgver}_aarch64.deb.sig"
)

sha256sums_x86_64=('SKIP')
sha256sums_aarch64=('SKIP')

package() {
    tar -xf "$srcdir/data.tar.xz" -C "$pkgdir"
}