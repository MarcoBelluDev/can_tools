
# can_tools

Utilities for parsing and working with **automotive CAN** data in Rust.

## Features

- **DBC parser** – load `.dbc` databases into structured types.
- **ASC parser** – read `.asc` traces and build an in-memory `CanLog`.
- **Decoupled model**:
  - `CanFrame`: timing + channel + direction + pointer to `MessageLog`.
  - `MessageLog`: id, name, payload, comment, list of signal indices.
  - `SignalLog`: aggregated time-series for a decoded signal (`values: Vec<[timestamp, value]>`).
- **Helpers** – `resolve_message_signals` and `SignalLog::value_text_at(ts)`.
- Normalizes units by stripping the `"Unit_"` prefix when present.

## Quickstart

```rust
use can_tools::{dbc, asc, CanLog, resolve_message_signals};

fn main() -> Result<(), String> {
    // Load one or more DBCs and map them to channels
    let db = dbc::parse_from_file("path/to/file.dbc")?;
    let mut dbs = std::collections::HashMap::new();
    dbs.insert(1u8, db);

    // Parse an ASC trace
    let mut log = CanLog::default();
    let mut last_by_id_ch = std::collections::HashMap::new();
    let mut chart_by_key = std::collections::HashMap::new();
    asc::parse_from_file("path/to/file.asc", &mut log, &dbs, &mut last_by_id_ch, &mut chart_by_key)?;

    // Walk first frame
    let frame = &log.all_frame[0];
    let msg = &log.messages[frame.message];
    println!("id={} name={}", msg.id, msg.name);

    // Iterate message signals
    for sig in resolve_message_signals(&log, frame.message) {
        if let Some((v, txt)) = sig.value_text_at(frame.timestamp as f64) {
            println!("{} = {} ({})", sig.name, v, txt);
        }
    }
    Ok(())
}
```

## Design notes

- `CanFrame` is intentionally small: sorting and paging remain fast.
- Message and Signal data are centralized to avoid duplication and to make charting efficient.
- `SignalLog.values` uses pairs `[timestamp, value]` in seconds.
- `SignalLog.value_text_at(ts)` uses the signal's own `factor/offset` to reverse-map the raw integer and look up its text via `value_table`.

## License

MIT
