# SNACK 0.1.0 — Manuale

> Riscrittura completa da zero. Sostituisce ogni versione precedente. Basata sulle decisioni raccolte in `SNACK_v3_design.md`.

---

## 0. Introduzione

SNACK è un linguaggio per sistemi embedded, compilato in C. Ogni decisione di design punta a una cosa sola: **sapere a compile-time quanta memoria serve**, zero sorprese a runtime. Filosofia: prendere il meglio dai linguaggi esistenti (Rust, JS, C), renderlo esplicito, provare a fare meglio — senza copiare alla cieca ciò che non si adatta a un target con RAM misurata in kilobyte.

Estensione file: **`.spln`**.

---

## 1. Sintassi Base

**Encoding:** UTF-8 obbligatorio. Case-sensitive (`Speed` ≠ `speed`). Newline LF o CRLF, indifferente.

**Commenti:**
```
// commento su una riga

/* commento
   su più righe */
```

**Nomi validi:** iniziano con lettera o `_`, poi lettere/numeri/underscore/UTF-8. Niente spazi né trattini.
```
let.mut.32bit contatore_1 = 0;   // valido
let.mut.32bit _privata = 0;      // valido
let.mut.32bit 1nome = 0;         // NON valido: inizia con numero
```

**Punteggiatura:** un solo significato a simbolo, niente più eccezioni contestuali.

| Simbolo | Uso |
|---|---|
| `;` | fine istruzione |
| `,` | separatore in liste/parametri (array, argomenti funzione) |
| `{ }` | delimitano sempre i blocchi |
| `/` | separatore di path nell'albero dei chunk (unico uso) |
| `checkpoint;` | salva lo stato per recovery in caso di crash |
| `reset;` | azzera lo stato corrente |

**Literal:**
```
42                  // intero
-3.14               // decimale
"testo"             // stringa
true                false   // booleani
```

Non esiste `null`: ogni variabile dichiarata deve avere un valore concreto del proprio tipo fin dalla dichiarazione — stessa filosofia di Rust, niente bug da valore assente.

**Eccezione controllata:** `let x;` senza valore è permesso, ma il compilatore vieta di **leggere** `x` finché non può dimostrare, su ogni percorso di codice possibile, che è già stata assegnata (definite assignment analysis, stile Rust — analisi a compile-time, zero costo a runtime).
```
let.mut.32bit result;

if (sensor_ready) {
    result = read_analog(A0);
} else {
    result = 0;
}

display(result);   // OK: entrambi i rami assegnano result prima di questo punto
```
```
let.mut.32bit result;
if (sensor_ready) {
    result = read_analog(A0);
}
display(result);   // ERRORE di compilazione: il ramo else non assegna result
```

---

## 2. Struttura del Programma

Ogni file `.spln` ha tre blocchi obbligatori, in **ordine fisso**, seguiti dal punto d'ingresso `fn main`:

```
env {
    target: "arduino_uno";
    ram: 2048;
    hardware: [GPIO_13, GPIO_17, ADC_A0, ADC_A1];
    sensors: [DHT22_temp, DHT22_humidity];
    watchdog_timeout: 5000;
    rules_file: "./project.sr";
}

capabilities {
    import: "sensors_lib";
    uses: PWM;
    uses: sensors_analog;
}

identity {
    name: "scooter_controller";

    let.mut.32bit total_distance = 0;      // variabile globale
    fn compute_speed(d: 32bit, t: 32bit) -> 32bit {
        return d / t;
    }

    rule {
        let.mut.unsigned.8bit max_speed = 25;   // modificabile SOLO da listener
    }
}

fn main {
    // tutta la logica, chunk annidati
}
```

- **`env`**: info di sistema puro — hardware, RAM, target, percorso del file di sicurezza `.sr`.
- **`capabilities`**: permessi hardware (`uses:`) e librerie importate (`import:`) — sistema di permessi esplicito. Usare un comando hardware senza la capacità dichiarata è **errore di compilazione**.
- **`identity`**: nome del programma, variabili/funzioni globali, e il sotto-blocco `rule{}` protetto (unico costrutto che `listener` può modificare).
- **`fn main`**: unico punto d'ingresso della logica eseguibile.

