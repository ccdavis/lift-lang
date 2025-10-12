// Lift language library
// This file exposes modules for integration tests

pub mod interpreter;
// pub mod semantic_analysis; // OLD - migrated to semantic/
pub mod compile_types;
pub mod compiler;
pub mod cranelift;
pub mod runtime;
pub mod semantic; // New modular semantic analysis
pub mod symboltable;
pub mod syntax;

// Re-export the grammar module
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);
