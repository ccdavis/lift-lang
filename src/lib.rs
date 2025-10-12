// Lift language library
// This file exposes modules for integration tests

pub mod interpreter;
// pub mod semantic_analysis; // OLD - migrated to semantic/
pub mod semantic; // New modular semantic analysis
pub mod symboltable;
pub mod syntax;
pub mod compile_types;
pub mod runtime;
pub mod cranelift;
pub mod compiler;

// Re-export the grammar module
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);
