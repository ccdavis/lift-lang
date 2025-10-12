// Type conversion utilities for Cranelift code generation

use crate::syntax::DataType;
use crate::symboltable::SymbolTable;
use cranelift::prelude::*;
use cranelift_codegen::ir::StackSlot;

/// Information about a variable in the compiled code
#[derive(Clone, Copy)]
pub(crate) struct VarInfo {
    pub(crate) slot: StackSlot,
    pub(crate) cranelift_type: Type,  // I64, F64, or pointer type
}

/// Convert DataType to runtime type tag (matches constants in runtime.rs)
pub(crate) fn data_type_to_type_tag(data_type: &DataType) -> i8 {
    match data_type {
        DataType::Int => 0,      // TYPE_INT
        DataType::Flt => 1,      // TYPE_FLT
        DataType::Bool => 2,     // TYPE_BOOL
        DataType::Str => 3,      // TYPE_STR
        DataType::List { .. } => 4,  // TYPE_LIST
        DataType::Map { .. } => 5,   // TYPE_MAP
        DataType::Range(_) => 6,     // TYPE_RANGE
        DataType::Struct(_) => 7,    // TYPE_STRUCT
        _ => 0,  // Fallback to Int for unknown types
    }
}

/// Convert Lift DataType to Cranelift Type
pub(crate) fn data_type_to_cranelift_type(dt: &DataType, pointer_type: Type) -> Type {
    match dt {
        DataType::Int | DataType::Bool => types::I64,
        DataType::Flt => types::F64,
        DataType::Str => pointer_type,
        DataType::List { .. } => pointer_type,
        DataType::Map { .. } => pointer_type,
        DataType::Range(_) => pointer_type,
        DataType::Unsolved => types::I64,  // Fallback
        DataType::TypeRef(_) => pointer_type,  // User-defined types treated as pointers for now
        DataType::Optional(_) => pointer_type,  // Optionals treated as pointers
        DataType::Set(_) => pointer_type,
        DataType::Enum(_) => types::I64,  // Enums as integers
        DataType::Struct(_) => pointer_type,  // Structs as pointers
    }
}

/// Resolve type aliases to their underlying types
pub(crate) fn resolve_type_alias(data_type: &DataType, symbols: &SymbolTable) -> DataType {
    let mut resolved = data_type.clone();
    let mut visited = std::collections::HashSet::new();

    while let DataType::TypeRef(name) = &resolved {
        // Prevent infinite loops
        if !visited.insert(name.clone()) {
            break;
        }

        // Look up the type in all scopes (start from the deepest scope)
        let mut found = None;
        for scope_idx in (0..symbols.scope_count()).rev() {
            if let Some(underlying_type) = symbols.lookup_type(name, scope_idx) {
                found = Some(underlying_type);
                break;
            }
        }

        if let Some(underlying_type) = found {
            resolved = underlying_type;
        } else {
            // Type not found, leave as TypeRef
            break;
        }
    }

    resolved
}
