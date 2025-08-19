//! DBC data model.
//!
//! Questo modulo definisce i tipi “DB-side” usati per rappresentare un database CAN
//! (file `.dbc` o `.arxml`) una volta parsato. I tipi qui descritti sono pensati per:
//! - Navigare messaggi, segnali e nodi (ECU);
//! - Effettuare ricerche veloci tramite lookup normalizzati;
//! - Fornire utilità per l’estrazione/decodifica del valore grezzo di un segnale
//!   a partire da un payload di byte.

use std::collections::HashMap;

use crate::types::canlog::SignalLog;

// --- Typed indices (semplici wrapper; si possono evolvere in newtype robusti in futuro) ---

/// Identificatore indicizzato di un nodo (ECU) all’interno di `Database.nodes`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct NodeId(pub usize);

/// Identificatore indicizzato di un messaggio all’interno di `Database.messages`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct MessageId(pub usize);

/// Identificatore indicizzato di un segnale all’interno di `Database.signals`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct SignalId(pub usize);

/// Rappresentazione in memoria di un database CAN (DBC/ARXML).
///
/// Contiene metadati (nome, tipo bus, baudrate, versione), gli elenchi di nodi/messaggi/segnali
/// e alcuni dizionari di lookup normalizzati per ricerche efficienti.
///
/// ### Lookup interni
/// - `msg_by_id`: ricerca per CAN ID numerico (`u64`);
/// - `msg_by_hex`: ricerca per CAN ID in forma esadecimale normalizzata (`"0x..."`, maiuscolo);
/// - `msg_by_name`: ricerca per nome messaggio **case-insensitive** (chiave in minuscolo);
/// - `node_by_name`: ricerca per nome nodo **case-insensitive** (chiave in minuscolo).
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Database {
    // --- Informazioni generali ---
    /// Nome logico del database (se disponibile).
    pub name: String,
    /// Tipo di bus (es. `"CAN"`).
    pub bustype: String,
    /// Baudrate classico (bit/s). `0` se non specificato.
    pub baudrate: u32,
    /// Baudrate CAN FD (bit/s). `0` se non specificato.
    pub baudrate_canfd: u32,
    /// Stringa di versione del database.
    pub version: String,

    // --- Storage principale (liste indicizzate) ---
    /// Elenco dei nodi/ECU presenti.
    pub nodes: Vec<NodeDB>,
    /// Elenco dei messaggi definiti.
    pub messages: Vec<MessageDB>,
    /// Elenco dei segnali definiti.
    pub signals: Vec<SignalDB>,

    // --- Lookup interni (chiavi normalizzate) ---
    msg_by_id: HashMap<u64, MessageId>,
    msg_by_hex: HashMap<String, MessageId>,  // esadecimale normalizzato “0x…”, maiuscolo
    msg_by_name: HashMap<String, MessageId>, // nome messaggio in minuscolo
    node_by_name: HashMap<String, NodeId>,   // nome nodo in minuscolo
}

impl Database {
    // ---- Adders: mantengono le relazioni e gli indici coerenti ----

    /// Aggiunge un nodo al database e restituisce il relativo `NodeId`.
    ///
    /// Aggiorna automaticamente il lookup `node_by_name` (case-insensitive).
    pub fn add_node(&mut self, node: NodeDB) -> NodeId {
        let id: NodeId = NodeId(self.nodes.len());
        let key: String = node.name.to_lowercase();
        self.nodes.push(node);
        self.node_by_name.insert(key, id);
        id
    }

