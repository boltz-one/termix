//! chat2 doc epoch (docs/chat2-sync.md M1): thin docs are lineage epoch 2;
//! M3 readers discard-and-adopt below it. The chat2 rooms/edge networking
//! (sinks, checkpoint fetchers, transports) that used to live here was
//! removed along with `termix_sync::chat_client`; chat persistence is local
//! via `DocsStore`.

/// Doc epoch stamped on every chat2-lineage snapshot (docs/chat2-sync.md M1:
/// thin docs are lineage epoch 2; M3 readers discard-and-adopt below it).
pub const CHAT2_DOC_EPOCH: u32 = 2;