**Persistenza delle variabili:** una `let` dichiarata al livello top di `fn main` persiste tra chunk/checkpoint/reset/riavvio, come una globale. Una `let` dichiarata dentro un chunk è locale e sparisce quando il chunk finisce.

---

## 3. Manifest di Progetto — `snack.toml`

`capabilities.import` (sezione 2) dichiara *cosa* importi, ma non *da dove* o *quale versione* — quel lavoro è di un manifest separato, un file per progetto, non per sorgente. Tenuto fuori da `.sr` di proposito: `.sr` ha un solo compito (limiti operativi immutabili), mischiarci metadata di progetto romperebbe quella separazione.

```toml
[project]
name = "scooter_controller"
version = "0.1.0"

[dependencies]
sensors_lib = "1.2.0"
```

---

## 4. Variabili e Tipi

**Forma semplice**, stile JS — mutabile, tipo dedotto, default numerico `i32`:
```
let x = 10;
let ok = true;
let name = "Ada";
```

**Forma esplicita**, per controllo preciso su mutabilità/segno/dimensione:
```
let.<mut|const>[.contenitore].[unsigned].<n>bit[.float|.string|.bool] nome = valore;
```
`let` è sempre il primo token; i tag dopo `let` seguono un **ordine fisso**: `mut/const → contenitore (array/struct/tuple, se presente) → unsigned → Nbit → float/string/bool`. Il peso (`8bit`/`16bit`/`32bit`) va sempre indicato quando si dichiara esplicitamente il tipo, anche per stringhe e booleani.

```
let.mut.unsigned.8bit age = 30;
let.const.32bit.float pi = 3.14159;
let.const.16bit.string label = "temp_A0";
let.mut.bool active = true;         // .bool facoltativo, dedotto da true/false
```

`mut`/`const` e il resto della logica dei tipi sono ispirati a Rust.

**Overflow:** un valore che eccede il range del tipo dichiarato **non** viene convertito automaticamente (romperebbe la dimensione fissa nota a compile-time) — causa un crash e la ripartenza dall'ultimo `checkpoint`, stesso comportamento della divisione per zero (vedi Gestione Errori).

---

## 5. Tipi Composti

**`struct`**: campi fissi con nome, ognuno tipizzato indipendentemente — definito una volta, riusabile. Nessuna chiave dinamica a runtime (a differenza di un oggetto JS): tutto noto a compile-time.
```
struct SensorData {
    readings: let.mut.array[10].unsigned.16bit,
    label: let.const.string,
    average: let.mut.32bit.float
}

let.const.SensorData sensor = { readings: [0,0,0,0,0,0,0,0,0,0], label: "temp", average: 0.0 };
```
**Niente `private`/`public` sui campi, niente get/set:** ogni campo è sempre liberamente leggibile e scrivibile da chiunque abbia l'istanza — scelta esplicita, non una lacuna. Senza visibilità privata, un get/set sarebbe solo una funzione che restituisce il campo senza validarlo né proteggerlo davvero, zero valore aggiunto rispetto ad accedere al campo direttamente (`sensor.average`).

**`array`**: dimensione fissa, bounds checking automatico, integrato nella grammatica di `let`.
```
let.mut.array[10].unsigned.16bit temperature = 0;
let.const.array[6].unsigned.8bit pwm_levels = [0, 50, 100, 150, 200, 255];

temperature[3] = 22;
let x = temperature[3];

length(temperature); average(temperature); sum(temperature); min(temperature); max(temperature);
```

**`tuple`**: sequenza ordinata a lunghezza fissa, tipo diverso per slot.
```
let.const.tuple(32bit.float, 32bit.float, string) coordinate = (45.4642, 9.1900, "deposito");
```

**Array di struct**: si usa il nome dello struct già definito dentro `array[N]`. Ogni variabile deve avere un valore concreto (sezione 1) — `Nome::default()` genera il valore di default dello struct (zero/stringa vuota/false per ogni campo, ricorsivamente) ripetuto per ogni elemento:
```
let.const.array[5].SensorData readings_log = SensorData::default();
```

L'ordine dei tag è **fisso ovunque**, non solo per i contenitori: `mut/const → contenitore (struct/array/tuple) → unsigned → Nbit → float/string/bool` (sezione 4). Il contenitore annidato indica comunque chi contiene chi, quando presente.

---

## 6. Enum e Switch

