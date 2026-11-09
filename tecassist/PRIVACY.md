# Privacy e comunicazioni di rete di Tec Assist

Tec Assist è un programma di assistenza remota basato su RustDesk. All'avvio può registrarsi e comunicare con il server ID/relay preconfigurato per consentire il collegamento. Il server può osservare dati tecnici della connessione, quali indirizzi IP, identificatori e tempi. Il trasferimento diretto o tramite relay dipende dalla rete.

L'accesso remoto ordinario richiede password temporanea e accettazione locale. Le funzionalità attivate durante l'assistenza possono trasmettere contenuti dello schermo, input, appunti, audio e file secondo i relativi permessi. I dati possono essere visibili al tecnico autorizzato. Non inserire dati riservati in una sessione se non intendi condividerli con il tecnico.

L'autorizzazione ai file senza seconda accettazione richiede un consenso iniziale distinto; è legata a tecnico e sessione e scade entro 30 minuti dal consenso, senza rinnovo dovuto a riconnessione o riavvio. La ripresa temporanea dopo riavvio richiede un'ulteriore autorizzazione locale e, dove necessario, la conferma Windows.

Il programma mantiene configurazioni e log tecnici locali. I ticket temporanei di ripresa persistiti utilizzano DPAPI e controlli di accesso Windows. Nessuna password permanente o accesso automatico a Windows viene introdotto dalla funzione di ripresa.

Per terminare l'assistenza usare il pulsante locale «Termina assistenza». La funzione di ripresa crea temporaneamente un'attività Windows e file sotto `Program Files/TecAssistResume`; i componenti di manutenzione provvedono alla pulizia secondo i controlli del progetto. Non eliminare arbitrariamente file appartenenti ad altre installazioni RustDesk.

Questa descrizione tecnica non sostituisce l'informativa del gestore del server e del servizio di assistenza: contatti, tempi di conservazione dei log sul server e ulteriori trattamenti devono essere completati dal gestore sul sito prima del rilascio pubblico candidato alla firma. Licenze e informazioni del software upstream: https://github.com/rustdesk/rustdesk .
