# Arch Linux packaging

Install from the AUR package source:

```sh
cd packaging/aur
makepkg -si
```

The package builds from a pinned commit. Update `_commit`, `pkgver`, and
`sha256sums` when publishing a new version:

```sh
commit=$(git rev-parse HEAD)
curl -L -o "lounge-native.tar.gz" "https://github.com/ddxtcc/thelounge-native/archive/$commit.tar.gz"
sha256sum lounge-native.tar.gz
makepkg --printsrcinfo > .SRCINFO
```

Publish `PKGBUILD`, `lounge-native.install`, and `.SRCINFO` to the AUR package
repository:

```sh
git clone ssh://aur@aur.archlinux.org/lounge-native.git
cp PKGBUILD lounge-native.install .SRCINFO lounge-native/
cd lounge-native
git add PKGBUILD lounge-native.install .SRCINFO
git commit -m "upgpkg: lounge-native 1.1.8-1"
git push
```

## Pacman repository

Create or refresh a repository database from built packages:

```sh
./build-repo.sh /path/to/repository
```

The script copies `*.pkg.tar.zst` files into the repository and updates
`lounge-native.db.tar.gz`. Set `GPG_KEY` to sign the database with
`repo-add --sign`:

```sh
GPG_KEY=YOUR_KEY_ID ./build-repo.sh /path/to/repository
```

Serve or synchronize the repository directory with any static file host. Clients
add it to `/etc/pacman.conf` and refresh with `pacman -Sy`.