    /// Aggiunge un messaggio e ne indicizza id/nome.
    ///
    /// Aggiorna:
    /// - `msg_by_id` con l’ID numerico;
    /// - `msg_by_hex` con l’ID esadecimale **normalizzato**;
    /// - `msg_by_name` con il nome in minuscolo.
    ///
    /// In più, registra il messaggio tra i `messages_sent` di ciascun nodo trasmittente.
    pub fn add_message(&mut self, mut msg: MessageDB) -> MessageId {
        let id: MessageId = MessageId(self.messages.len());

        // normalizza e indicizza id/nome
        let hex: String = normalize_id_hex(&msg.id_hex);
        msg.id_hex = hex.clone();
        self.msg_by_id.insert(msg.id, id);
        self.msg_by_hex.insert(hex, id);
        self.msg_by_name.insert(msg.name.to_lowercase(), id);

        // back-reference: dai nodi mittenti al messaggio
        for &nid in &msg.sender_nodes {
            if let Some(node) = self.nodes.get_mut(nid.0) {
                node.messages_sent.push(id);
            }
        }

        self.messages.push(msg);
        id
    }

    /// Aggiunge un segnale e lo collega al messaggio padre (`MessageDB.signals`).
    pub fn add_signal(&mut self, sig: SignalDB) -> SignalId {
        let id: SignalId = SignalId(self.signals.len());

        // collega il segnale al suo messaggio
        let midx: MessageId = sig.message;
        if let Some(msg) = self.messages.get_mut(midx.0) {
            msg.signals.push(id);
        }

        self.signals.push(sig);
        id
    }

    /// Ripulisce completamente il database (metadati, liste e lookup).
    pub fn clear(&mut self) {
        self.name.clear();
        self.bustype.clear();
        self.baudrate = 0;
        self.baudrate_canfd = 0;
        self.version.clear();

        self.nodes.clear();
        self.messages.clear();
        self.signals.clear();
        self.msg_by_id.clear();
        self.msg_by_hex.clear();
        self.msg_by_name.clear();
        self.node_by_name.clear();
    }

    // ---- Public accessors ----

    /// Restituisce un `&MessageDB` dato il CAN ID numerico.
    pub fn get_message_by_id(&self, id: u64) -> Option<&MessageDB> {
        self.msg_by_id.get(&id).map(|&mid| &self.messages[mid.0])
    }

    /// Restituisce un `&mut MessageDB` dato il CAN ID numerico.
    pub fn get_message_by_id_mut(&mut self, id: u64) -> Option<&mut MessageDB> {
        if let Some(&mid) = self.msg_by_id.get(&id) {
            self.messages.get_mut(mid.0)
        } else {
            None
        }
    }

    /// Restituisce un `&MessageDB` dato l’ID esadecimale (case-insensitive).
    ///
    /// L’argomento può essere in forme varie, ad es. `"12dd54e3"`, `"0x12dd54e3"`, `"12DD54E3x"`;
    /// sarà normalizzato internamente a `"0x12DD54E3"`.
    pub fn get_message_by_id_hex(&self, id_hex: &str) -> Option<&MessageDB> {
        let key: String = normalize_id_hex(id_hex);
        self.msg_by_hex.get(&key).map(|&mid| &self.messages[mid.0])
    }

    /// Restituisce un `&mut MessageDB` dato l’ID esadecimale (case-insensitive).
    pub fn get_message_by_id_hex_mut(&mut self, id_hex: &str) -> Option<&mut MessageDB> {
        let key: String = normalize_id_hex(id_hex);
        if let Some(&mid) = self.msg_by_hex.get(&key) {
            self.messages.get_mut(mid.0)
        } else {
            None
        }
    }

    /// Restituisce un `&MessageDB` dato il nome (case-insensitive).
    pub fn get_message_by_name(&self, name: &str) -> Option<&MessageDB> {
        self.msg_by_name
            .get(&name.to_lowercase())
            .map(|&mid| &self.messages[mid.0])
    }

    /// Restituisce un `&mut MessageDB` dato il nome (case-insensitive).
    pub fn get_message_by_name_mut(&mut self, name: &str) -> Option<&mut MessageDB> {
        if let Some(&mid) = self.msg_by_name.get(&name.to_lowercase()) {
            self.messages.get_mut(mid.0)
        } else {
            None
        }
    }

