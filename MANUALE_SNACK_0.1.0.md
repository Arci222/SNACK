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
```text
// commento su una riga

/* commento
   su più righe */
```

**Nomi validi:** iniziano con lettera o `_`, poi lettere/numeri/underscore/UTF-8. Niente spazi né trattini.
```text
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
```text
42                  // intero
-3.14               // decimale
"testo"             // stringa
true                false   // booleani
```

Non esiste `null`: ogni variabile dichiarata deve avere un valore concreto del proprio tipo fin dalla dichiarazione — stessa filosofia di Rust, niente bug da valore assente.

**Eccezione controllata:** `let x;` senza valore è permesso, ma il compilatore vieta di **leggere** `x` finché non può dimostrare, su ogni percorso di codice possibile, che è già stata assegnata (definite assignment analysis, stile Rust — analisi a compile-time, zero costo a runtime).
```text
let.mut.32bit result;

if (sensor_ready) {
    result = read_analog(A0);
} else {
    result = 0;
}

display(result);   // OK: entrambi i rami assegnano result prima di questo punto
```
```text
let.mut.32bit result;
if (sensor_ready) {
    result = read_analog(A0);
}
display(result);   // ERRORE di compilazione: il ramo else non assegna result
```

---

## 2. Struttura del Programma

Ogni file `.spln` ha tre blocchi obbligatori, in **ordine fisso**, seguiti dal punto d'ingresso `fn main`:

```text
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
```text
let x = 10;
let ok = true;
let name = "Ada";
```

**Forma esplicita**, per controllo preciso su mutabilità/segno/dimensione:
```text
let.<mut|const>[.contenitore].[unsigned].<n>bit[.float|.string|.bool] nome = valore;
```
`let` è sempre il primo token; i tag dopo `let` seguono un **ordine fisso**: `mut/const → contenitore (array/struct/tuple, se presente) → unsigned → Nbit → float/string/bool`. Il peso (`8bit`/`16bit`/`32bit`) va sempre indicato quando si dichiara esplicitamente il tipo, anche per stringhe e booleani.

```text
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
```text
struct SensorData {
    readings: let.mut.array[10].unsigned.16bit,
    label: let.const.string,
    average: let.mut.32bit.float
}

let.const.SensorData sensor = { readings: [0,0,0,0,0,0,0,0,0,0], label: "temp", average: 0.0 };
```
**Niente `private`/`public` sui campi, niente get/set:** ogni campo è sempre liberamente leggibile e scrivibile da chiunque abbia l'istanza — scelta esplicita, non una lacuna. Senza visibilità privata, un get/set sarebbe solo una funzione che restituisce il campo senza validarlo né proteggerlo davvero, zero valore aggiunto rispetto ad accedere al campo direttamente (`sensor.average`).

**`array`**: dimensione fissa, bounds checking automatico, integrato nella grammatica di `let`.
```text
let.mut.array[10].unsigned.16bit temperature = 0;
let.const.array[6].unsigned.8bit pwm_levels = [0, 50, 100, 150, 200, 255];

temperature[3] = 22;
let x = temperature[3];

length(temperature); average(temperature); sum(temperature); min(temperature); max(temperature);
```

**`tuple`**: sequenza ordinata a lunghezza fissa, tipo diverso per slot.
```text
let.const.tuple(32bit.float, 32bit.float, string) coordinate = (45.4642, 9.1900, "deposito");
```

**Array di struct**: si usa il nome dello struct già definito dentro `array[N]`. Ogni variabile deve avere un valore concreto (sezione 1) — `Nome::default()` genera il valore di default dello struct (zero/stringa vuota/false per ogni campo, ricorsivamente) ripetuto per ogni elemento:
```text
let.const.array[5].SensorData readings_log = SensorData::default();
```

L'ordine dei tag è **fisso ovunque**, non solo per i contenitori: `mut/const → contenitore (struct/array/tuple) → unsigned → Nbit → float/string/bool` (sezione 4). Il contenitore annidato indica comunque chi contiene chi, quando presente.

---

## 6. Array Dinamici

**`array[? max N]`**: dimensione decisa solo a runtime, ma con un **tetto massimo sempre obbligatorio**, anche su target desktop — coerente con "sapere a compile-time quanta memoria serve" (sezione 0): un array che cresce senza limite dichiarato è la stessa promessa infranta che abbiamo escluso con `malloc`/`free` general-purpose. `max` non è un vincolo IoT, è la regola del linguaggio.
```text
let.mut.array[? max 100].unsigned.16bit temperature = [];

