//! CSS parsing + scoping.

mod engine;
mod parser;

pub(crate) use engine::{ScopeEngineKind, parse_and_scope_with_engine};
pub(crate) use parser::parse_and_scope;
