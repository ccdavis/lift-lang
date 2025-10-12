// Cranelift JIT compilation for Lift language
//
// This module provides native code generation using the Cranelift compiler backend.
// The code is organized into topical submodules for maintainability.

// Type system and conversions
mod types;
pub(crate) use types::{VarInfo, data_type_to_type_tag};

// Core code generator
mod codegen;
pub use codegen::CodeGenerator;

// Runtime function declarations
mod runtime;

// Compilation methods organized by topic
mod expressions;
mod variables;
mod collections;
mod structs;
mod functions;
