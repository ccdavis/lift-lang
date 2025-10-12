// Cranelift JIT compilation for Lift language
//
// This module provides native code generation using the Cranelift compiler backend.
// The code is organized into topical submodules for maintainability.

// Type system and conversions
mod types;

// Core code generator
mod codegen;
pub use codegen::CodeGenerator;

// Runtime function declarations
mod runtime;

// Compilation methods organized by topic
mod collections;
mod expressions;
mod functions;
mod structs;
mod variables;
