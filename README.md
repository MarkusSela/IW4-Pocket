<table align="center">
<tr>
<td align="center" width="50%">
<img src="media/demo.gif" alt="IW4 Pocket demo, iPhone 13 Pro Max">
</td>
<td align="center" width="50%">
<img src="media/treuenten.gif" alt="IW4 Pocket match, iPhone 17 Pro Max, by treuenten">
</td>
</tr>
</table>

<p align="center">
  <img src="media/icon.png" width="110" alt="IW4 Pocket icon">
</p>

<h1 align="center">IW4 Pocket</h1>

<p align="center">Modern Warfare 2 (2009) on your iPhone, running on the open-source IW4L engine. Unofficial and experimental.</p>

<p align="center"><a href="https://ko-fi.com/marukoshi"><img src="https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi"></a></p>

<p align="center">🌍 <b>English</b> · <a href="README.it.md">Italiano</a> · <a href="README.es.md">Español</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a></p>

## 🎯 What is this?

A port of the open-source [IW4L](https://github.com/vladtrc/iw4L) engine to iOS. It **does not contain the game**: it reads the game files **you already own** on PC. Think of it as a new player for files you have.

## 📊 Where it stands

- ✅ Runs natively on iPhone (Rust + Bevy + Metal)
- ✅ Reaches the main menu. Touch works as a click and a PS4 controller works
- ✅ On an iPhone 17 Pro Max (6 GB granted to the app) full matches with bots run: [watch the clip](https://jumpshare.com/s/rFCD5I9ZtWvcN3gcBO8e), by [treuenten](https://github.com/treuenten)
- ✅ The app picks its memory settings by itself from the memory iOS grants the phone (low / mid / high)
- ⚠️ On an iPhone 13 Pro Max (about 3 GB granted), `mp_rust` now loads up to the shaders, then the app quits by itself. Still being investigated

## 🙋 Try it, step by step

No coding needed. It takes about 15 minutes plus the file copy.

1. **Get the game files.** You need your own copy of *Modern Warfare 2 (2009)* for **PC** (the Steam version). The folder must contain a subfolder named `zone`. This project does not provide or link to game files
2. **Install SideStore** on your iPhone by following the [official guide](https://sidestore.io). It is the tool that installs apps like this one
3. **Download the app:** [IW4.Pocket.ipa](https://github.com/MarkusSela/IW4-Pocket/releases/latest/download/IW4.Pocket.ipa) (or open the [release page](https://github.com/MarkusSela/IW4-Pocket/releases/latest))
4. **Install it with SideStore** (tap **+**, pick the file). If you already have an older version, install **over** it: uninstalling deletes your game files
5. **Open the app once**, then close it. This creates its folder
6. **Copy your MW2 folder** into the Files app: *On My iPhone > IW4 Pocket > Games*. Any folder name works, as long as it contains `zone`
7. **Pair a controller** in Bluetooth settings (PS4/DualShock 4 is verified), then open the app. The first load is slow and the screen can stay pink for a while

## 📱 Will it work on my device?

- ✅ **Tested:** iPhone 13 Pro Max (6 GB RAM, ~3 GB granted), iOS 27: menus work, match loading gets as far as the shaders. iPhone 17 Pro Max (6 GB granted): full matches with bots, reported by treuenten
- ❓ **Everything else is unverified.** Technically it should run on any iPhone or iPad with Metal; the app declares iOS 15 as its minimum, but I never tried it there
- 🧠 **Memory is the limit.** On the tested phone iOS allows the app about 3 GB. Devices with less RAM will likely fail sooner; 8 GB+ iPhones and M-series iPads are the most interesting to test
- 🎮 **A controller is needed to play.** Touch only works in the menus

## 🐞 Help me test

Tried it? Please [open an issue](https://github.com/MarkusSela/IW4-Pocket/issues/new) with:

- your device and iOS version
- what happened (menu? map loaded? app closed?)
- the file `iw4l-boot.log` from *On My iPhone > IW4 Pocket*. It records startup, memory use and crashes

## 🔧 Something went wrong?

- **"MW2 Multiplayer was not found"**: the folder must be inside `Games` and contain `zone`
- **Pink screen for a long time**: wait, the first load is slow
- **Controller not detected**: pair it in Bluetooth first, then reopen the app
- **App closes on a map**: that is the known memory issue. Send the log
- **To limit texture size**: add a text file `iw4l-texture-cap.txt` containing only a number such as `256`, then fully restart the app
- **Advanced:** a text file `iw4l-env.txt` with lines like `IW4L_NAME=value` sets the engine's own switches without a rebuild. Names must start with `IW4L_`

## 🛠️ Build it yourself

Run the **ios-release** workflow from the Actions tab (macOS runner) and give it a tag.

## ⚖️ Credits and legal

Port of [IW4L](https://github.com/vladtrc/iw4L) by vladtrc and contributors (Apache-2.0, see `LICENSE`, `NOTICE`; original README in `README.upstream.md`). Memory fixes, BC textures, frame timing and engine fixes: [treuenten](https://github.com/treuenten) ([issue #1](https://github.com/MarkusSela/IW4-Pocket/issues/1)).

> Unofficial fan project, not affiliated with Activision, Infinity Ward, Apple or the IW4L authors. Call of Duty and Modern Warfare are trademarks of their owners. You must own a legitimate copy of the game.

---

<p align="center">☕ Like it? Support the project on <a href="https://ko-fi.com/marukoshi">Ko-fi</a></p>
