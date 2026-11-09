# Code signing policy

## Stato attuale

Tec Assist sta preparando la candidatura al programma gratuito SignPath Foundation. Non è attualmente ammesso, non dispone ancora di una firma SignPath e non dichiara certificati o garanzie che non ha ottenuto. Questa pagina va aggiornata dopo una decisione del servizio, prima di distribuire pacchetti presentati come firmati.

## Responsabilità proposte

- Manutentore, revisore e approvatore del rilascio: account GitHub `scorajeteam`.
- Ogni nuova persona, ruolo o autorizzazione di firma deve essere dichiarata e verificata.
- Autenticazione a più fattori richiesta per gli account del repository e del servizio di firma. L'utente ha riferito di averla attivata su GitHub; non sono conservati qui codici o credenziali.

## Procedura prevista

Solo build create dal codice sorgente pubblico di questo repository, con revisione della provenienza e dei test, possono essere candidate alla firma. Il percorso di firma richiederà approvazione manuale per ogni rilascio. La build CI corrente crea soltanto artefatti **non firmati**, senza pubblicazione automatica.

I componenti di terzi mantengono la propria provenienza e le rispettive firme: non vengono presentati come codice originale del progetto. Le modifiche derivate da RustDesk restano identificabili come fork con la licenza e gli avvisi upstream.

Firma e controllo antivirus sono distinti. Le segnalazioni devono essere investigate sul file effettivamente candidato, senza presumere che la firma le risolva. I controlli di consenso, scadenza e revoca non devono essere disabilitati per ottenere una firma o semplificare il download.

## Privacy

Vedere [privacy e comunicazioni di rete](PRIVACY.md). Non affermiamo che il programma non effettui comunicazioni automatiche: lo Standalone contatta un server ID/relay preconfigurato.
