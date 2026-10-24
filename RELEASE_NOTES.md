# IW4 Pocket 0.3.9 — build 23

**Stable release, with a known limit: memory.** Tried on an iPhone 13 Pro Max (about 3 GB granted): 17 bots, one match of about 9 minutes, then iOS closed the app. Tried on an M1 iPad (5.1 GB granted): 4-5 matches in a row, then the memory ran out while loading a bigger map. The app does not give memory back between matches; 0.4.0 works on that first.

0.3.9 is a stability release. It fixes problems found by reading the 0.3.8 code and its first real logs. Selective weapon loading now starts (it never did in 0.3.8), but it saves little: the weapon models skipped were about 10% of the total.

## Selective weapon loading (it never ran in 0.3.8)
- In the 0.3.8 log the feature fell back to loading everything: `selective weapons FAILED -> full preload: unknown weapon "t5:weapon/spectre_mp"`. The class presets name weapons of games that are not installed, and one such name cancelled the whole selection. Weapons of a game that is not installed are now skipped (one log line lists them); an unknown weapon of an installed game is still an error and still falls back, visibly.
- Airdrop markers, insertion flares, launchers and the riot shield are always loaded, so `give` and pickups no longer refuse them. Names missing from the catalog are logged.
- Auto means on when iOS grants less than 4 GB or the amount is unknown (the Low tier; `IW4L_TIER` overrides it). Options > Selective weapon loading > Off loads everything.
- Choosing a class whose weapon is not loaded now tells the client, so the class screen does not wait. Bots fall back to the default class instead of staying without one. A weapon that is not loaded stays unavailable until the next match: these limits come from the code and have not been tried.
- One plan per set of inputs: the match no longer rebuilds it and logs it twice.

## Memory
- On iOS the app now hands free memory back to the system (`malloc_zone_pressure_relief`). Before 0.3.9 that call existed only for Linux and did nothing on iPhone, while the heap held about 490 MiB more than it used. It runs before the match is installed, after the load, and when less than 400 MiB are left (at most every 2 s). The log lines say how many MiB came back; the real gain is not measured yet.

## Text/UI sharpness
- Auto is now Medium. Menus, HUD and text are drawn at half the screen resolution or better while the 3D scene stays at your render scale. It only does something when the 3D scale is below 0.6; at 0.6 or more nothing changes. Off restores the old behaviour (everything at the 3D scale). If the screen goes black, set it to Off.
- Native and High are sharper and use more memory.

## Players and bots
- Client ids are reused: the lowest free id below 18 is given, so many joins and leaves no longer push ids into the corpse entities (18 and up).
- A human enrolled in the same frame a bot is created no longer receives that bot's id.
- The simulation no longer panics on a client id of 1024 or more, or on a player position that is not finite. It refuses that one link and logs it.

## Diagnostics and release
- A match that ends now writes a closing `match summary` line. Before, matches shorter than 30 s left none.
- The SideStore source no longer repeats the "Experimental prerelease" warning twice in a TEST description.
- CI runs the source tests and has a guard against the mistake that broke the first 0.3.8 build (`memory_settings.rs` is compiled alone and cannot use other modules).

## Version
- Label **0.3.9**; Cargo `0.3.9`; bundle build **23**.

Verification scope: Python source-contract checks run on the server. The Rust crates are compiled and tested only by the iOS CI.
