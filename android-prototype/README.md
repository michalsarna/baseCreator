# BaseCreator on Android: prototype

A throwaway test, not part of the library: does egui on an Android tablet get the soft keyboard, letters beyond ASCII,
touch and rotation? Results and the whole plan of the BaseCreator editor (desktop + Android): `PLAN.md` (Polish).

Build (macOS, tools from Homebrew: rustup with the Android targets, openjdk@17, android-commandlinetools with NDK 29,
gradle, cargo-ndk):

```sh
. ./env.sh
cargo ndk -t arm64-v8a -P 29 -o android/app/src/main/jniLibs build
cd android && ./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```