**`enum`**: tipo che può assumere solo uno tra un set fisso di valori nominati — il compilatore rifiuta qualunque altro valore, a differenza di una stringa qualsiasi.
```
enum ConnectionPolicy {
    Continue,
    Suspend
}

bridge #telemetry {
    on_offline: ConnectionPolicy::Continue;
}
```

**`switch`**, stile C, si abbina naturalmente a un enum:
```
switch (policy) {
    case ConnectionPolicy::Continue:
        display("Continuo offline");
        break;
    case ConnectionPolicy::Suspend:
        display("Sospendo il codice");
        break;
    default:
        display("Policy sconosciuta");
        break;
}
```
`default` è **sempre obbligatorio** — garantisce che ogni switch gestisca comunque ogni caso possibile, anche senza dover elencare esplicitamente ogni valore dell'enum (a differenza di `match` in Rust, che obbliga a coprirli tutti uno per uno).

---

## 7. Operatori

**Matematici:** `+ - * / % **` (potenza), precedenza standard PEMDAS.
```
let area = 3.14 * radius ** 2;
```

**Comparazione:** `== != > < >= <=` — restituiscono `bool`.

**Logici:** `&& || !`, valutazione short-circuit.
```
if (battery > 20 && temperature < 80) { ... }
```

**Bitwise:** `& | ^ ~ << >>` (XOR distinto da `**`).
```
let mask = flags & 0b00001111;
```

**Divisione per zero e overflow:** stesso comportamento — crash immediato, ripartenza dall'ultimo `checkpoint`. Se troppi crash al secondo, il sistema si blocca completamente (protezione hardware).

---

## 8. Funzioni Built-in

```
sqrt(x)  abs(x)  round(x)  floor(x)  ceil(x)  trunc(x)  sign(x)
sin(a)  cos(a)  tan(a)  asin(x)  acos(x)  atan(x)  atan2(y, x)   // radianti
log(x)  ln(x)  log2(x)  exp(x)
min(a, b)  max(a, b)
random()  random(max)  random(min, max)  seed(value)
map(value, in_min, in_max, out_min, out_max)   // stile Arduino
constrain(value, min, max)
length(s)   // lunghezza stringa o array
```
```
let radius = sqrt(16);          // 4.0
let temp = map(adc_value, 0, 1023, 0, 100);
let safe = constrain(input, 0, 255);
```

---

## 9. Introspezione dei Tipi

Il tipo, il segno, la dimensione e la mutabilità di una variabile sono decisi nella sua dichiarazione e non cambiano mai durante l'esecuzione — quindi queste funzioni sono risolte **interamente a compile-time**, come `sizeof` in C: il compilatore le sostituisce con una costante, costo zero a runtime, nessun dato extra da portarsi dietro per ogni variabile.

```
type_of(x)       // es. "unsigned.8bit"
sign_of(x)       // "signed" o "unsigned"
bit_size_of(x)   // es. 8, 16, 32
is_mut(x)        // true o false
identity_of(x)   // tutte le precedenti insieme, come struct
```
```
let.mut.unsigned.8bit age = 30;

display(type_of(age));        // "unsigned.8bit"
display(is_mut(age));         // true

let info = identity_of(age);
display(info.type, info.sign, info.bits, info.mut);
```
Conseguenza diretta dell'essere risolte a compile-time: non si può passare una variabile il cui tipo dipende da un ramo `if` deciso solo a runtime — il compilatore deve conoscere la risposta prima ancora di generare il codice.

---

## 10. Variabili Built-in

```
time  date  millis  micros            // tempo
free_ram  voltage  battery            // sistema
temperature  humidity  light  sound  pressure   // sensori, se dichiarati in env
dht_temperature  dht_humidity         // DHT, dopo read_dht()
PI  E  TAU                            // costanti matematiche
```
```
display("Uptime:", millis, "ms");
let elapsed = millis - start_time;
```

Non esistono più `last_value`/`last_error`: le funzioni hanno tipo di ritorno esplicito (sezione 12), e gli errori si gestiscono con `try`/`catch` (sezione 22).

---

## 11. Controllo di Flusso

**If/else**, parentesi tonde obbligatorie attorno alla condizione, utilizzabile anche come **espressione**:
```
if (battery < 15) {
    display("Batteria scarica");
} else {
    display("OK");
}

let mode = if (time > 20.00) { "night" } else { "day" };
```

