# Build Windows x64 della 1.4.4.25

La compilazione usa i sorgenti di questo ramo, Rust 1.75.0, Flutter 3.24.5 e vcpkg alla revisione fissata in `source-lock.json`. Richiede Visual Studio 2022 C++ Build Tools e Windows SDK (linker, resource compiler). Il workflow `Tec Assist Windows` usa un runner Windows 2022 pubblico GitHub, senza segreti e senza pubblicazione automatica sul sito o nelle release.

## Comandi

Da una PowerShell 7 con gli strumenti C++ disponibili:

```powershell
./tecassist/setup.ps1
./tecassist/build.ps1
```

`setup.ps1` prepara le dipendenze nella cartella `tecassist-deps` e imposta l'ambiente del processo. Se lo si esegue separatamente, importarlo con `. ./tecassist/setup.ps1` nella stessa sessione del comando di build. Sul runner il workflow esegue entrambi nella stessa sessione.

Per i controlli di sorgente e le policy Rust, senza creare i pacchetti:

```powershell
./tecassist/build.ps1 -Phase Checks
```

I sorgenti canonici nella radice corrispondono al motore condiviso e alla Console. Le due varianti Flutter sono in `tecassist/overlays`. La build crea una copia UI isolata in `tecassist-ui`, compila entrambe e salva i risultati in `tecassist-out`; non applica l'overlay al repository originale sul PC di sviluppo.

I tre componenti Tec Assist e i launcher ricevono metadati di versione. Gli artefatti comprendono bundle, due eseguibili, log, hash SHA-256 e risultati degli autotest. Il workflow conserva gli artefatti della singola esecuzione e non li presenta come firmati.

## Provenienza delle dipendenze

L'engine Flutter personalizzato viene scaricato dalla release pubblica RustDesk, con SHA-256 fissato e controllo prima dell'uso. Quel digest proviene dal download ufficiale verificato nell'ambiente di sviluppo, non da un digest separato firmato dal produttore. Se la release mobile `main` cambia, la build si interrompe; occorre rivedere esplicitamente la revisione/digest, non eliminare il controllo.

`Cargo.lock` e `flutter/pubspec.lock` sono conservati. Il bridge generato è incluso nel controllo versione; per rigenerarlo usare `flutter_rust_bridge_codegen 1.80.1 --features uuid` con input `src/flutter_ffi.rs`, output Dart `flutter/lib/generated_bridge.dart` e header `flutter/macos/Runner/bridge_generated.h`. La rigenerazione richiede revisione e nuova compilazione prima del commit.

## Limiti

La prima esecuzione pubblica deve dimostrare che le dipendenze e la procedura funzionano su un runner pulito. Non è ancora dichiarata riproducibilità bit per bit. Autotest locali e CI non eseguono collegamenti remoti, non approvano consensi sul PC cliente e non certificano un collaudo reale del riavvio tra due PC. La versione 1.4.4.24 rimane conservata fuori da questa copia di lavoro.
