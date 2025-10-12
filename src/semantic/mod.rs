// Semantic Analysis Module
// This module handles type checking, symbol table population, and semantic validation

use crate::symboltable::SymbolTable;
use crate::syntax::DataType;

// Export submodules
pub mod symbol_processing;
pub mod type_inference;
pub mod typecheck;
pub mod typecheck_collections;
pub mod typecheck_control;
pub mod typecheck_expr;
pub mod typecheck_functions;
pub mod typecheck_structs;

// Re-export main public functions
pub use symbol_processing::add_symbols;
pub use type_inference::{determine_type, determine_type_with_symbols};
pub use typecheck::typecheck;

// Debug flag for semantic analysis
pub const DEBUG: bool = false;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Clone, Debug)]
pub enum CompileErrorType {
    Structure,
    Name,
    TypeCheck,
}

impl CompileErrorType {
    pub fn name(&self) -> String {
        match self {
            CompileErrorType::TypeCheck => "Type check Error",
            CompileErrorType::Name => "Name Error",
            CompileErrorType::Structure => "Structure Error",
        }
        .to_string()
    }
}

#[derive(Debug, Clone)]
pub struct CompileError {
    error_type: CompileErrorType,
    location: (usize, usize),
    msg: String,
}

impl CompileError {
    pub fn structure(msg: &str, location: (usize, usize)) -> Self {
        Self {
            error_type: CompileErrorType::Structure,
            location,
            msg: msg.to_string(),
        }
    }

    pub fn name(msg: &str, location: (usize, usize)) -> Self {
        Self {
            error_type: CompileErrorType::Name,
            location,
            msg: msg.to_string(),
        }
    }

    pub fn typecheck(msg: &str, location: (usize, usize)) -> Self {
        Self {
            error_type: CompileErrorType::TypeCheck,
            location,
            msg: msg.to_string(),
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, column) = self.location;
        write!(
            f,
            "{}: {}, {}: {}",
            &self.error_type.name(),
            line,
            column,
            self.msg
        )
    }
}

impl std::error::Error for CompileError {}

// ============================================================================
// Type Resolution Helpers
// ============================================================================

/// Helper: Resolve TypeRef to underlying type (internal use only)
pub(crate) fn resolve_type_alias(data_type: &DataType, symbols: &SymbolTable) -> DataType {
    let mut resolved = data_type.clone();
    let mut visited = std::collections::HashSet::new();

    while let DataType::TypeRef(name) = &resolved {
        // Prevent infinite loops
        if !visited.insert(name.clone()) {
            break;
        }

        // Look up the type using lookup_type
        if let Some(underlying_type) = symbols.lookup_type(name, 0) {
            resolved = underlying_type;
        } else {
            // Type not found, leave as TypeRef
            break;
        }
    }

    resolved
}

/// Resolve TypeRef to actual types (public API)
pub fn resolve_type(
    data_type: &DataType,
    symbols: &SymbolTable,
    scope: usize,
) -> Result<DataType, CompileError> {
    match data_type {
        DataType::TypeRef(name) => match symbols.lookup_type(name, scope) {
            Some(resolved_type) => Ok(resolved_type),
            None => Err(CompileError::name(
                &format!("Unknown type: {}", name),
                (0, 0),
            )),
        },
        DataType::List { element_type } => {
            let resolved_element = resolve_type(element_type, symbols, scope)?;
            Ok(DataType::List {
                element_type: Box::new(resolved_element),
            })
        }
        DataType::Map {
            key_type,
            value_type,
        } => {
            let resolved_key = resolve_type(key_type, symbols, scope)?;
            let resolved_value = resolve_type(value_type, symbols, scope)?;
            Ok(DataType::Map {
                key_type: Box::new(resolved_key),
                value_type: Box::new(resolved_value),
            })
        }
        DataType::Set(element_type) => {
            let resolved_element = resolve_type(element_type, symbols, scope)?;
            Ok(DataType::Set(Box::new(resolved_element)))
        }
        DataType::Optional(inner_type) => {
            let resolved_inner = resolve_type(inner_type, symbols, scope)?;
            Ok(DataType::Optional(Box::new(resolved_inner)))
        }
        // Built-in types and others pass through unchanged
        _ => Ok(data_type.clone()),
    }
}

// ============================================================================
// Type Compatibility Helpers
// ============================================================================

/// Helper function to check if two types are compatible
pub(crate) fn types_compatible(t1: &DataType, t2: &DataType) -> bool {
    match (t1, t2) {
        (DataType::Unsolved, _) | (_, DataType::Unsolved) => true,
        (DataType::Int, DataType::Int) => true,
        (DataType::Flt, DataType::Flt) => true,
        (DataType::Str, DataType::Str) => true,
        (DataType::Bool, DataType::Bool) => true,
        (DataType::Int, DataType::Flt) | (DataType::Flt, DataType::Int) => true, // Numeric compatibility
        (DataType::List { element_type: e1 }, DataType::List { element_type: e2 }) => {
            types_compatible(e1, e2)
        }
        (
            DataType::Map {
                key_type: k1,
                value_type: v1,
            },
            DataType::Map {
                key_type: k2,
                value_type: v2,
            },
        ) => types_compatible(k1, k2) && types_compatible(v1, v2),
        (DataType::Range(_), DataType::Range(_)) => {
            // For now, all ranges are compatible with each other
            // In the future we might want to check the bounds
            true
        }
        // Struct compatibility - check field names and types match
        (DataType::Struct(params1), DataType::Struct(params2)) => {
            if params1.len() != params2.len() {
                return false;
            }
            // Check that all field names and types match (order matters)
            params1.iter().zip(params2.iter()).all(|(p1, p2)| {
                p1.name == p2.name && types_compatible(&p1.data_type, &p2.data_type)
            })
        }
        // TypeRef compatibility - same name means compatible
        (DataType::TypeRef(name1), DataType::TypeRef(name2)) => name1 == name2,
        _ => false,
    }
}