push(temperature, 22);
push(temperature, 23);
let n = length(temperature);   // 2
```
Il compilatore pre-alloca lo spazio per `N` elementi nell'arena del chunk — zero riallocazioni nascoste, zero sorprese. Funzioni: `push`, `pop`, `insert`, `remove`, `clear` — oltre a `length`/`sum`/`average`/`min`/`max`, già disponibili sugli array statici. **Niente `reserve()`/`capacity()`**: con `max N` già pre-allocato dalla dichiarazione, non c'è nulla da "riservare in più" — a differenza di un `vector` C++, che rialloca quando cresce, qui la capacità è fissa fin dall'inizio, quindi quelle due funzioni non avrebbero mai un effetto reale da avere.

Un array dinamico vive nell'arena del chunk in cui è dichiarato, come qualsiasi altra variabile — nessuna differenza rispetto a un array statico quando il chunk finisce (sezione 14): l'arena si libera tutta insieme.

**`take(array, indice)` / `take(array, inizio, fine)`**: restituisce un **riferimento** (`&T`/`&mut T`, o `&[T]` per un intervallo), non una copia — segue le stesse regole di vita della sezione 14. Come `pointer()` (sezione 27), non è una funzione qualunque: **un solo `take()` mutabile attivo per array alla volta**, indipendentemente dall'indice o intervallo preso — più semplice da verificare rispetto a tracciare ogni indice singolarmente, e per lo stesso motivo `push`/`insert`/`remove` (che richiedono anch'essi accesso mutabile all'array, per farlo crescere o riorganizzarlo) sono **rifiutati mentre esiste una vista attiva** da `take()`: evita che l'array venga riallocato altrove in memoria lasciando la vista a puntare a un indirizzo ormai vecchio.
```text
let vista = take(temperature, 0, 2);
// push(temperature, 30);   // ERRORE: vista è ancora attiva su temperature
display(average(vista));
```

---

## 7. Enum e Switch

**`enum`**: tipo che può assumere solo uno tra un set fisso di valori nominati — il compilatore rifiuta qualunque altro valore, a differenza di una stringa qualsiasi.
```text
enum ConnectionPolicy {
    Continue,
    Suspend
}

bridge #telemetry {
    on_offline: ConnectionPolicy::Continue;
}
```

**`switch`**, stile C, si abbina naturalmente a un enum:
```text
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

## 8. Operatori

**Matematici:** `+ - * / % **` (potenza), precedenza standard PEMDAS.
```text
let area = 3.14 * radius ** 2;
```

**Comparazione:** `== != > < >= <=` — restituiscono `bool`.

**Logici:** `&& || !`, valutazione short-circuit.
```text
if (battery > 20 && temperature < 80) { ... }
```

**Bitwise:** `& | ^ ~ << >>` (XOR distinto da `**`).
```text
let mask = flags & 0b00001111;
```

**Divisione per zero e overflow:** stesso comportamento — crash immediato, ripartenza dall'ultimo `checkpoint`. Se troppi crash al secondo, il sistema si blocca completamente (protezione hardware).

---

## 9. Funzioni Built-in

```text
sqrt(x)  abs(x)  round(x)  floor(x)  ceil(x)  trunc(x)  sign(x)
sin(a)  cos(a)  tan(a)  asin(x)  acos(x)  atan(x)  atan2(y, x)   // radianti
log(x)  ln(x)  log2(x)  exp(x)
min(a, b)  max(a, b)
random()  random(max)  random(min, max)  seed(value)
map(value, in_min, in_max, out_min, out_max)   // stile Arduino
constrain(value, min, max)
length(s)   // lunghezza stringa o array
```
```text
let radius = sqrt(16);          // 4.0
let temp = map(adc_value, 0, 1023, 0, 100);
let safe = constrain(input, 0, 255);
```

---

## 10. Introspezione dei Tipi

Il tipo, il segno, la dimensione e la mutabilità di una variabile sono decisi nella sua dichiarazione e non cambiano mai durante l'esecuzione — quindi queste funzioni sono risolte **interamente a compile-time**, come `sizeof` in C: il compilatore le sostituisce con una costante, costo zero a runtime, nessun dato extra da portarsi dietro per ogni variabile.

```text
type_of(x)       // es. "unsigned.8bit"
sign_of(x)       // "signed" o "unsigned"
bit_size_of(x)   // es. 8, 16, 32
is_mut(x)        // true o false
identity_of(x)   // tutte le precedenti insieme, come struct
```
```text
let.mut.unsigned.8bit age = 30;

display(type_of(age));        // "unsigned.8bit"
display(is_mut(age));         // true

let info = identity_of(age);
display(info.type, info.sign, info.bits, info.mut);
```
Conseguenza diretta dell'essere risolte a compile-time: non si può passare una variabile il cui tipo dipende da un ramo `if` deciso solo a runtime — il compilatore deve conoscere la risposta prima ancora di generare il codice.