**Loop — `repeat`**, unico costrutto di loop (nessun `for`/`while` tradizionale, scelta filosofica), semantica do-while:
```
let.mut.32bit count = 10;
repeat {
    display(count);
    count = count - 1;
    delay(1000);
} until (count == 0);
```
Per comportamento continuo/persistente si usa `listener` (sezione 15), non `repeat`.

---

## 12. Funzioni

**Dichiarazione**, tipizzata come tutto il resto:
```
fn compute_speed(distance: 32bit, elapsed: 32bit) -> 32bit {
    return distance / elapsed;
}
```

**Funzioni come valori**, per passarle come argomenti/callback:
```
let.const.fn(32bit, 32bit) -> 32bit operation = compute_speed;
```

**Arrow function**, forma anonima e compatta, corpo a singola espressione con return implicito:
```
let doubler = (x: 32bit) -> 32bit => x * 2;
```

**Closure**, keyword esplicita `closure`, cattura per riferimento sfruttando l'arena del chunk in cui nasce (nessuna allocazione extra):
```
let.mut.32bit factor = 3;
let multiplier = closure (x: 32bit) -> 32bit => x * factor;
```
Default (nessuna keyword) = funzione pura, zero costo, passabile ovunque. `closure` = cattura per riferimento ma **non può sopravvivere al chunk in cui è nata** — verificato a compile-time; farla "scappare" (es. salvarla in `identity`) è errore di compilazione, non crash a runtime.

---

## 13. Riferimenti

Niente puntatori grezzi — un puntatore rimasto vivo dopo che l'arena del suo chunk è stata liberata (sezione 14) sarebbe esattamente il tipo di bug (use-after-free) che l'intero sistema a chunk/arena è pensato per rendere impossibile. Al loro posto, **riferimenti in stile Rust**: prendono in prestito una variabile senza copiarne la memoria, verificati a compile-time contro la vita del chunk in cui vivono — stesso principio già usato per le closure (sezione 12).

```
fn total(readings: &SensorReading) -> 32bit {
    return sum(readings.values);
}

#trip {
    let.const.SensorReading log = { values: [10,12,11,9,13], label: "trip_1", average: 11.0 };
    let t = total(&log);   // passato per riferimento, nessuna copia dell'array interno
}
```

- `&T` = riferimento in sola lettura al tipo `T`
- `&mut T` = riferimento in scrittura — **una sola** referenza mutabile attiva alla volta sulla stessa variabile, verificato a compile-time (stessa regola del borrow checker Rust, evita scritture concorrenti)
- un riferimento non può sopravvivere al chunk in cui è stata dichiarata la variabile a cui punta — tentare di farlo è **errore di compilazione**, non crash a runtime

---

## 14. Gerarchia dei Chunk

Se `env.ram < 100000` (100 KB), il chunking si attiva automaticamente: la memoria si gestisce a **blocchi**, non tutta insieme. Un blocco che supera la dimensione massima viene diviso automaticamente dal compiler (con warning).

I chunk si annidano ad **albero** (non più solo una sequenza lineare) — ogni chunk può farne nascere altri al suo interno, come pale di fico d'india che crescono l'una dall'altra:

```
fn main {
    #trip {
        let.mut.32bit distance = 0;

        #measurement {
            // pala figlia: eredita distance in lettura dal genitore
            distance = distance + 120;
            display("Distanza parziale:", distance);
        } // #measurement finisce, le sue modifiche locali si perdono

        let.const.32bit final_distance = fn_main/#trip/#measurement/distance;  // pull esplicito
    }
}
```

- Ogni chunk si chiama `#nome` — il prefisso evita ambiguità con variabili omonime.
- Ogni chunk alloca la propria arena alla nascita, la libera alla fine, in ordine **LIFO** (come lo stack delle chiamate a funzione). Una sola `arena_alloc`/`arena_free` per chunk, zero frammentazione.
- Un chunk figlio eredita in **lettura** le variabili del genitore. Le modifiche a variabili ereditate si perdono di default quando il figlio finisce — isolamento, niente race condition.
- Per recuperare esplicitamente un valore da un chunk già concluso: path separato da `/`, leggibile **solo dopo** che il chunk referenziato è terminato (come un return di funzione).
- `free(variabile)` resta disponibile come escape hatch esplicito, per liberare qualcosa prima della fine naturale del chunk — solo sull'ultimo elemento allocato, vedi Gestione Memoria (sezione 21) per il dettaglio.

