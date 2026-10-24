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

<p align="center">Modern Warfare 2 (2009) auf deinem iPhone, mit der Open-Source-Engine IW4L. Inoffiziell und experimentell.</p>

<p align="center"><a href="https://ko-fi.com/marukoshi"><img src="https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi"></a></p>

<p align="center">🌍 <a href="README.md">English</a> · <a href="README.it.md">Italiano</a> · <a href="README.es.md">Español</a> · <a href="README.fr.md">Français</a> · <b>Deutsch</b></p>

<p align="center"><a href="https://markussela.github.io/IW4-Pocket/sidestore/"><img src="media/add-to-sidestore.png" width="230" alt="Add to AltStore or SideStore"></a></p>

## 🎯 Was ist das?

Ein iOS-Port der Open-Source-Engine [IW4L](https://github.com/vladtrc/iw4L). Er **enthält das Spiel nicht**: er liest die Spieldateien, **die du bereits besitzt**, vom PC. Ein neuer Player für Dateien, die du schon hast.

## 📊 Aktueller Stand

- ✅ Läuft nativ auf dem iPhone (Rust + Bevy + Metal)
- ✅ Erreicht das Hauptmenü. Touch funktioniert als Klick und ein PS4-Controller funktioniert
- ✅ Auf einem iPhone 17 Pro Max (6 GB für die App) laufen komplette Matches mit Bots: [Clip ansehen](https://jumpshare.com/s/rFCD5I9ZtWvcN3gcBO8e), von [treuenten](https://github.com/treuenten)
- ✅ Die App wählt ihre Speichereinstellungen selbst anhand des Speichers, den iOS dem Telefon gewährt (niedrig / mittel / hoch). Alles lässt sich im Spiel unter **Optionen > IW4 Pocket options** ändern, und **Send logs** öffnet einen vorausgefüllten Fehlerbericht
- ⚠️ Auf einem iPhone 13 Pro Max (etwa 3 GB gewährt) lädt 0.3.4 `mp_rust` vollständig und ein Match lief etwa eine Minute, dann beendete iOS die App wegen Speichermangels. Der Speicher wächst während des Matches langsam; die nächste Version misst, wo. **Wenn dein Telefon der App etwa 3 GB gibt, stelle Render scale auf 0.67 oder 0.5** (Optionen > IW4 Pocket options): das gibt Speicher frei und das Bild wird nur etwas weicher.

## 🙋 Probier es aus, Schritt für Schritt

Keine Programmierkenntnisse nötig. Etwa 15 Minuten plus das Kopieren der Dateien.

1. **Besorge die Spieldateien.** Du brauchst deine eigene Kopie von *Modern Warfare 2 (2009)* für den **PC** (Steam-Version). Der Ordner muss einen Unterordner namens `zone` enthalten. Dieses Projekt liefert oder verlinkt keine Spieldateien
2. **Installiere SideStore** auf deinem iPhone mit der [offiziellen Anleitung](https://sidestore.io). Es ist das Werkzeug, das Apps wie diese installiert
3. **Lade die App herunter:** [IW4.Pocket.ipa](https://github.com/MarkusSela/IW4-Pocket/releases/latest/download/IW4.Pocket.ipa) (oder öffne die [Release-Seite](https://github.com/MarkusSela/IW4-Pocket/releases/latest))
4. **Installiere sie mit SideStore** (tippe auf **+**, wähle die Datei). Hast du schon eine ältere Version, installiere **darüber**: Deinstallieren löscht deine Spieldateien
5. **Öffne die App einmal** und schließe sie wieder. So legt sie ihren Ordner an
6. **Kopiere deinen MW2-Ordner** in die Dateien-App: *Auf meinem iPhone > IW4 Pocket > Games*. Jeder Name geht, solange er `zone` enthält
7. **Koppele einen Controller** in den Bluetooth-Einstellungen (PS4/DualShock 4 verifiziert) und öffne die App. Das erste Laden ist langsam und der Bildschirm kann eine Weile rosa bleiben

### Stable oder TEST in SideStore

Füge die Quelle über den Button oben hinzu und aktualisiere sie in SideStore. Es ist **eine einzige App**, keine getrennten Stable-/Test-Installationen. Die Quelle listet **alle Veröffentlichungen**, Stable und TEST, die neueste zuerst; wähle in SideStore die gewünschte. TEST-Versionen sind in der Beschreibung mit **TEST** gekennzeichnet. Eine Beta-Einstellung ist nicht nötig. Der neueste Eintrag kann eine TEST sein: sie ist experimentell, und die Behebung des Speicherabsturzes ist nicht bestätigt.

Die Oberfläche zeigt Version und Build der App, zum Beispiel `0.3.9 (23)`. **Nicht deinstallieren**: über die vorhandene App installieren, damit die Spieldateien erhalten bleiben. Diese Schritte wurden nicht auf einem Telefon geprüft.

## 📱 Läuft es auf meinem Gerät?

- ✅ **Getestet (0.3.9):** iPhone 13 Pro Max (~3 GB gewährt), iOS 27: 17 Bots, ein Match von etwa 9 Minuten, dann hat iOS die App mit nur wenigen MiB Rest beendet. iPad Pro/Air M1 (5,1 GB gewährt): 17 Bots, 4-5 Matches hintereinander (das letzte auf Afghan endete mit geschlossener App), 35-48 FPS. iPhone 17 Pro Max (6 GB gewährt): komplette Matches mit Bots, laut treuenten
- ❓ **Alles andere ist nicht verifiziert.** Technisch sollte es auf jedem iPhone oder iPad mit Metal laufen; die App deklariert iOS 15 als Minimum, ich habe es dort aber nie ausprobiert
- 🧠 **Der Speicher ist die Grenze, und er hängt von der Karte ab.** Nach dem ersten Match bleibt etwa 1 GB belegt (es wächst mit weiteren Matches nicht). Große Karten (Afghan, Terminal) brauchen beim Laden etwa 1,7 GB mehr: auf dem M1-iPad (5,1 GB) blieben nur 40-290 MiB frei, selbst nach frischem Start. Auf einem Telefon mit etwa 3 GB kann schon ein langes Match auf einer kleinen Karte die App beenden. Bei etwa 3 GB probiere Render scale 0.3 bis 0.5 und wenige Bots. Das zu beheben ist das erste Ziel von 0.4.0.
- 🎮 **Zum Spielen braucht man einen Controller.** Touch funktioniert nur in den Menüs

## 🐞 Hilf mir beim Testen

Ausprobiert? [Eröffne ein Issue](https://github.com/MarkusSela/IW4-Pocket/issues/new) mit:

- deinem Gerät und der iOS-Version
- was passiert ist (Menü? Karte geladen? App geschlossen?)
- der Datei `iw4l-boot.log` in *Auf meinem iPhone > IW4 Pocket*. Sie protokolliert Start, Speicher und Abstürze

## 🔧 Probleme?

- **Am einfachsten ein Problem melden:** im Spiel *Optionen > IW4 Pocket options > Send logs* öffnen. GitHub öffnet sich mit der Zusammenfassung; hänge `iw4l-boot.log` und `iw4l-memory-settings.txt` an
- **"MW2 Multiplayer was not found"**: der Ordner muss in `Games` liegen und `zone` enthalten
- **Lange rosa Bildschirm**: warten, das erste Laden ist langsam
- **Controller nicht erkannt**: erst in Bluetooth koppeln, dann die App neu öffnen
- **App schließt sich auf einer Karte**: das ist das bekannte Speicherproblem. Sende das Protokoll
- **Texturen begrenzen**: lege eine Textdatei `iw4l-texture-cap.txt` mit nur einer Zahl an, z. B. `256`, und starte die App komplett neu
- **Fortgeschritten:** eine Textdatei `iw4l-env.txt` mit Zeilen `IW4L_NAME=Wert` setzt die Schalter der Engine ohne Neubau. Akzeptiert werden nur die Einstellungen des Menüs IW4 Pocket (zum Beispiel `IW4L_RENDER_SCALE=0.67`), dazu `IW4L_BOTS=5` (Bots zu Beginn des Matches), `IW4L_SKIP_CAMO=1` (ohne Waffen-Tarnmuster) und `IW4L_CMDS=map mp_rust; bot add 5` (nur `map`, `bot` und Match- oder Bot-Regeln). Alles andere wird ignoriert und das Log nennt die Zeile

## 🛠️ Selbst bauen

Starte den Workflow **ios-release** im Actions-Tab (macOS-Runner) und gib einen Tag an.

## ⚖️ Credits und Rechtliches

Port von [IW4L](https://github.com/vladtrc/iw4L) von vladtrc und Mitwirkenden (Apache-2.0, siehe `LICENSE`, `NOTICE`; Original-README in `README.upstream.md`). Speicherkorrekturen, Lean Shaders, BC-Texturen, 60-Hz-Pause, Render Scale, Frame-Timing und Engine-Fixes: [treuenten](https://github.com/treuenten) ([issue #1](https://github.com/MarkusSela/IW4-Pocket/issues/1)). Bots: [LemonMoo](https://github.com/LemonMoo) und [AdrianTomasDiez](https://github.com/AdrianTomasDiez). Idee der Tarnmuster: [mk2rs2000](https://github.com/mk2rs2000).

> Inoffizielles Fanprojekt, nicht verbunden mit Activision, Infinity Ward, Apple oder den IW4L-Autoren. Call of Duty und Modern Warfare sind Marken ihrer Inhaber. Du benötigst eine legitime Kopie des Spiels.

---

<p align="center">☕ Gefällt es dir? Unterstütze das Projekt auf <a href="https://ko-fi.com/marukoshi">Ko-fi</a></p>