---

## 11. Variabili Built-in

```text
time  date  millis  micros            // tempo
free_ram  voltage  battery            // sistema
temperature  humidity  light  sound  pressure   // sensori, se dichiarati in env
dht_temperature  dht_humidity         // DHT, dopo read_dht()
PI  E  TAU                            // costanti matematiche
```
```text
display("Uptime:", millis, "ms");
let elapsed = millis - start_time;
```

Non esistono più `last_value`/`last_error`: le funzioni hanno tipo di ritorno esplicito (sezione 13), e gli errori si gestiscono con `try`/`catch` (sezione 23).

---

## 12. Controllo di Flusso

**If/else**, parentesi tonde obbligatorie attorno alla condizione, utilizzabile anche come **espressione**:
```text
if (battery < 15) {
    display("Batteria scarica");
} else {
    display("OK");
}

let mode = if (time > 20.00) { "night" } else { "day" };
```

**Loop — tre costrutti, ognuno per una forma diversa** (non ridondanti tra loro):

`repeat { } until (condizione);` — esegue il corpo, poi controlla (do-while):
```text
let.mut.32bit count = 10;
repeat {
    display(count);
    count = count - 1;
    delay(1000);
} until (count == 0);
```

`while (condizione) { }` — controlla, poi eventualmente esegue il corpo (condizione-prima, per quando il corpo potrebbe non dover girare nemmeno una volta):
```text
while (queue_has_data()) {
    let item = pop(queue);
    display(item);
}
```

`for (elemento in collezione) { }` — itera su un array o un range, senza contatore manuale né controllo di bordo scritto a mano:
```text
let.const.array[4].32bit.ptr registers = [pointer(0x24), pointer(0x25), pointer(0x26), pointer(0x27)];

for (reg in registers) {
    *reg = 0;
}

for (i in 0..4) {
    display(i);
}
```

Per comportamento continuo/persistente (osservare una condizione in background, non iterare un blocco lineare) si usa `listener` (sezione 16), non uno di questi tre.

---

## 13. Funzioni

**Dichiarazione**, tipizzata come tutto il resto:
```text
fn compute_speed(distance: 32bit, elapsed: 32bit) -> 32bit {
    return distance / elapsed;
}
```

**Funzioni come valori**, per passarle come argomenti/callback:
```text
let.const.fn(32bit, 32bit) -> 32bit operation = compute_speed;
```

**Arrow function**, forma anonima e compatta, corpo a singola espressione con return implicito:
```text
let doubler = (x: 32bit) -> 32bit => x * 2;
```

**Closure**, keyword esplicita `closure`, cattura per riferimento sfruttando l'arena del chunk in cui nasce (nessuna allocazione extra):
```text
let.mut.32bit factor = 3;
let multiplier = closure (x: 32bit) -> 32bit => x * factor;
```
Default (nessuna keyword) = funzione pura, zero costo, passabile ovunque. `closure` = cattura per riferimento ma **non può sopravvivere al chunk in cui è nata** — verificato a compile-time; farla "scappare" (es. salvarla in `identity`) è errore di compilazione, non crash a runtime.

---

## 14. Riferimenti

Niente puntatori grezzi nel codice normale — un puntatore rimasto vivo dopo che l'arena del suo chunk è stata liberata sarebbe esattamente il tipo di bug (use-after-free) che l'intero sistema a chunk/arena è pensato per rendere impossibile. Lo strumento di default sono i **riferimenti in stile Rust**: prendono in prestito una variabile senza copiarne la memoria, verificati a compile-time contro la vita del chunk in cui vivono — stesso principio già usato per le closure (sezione 13). Per l'accesso a indirizzi hardware fissi, dove i puntatori grezzi sono davvero necessari, esiste `unsafe` — vedi sezione 27.

```text
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

## 15. Gerarchia dei Chunk

Se `env.ram < 100000` (100 KB), il chunking si attiva automaticamente: la memoria si gestisce a **blocchi**, non tutta insieme. Un blocco che supera la dimensione massima viene diviso automaticamente dal compiler (con warning).

I chunk si annidano ad **albero** (non più solo una sequenza lineare) — ogni chunk può farne nascere altri al suo interno, come pale di fico d'india che crescono l'una dall'altra:

```text
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
- Non esiste `free()`: l'unico modo per liberare memoria prima della fine naturale del chunk è annidare la parte da liberare in anticipo in un chunk figlio più piccolo — vedi Gestione Memoria (sezione 22) per il dettaglio.

---

## 16. Listener