    /// Restituisce un `&NodeDB` dato il nome (case-insensitive).
    ///
    /// _Nota_: il nome del metodo è al plurale per ragioni di retrocompatibilità,
    /// ma restituisce un singolo nodo se presente.
    pub fn get_nodes_by_name(&self, name: &str) -> Option<&NodeDB> {
        self.node_by_name
            .get(&name.to_lowercase())
            .map(|&nid| &self.nodes[nid.0])
    }

    /// Restituisce un `&mut NodeDB` dato il nome (case-insensitive).
    ///
    /// _Nota_: il nome del metodo è al plurale per ragioni di retrocompatibilità,
    /// ma restituisce un singolo nodo se presente.
    pub fn get_nodes_by_name_mut(&mut self, name: &str) -> Option<&mut NodeDB> {
        if let Some(&nid) = self.node_by_name.get(&name.to_lowercase()) {
            self.nodes.get_mut(nid.0)
        } else {
            None
        }
    }

    /// Restituisce l’`NodeId` di un nodo dato il nome (case-insensitive).
    pub fn get_node_id_by_name(&self, name: &str) -> Option<NodeId> {
        self.node_by_name.get(&name.to_lowercase()).copied()
    }
}

/// Messaggio CAN definito nel database (DBC/ARXML).
///
/// Mantiene l’ID numerico (`id`), l’ID in esadecimale normalizzato (`id_hex`),
/// il `name`, la lunghezza del payload (`byte_length`) e metadati come `msgtype`, `cycle_time`,
/// i nodi trasmittenti (`sender_nodes`) e la lista dei segnali (`signals`) che lo compongono.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct MessageDB {
    /// CAN ID numerico (base 10).
    pub id: u64,
    /// CAN ID esadecimale **normalizzato** (`"0x..."`, maiuscolo).
    pub id_hex: String,
    /// Nome del messaggio.
    pub name: String,
    /// Lunghezza payload in byte.
    pub byte_length: u16,
    /// Tipo messaggio (campo libero; se presente nel DBC).
    pub msgtype: String,
    /// Tempo di ciclo in millisecondi (se definito; 0 se non noto).
    pub cycle_time: u16,
    /// Nodi (ECU) mittenti per questo messaggio.
    pub sender_nodes: Vec<NodeId>,
    /// Segnali che appartengono a questo messaggio.
    pub signals: Vec<SignalId>,
    /// Commento associato (sezione `CM_ BO_` nel DBC).
    pub comment: String,
}

impl MessageDB {
    /// Reimposta tutti i campi ai valori di default.
    pub fn clear(&mut self) {
        self.id = 0;
        self.id_hex.clear();
        self.name.clear();
        self.byte_length = 0;
        self.msgtype.clear();
        self.cycle_time = 0;
        self.sender_nodes.clear();
        self.signals.clear();
        self.comment.clear();
    }

    /// Iteratore di comodo sui `SignalDB` appartenenti a questo messaggio.
    ///
    /// Esempio:
    /// ```
    /// # use can_tools::types::database::{Database, MessageDB, SignalDB, MessageId, NodeDB};
    /// # let db = Database::default();
    /// # let msg = MessageDB::default();
    /// # let _ = msg.signals(&db).count();
    /// ```
    pub fn signals<'a>(&'a self, db: &'a Database) -> impl Iterator<Item = &'a SignalDB> + 'a {
        self.signals
            .iter()
            .filter_map(move |&sid| db.signals.get(sid.0))
    }
}

/// Step elementare per l’estrazione di un campo di bit da un payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Step {
    /// Indice del byte sorgente.
    pub(crate) byte_index: u8,
    /// LSB all’interno del byte sorgente (0..7).
    pub(crate) src_lsb: u8,
    /// Numero di bit da prelevare (1..8).
    pub(crate) width: u8,
    /// LSB di destinazione nel valore finale (LSB-first).
    pub(crate) dst_lsb: u16,
}

