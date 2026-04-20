//! Batch calling — submit a batch, list workspace batches, show detail, and
//! cancel / retry / delete individual batches.
//!
//! Endpoints:
//!   - POST   /v1/convai/batch-calling/submit
//!   - GET    /v1/convai/batch-calling/workspace
//!   - GET    /v1/convai/batch-calling/{id}
//!   - POST   /v1/convai/batch-calling/{id}/cancel
//!   - POST   /v1/convai/batch-calling/{id}/retry
//!   - DELETE /v1/convai/batch-calling/{id}

pub mod cancel;
pub mod delete;
pub mod list;
pub mod retry;
pub mod show;
pub mod submit;