Loop infinito che ricontrolla sempre la propria condizione, con priorità assoluta — unico costrutto che può modificare `rule{}`. Sostituisce il vecchio sistema a numerazione automatica: ogni `listener` ha un nome proprio, e la priorità è sempre l'ordine di dichiarazione.

```text
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

## 17. On-Event

Stesso schema sintattico di `listener`, ma scatta **una volta sola** invece che in loop continuo — poi si disattiva da solo.

```text
on_event #download_complete (download_status == "completed") {
    display("Download finito!");
    display("File salvato in:", download_path);
}

on_event #wifi_ready (wifi_connected == true) {
    display("WiFi connesso, IP:", ip_address);
}
```

---

## 18. Rule{} e File di Sicurezza

`rule{}` vive dentro `identity{}` (sezione 2), modificabile **solo** da `listener`, accesso in lettura/scrittura con `rule.campo`. Le modifiche persistono anche dopo il riavvio, scritte in **EEPROM** (non Flash): l'EEPROM è pensata apposta per piccoli valori che cambiano nel tempo, con circa 100.000 cicli di scrittura garantiti — la Flash, dove vive il programma stesso, ne sopporta molti meno (circa 10.000) prima di degradarsi, e `rule{}` può essere riscritta spesso da un `listener` attivo. Su target senza EEPROM disponibile, `rule{}` non persiste tra un riavvio e l'altro — torna ai valori dichiarati nel sorgente.

```text
if (current_speed > rule.max_speed) {
    display("ATTENZIONE: superato il limite!");
}
```

**`SecurityRule`**, file separato `.sr` (percorso in `env.rules_file`): regole immutabili che **nessun** codice può modificare, nemmeno `listener`. Protetto da password OS, auto-logging di ogni modifica a `rule{}`.

```text
SecurityRule {
    max_hardware_speed: 30;
    max_hardware_temp: 85;
    can_modify_max_speed: false;
    can_disable_watchdog: false;
    log_all_rule_changes: true;
    log_path: "./logs/security.log";
    auto_lock_after_crash: 3;
    reset_speed_after_crash: true;

    chunk_ram_ratio: 0.6;
    max_string_multiplier: 2;
    max_delay_ms: 10000;

    bridge_keys: injected;   // iniettate al momento del flashing, mai in chiaro nel .sr
}

