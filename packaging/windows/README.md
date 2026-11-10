# Windows packaging

Lounge Native no longer bundles GStreamer. The Windows archive and installer
contain the application, a small launcher, and the license/readme files.

The launcher, `lounge-native.exe`, locates an installed GStreamer 1.26 MSVC
x86_64 runtime in `GSTREAMER_1_0_ROOT_MSVC_X86_64`, `GSTREAMER_ROOT_X86_64`,
`GSTREAMER_ROOT`, or the default `C:\gstreamer\1.0\msvc_x86_64` path, adds its `bin` directory to
`PATH`, configures the plugin paths, and starts `lounge-native-app.exe`.

Install the GStreamer runtime from:

https://gstreamer.freedesktop.org/download/#windows

The installer creates a Start Menu shortcut to the launcher. The zip is
portable and can be unpacked anywhere; run `lounge-native.exe` after GStreamer
is installed.
