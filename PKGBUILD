pkgname=shurikenctl
pkgver=1.15.0
pkgrel=1
depends=(
    'cairo' 'desktop-file-utils' 'gdk-pixbuf2' 'glib2' 'gtk3'
    'hicolor-icon-theme' 'libsoup' 'pango' 'webkit2gtk-4.1'
)
makedepends=("cargo" "python" "nodejs")
arch=('i686' 'x86_64' 'aarch64' 'armv7h')
source=()
b2sums=('SKIP')

prepare() {
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked
}

build() {
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-features
}

#check() {
#    export RUSTUP_TOOLCHAIN=stable
#    cargo test --frozen --all-features
#}

package() {
    mv "target/release/ninja" "target/release/ninja-app"
    install -Dm0755 -t "$pkgdir/usr/bin/" "target/release/{$pkgname, ninja-app}"
    # for custom license, e.g. MIT
    # install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
