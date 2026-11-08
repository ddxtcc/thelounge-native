# AppImage packaging

Build an AppImage from a release binary:

```sh
packaging/appimage/build-appimage.sh target/release/lounge-native
```

The output is written to `dist/lounge-native_<version>_x86_64.AppImage`. The
script downloads `linuxdeploy`, its GStreamer plugin, and `appimagetool` into
`.appimage-tools` on first use.

GStreamer plugins and the plugin scanner are bundled into the AppImage. Set
`GPG_KEY` to sign the AppImage and its embedded update information:

```sh
GPG_KEY=YOUR_KEY_ID packaging/appimage/build-appimage.sh target/release/lounge-native
```

The AppImage embeds GitHub release zsync update information and writes the
matching `.zsync` file next to the AppImage.

Requirements: `curl`, `patchelf`, `zsyncmake`, `mksquashfs`, and the GStreamer
runtime/plugins used by the build.
