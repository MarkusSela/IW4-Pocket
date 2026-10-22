# IW4 Pocket 0.3.8 — build 22

**Experimental prerelease. Not verified on a phone.** This build does not claim to fix the memory crash: that is still unproven.

## Bots
- Up to **17 bots**: with you that makes **18 participants**, the protocol cap. Before, the options file, the console and the lobby stopped lower.
- `IW4L_BOTS` now accepts 0 to 17; `bot add` and `bot dummy` accept up to 17.
- Bot names are stable: the same number of bots keeps the same names, and names are not redrawn on every lobby change.

## Selective weapon loading
- Auto = on for the Low memory tier: only the weapons in use are loaded.
- Limitations: a class change mid-match to a weapon that is not loaded, and weapons placed on the map, are refused with a "preparing" log line until the next match.
- Options > Selective weapon loading > Off restores full loading (`IW4L_SELECTIVE_WEAPONS=off`).

## Text/UI sharpness
- Auto is Medium: menus, HUD and text are drawn at half resolution or better while the 3D scene stays at your render scale. It only does something when the 3D scale is below 0.6; at 0.6 or more nothing changes.
- Native and High are sharper but use more memory. Off restores the old behaviour (everything at the 3D scale).
- Experimental: if the screen goes black, set it to Off.
- Font atlases are exempt from the texture cap.
- The new settings are rows in the IW4 Pocket options menu (now 8 + 8 rows over two pages).

## Diagnostics
- Diagnostic detail Normal/Detailed (Options > Diagnostics, `IW4L_DIAG`).
- A memory reading of 0 MiB is now a valid sample. Before, "iOS still allows 0 MiB" was thrown away, so the session summary could hide the moment with the least headroom. Only a missing reading counts as unknown (`?`).
- The note about the previous run is neutral. A log with no exit line is reported as "no exit line (closed by you, by iOS, or a crash; cause not proven)" with the last memory sample, instead of saying iOS closed the app.

## Version
- Label **0.3.8**; Cargo `0.3.8`; bundle build **22**.

Verification scope: Python source-contract checks run on the server. The Rust crates (`diag`, `console`, `bots`, `launcher`) are compiled only by the iOS CI. Nothing here has been tried on a device.
