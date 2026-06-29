//! Lode source adapters — file / stdin / docker / journald ingestion (RFC-0001).
//! Each source implements the `SourceAdapter` trait (RFC-0014) and produces raw
//! `LogEvent`s with a `source_offset`; the durable cursor is owned by `lode-storage`.
