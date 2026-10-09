<table align="center">
<tr>
<td align="center" width="50%">
<a href="https://github.com/MarkusSela/IW4-Pocket/blob/main/media/demo.mp4"><img src="media/demo.gif" alt="IW4 Pocket demo"></a>
<br><sub>▶️ <a href="https://github.com/MarkusSela/IW4-Pocket/blob/main/media/demo.mp4">Guarda la demo completa</a> · 18 s, iPhone 13 Pro Max</sub>
</td>
<td align="center" width="50%">
<a href="https://github.com/MarkusSela/IW4-Pocket/blob/main/media/treuenten_tes-0.2.1.mov"><img src="media/treuenten.gif" alt="IW4 Pocket match on iPhone 17 Pro Max"></a>
<br><sub>▶️ <a href="https://github.com/MarkusSela/IW4-Pocket/blob/main/media/treuenten_tes-0.2.1.mov">Guarda la demo completa</a> · una partita su iPhone 17 Pro Max, di treuenten</sub>
</td>
</tr>
</table>

<p align="center">
  <img src="media/icon.png" width="110" alt="IW4 Pocket icon">
</p>

<h1 align="center">IW4 Pocket</h1>

<p align="center">Modern Warfare 2 (2009) sul tuo iPhone, con il motore open source IW4L. Non ufficiale e sperimentale.</p>

<p align="center"><a href="https://ko-fi.com/marukoshi"><img src="https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi"></a></p>

<p align="center">🌍 <a href="README.md">English</a> · <b>Italiano</b> · <a href="README.es.md">Español</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a></p>

## 🎯 Cos'è?

Un port su iOS del motore open source [IW4L](https://github.com/vladtrc/iw4L). **Non contiene il gioco**: legge i file del gioco **che già possiedi** su PC. È un nuovo lettore per file che hai già.

## 📊 A che punto siamo

- ✅ Gira nativamente su iPhone (Rust + Bevy + Metal)
- ✅ Arriva al menu principale. Il tocco funziona come clic e il controller PS4 funziona
- ✅ Su un iPhone 17 Pro Max (6 GB concessi all'app) girano partite complete con i bot: [guarda il video](https://jumpshare.com/s/rFCD5I9ZtWvcN3gcBO8e), di [treuenten](https://github.com/treuenten)
- ✅ L'app sceglie da sola le impostazioni di memoria in base alla memoria che iOS concede al telefono (basso / medio / alto)
- ⚠️ Su un iPhone 13 Pro Max (circa 3 GB concessi), `mp_rust` ora carica fino agli shader, poi l'app si chiude da sola. In analisi

## 🙋 Provalo, passo per passo

Non serve saper programmare. Ci vogliono circa 15 minuti più la copia dei file.

1. **Procurati i file del gioco.** Serve la tua copia di *Modern Warfare 2 (2009)* per **PC** (versione Steam). La cartella deve contenere una sottocartella chiamata `zone`. Questo progetto non fornisce né linka file del gioco
2. **Installa SideStore** sul tuo iPhone seguendo la [guida ufficiale](https://sidestore.io). È lo strumento che installa app come questa
3. **Scarica l'app:** [IW4.Pocket.ipa](https://github.com/MarkusSela/IW4-Pocket/releases/latest/download/IW4.Pocket.ipa) (o apri la [pagina della release](https://github.com/MarkusSela/IW4-Pocket/releases/latest))
4. **Installala con SideStore** (tocca **+**, scegli il file). Se hai già una versione precedente, installa **sopra**: disinstallare cancella i file di gioco
5. **Apri l'app una volta**, poi chiudila. Così crea la sua cartella
6. **Copia la cartella di MW2** nell'app File: *Su iPhone > IW4 Pocket > Games*. Va bene qualsiasi nome, purché contenga `zone`
7. **Collega un controller** nelle impostazioni Bluetooth (PS4/DualShock 4 verificato), poi apri l'app. Il primo caricamento è lento e lo schermo può restare rosa per un po'

## 📱 Funzionerà sul mio dispositivo?

- ✅ **Testato:** iPhone 13 Pro Max (6 GB di RAM, ~3 GB concessi), iOS 27: i menu funzionano, il caricamento della mappa arriva fino agli shader. iPhone 17 Pro Max (6 GB concessi): partite complete con i bot, segnalate da treuenten
- ❓ **Tutto il resto non è verificato.** Tecnicamente dovrebbe girare su qualsiasi iPhone o iPad con Metal; l'app dichiara iOS 15 come minimo, ma non l'ho mai provata lì
- 🧠 **Il limite è la memoria.** Sul telefono testato iOS concede all'app circa 3 GB. I dispositivi con meno RAM falliranno prima; i più interessanti da provare sono iPhone da 8 GB+ e iPad con chip M
- 🎮 **Per giocare serve un controller.** Il tocco funziona solo nei menu

## 🐞 Aiutami a testare

L'hai provato? [Apri una segnalazione](https://github.com/MarkusSela/IW4-Pocket/issues/new) con:

- dispositivo e versione di iOS
- cosa è successo (menu? mappa caricata? app chiusa?)
- il file `iw4l-boot.log` in *Su iPhone > IW4 Pocket*. Registra avvio, memoria e crash

## 🔧 Qualcosa non va?

- **"MW2 Multiplayer was not found"**: la cartella deve stare dentro `Games` e contenere `zone`
- **Schermo rosa a lungo**: aspetta, il primo caricamento è lento
- **Controller non rilevato**: collegalo prima nel Bluetooth, poi riapri l'app
- **L'app si chiude su una mappa**: è il problema di memoria noto. Mandami il log
- **Per limitare le texture**: aggiungi un file di testo `iw4l-texture-cap.txt` con solo un numero, ad esempio `256`, poi riavvia del tutto l'app
- **Avanzate:** un file di testo `iw4l-env.txt` con righe `IW4L_NOME=valore` imposta gli interruttori del motore senza ricompilare. I nomi devono iniziare con `IW4L_`

## 🛠️ Compilarlo da solo

Avvia il workflow **ios-release** dalla scheda Actions (runner macOS) e indica un tag.

## ⚖️ Crediti e note legali

Port di [IW4L](https://github.com/vladtrc/iw4L) di vladtrc e collaboratori (Apache-2.0, vedi `LICENSE`, `NOTICE`; README originale in `README.upstream.md`). Correzioni di memoria, texture BC, gestione dei frame e correzioni al motore: [treuenten](https://github.com/treuenten) ([issue #1](https://github.com/MarkusSela/IW4-Pocket/issues/1)).

> Progetto non ufficiale, non affiliato ad Activision, Infinity Ward, Apple né agli autori di IW4L. Call of Duty e Modern Warfare sono marchi dei rispettivi proprietari. Serve una copia legittima del gioco.

---

<p align="center">☕ Ti piace? Sostieni il progetto su <a href="https://ko-fi.com/marukoshi">Ko-fi</a></p>
