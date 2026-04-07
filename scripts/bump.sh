# 1. bump Cargo.toml
version = "$1"

# 2. tag it
git tag v$1
git push --tags

# 3. update PKGBUILD
pkgver=$1

# 4. update checksums
updpkgsums

# 5. publish to AUR
