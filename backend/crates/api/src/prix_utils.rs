//! Shim — prix_utils désormais dans `data` (phase 1.6d3).
//! NOTE : `client_http()` a été supprimé du code déplacé (plus d'accès à
//! `api::http_client`). Les consumers api utilisent directement
//! `&*crate::http_client::HTTP_CLIENT`.
pub use data::prix_utils::*;