authorized_listeners: [#battery_check];
```
**`bridge_keys: injected`**: nemmeno nel file `.sr` le chiavi vivono in chiaro — un file può finire copiato, versionato, letto da chi non dovrebbe. Il valore vero viene scritto direttamente nella memoria del dispositivo al momento del flashing (dal toolchain di build, fuori da qualsiasi file di progetto), lo stesso approccio di un secure element hardware quando disponibile. `injected` dice al compilatore "questo valore arriva da fuori, non aspettartelo nel sorgente né nella configurazione".

| | `rule{}` | `SecurityRule` |
|---|---|---|
| Modificabile da | `listener` | Nessuno |
| File | Codice sorgente | `.sr` separato |
| Scopo | Limiti software | Limiti hardware critici |

---

## 19. Comandi Hardware

Sintassi a chiamata di funzione, ritorno diretto del valore — nessun prefisso speciale.

```text
set_pin_mode(13, OUTPUT);
write_pin(13, HIGH);
let button = read_pin(5);

let raw = read_analog(A0);
let voltage = raw * 5.0 / 1023.0;

set_pwm(motor_pin, speed);

delay(1000);              // OK
delay(rule.interval);     // OK
delay(wait_time);         // OK, anche se wait_time è let.mut: il valore è comunque fisso al momento della chiamata
```
`delay()` non impone restrizioni su mutabile/costante — non è la mutabilità della variabile a renderla rischiosa (il valore passato è comunque fisso in quel preciso istante), ma la possibilità di un valore anomalo (es. un bug che assegna un numero enorme). Il limite reale, se serve, è esplicito in `SecurityRule` (sezione 18):
```text
SecurityRule {
    max_delay_ms: 10000;   // delay() oltre questo valore è errore di compilazione se il valore è noto staticamente, altrimenti errore a runtime
}
```

Stessa lista di comandi avanzati disponibile: I2C, SPI, UART, `tone()`, `pulse_in()`, shift register, servo, DHT (`read_dht()` popola `dht_temperature`/`dht_humidity`).

Ogni comando richiede la relativa capacità dichiarata in `capabilities` (sezione 2) — altrimenti errore di compilazione.

---

## 20. File I/O e Include

```text
let config = read_file("config.txt");
write_file("log.txt", "Temperatura: 25 C\n");

include("./sensori.spln");
include("./display.spln");

display("Sensore:", sensori.temperatura);
```

Namespace automatico = nome file (senza estensione); le variabili locali hanno sempre priorità su quelle importate in caso di collisione.

---

## 21. Multi-Ambiente e Bridge

`env.target` è sempre **statico**: descrive l'hardware fisico che il compilatore deve conoscere per generare il codice giusto (pin, RAM disponibile) — fisso nel momento in cui il programma viene flashato, non può dipendere da una condizione valutata a runtime.
```text
env {
    target: "arduino_uno";
}
```

Per un comportamento che il programma sceglie mentre gira (non l'hardware su cui gira, ma *come* si comporta) si usa `identity.mode`, una variabile normale — questa sì può dipendere da un'espressione:
```text
identity {
    let.const.string mode = if (time > 20.00) { "night" } else { "day" };
}
```

**Codice specifico per ambiente hardware**: un `if` normale su `env.target`, nessuna sintassi dedicata.
```text
if (env.target == "arduino_uno") {
    set_pin_mode(13, OUTPUT);
    display("LED Arduino acceso");
}
```
Hardware dichiarato su più ambienti senza essere specificato esplicitamente resta errore di compilazione.

**Bridge tra ambienti**: blocco con nome, comunicazione cifrata tra dispositivi. **Le chiavi non vivono mai nel sorgente**: il bridge referenzia solo il nome, il valore vero sta in `SecurityRule` (sezione 18), nel file `.sr` protetto da password OS — mai in un file `.spln` che finisce facilmente in un repository o in un log.

**Cifratura, due opzioni:**
| | `e2e_ecc` (default) | `e2e_rsa` |
|---|---|---|
| Algoritmo | Curve25519 | RSA-2048+ |
| Dimensione chiave | ~32 byte | ~256 byte |
| Costo su microcontrollore | Basso | Alto — spesso proibitivo senza coprocessore dedicato |
| Quando usarlo | Sempre, salvo motivo specifico | Solo per compatibilità con un sistema esterno che richiede RSA |

Se `encryption` non è specificato, il default è `e2e_ecc` — stessa filosofia di `unsafe` (sezione 27): sicurezza (qui, leggerezza) come default, l'opzione più pesante è una scelta esplicita, non quella in cui si inciampa per abitudine.
```text
bridge #telemetry {
    from: "arduino";
    to: "smartphone";
    method: "bluetooth";
    encryption: "e2e_ecc";   // o "e2e_rsa" se serve compatibilità con un sistema esterno
    keys: injected;   // vedi SecurityRule.bridge_keys, sezione 18
    data: [temperature, battery];
    frequency: 1000;
    timeout: 5000;
    on_offline: ConnectionPolicy::Continue;   // o ConnectionPolicy::Suspend
}
```
Il bridge non invia uno snapshot ma una **media pesata temporale** (i valori recenti contano di più, campionamento ogni `frequency/10` ms), accessibile con `telemetry.temperature`. Bridge multipli: più blocchi `bridge #nome { }`, nessuna sintassi speciale.

---

## 22. Gestione Memoria

Limite lunghezza stringhe basato sulla RAM disponibile — **moltiplicatori configurabili in `SecurityRule`**, non fissi nel linguaggio: sono limiti operativi, appartengono al file di sicurezza come ogni altro limite (sezione 18), non a una formula scolpita nel compilatore.
```text
SecurityRule {
    chunk_ram_ratio: 0.6;         // frazione di env.ram usata per CHUNK_SIZE
    max_string_multiplier: 2;     // moltiplicatore per MAX_STRING_LENGTH
}
```
```text
CHUNK_SIZE = env.ram * chunk_ram_ratio
MAX_STRING_LENGTH = CHUNK_SIZE * max_string_multiplier
```
Se `chunk_ram_ratio` produce un `CHUNK_SIZE` maggiore di `env.ram` stesso (impossibile da rispettare fisicamente), è **errore di compilazione**, non un valore silenziosamente limitato — stessa filosofia di ogni altro limite in SNACK: scoperto prima di far girare il programma, non a runtime.
La memoria si libera **tutta insieme, in modo deterministico**, quando il chunk che la contiene finisce (sezione 15) — nessuna garbage collection euristica a runtime.