/// Definizione di un segnale all’interno di un messaggio CAN (DBC).
///
/// Descrive posizione/bit-length, endianness, segno, scalatura (factor/offset),
/// range valido, unità di misura, tabelle di valori e nodi riceventi.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct SignalDB {
    /// Messaggio padre (indice in `Database.messages`).
    pub message: MessageId,
    /// Nome del segnale.
    pub name: String,
    /// Bit start nel payload (bit 0 = LSB del primo byte).
    pub bit_start: u16,
    /// Lunghezza in bit.
    pub bit_length: u16,
    /// Endianness: `1` = little-endian (Intel), `0` = big-endian (Motorola).
    pub endian: u8,
    /// Segno: `1` = signed, `0` = unsigned.
    pub sign: u8,
    /// Fattore di scalatura.
    pub factor: f64,
    /// Offset di scalatura.
    pub offset: f64,
    /// Valore minimo fisico.
    pub min: f64,
    /// Valore massimo fisico.
    pub max: f64,
    /// Unità di misura (normalizzata altrove rimuovendo l’eventuale prefisso `"Unit_"`).
    pub unit_of_measurement: String,
    /// Nodi riceventi.
    pub receiver_nodes: Vec<NodeId>,
    /// Commento associato (sezione `CM_ SG_` nel DBC).
    pub comment: String,
    /// Tabella di mapping valore→testo (value table).
    pub value_table: HashMap<i32, String>,
    /// Sequenza di step precalcolati per l’estrazione veloce.
    pub(crate) steps: Vec<Step>,
}

