// Type mapping from Lift types to Cranelift types

use crate::syntax::DataType;
use cranelift::prelude::types;
use cranelift::prelude::Type;

/// Maps Lift DataType to Cranelift IR Type
pub fn lift_type_to_cranelift(data_type: &DataType) -> Result<Type, String> {
    match data_type {
        DataType::Int => Ok(types::I64),
        DataType::Flt => Ok(types::F64),
        DataType::Bool => Ok(types::I8),

        // Heap-allocated types are represented as pointers (I64)
        DataType::Str => Ok(types::I64),
        DataType::List { .. } => Ok(types::I64),
        DataType::Map { .. } => Ok(types::I64),
        DataType::Range { .. } => Ok(types::I64),

        DataType::Optional(inner) => {
            // For now, treat optional as pointer (nullable)
            lift_type_to_cranelift(inner)
        }

        DataType::TypeRef(name) => {
            // User-defined types should be resolved by type checker
            Err(format!(
                "TypeRef {} should be resolved before compilation",
                name
            ))
        }

        DataType::Unsolved => Err("Unsolved type should not reach compilation stage".to_string()),

        DataType::Set(_) => Ok(types::I64),
        DataType::Enum(_) => Ok(types::I64),
        DataType::Struct(_) => Ok(types::I64),
    }
}

/// Returns true if the type requires heap allocation
pub fn is_heap_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Str
            | DataType::List { .. }
            | DataType::Map { .. }
            | DataType::Range { .. }
            | DataType::Set(_)
            | DataType::Enum(_)
            | DataType::Struct(_)
    )
}

/// Returns true if the type is a primitive value type
pub fn is_value_type(data_type: &DataType) -> bool {
    matches!(data_type, DataType::Int | DataType::Flt | DataType::Bool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(lift_type_to_cranelift(&DataType::Int).unwrap(), types::I64);
        assert_eq!(lift_type_to_cranelift(&DataType::Flt).unwrap(), types::F64);
        assert_eq!(lift_type_to_cranelift(&DataType::Bool).unwrap(), types::I8);
    }

    #[test]
    fn test_heap_types() {
        assert_eq!(lift_type_to_cranelift(&DataType::Str).unwrap(), types::I64);
        assert!(is_heap_type(&DataType::Str));
        assert!(!is_value_type(&DataType::Str));
    }

    #[test]
    fn test_value_types() {
        assert!(is_value_type(&DataType::Int));
        assert!(is_value_type(&DataType::Flt));
        assert!(is_value_type(&DataType::Bool));
        assert!(!is_heap_type(&DataType::Int));
    }
}
