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

<p align="center">Modern Warfare 2 (2009) sul tuo iPhone, con il motore open source IW4L. Non ufficiale e sperimentale.</p>

<p align="center"><a href="https://ko-fi.com/marukoshi"><img src="https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi"></a></p>

<p align="center">🌍 <a href="README.md">English</a> · <b>Italiano</b> · <a href="README.es.md">Español</a> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a></p>

<p align="center"><a href="https://markussela.github.io/IW4-Pocket/sidestore/"><img src="media/add-to-sidestore.png" width="230" alt="Add to AltStore or SideStore"></a></p>

## 🎯 Cos'è?

Un port su iOS del motore open source [IW4L](https://github.com/vladtrc/iw4L). **Non contiene il gioco**: legge i file del gioco **che già possiedi** su PC. È un nuovo lettore per file che hai già.

## 📊 A che punto siamo

- ✅ Gira nativamente su iPhone (Rust + Bevy + Metal)
- ✅ Arriva al menu principale. Il tocco funziona come clic e il controller PS4 funziona
- ✅ Su un iPhone 17 Pro Max (6 GB concessi all'app) girano partite complete con i bot: [guarda il video](https://jumpshare.com/s/rFCD5I9ZtWvcN3gcBO8e), di [treuenten](https://github.com/treuenten)
- ✅ L'app sceglie da sola le impostazioni di memoria in base alla memoria che iOS concede al telefono (basso / medio / alto). Si può cambiare tutto nel gioco in **Opzioni > IW4 Pocket options**, e **Send logs** apre una segnalazione già compilata
- ⚠️ Su un iPhone 13 Pro Max (circa 3 GB concessi) la 0.3.4 carica `mp_rust` per intero e una partita è durata circa un minuto, poi iOS ha chiuso l'app per mancanza di memoria. La memoria cresce lentamente durante la partita; la prossima build misura dove. **Se il tuo telefono concede circa 3 GB all'app, imposta Render scale a 0.67 o 0.5** (Opzioni > IW4 Pocket options): libera memoria e l'immagine è solo un po' più morbida.

## 🙋 Provalo, passo per passo

Non serve saper programmare. Ci vogliono circa 15 minuti più la copia dei file.

1. **Procurati i file del gioco.** Serve la tua copia di *Modern Warfare 2 (2009)* per **PC** (versione Steam). La cartella deve contenere una sottocartella chiamata `zone`. Questo progetto non fornisce né linka file del gioco
2. **Installa SideStore** sul tuo iPhone seguendo la [guida ufficiale](https://sidestore.io). È lo strumento che installa app come questa
3. **Scarica l'app:** [IW4.Pocket.ipa](https://github.com/MarkusSela/IW4-Pocket/releases/latest/download/IW4.Pocket.ipa) (o apri la [pagina della release](https://github.com/MarkusSela/IW4-Pocket/releases/latest))
4. **Installala con SideStore** (tocca **+**, scegli il file). Se hai già una versione precedente, installa **sopra**: disinstallare cancella i file di gioco
5. **Apri l'app una volta**, poi chiudila. Così crea la sua cartella
6. **Copia la cartella di MW2** nell'app File: *Su iPhone > IW4 Pocket > Games*. Va bene qualsiasi nome, purché contenga `zone`
7. **Collega un controller** nelle impostazioni Bluetooth (PS4/DualShock 4 verificato), poi apri l'app. Il primo caricamento è lento e lo schermo può restare rosa per un po'

### Stabile o TEST in SideStore

Aggiungi la sorgente con il pulsante sopra, poi aggiornala in SideStore. È **un'unica app**, non due installazioni stabile/test. La sorgente elenca **tutte le uscite**, stabili e TEST, dalla più recente; scegli quella che vuoi in SideStore. Le TEST sono indicate con **TEST** nella descrizione. Non serve nessuna impostazione beta. La voce più recente può essere una TEST: è sperimentale e il crash di memoria non è confermato risolto.

L'interfaccia mostra versione e build dell'app, per esempio `0.3.9 (23)`. **Non disinstallare**: aggiorna sopra l'app esistente per conservare i file di gioco. Questi passaggi non sono stati verificati su un telefono.

## 📱 Funzionerà sul mio dispositivo?

- ✅ **Testato (0.3.9):** iPhone 13 Pro Max (~3 GB concessi), iOS 27: 17 bot, una partita di circa 9 minuti, poi iOS ha chiuso l'app con pochi MiB liberi. iPad Pro/Air M1 (5,1 GB concessi): 17 bot, 4-5 partite di fila (l'ultima su Afghan è finita con l'app chiusa), 35-48 FPS. iPhone 17 Pro Max (6 GB concessi): partite complete con i bot, segnalate da treuenten
- ❓ **Tutto il resto non è verificato.** Tecnicamente dovrebbe girare su qualsiasi iPhone o iPad con Metal; l'app dichiara iOS 15 come minimo, ma non l'ho mai provata lì
- 🧠 **Il limite è la memoria, e dipende dalla mappa.** Dopo la prima partita circa 1 GB resta occupato (non cresce con le partite successive). Le mappe grandi (Afghan, Terminal) ne richiedono circa 1,7 GB in più al caricamento: sull'iPad M1 (5,1 GB) lasciavano liberi solo 40-290 MiB, anche da app appena avviata. Su un telefono con circa 3 GB una partita lunga su una mappa piccola può già chiudere l'app. Con circa 3 GB prova Render scale da 0.3 a 0.5 e pochi bot. Risolverlo è il primo obiettivo della 0.4.0.
- 🎮 **Per giocare serve un controller.** Il tocco funziona solo nei menu

## 🐞 Aiutami a testare

L'hai provato? [Apri una segnalazione](https://github.com/MarkusSela/IW4-Pocket/issues/new) con:

- dispositivo e versione di iOS
- cosa è successo (menu? mappa caricata? app chiusa?)
- il file `iw4l-boot.log` in *Su iPhone > IW4 Pocket*. Registra avvio, memoria e crash

## 🔧 Qualcosa non va?

- **Modo più semplice per segnalare un problema:** nel gioco apri *Opzioni > IW4 Pocket options > Send logs*. Si apre GitHub con il riassunto già compilato; allega `iw4l-boot.log` e `iw4l-memory-settings.txt`
- **"MW2 Multiplayer was not found"**: la cartella deve stare dentro `Games` e contenere `zone`
- **Schermo rosa a lungo**: aspetta, il primo caricamento è lento
- **Controller non rilevato**: collegalo prima nel Bluetooth, poi riapri l'app
- **L'app si chiude su una mappa**: è il problema di memoria noto. Mandami il log
- **Per limitare le texture**: aggiungi un file di testo `iw4l-texture-cap.txt` con solo un numero, ad esempio `256`, poi riavvia del tutto l'app
- **Avanzate:** un file di testo `iw4l-env.txt` con righe `IW4L_NOME=valore` imposta gli interruttori del motore senza ricompilare. Sono accettate solo le impostazioni del menu IW4 Pocket (ad esempio `IW4L_RENDER_SCALE=0.67`), più `IW4L_BOTS=5` (bot all'inizio della partita), `IW4L_SKIP_CAMO=1` (senza mimetiche delle armi) e `IW4L_CMDS=map mp_rust; bot add 5` (solo `map`, `bot` e regole di partita o dei bot). Il resto viene ignorato e il log indica la riga

## 🛠️ Compilarlo da solo

Avvia il workflow **ios-release** dalla scheda Actions (runner macOS) e indica un tag.

## ⚖️ Crediti e note legali

Port di [IW4L](https://github.com/vladtrc/iw4L) di vladtrc e collaboratori (Apache-2.0, vedi `LICENSE`, `NOTICE`; README originale in `README.upstream.md`). Correzioni di memoria, lean shaders, texture BC, pausa a 60 Hz, render scale, gestione dei frame e correzioni al motore: [treuenten](https://github.com/treuenten) ([issue #1](https://github.com/MarkusSela/IW4-Pocket/issues/1)). Bot: [LemonMoo](https://github.com/LemonMoo) e [AdrianTomasDiez](https://github.com/AdrianTomasDiez). Idea delle mimetiche: [mk2rs2000](https://github.com/mk2rs2000).

> Progetto non ufficiale, non affiliato ad Activision, Infinity Ward, Apple né agli autori di IW4L. Call of Duty e Modern Warfare sono marchi dei rispettivi proprietari. Serve una copia legittima del gioco.

---

<p align="center">☕ Ti piace? Sostieni il progetto su <a href="https://ko-fi.com/marukoshi">Ko-fi</a></p>