impl SignalDB {
    /// Restituisce un riferimento immutabile a un nodo ricevente dato il nome (case-insensitive).
    pub fn get_receiver_nodes_by_name<'a>(
        &self,
        db: &'a Database,
        name: &str,
    ) -> Option<&'a NodeDB> {
        let key: String = name.to_lowercase();
        self.receiver_nodes
            .iter()
            .filter_map(|&nid| db.nodes.get(nid.0))
            .find(|node| node.name.to_lowercase() == key)
    }

    /// Restituisce un riferimento mutabile a un nodo ricevente dato il nome (case-insensitive).
    pub fn get_receiver_nodes_by_name_mut<'a>(
        &self,
        db: &'a mut Database,
        name: &str,
    ) -> Option<&'a mut NodeDB> {
        let key: String = name.to_lowercase();
        let nid = self.receiver_nodes.iter().copied().find(|&nid| {
            db.nodes
                .get(nid.0)
                .map(|n| n.name.to_lowercase() == key)
                .unwrap_or(false)
        })?;
        db.nodes.get_mut(nid.0)
    }

    /// Precalcola gli step di estrazione bit → valore per accelerare la decodifica.
    pub fn compile_inline(&mut self) {
        if !self.steps.is_empty() {
            return;
        }
        // ceil((bit_len + (bit_start % 8)) / 8)
        let n_steps: usize = (self.bit_length as usize + (self.bit_start as usize & 7))
            .div_ceil(8)
            .max(1);
        self.steps.reserve_exact(n_steps);

        if self.endian == 1 {
            self.compile_intel();
        } else {
            self.compile_motorola();
        }
    }

    #[inline]
    fn push_step(&mut self, st: Step) {
        self.steps.push(st);
    }

    /// Compilazione degli step per segnali little-endian (Intel).
    fn compile_intel(&mut self) {
        let mut remaining: u16 = self.bit_length;
        let mut bit: u16 = self.bit_start;
        let mut dst: u16 = 0u16;

        while remaining > 0 {
            let byte_idx: u8 = (bit / 8) as u8;
            let bit_off: u8 = (bit % 8) as u8;
            let avail: u8 = 8 - bit_off;
            let take: u8 = remaining.min(avail as u16) as u8;

            self.push_step(Step {
                byte_index: byte_idx,
                src_lsb: bit_off,
                width: take,
                dst_lsb: dst,
            });

            bit += take as u16;
            dst += take as u16;
            remaining -= take as u16;
        }
    }

    /// Compilazione degli step per segnali big-endian (Motorola).
    fn compile_motorola(&mut self) {
        // In DBC, @0: il bit di start è l’MSB del segnale; si avanza MSB-first.
        let mut remaining: u16 = self.bit_length;
        let mut byte: usize = (self.bit_start / 8) as usize;
        let mut bit_msb: u8 = 7 - (self.bit_start % 8) as u8;

        while remaining > 0 {
            let can_take: u16 = (bit_msb as u16 + 1).min(remaining);
            let src_lsb: u8 = bit_msb + 1 - can_take as u8;
            let dst_lsb: u16 = remaining - can_take;

            self.push_step(Step {
                byte_index: byte as u8,
                src_lsb,
                width: can_take as u8,
                dst_lsb,
            });

            remaining -= can_take;
            if src_lsb == 0 {
                byte += 1;
                bit_msb = 7;
            } else {
                bit_msb = src_lsb - 1;
            }
        }
    }

    /// Estrae il valore grezzo **unsigned** (accumulo LSB-first) dal payload.
    #[inline]
    pub fn extract_raw_u64(&self, bytes: &[u8]) -> u64 {
        let mut out: u64 = 0;
        for st in &self.steps {
            if let Some(&b) = bytes.get(st.byte_index as usize) {
                let mask: u8 = if st.width == 8 {
                    0xFF
                } else {
                    ((1u16 << st.width) - 1) as u8
                };
                let chunk = ((b >> st.src_lsb) & mask) as u64;
                out |= chunk << st.dst_lsb;
            }
        }
        out
    }

    /// Estrae il valore grezzo **signed** dal payload eseguendo sign-extension se necessario.
    #[inline]
    pub fn extract_raw_i64(&self, bytes: &[u8]) -> i64 {
        let raw_u: u64 = self.extract_raw_u64(bytes);
        let n: u16 = self.bit_length.min(64);
        if self.sign == 1 && n > 0 {
            let sign_bit = 1u64 << (n - 1);
            if (raw_u & sign_bit) != 0 {
                let mask = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
                (raw_u | !mask) as i64
            } else {
                raw_u as i64
            }
        } else {
            raw_u as i64
        }
    }

    /// Converte un valore grezzo in un `SignalLog` “istantaneo” con valore fisico, testo e metadati.
    ///
    /// *Nota*: l’unità viene normalizzata rimuovendo un eventuale prefisso `"Unit_"`.
    #[inline]
    pub fn to_sigframe(&self, raw_i: i64) -> SignalLog {
        let value: f64 = (raw_i as f64) * self.factor + self.offset;
        let text: String = self
            .value_table
            .get(&(raw_i as i32))
            .cloned()
            .unwrap_or_default();
        SignalLog {
            message: 0,
            name: self.name.clone(),
            factor: self.factor,
            offset: self.offset,
            channel: 0,
            raw: raw_i,
            value,
            unit: self
                .unit_of_measurement
                .strip_prefix("Unit_")
                .unwrap_or(&self.unit_of_measurement)
                .to_string(),
            text,
            comment: self.comment.clone(),
            value_table: self.value_table.clone(),
            values: Vec::new(),
        }
    }
}

/// Nodo/ECU definito nel database.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct NodeDB {
    /// Nome del nodo/ECU.
    pub name: String,
    /// Commento associato (se presente).
    pub comment: String,
    /// Messaggi trasmessi da questo nodo.
    pub messages_sent: Vec<MessageId>,
}

impl NodeDB {
    /// Ripulisce tutti i campi ai valori di default.
    pub fn clear(&mut self) {
        self.name.clear();
        self.comment.clear();
        self.messages_sent.clear();
    }
}

// --- helpers ---

/// Normalizza una stringa di ID esadecimale.
///
/// Converte varianti come `"12DD54E3x"`, `"0x12dd54e3"`, `"12dd54e3"`
/// nella forma canonica `"0x12DD54E3"`.
fn normalize_id_hex(s: &str) -> String {
    let t: &str = s.trim();
    let t: &str = t
        .strip_suffix('x')
        .or_else(|| t.strip_suffix('X'))
        .unwrap_or(t);
    let t: &str = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    format!("0x{}", t.to_uppercase())
}
