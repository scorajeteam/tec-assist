# Tec Assist

Personalizzazione open source di [RustDesk](https://github.com/rustdesk/rustdesk), con Console per il tecnico e Standalone Windows per il cliente. Sito: https://www.scorajeteam.com/tec-assist/

Questo ramo prepara la versione **1.4.4.25 TEST** per una build pubblica verificabile. La prima compilazione GitHub deve ancora essere verificata. Non dichiara una release firmata né l'ammissione a SignPath Foundation.

- Consenso locale e password temporanea per l'assistenza.
- File senza una seconda accettazione soltanto con consenso iniziale specifico, ticket tecnico/sessione, revoca e massimo 30 minuti dal consenso iniziale, anche dopo il riavvio.
- Password visualizzata a 24 px in grassetto.
- Ripresa dopo riavvio con autorizzazione temporanea, senza password permanente né accesso automatico a Windows.

La versione 25 ha superato controlli locali di compilazione e policy; l'utente ha riferito un trasferimento riuscito dopo riavvio su due PC. Questo riscontro non certifica tutte le combinazioni di rete, la revoca o la scadenza in un collaudo remoto completo.

## Build e licenza

Vedere [istruzioni di compilazione](BUILD.md), [privacy](PRIVACY.md) e [Code signing policy](SIGNING-POLICY.md) nella cartella `tecassist`. Dalla radice del repository i percorsi sono `tecassist/BUILD.md`, `tecassist/PRIVACY.md` e `tecassist/SIGNING-POLICY.md`.

Licenza del codice derivato: **AGPL-3.0**, con avvisi upstream conservati; vedere `LICENCE`. La titolarità dei componenti di terzi resta ai rispettivi autori. Questa distribuzione non dichiara approvazione o sponsorizzazione da RustDesk.

I file generati del bridge sono inclusi come sorgente e provengono da `flutter_rust_bridge_codegen 1.80.1`. Le revisioni di base sono documentate in `tecassist/source-lock.json`. La libreria `libs/hbb_common` è inclusa con le modifiche Tec Assist e la provenienza upstream; non è richiesta una modifica del repository originale.

## Stato della firma

Il progetto sta preparando una candidatura al programma gratuito SignPath Foundation. Non è ancora ammesso e gli eseguibili esistenti sono **non firmati**. Gli avvisi del browser non devono essere aggirati disattivando le protezioni.
