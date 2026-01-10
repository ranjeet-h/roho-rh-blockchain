//! JSON-RPC API Module
//! 
//! Provides HTTP interface for external applications to query the node.

mod methods;
mod server;

pub use methods::*;
pub use server::*;
