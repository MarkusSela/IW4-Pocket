# IW4 Pocket 0.4.0 — build 24

**Experimental prerelease (TEST). Not verified on a phone.** The stable release is still 0.3.9. Nothing below has been compiled or tried on a device before this build: it is the first CI build of this code. This build does not claim to fix the memory crash.

## Bots
- Names, clan tags and levels are drawn again by themselves at the end of every match. There is no button. Adding or removing a bot in a stopped lobby still keeps the names of the bots that stay.
- Clan tags appear more often: 9 in 10 of the names that have room for one get a tag (before: 7 in 10), which is about half of all names (before: about 4 in 10; long names have no room within the 15 characters). `[IRS]` is eight times as likely as any other tag.
- Every bot gets a random level (1 to 70, as in the original game) and sometimes a prestige. The private lobby list shows it after the name (`Name  Lv42`, `Name  P3 Lv70`), and the match uses the same name and level. The host's choice is kept even if the game's own scripts try to set the rank when the bot joins. Whether the scoreboard shows it is not verified on a device.
- Bots no longer all pick the same goal: a bot avoids a roam spot, objective or firing position that another bot already walks to. Guards near an objective walk to their own slot instead of only turning their head. Without navigation data a blocked bot tries to step around the obstacle before it stops.
- One line per bot every 10 s (`bots: snapshot id=… pos=… task=… goal=… path=…`) in the log, so a "bots hide" report can be read from `iw4l-boot.log`.

## Menus of the original game
- At launch the log lists the name of every menu in the game files (`menu inventory: …`) and the entries of the original main menu with their actions. Nothing is shown differently yet: this is what is needed to bring the original multiplayer menus out in the next build. Please send `iw4l-boot.log` from this build.

## Name and class names
- While a menu text field is active the iOS keyboard is shown, so the player name (Options > Multiplayer) and the class names (Create a class > Rename) can be typed. Tapping elsewhere closes the field. Not tried on a device.

## Memory (diagnostics first)
- The log now says how many bytes the heap release really returned and the footprint before and after (the old line rounded to 0 MiB). The "under pressure" line is limited to one every 10 s. The end of a map load has one line with footprint, heap and the iOS allowance together.
- The RGBA expansion of textures reserves its exact size, and the copy of the scene made for the text/3D split is freed when the split is off. Both are small; the gain is not measured.
- Not changed: the 995 MiB of kept models and the map models. They need a device to test and are the next step.

## Stability
- A second human can no longer be given an id above 17 (the session is refused as full instead of 19 occupants).
- A bot that held the local player's id is now properly removed (leave and profile unbind), not only dropped from the list.
- `bot add` and `bot dummy` handle a too-big number the same way and say it was clamped.
- CI uses the build number of `ios/Info.plist` for the binary (it was always 1).

## Evidence for later fixes (no behaviour change)
- Helicopter: logs of spawn, gunner fire and the `fireweapon` result, to find out why the Chopper does not hit.
- If you test the Chopper, send `iw4l-boot.log` with lines starting with `heli_`.

## Known, not done in this build
- Campaign and Spec Ops: the runtime only has multiplayer modes and map lists (`mp_` zones only); they are not offered, not shown as broken buttons.
- Home-screen widgets (they need an extension that SideStore with a free Apple ID may not sign).
- Level 70 at the start of a profile: it comes from the game's own files; this build only adds what is needed to find it.
- Weapon accuracy: no bug was found in the code, so nothing was changed.
