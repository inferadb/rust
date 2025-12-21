//! Core types for the InferaDB SDK.
//!
//! This module provides the fundamental types used throughout the SDK:
//!
//! - [`Relationship`]: Represents a relationship tuple (resource, relation, subject)
//! - [`Context`]: ABAC context for attribute-based conditions
//! - [`ConsistencyToken`]: Snapshot token for read-after-write consistency
//! - [`Decision`]: Authorization decision result with metadata

mod consistency;
mod context;
mod decision;
mod relationship;

pub use consistency::ConsistencyToken;
pub use context::{Context, ContextValue};
pub use decision::{Decision, DecisionMetadata, DecisionReason};
pub use relationship::Relationship;