**Non esiste `free()`.** Nessuna eccezione: l'unico modo per liberare memoria in SNACK è terminare il chunk che la contiene — niente via di mezzo tra "vivo" e "morto" per una singola variabile. Se serve liberare qualcosa prima della fine naturale di un chunk lungo (es. un buffer grande usato solo all'inizio), la soluzione è annidare quella parte in un chunk figlio più piccolo (sezione 15):
```text
#trip {
    let.mut.32bit distance = 0;

    #load_buffer {
        let.mut.array[1000].unsigned.16bit big_buffer;
        // ... uso big_buffer ...
    }   // #load_buffer finisce qui: big_buffer libero, distance ancora vivo in #trip

    display(distance);
}
```
Un'unica regola, senza eccezioni: la vita di una variabile è sempre esattamente la vita del chunk in cui è dichiarata — mai più corta, mai gestita a mano elemento per elemento.

**Nessuna scelta manuale tra stack e heap.** Quando una variabile ha dimensione sconosciuta a compile-time (es. `read_file()`, la cui lunghezza si conosce solo leggendo il file), l'arena del chunk che la contiene **si espande** invece di restare fissa — resta comunque un'unica arena, liberata tutta insieme alla fine del chunk, zero frammentazione tra variabili diverse. Il compilatore decide da solo, variabile per variabile, se la dimensione è nota subito (arena normale, costo zero) o dipende da qualcosa noto solo a runtime (arena espandibile) — stessa sintassi `let` in entrambi i casi, nessun tag `.stack`/`.heap` da scrivere.
```text
#load_config {
    let content = read_file("config.json");   // lunghezza nota solo a runtime
    display("Config caricata, byte:", length(content));
}   // arena di #load_config, inclusa l'espansione, liberata tutta insieme
```
Su target con RAM molto limitata (es. Arduino), se l'espansione potrebbe non entrarci il compilatore lo segnala come **errore in fase di compilazione**, non come crash scoperto a runtime.

---

## 23. Gestione Errori

Un solo stile: `try`/`catch`, familiare da JS.
```text
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

## 24. Checkpoint e Reset

**`checkpoint;`** salva un'istantanea di **tutte le variabili top-level di `fn main`** (quelle con scope persistente, sezione 2) — non lo stato delle periferiche hardware (pin, PWM, connessioni), che vengono ri-inizializzate da `env`/`capabilities` al riavvio, non ripristinate da uno snapshot.

```text
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

**Cosa salva `checkpoint` e cosa no — nel dettaglio, senza zone grigie:**

| | Salvato da `checkpoint` | Comportamento dopo crash/reset |
|---|---|---|
| Variabili top-level di `fn main` | Sì | Tornano al valore dell'ultimo checkpoint |
| Variabili dentro un chunk | No | Sempre azzerate — vivono e muoiono col chunk (sezione 15), un checkpoint non le può "congelare" |
| `rule{}` | No, separatamente | Persiste per conto suo in EEPROM (sopra), non serve un checkpoint |
| Funzioni, closure | No | Sono codice, non stato — nessun bisogno di essere "salvate", esistono già a compile-time |
| `listener`/`on_event` | No | Ripartono da zero: ogni condizione viene rivalutata da capo al riavvio, nessuno stato di "ero già attivo" da ripristinare |
| Periferiche hardware (pin, PWM, timer) | No | Ri-inizializzate da `env`/`capabilities`, mai da uno snapshot |
| Connessioni (UART, bridge, I2C/SPI) | No | Richiudi la connessione da zero — un canale di comunicazione non è un dato, ripristinarlo "a metà" da un crash sarebbe pericoloso, non utile |
| Stack dei chunk annidati | No | Il crash termina l'esecuzione del chunk corrente e di tutti i suoi figli; si riparte da `fn main`, non da dentro il chunk che ha causato il crash |

Il principio dietro la tabella è lo stesso di sempre: **solo ciò che è puro dato persistente ha senso salvare**; codice, connessioni e stato hardware non sono "dati da ricordare", sono cose che si ri-creano sempre allo stesso modo partendo da `env`/`capabilities`/`identity` — checkpointarli sarebbe ridondante nella migliore delle ipotesi, pericoloso nella peggiore (es. riprendere una scrittura UART a metà).

---

## 25. Ottimizzazioni Compiler e Runtime

Applicate automaticamente, nessuna azione richiesta:

**Compile-time:** constant folding, dead code elimination, common subexpression elimination, inlining di funzioni piccole, string interning.

**Runtime:** copy-on-write per le stringhe, small string optimization, arena allocator per chunk (sezione 15), variable reuse, lazy evaluation, watchdog automatico, auto-retry su operazioni hardware transitorie.

---

## 26. Tooling e Debug

```text
snack build main.spln --profile          // profiler tempo di esecuzione
snack build main.spln --memory-profile   // profiler memoria per chunk
snack build main.spln --static-analysis  // analisi statica, warning su codice sospetto
```
Errori di compilazione riportano file, riga, colonna e un messaggio descrittivo; disponibile anche in formato JSON per integrazione con altri strumenti.

