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

<p align="center">Modern Warfare 2 (2009) en tu iPhone, con el motor de código abierto IW4L. No oficial y experimental.</p>

<p align="center"><a href="https://ko-fi.com/marukoshi"><img src="https://img.shields.io/badge/Ko--fi-Support-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi"></a></p>

<p align="center">🌍 <a href="README.md">English</a> · <a href="README.it.md">Italiano</a> · <b>Español</b> · <a href="README.fr.md">Français</a> · <a href="README.de.md">Deutsch</a></p>

<p align="center"><a href="https://markussela.github.io/IW4-Pocket/sidestore/"><img src="media/add-to-sidestore.png" width="230" alt="Add to AltStore or SideStore"></a></p>

## 🎯 ¿Qué es?

Un port a iOS del motor de código abierto [IW4L](https://github.com/vladtrc/iw4L). **No contiene el juego**: lee los archivos del juego **que ya tienes** en PC. Es un nuevo lector para archivos que ya posees.

## 📊 Cómo va

- ✅ Se ejecuta de forma nativa en iPhone (Rust + Bevy + Metal)
- ✅ Llega al menú principal. El toque funciona como clic y un mando PS4 funciona
- ✅ En un iPhone 17 Pro Max (6 GB concedidos a la app) se juegan partidas completas con bots: [mira el clip](https://jumpshare.com/s/rFCD5I9ZtWvcN3gcBO8e), de [treuenten](https://github.com/treuenten)
- ✅ La app elige sola su configuración de memoria según la memoria que iOS concede al teléfono (baja / media / alta). Todo se puede cambiar en el juego en **Opciones > IW4 Pocket options**, y **Send logs** abre un informe ya rellenado
- ⚠️ En un iPhone 13 Pro Max (unos 3 GB concedidos) la 0.3.4 carga `mp_rust` por completo y se jugó una partida de aproximadamente un minuto; después iOS cerró la app por falta de memoria. La memoria crece despacio durante la partida; la próxima versión medirá dónde. **Si tu teléfono concede unos 3 GB a la app, pon Render scale en 0.67 o 0.5** (Opciones > IW4 Pocket options): libera memoria y la imagen solo se ve un poco más suave.

## 🙋 Pruébalo, paso a paso

No hace falta saber programar. Lleva unos 15 minutos más la copia de archivos.

1. **Consigue los archivos del juego.** Necesitas tu propia copia de *Modern Warfare 2 (2009)* para **PC** (versión de Steam). La carpeta debe contener una subcarpeta llamada `zone`. Este proyecto no proporciona ni enlaza archivos del juego
2. **Instala SideStore** en tu iPhone con la [guía oficial](https://sidestore.io). Es la herramienta que instala apps como esta
3. **Descarga la app:** [IW4.Pocket.ipa](https://github.com/MarkusSela/IW4-Pocket/releases/latest/download/IW4.Pocket.ipa) (o abre la [página de la release](https://github.com/MarkusSela/IW4-Pocket/releases/latest))
4. **Instálala con SideStore** (toca **+**, elige el archivo). Si ya tienes una versión anterior, instala **encima**: desinstalar borra tus archivos del juego
5. **Abre la app una vez** y ciérrala. Así crea su carpeta
6. **Copia tu carpeta de MW2** en la app Archivos: *En mi iPhone > IW4 Pocket > Games*. Vale cualquier nombre, si contiene `zone`
7. **Empareja un mando** en los ajustes de Bluetooth (PS4/DualShock 4 verificado) y abre la app. La primera carga es lenta y la pantalla puede quedarse rosa un rato

### Estable o TEST en SideStore

Añade la fuente con el botón de arriba y actualízala en SideStore. Es **una sola app**, no dos instalaciones estable/test. La fuente lista **todas las versiones**, estables y TEST, de la más reciente a la más antigua; elige la que quieras en SideStore. Las TEST llevan **TEST** en la descripción. No hace falta ningún ajuste beta. La entrada más reciente puede ser una TEST: es experimental y no se ha confirmado que el fallo de memoria esté resuelto.

La interfaz muestra la versión y el build de la app, por ejemplo `0.3.9 (23)`. **No desinstales**: actualiza sobre la app existente para conservar los archivos del juego. Estos pasos no se han verificado en un teléfono.

## 📱 ¿Funcionará en mi dispositivo?

- ✅ **Probado (0.3.9):** iPhone 13 Pro Max (~3 GB concedidos), iOS 27: 17 bots, una partida de unos 9 minutos y luego iOS cerró la app con pocos MiB libres. iPad Pro/Air M1 (5,1 GB concedidos): 17 bots, 4-5 partidas seguidas en el mismo mapa, 35-48 FPS. iPhone 17 Pro Max (6 GB concedidos): partidas completas con bots, según treuenten
- ❓ **Todo lo demás no está verificado.** Técnicamente debería funcionar en cualquier iPhone o iPad con Metal; la app declara iOS 15 como mínimo, pero nunca la probé ahí
- 🧠 **La memoria es el límite.** La app no devuelve memoria entre partidas. En un teléfono con unos 3 GB una partida larga ya puede cerrarla; en el iPad M1 (5,1 GB) se acabó en la 4.ª-5.ª partida, al cargar un mapa más grande (Afghan). Cierra la app entre sesiones. Con unos 3 GB prueba Render scale de 0.3 a 0.5 y pocos bots. Arreglarlo es el primer objetivo de 0.4.0.
- 🎮 **Se necesita un mando para jugar.** El toque solo funciona en los menús

## 🐞 Ayúdame a probar

¿Lo probaste? [Abre un issue](https://github.com/MarkusSela/IW4-Pocket/issues/new) con:

- tu dispositivo y versión de iOS
- qué pasó (¿menú? ¿mapa cargado? ¿app cerrada?)
- el archivo `iw4l-boot.log` en *En mi iPhone > IW4 Pocket*. Registra el inicio, la memoria y los fallos

## 🔧 ¿Algo falla?

- **La forma más fácil de informar de un problema:** en el juego abre *Opciones > IW4 Pocket options > Send logs*. Abre GitHub con el resumen ya escrito; adjunta `iw4l-boot.log` y `iw4l-memory-settings.txt`
- **"MW2 Multiplayer was not found"**: la carpeta debe estar dentro de `Games` y contener `zone`
- **Pantalla rosa mucho rato**: espera, la primera carga es lenta
- **Mando no detectado**: empárejalo primero en Bluetooth y reabre la app
- **La app se cierra en un mapa**: es el problema de memoria conocido. Envía el registro
- **Para limitar texturas**: añade un archivo de texto `iw4l-texture-cap.txt` con solo un número, por ejemplo `256`, y reinicia la app por completo
- **Avanzado:** un archivo de texto `iw4l-env.txt` con líneas `IW4L_NOMBRE=valor` ajusta los interruptores del motor sin recompilar. Solo se aceptan los ajustes del menú IW4 Pocket (por ejemplo `IW4L_RENDER_SCALE=0.67`), más `IW4L_BOTS=5` (bots al empezar la partida), `IW4L_SKIP_CAMO=1` (sin camuflajes de armas) y `IW4L_CMDS=map mp_rust; bot add 5` (solo `map`, `bot` y reglas de partida o de bots). Lo demás se ignora y el registro indica la línea

## 🛠️ Compilarlo tú mismo

Ejecuta el workflow **ios-release** desde la pestaña Actions (runner macOS) e indica una etiqueta.

## ⚖️ Créditos y aviso legal

Port de [IW4L](https://github.com/vladtrc/iw4L) de vladtrc y colaboradores (Apache-2.0, ver `LICENSE`, `NOTICE`; README original en `README.upstream.md`). Correcciones de memoria, lean shaders, texturas BC, pausa a 60 Hz, render scale, gestión de fotogramas y correcciones del motor: [treuenten](https://github.com/treuenten) ([issue #1](https://github.com/MarkusSela/IW4-Pocket/issues/1)). Bots: [LemonMoo](https://github.com/LemonMoo) y [AdrianTomasDiez](https://github.com/AdrianTomasDiez). Idea de los camuflajes: [mk2rs2000](https://github.com/mk2rs2000).

> Proyecto no oficial, sin afiliación con Activision, Infinity Ward, Apple ni los autores de IW4L. Call of Duty y Modern Warfare son marcas de sus propietarios. Necesitas una copia legítima del juego.

---

<p align="center">☕ ¿Te gusta? Apoya el proyecto en <a href="https://ko-fi.com/marukoshi">Ko-fi</a></p>
