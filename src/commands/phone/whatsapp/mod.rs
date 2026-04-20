//! WhatsApp channel — outbound voice calls, outbound messages, and the
//! `whatsapp-accounts` CRUD surface.
//!
//! Endpoints:
//!   - POST   /v1/convai/whatsapp/outbound-call
//!   - POST   /v1/convai/whatsapp/outbound-message
//!   - GET    /v1/convai/whatsapp-accounts
//!   - GET    /v1/convai/whatsapp-accounts/{id}
//!   - PATCH  /v1/convai/whatsapp-accounts/{id}
//!   - DELETE /v1/convai/whatsapp-accounts/{id}

pub mod accounts;
pub mod call;
pub mod message;
