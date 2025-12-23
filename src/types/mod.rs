//! Core types for the InferaDB SDK.
//!
//! This module provides the fundamental types used throughout the SDK:
//!
//! - [`Relationship`]: Represents a relationship tuple (resource, relation, subject)
//! - [`Context`]: ABAC context for attribute-based conditions
//! - [`ConsistencyToken`]: Snapshot token for read-after-write consistency
//! - [`Decision`]: Authorization decision result with metadata
//! - [`Resource`]: Trait for types that can be used as resources
//! - [`Subject`]: Trait for types that can be used as subjects
//! - [`EntityRef`]: Parsed entity reference in "type:id" format
//! - [`SubjectRef`]: Subject reference with optional relation for usersets

// Allow dead code for types not yet integrated
#![allow(dead_code)]

mod consistency;
mod context;
mod decision;
mod entity;
mod relationship;

pub use consistency::ConsistencyToken;
pub use context::{Context, ContextValue};
pub use decision::{Decision, DecisionMetadata, DecisionReason};
pub use entity::{EntityRef, ParseError, Resource, Subject, SubjectRef};
pub use relationship::Relationship;