---

## 27. Riferimenti e Puntatori Grezzi

**Filosofia:** un solo spazio di memoria, osservabile con due strumenti. `&T`/`&mut T` (sezione 14) sono la vista sorvegliata — il compilatore ne conosce nascita, vita e morte. `*T`/`*mut T` sono la vista nuda — un indirizzo, nessun controllo, il developer se ne assume la responsabilità. Fuori da `unsafe` esistono solo riferimenti; dentro `unsafe`, riferimenti e puntatori grezzi convivono e si convertono l'uno nell'altro.

**Puntatori grezzi**, per indirizzi fissi (registri hardware):
```text
unsafe {
    let.mut.32bit.ptr portb = pointer(0x24);   // 0x24: indirizzo del registro PORTB su AVR
    *portb = 255;
}
```
Ogni puntatore creato con `pointer()` è automaticamente `volatile`: il compilatore non riordina né elimina le letture/scritture su quell'indirizzo — essenziale quando due scritture identiche consecutive hanno un significato fisico diverso (es. due impulsi su un pin).

**Conversione tra `&` e `*`**, solo dentro `unsafe`:
```text
unsafe {
    let portb: &mut 32bit = &mut *pointer(0x24);   // da indirizzo grezzo a riferimento sicuro
    *portb = 255;
}
```
Il punto chiave: **il confine "non verificato" esiste solo dentro `unsafe`**. Nel momento in cui `&mut *pointer(0x24)` ridiventa un riferimento (`portb`), il borrow checker lo riprende in carico immediatamente — non può sopravvivere al chunk in cui è nato, non può coesistere con un altro `&mut` sullo stesso indirizzo, esattamente come un riferimento nato in modo ordinario. `unsafe` non è una zona franca permanente: è solo il punto di passaggio tra le due viste.

**`pointer()` non è una funzione qualunque — è un costruttore riconosciuto dal compilatore, tracciato per valore di indirizzo, non per chiamata.** Se fosse trattata come una funzione normale, due chiamate `pointer(0x24)` sarebbero due valori indipendenti agli occhi del borrow checker: due `&mut` distinti, nessun conflitto rilevato, mentre in realtà scriverebbero entrambi sullo stesso registro hardware — la stessa scrittura concorrente non sorvegliata che i riferimenti esistono per impedire. Il compilatore confronta invece il valore numerico dell'indirizzo (`0x24` e `36` sono lo stesso indirizzo, scritti in modo diverso) e rifiuta un secondo `&mut` attivo sullo stesso indirizzo, anche a partire da chiamate `pointer()` distinte:
```text
unsafe {
    let a = &mut *pointer(0x24);
    let b = &mut *pointer(0x24);   // ERRORE: 0x24 ha già un &mut attivo (tramite a)
}
```
La tracciatura vale per **tutto il modulo** autorizzato a `unsafe` (sezione `unsafe_allowed_modules` del `.sr`), non solo per un singolo blocco `unsafe` — altrimenti basterebbe scrivere due blocchi separati nello stesso file per aggirare il controllo.

**Permessi nel file di sicurezza**, con granularità **per modulo** — non un flag unico per tutto il progetto, coerente con `include()` (sezione 20):
```text
SecurityRule {
    allow_unsafe_blocks: false;      // default globale: niente unsafe
    allow_raw_pointers: false;
    allow_inline_asm: false;

    unsafe_allowed_modules: ["./hardware_driver.spln"];   // eccezione per modulo specifico
}
```
Un blocco `unsafe` in un file non elencato in `unsafe_allowed_modules` è errore di compilazione, anche se `allow_unsafe_blocks` fosse `true` globalmente — così solo il modulo che deve davvero parlare con l'hardware ottiene il permesso, non ogni file che lo importa.

**Niente indirezione multipla** (`**`, puntatore-a-puntatore) — né in codice safe né dentro `unsafe`. Non c'è una posizione per un secondo livello nell'ordine fisso dei tag (sezione 4), e la giustificazione tipica (una tabella di indirizzi hardware simili) è già coperta, in modo più semplice e senza introdurre un concetto nuovo, da un **array di puntatori singoli**:
```text
unsafe {
    let.const.array[4].32bit.ptr registers = [pointer(0x24), pointer(0x25), pointer(0x26), pointer(0x27)];

    for (i in 0..4) {
        *registers[i] = 0;
    }
}
```
Ogni elemento dell'array resta un puntatore semplice, tracciato per valore di indirizzo come ogni altro `pointer()` (sopra): niente ambiguità aggiuntiva, niente sintassi nuova da imparare.

---

## 28. Linked List, Grafi e Alberi