---

## 15. Listener

Loop infinito che ricontrolla sempre la propria condizione, con priorità assoluta — unico costrutto che può modificare `rule{}`. Sostituisce il vecchio sistema a numerazione automatica: ogni `listener` ha un nome proprio, e la priorità è sempre l'ordine di dichiarazione.

```
listener #battery_check (battery < rule.min_battery) {
    set_pwm(motor_pin, 0);
    display("BATTERIA CRITICA! Sistema arrestato");
    pause(#temp_check);

    if (battery >= 80) {
        rule.max_speed = 25;
        pause(self);
    }
}

listener #temp_check (temperature > rule.max_temp) {
    rule.max_speed = 10;
    display("TEMPERATURA ALTA! Velocità ridotta");
    if (temperature <= 70) {
        rule.max_speed = 25;
        pause(self);
    }
}
```

`pause(#nome)` sospende un listener, `stop(#nome)` lo termina in modo permanente; `self` si riferisce al listener corrente. Race-condition detection e watchdog configurabile restano attivi automaticamente.

---

## 16. On-Event

Stesso schema sintattico di `listener`, ma scatta **una volta sola** invece che in loop continuo — poi si disattiva da solo.

```
on_event #download_complete (download_status == "completed") {
    display("Download finito!");
    display("File salvato in:", download_path);
}

on_event #wifi_ready (wifi_connected == true) {
    display("WiFi connesso, IP:", ip_address);
}
```

---

## 17. Rule{} e File di Sicurezza

`rule{}` vive dentro `identity{}` (sezione 2), modificabile **solo** da `listener`, accesso in lettura/scrittura con `rule.campo`. Le modifiche persistono anche dopo il riavvio.

```
if (current_speed > rule.max_speed) {
    display("ATTENZIONE: superato il limite!");
}
```

**`SecurityRule`**, file separato `.sr` (percorso in `env.rules_file`): regole immutabili che **nessun** codice può modificare, nemmeno `listener`. Protetto da password OS, auto-logging di ogni modifica a `rule{}`.

```
SecurityRule {
    max_hardware_speed: 30;
    max_hardware_temp: 85;
    can_modify_max_speed: false;
    can_disable_watchdog: false;
    log_all_rule_changes: true;
    log_path: "./logs/security.log";
    auto_lock_after_crash: 3;
    reset_speed_after_crash: true;

    bridge_public_key: "-----BEGIN PUBLIC KEY-----...";
    bridge_private_key: "-----BEGIN PRIVATE KEY-----...";
}

authorized_listeners: [#battery_check];
```

| | `rule{}` | `SecurityRule` |
|---|---|---|
| Modificabile da | `listener` | Nessuno |
| File | Codice sorgente | `.sr` separato |
| Scopo | Limiti software | Limiti hardware critici |

---

## 18. Comandi Hardware

Sintassi a chiamata di funzione, ritorno diretto del valore — nessun prefisso speciale.

```
set_pin_mode(13, OUTPUT);
write_pin(13, HIGH);
let button = read_pin(5);

let raw = read_analog(A0);
let voltage = raw * 5.0 / 1023.0;

set_pwm(motor_pin, speed);

delay(1000);              // OK: letterale
delay(rule.interval);     // OK: da un let.const
delay(wait_time);         // ERRORE se wait_time è let.mut — previene delay accidentali variabili
```

Stessa lista di comandi avanzati disponibile: I2C, SPI, UART, `tone()`, `pulse_in()`, shift register, servo, DHT (`read_dht()` popola `dht_temperature`/`dht_humidity`).

Ogni comando richiede la relativa capacità dichiarata in `capabilities` (sezione 2) — altrimenti errore di compilazione.

---

## 19. File I/O e Include

```
let config = read_file("config.txt");
write_file("log.txt", "Temperatura: 25 C\n");

include("./sensori.spln");
include("./display.spln");

display("Sensore:", sensori.temperatura);
```

Namespace automatico = nome file (senza estensione); le variabili locali hanno sempre priorità su quelle importate in caso di collisione.

---

## 20. Multi-Ambiente e Bridge