Una linked list in SNACK è un **array di nodi collegati da indici**, non una catena di puntatori sparsi — riusa esattamente array dinamico (sezione 6), struct (sezione 5) ed enum (sezione 7), niente di nuovo. Niente `malloc`/`free`, niente puntatori grezzi, niente indirezione multipla: l'array ha un `max` obbligatorio, quindi il compilatore sa quanta RAM serve, e quando il chunk finisce l'intero array muore in un colpo solo.

**Nodo**, con `Link` come enum invece di un numero sentinella (`-1`) — un indice che assomiglia a un numero valido è un errore in agguato; un tipo con due soli casi non lascia spazio all'ambiguità:
```text
enum Link {
    Node(32bit),   // indice del prossimo nodo
    End
}

struct Nodo {
    valore: let.mut.32bit,
    prossimo: let.mut.Link
}
```

**Creazione e lettura:**
```text
fn main {
    #lista {
        let.mut.array[? max 100].Nodo lista = [];

        push(lista, { valore: 10, prossimo: Link::End });
        push(lista, { valore: 20, prossimo: Link::End });
        push(lista, { valore: 30, prossimo: Link::End });

        lista[0].prossimo = Link::Node(1);
        lista[1].prossimo = Link::Node(2);

        let.mut.Link cursore = Link::Node(0);
        while (cursore != Link::End) {
            switch (cursore) {
                case Link::Node(i):
                    display(lista[i].valore);
                    cursore = lista[i].prossimo;
                    break;
                default:
                    break;
            }
        }
    }   // #lista finisce, l'array muore, zero frammentazione
}
```

**Inserimento**: creare il nuovo nodo puntando al vecchio successore, poi aggiornare il `prossimo` del nodo precedente — nessun nodo viene spostato, cambiano solo due collegamenti.
```text
push(lista, { valore: 10, prossimo: Link::Node(1) });
push(lista, { valore: 30, prossimo: Link::End });

push(lista, { valore: 20, prossimo: Link::Node(1) });   // nuovo nodo indice 2
lista[0].prossimo = Link::Node(2);
// Lista: 0 (10) -> 2 (20) -> 1 (30) -> fine
```

**Rimozione**: si salta il nodo, il precedente punta direttamente al successore del rimosso.
```text
lista[0].prossimo = lista[1].prossimo;   // salta il nodo 1
```

**`remove()` è vietato sugli array dichiarati come nodi di una linked list** (marcati con `struct` contenente un campo `Link`, verificato a compile-time). `remove()` sposta fisicamente gli elementi successivi di una posizione (sezione 6) — su un array-nodo, questo lascerebbe ogni `Link::Node(i)` esistente a puntare all'indice sbagliato, un bug silenzioso non rilevabile a compile-time perché gli indici sono numeri qualunque, non riferimenti tracciati.

**Free-list**, per non sprecare slot: uno scollegamento (`lista[0].prossimo = lista[1].prossimo;`) non sposta né cancella nulla — lascia lo slot occupato ma irraggiungibile dalla catena, esattamente come nell'esempio sopra. Invece di lasciarlo perso fino alla fine del chunk, `unlink()` lo registra automaticamente in una lista interna di slot liberi, che `push()` controlla per primo prima di far crescere l'array — nessun indice esistente si sposta mai, perché nessun nodo cambia posizione, solo lo stesso slot viene riscritto con un valore nuovo.
```text
lista[0].prossimo = lista[1].prossimo;   // scollega il nodo 1 dalla catena
unlink(lista, 1);                        // registra l'indice 1 come riutilizzabile

push(lista, { valore: 99, prossimo: Link::End });   // riusa l'indice 1 invece di crescere
```
Se non si chiama `unlink()`, lo slot resta occupato e inutilizzato fino alla fine del chunk (comportamento precedente) — `unlink()` è quindi la scelta consigliata ogni volta che si scollega un nodo con l'intenzione di non riusarlo più.

**Grafi e alberi binari non hanno bisogno di una struttura dedicata — sono lo stesso schema, riusato:**
```text
// Grafo: lista di adiacenza
struct NodoGrafo {
    valore: let.mut.32bit,
    vicini: let.mut.array[? max 8].32bit   // indici dei nodi collegati
}

// Albero binario: due Link invece di uno
struct NodoAlbero {
    valore: let.mut.32bit,
    sinistro: let.mut.Link,
    destro: let.mut.Link
}
```
Nessun concetto nuovo in nessuno dei due casi: `array[? max M].32bit` per i vicini di un nodo grafo, lo stesso `Link` già definito sopra per i figli di un nodo albero — solo composizione di quello che c'è già.

---

## 29. Esempio Completo

```text
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