`env.target` è sempre **statico**: descrive l'hardware fisico che il compilatore deve conoscere per generare il codice giusto (pin, RAM disponibile) — fisso nel momento in cui il programma viene flashato, non può dipendere da una condizione valutata a runtime.
```
env {
    target: "arduino_uno";
}
```

Per un comportamento che il programma sceglie mentre gira (non l'hardware su cui gira, ma *come* si comporta) si usa `identity.mode`, una variabile normale — questa sì può dipendere da un'espressione:
```
identity {
    let.const.string mode = if (time > 20.00) { "night" } else { "day" };
}
```

**Codice specifico per ambiente hardware**: un `if` normale su `env.target`, nessuna sintassi dedicata.
```
if (env.target == "arduino_uno") {
    set_pin_mode(13, OUTPUT);
    display("LED Arduino acceso");
}
```
Hardware dichiarato su più ambienti senza essere specificato esplicitamente resta errore di compilazione.

**Bridge tra ambienti**: blocco con nome, comunicazione cifrata tra dispositivi. **Le chiavi non vivono mai nel sorgente**: il bridge referenzia solo il nome, il valore vero sta in `SecurityRule` (sezione 17), nel file `.sr` protetto da password OS — mai in un file `.spln` che finisce facilmente in un repository o in un log.
```
bridge #telemetry {
    from: "arduino";
    to: "smartphone";
    method: "bluetooth";
    encryption: "e2e_rsa";
    keys: SecurityRule::bridge_public_key, SecurityRule::bridge_private_key;
    data: [temperature, battery];
    frequency: 1000;
    timeout: 5000;
    on_offline: ConnectionPolicy::Continue;   // o ConnectionPolicy::Suspend
}
```
Il bridge non invia uno snapshot ma una **media pesata temporale** (i valori recenti contano di più, campionamento ogni `frequency/10` ms), accessibile con `telemetry.temperature`. Bridge multipli: più blocchi `bridge #nome { }`, nessuna sintassi speciale.

---

## 21. Gestione Memoria

Limite lunghezza stringhe basato sulla RAM disponibile:
```
CHUNK_SIZE = env.ram * 0.6
MAX_STRING_LENGTH = CHUNK_SIZE * 2
```
La memoria si libera **tutta insieme, in modo deterministico**, quando il chunk che la contiene finisce (sezione 14) — nessuna garbage collection euristica a runtime.

**`free(variabile)`** con l'arena a stack (LIFO, sezione 14): funziona **solo sull'elemento più in cima allo stack** — l'ultima variabile allocata nel chunk corrente. Liberare qualcosa che non è in cima frammenterebbe l'arena, la stessa cosa che il sistema a chunk esiste per evitare — quindi è vietato staticamente, non lasciato a un comportamento silenzioso:
```
#trip {
    let.mut.32bit distance = 0;
    let.mut.array[10].unsigned.16bit big_buffer;

    free(big_buffer);   // OK: è l'ultima variabile allocata, in cima allo stack
    // free(distance);  // ERRORE di compilazione: distance non è in cima, big_buffer lo è ancora
}
```
Se serve liberare qualcosa che non è in cima, l'unica via è terminare il blocco che la contiene (il chunk libera tutto insieme) o riordinare le dichiarazioni in modo che l'elemento da liberare sia l'ultimo allocato.

---

## 22. Gestione Errori

Un solo stile: `try`/`catch`, familiare da JS.
```
try {
    let cfg = read_file("config.txt");
    let data = read_file("data.txt");
} catch (err) {
    display("Errore caricamento:", err);
}

ignore_errors {
    let opt1 = read_file("optional1.txt");
}
```
`throw` e `ignore_errors` sono utility complementari, non stili alternativi. `checkpoint`/`reset` restano il livello sotto: `try`/`catch` gestisce errori **previsti**, checkpoint/reset è la rete di sicurezza per crash **non gestiti** (overflow, divisione per zero, hardware fault) — definiti nel dettaglio nella prossima sezione.

---

## 23. Checkpoint e Reset

**`checkpoint;`** salva un'istantanea di **tutte le variabili top-level di `fn main`** (quelle con scope persistente, sezione 2) — non lo stato delle periferiche hardware (pin, PWM, connessioni), che vengono ri-inizializzate da `env`/`capabilities` al riavvio, non ripristinate da uno snapshot.

```
fn main {
    let.mut.32bit total_distance = 0;   // salvata da ogni checkpoint

    #trip {
        let.mut.32bit temp = 0;         // NON salvata: locale al chunk, non top-level
        checkpoint;
    }
}
```

Comportamento:
- Un solo checkpoint attivo alla volta: ogni chiamata a `checkpoint;` **sovrascrive** il precedente, non li accumula — costo di RAM/flash costante, non crescente con l'esecuzione.
- Costo: proporzionale alla somma delle variabili top-level dichiarate (calcolabile a compile-time, coerente con "memoria nota in anticipo" — nessuna sorpresa a runtime).
- Su crash (overflow, divisione per zero, troppi crash/secondo da un `listener`): le variabili top-level tornano al valore dell'ultimo checkpoint, l'esecuzione riparte da lì; le periferiche hardware si ri-inizializzano da zero.

**`reset;`** azzera lo stato corrente (le variabili top-level tornano al valore di dichiarazione, non a quello dell'ultimo checkpoint) — usato esplicitamente nel codice, non da un crash.

---

## 24. Ottimizzazioni Compiler e Runtime

Applicate automaticamente, nessuna azione richiesta:

**Compile-time:** constant folding, dead code elimination, common subexpression elimination, inlining di funzioni piccole, string interning.

**Runtime:** copy-on-write per le stringhe, small string optimization, arena allocator per chunk (sezione 14), variable reuse, lazy evaluation, watchdog automatico, auto-retry su operazioni hardware transitorie.

---

## 25. Tooling e Debug

```
snack build main.spln --profile          // profiler tempo di esecuzione
snack build main.spln --memory-profile   // profiler memoria per chunk
snack build main.spln --static-analysis  // analisi statica, warning su codice sospetto
```
Errori di compilazione riportano file, riga, colonna e un messaggio descrittivo; disponibile anche in formato JSON per integrazione con altri strumenti.

---

## 26. Esempio Completo

```
env {
    target: "arduino_uno";
    ram: 2048;
    hardware: [GPIO_13, GPIO_17, ADC_A0, ADC_A1];
    sensors: [DHT22_temp, DHT22_humidity];
    watchdog_timeout: 5000;
    rules_file: "./scooter.sr";
}

capabilities {
    import: "sensors_lib";
    uses: PWM;
    uses: sensors_analog;
    uses: sensors_digital;
}

struct SensorReading {
    values: let.mut.array[5].unsigned.16bit,
    label: let.const.string,
    average: let.mut.32bit.float
}

identity {
    name: "scooter_controller";

    let.mut.32bit total_distance = 0;
    let.const.array[3].SensorReading sensor_log;

    fn compute_speed(distance: 32bit, elapsed: 32bit) -> 32bit {
        return distance / elapsed;
    }

    fn average_reading(reading: &SensorReading) -> 32bit.float {
        return average(reading.values);
    }

    rule {
        let.mut.unsigned.8bit max_speed = 25;
        let.mut.unsigned.8bit max_temp = 80;
        let.mut.unsigned.8bit min_battery = 15;
    }
}

listener #battery_check (battery < rule.min_battery) {
    set_pwm(motor_pin, 0);
    display("BATTERIA CRITICA! Motore fermato");
    pause(#temp_check);

    if (battery >= 80) {
        rule.max_speed = 25;
        pause(self);
    }
}

listener #temp_check (temperature > rule.max_temp) {
    rule.max_speed = 10;
    display("TEMPERATURA ALTA! Velocità ridotta a", rule.max_speed);

    if (temperature <= 70) {
        rule.max_speed = 25;
        pause(self);
    }
}

fn main {
    let.const.tuple(32bit.float, 32bit.float, string) start_point = (45.4642, 9.1900, "deposito");

    #trip {
        let.mut.32bit distance = 0;
        let.mut.32bit elapsed = 0;

        #measurement {
            distance = distance + 120;
            elapsed = elapsed + 60;

            let speed = compute_speed(distance, elapsed);
            display("Velocità istantanea:", speed);
        }

        let.const.32bit final_distance = fn_main/#trip/#measurement/distance;
        total_distance = total_distance + final_distance;

        display("Distanza totale percorsa:", total_distance);
        checkpoint;
    }
}
```
