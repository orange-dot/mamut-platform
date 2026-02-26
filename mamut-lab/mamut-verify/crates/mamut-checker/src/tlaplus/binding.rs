//! Binding engine for mapping runtime events to TLA+ state.
//!
//! This module provides the machinery to map between runtime observations
//! (operations, events, state changes) and TLA+ specification elements
//! (variables, actions, state predicates).
//!
//! # Example
//!
//! ```rust,ignore
//! use mamut_checker::tlaplus::binding::{SpecBinding, VariableMapping, OperationMapping};
//!
//! let binding = SpecBinding::new("KVStore")
//!     .with_variable_mapping(
//!         VariableMapping::new("store", "kv_store")
//!             .with_transform(VariableTransform::JsonToTla)
//!     )
//!     .with_operation_mapping(
//!         OperationMapping::new("Put", "KVPut")
//!             .with_param_mapping("key", "k")
//!             .with_param_mapping("value", "v")
//!     );
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// A unique identifier for a TLA+ specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecId(pub String);

impl SpecId {
    /// Create a new specification ID.
    pub fn new(id: impl Into<String>) -> Self {
        SpecId(id.into())
    }
}

impl fmt::Display for SpecId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SpecId {
    fn from(s: &str) -> Self {
        SpecId(s.to_string())
    }
}

impl From<String> for SpecId {
    fn from(s: String) -> Self {
        SpecId(s)
    }
}

/// Errors that can occur during binding operations.
#[derive(Debug, Error)]
pub enum BindingError {
    /// A required variable mapping is missing.
    #[error("Missing variable mapping for '{0}'")]
    MissingVariable(String),

    /// A required operation mapping is missing.
    #[error("Missing operation mapping for '{0}'")]
    MissingOperation(String),

    /// Variable transformation failed.
    #[error("Variable transform failed for '{variable}': {message}")]
    TransformError { variable: String, message: String },

    /// Parameter mapping failed.
    #[error("Parameter mapping failed for operation '{operation}', param '{param}': {message}")]
    ParamError {
        operation: String,
        param: String,
        message: String,
    },

    /// Value conversion failed.
    #[error("Value conversion failed: {0}")]
    ConversionError(String),

    /// Invalid binding configuration.
    #[error("Invalid binding configuration: {0}")]
    InvalidConfig(String),
}

/// A binding between runtime events and a TLA+ specification.
///
/// This structure defines how to map:
/// - Runtime state to TLA+ variables
/// - Runtime operations to TLA+ actions
/// - Runtime values to TLA+ values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecBinding {
    /// The identifier of the TLA+ specification this binding is for.
    pub spec_id: SpecId,

    /// The name of the specification module.
    pub module_name: Option<String>,

    /// Mappings from runtime variables to TLA+ variables.
    pub variable_mappings: Vec<VariableMapping>,

    /// Mappings from runtime operations to TLA+ actions.
    pub operation_mappings: Vec<OperationMapping>,

    /// Constants to pass to TLC.
    pub constants: HashMap<String, serde_json::Value>,

    /// Custom type mappings.
    pub type_mappings: Vec<TypeMapping>,

    /// Description of this binding.
    pub description: Option<String>,
}

impl SpecBinding {
    /// Create a new specification binding.
    pub fn new(spec_id: impl Into<SpecId>) -> Self {
        SpecBinding {
            spec_id: spec_id.into(),
            module_name: None,
            variable_mappings: Vec::new(),
            operation_mappings: Vec::new(),
            constants: HashMap::new(),
            type_mappings: Vec::new(),
            description: None,
        }
    }

    /// Set the module name.
    pub fn with_module_name(mut self, name: impl Into<String>) -> Self {
        self.module_name = Some(name.into());
        self
    }

    /// Add a variable mapping.
    pub fn with_variable_mapping(mut self, mapping: VariableMapping) -> Self {
        self.variable_mappings.push(mapping);
        self
    }

    /// Add an operation mapping.
    pub fn with_operation_mapping(mut self, mapping: OperationMapping) -> Self {
        self.operation_mappings.push(mapping);
        self
    }

    /// Add a constant.
    pub fn with_constant(mut self, name: impl Into<String>, value: serde_json::Value) -> Self {
        self.constants.insert(name.into(), value);
        self
    }

    /// Add a type mapping.
    pub fn with_type_mapping(mut self, mapping: TypeMapping) -> Self {
        self.type_mappings.push(mapping);
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Find the variable mapping for a runtime variable name.
    pub fn find_variable_mapping(&self, runtime_name: &str) -> Option<&VariableMapping> {
        self.variable_mappings
            .iter()
            .find(|m| m.runtime_name == runtime_name)
    }

    /// Find the variable mapping for a TLA+ variable name.
    pub fn find_variable_mapping_by_tla(&self, tla_name: &str) -> Option<&VariableMapping> {
        self.variable_mappings
            .iter()
            .find(|m| m.tla_name == tla_name)
    }

    /// Find the operation mapping for a runtime operation name.
    pub fn find_operation_mapping(&self, runtime_name: &str) -> Option<&OperationMapping> {
        self.operation_mappings
            .iter()
            .find(|m| m.runtime_name == runtime_name)
    }

    /// Find the operation mapping for a TLA+ action name.
    pub fn find_operation_mapping_by_tla(&self, tla_name: &str) -> Option<&OperationMapping> {
        self.operation_mappings
            .iter()
            .find(|m| m.tla_action == tla_name)
    }

    /// Map a runtime variable value to a TLA+ value.
    pub fn map_variable(
        &self,
        runtime_name: &str,
        value: &serde_json::Value,
    ) -> Result<(String, serde_json::Value), BindingError> {
        let mapping = self
            .find_variable_mapping(runtime_name)
            .ok_or_else(|| BindingError::MissingVariable(runtime_name.to_string()))?;

        let transformed = mapping.transform(value)?;
        Ok((mapping.tla_name.clone(), transformed))
    }

    /// Map a runtime operation to a TLA+ action with parameters.
    pub fn map_operation(
        &self,
        runtime_name: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<(String, HashMap<String, serde_json::Value>), BindingError> {
        let mapping = self
            .find_operation_mapping(runtime_name)
            .ok_or_else(|| BindingError::MissingOperation(runtime_name.to_string()))?;

        let tla_params = mapping.map_params(params)?;
        Ok((mapping.tla_action.clone(), tla_params))
    }

    /// Validate that all required mappings are present.
    pub fn validate(&self) -> Result<(), BindingError> {
        // Check for duplicate variable mappings
        let mut seen_runtime = std::collections::HashSet::new();
        let mut seen_tla = std::collections::HashSet::new();

        for mapping in &self.variable_mappings {
            if !seen_runtime.insert(&mapping.runtime_name) {
                return Err(BindingError::InvalidConfig(format!(
                    "Duplicate runtime variable mapping: {}",
                    mapping.runtime_name
                )));
            }
            if !seen_tla.insert(&mapping.tla_name) {
                return Err(BindingError::InvalidConfig(format!(
                    "Duplicate TLA+ variable mapping: {}",
                    mapping.tla_name
                )));
            }
        }

        // Check for duplicate operation mappings
        let mut seen_ops = std::collections::HashSet::new();
        for mapping in &self.operation_mappings {
            if !seen_ops.insert(&mapping.runtime_name) {
                return Err(BindingError::InvalidConfig(format!(
                    "Duplicate operation mapping: {}",
                    mapping.runtime_name
                )));
            }
        }

        Ok(())
    }

    /// Get a list of all TLA+ variable names in this binding.
    pub fn tla_variable_names(&self) -> Vec<&str> {
        self.variable_mappings
            .iter()
            .map(|m| m.tla_name.as_str())
            .collect()
    }

    /// Get a list of all TLA+ action names in this binding.
    pub fn tla_action_names(&self) -> Vec<&str> {
        self.operation_mappings
            .iter()
            .map(|m| m.tla_action.as_str())
            .collect()
    }
}

/// A mapping from a runtime variable to a TLA+ variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableMapping {
    /// The name of the variable in the runtime/trace.
    pub runtime_name: String,

    /// The name of the variable in the TLA+ specification.
    pub tla_name: String,

    /// The source of the variable value.
    pub source: VariableSource,

    /// Optional transformation to apply.
    pub transform: Option<VariableTransform>,

    /// Optional type hint for TLA+.
    pub tla_type: Option<String>,

    /// Description of this mapping.
    pub description: Option<String>,
}

impl VariableMapping {
    /// Create a new variable mapping.
    pub fn new(runtime_name: impl Into<String>, tla_name: impl Into<String>) -> Self {
        VariableMapping {
            runtime_name: runtime_name.into(),
            tla_name: tla_name.into(),
            source: VariableSource::TraceVars,
            transform: None,
            tla_type: None,
            description: None,
        }
    }

    /// Set the variable source.
    pub fn with_source(mut self, source: VariableSource) -> Self {
        self.source = source;
        self
    }

    /// Set the transformation.
    pub fn with_transform(mut self, transform: VariableTransform) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Set the TLA+ type hint.
    pub fn with_tla_type(mut self, tla_type: impl Into<String>) -> Self {
        self.tla_type = Some(tla_type.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Apply the transformation to a value.
    pub fn transform(&self, value: &serde_json::Value) -> Result<serde_json::Value, BindingError> {
        match &self.transform {
            None => Ok(value.clone()),
            Some(transform) => transform
                .apply(value)
                .map_err(|msg| BindingError::TransformError {
                    variable: self.runtime_name.clone(),
                    message: msg,
                }),
        }
    }
}

/// The source of a variable value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableSource {
    /// The variable comes from the trace step's vars field.
    TraceVars,

    /// The variable comes from the trace step's params field.
    TraceParams,

    /// The variable comes from trace metadata.
    TraceMetadata,

    /// The variable is computed from other variables.
    Computed,

    /// The variable is a constant.
    Constant(serde_json::Value),
}

/// A transformation to apply to a variable value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableTransform {
    /// No transformation (identity).
    Identity,

    /// Convert JSON to TLA+ value representation.
    JsonToTla,

    /// Extract a field from an object.
    ExtractField(String),

    /// Map array to TLA+ sequence.
    ArrayToSequence,

    /// Map object to TLA+ function.
    ObjectToFunction,

    /// Map object to TLA+ record.
    ObjectToRecord,

    /// Apply a custom transformation expression.
    ///
    /// The expression can reference the input value as `$value`.
    Custom(String),

    /// Chain multiple transformations.
    Chain(Vec<VariableTransform>),
}

impl VariableTransform {
    /// Apply this transformation to a value.
    pub fn apply(&self, value: &serde_json::Value) -> Result<serde_json::Value, String> {
        match self {
            VariableTransform::Identity => Ok(value.clone()),

            VariableTransform::JsonToTla => {
                // Convert JSON to TLA+ compatible representation
                convert_json_to_tla(value)
            }

            VariableTransform::ExtractField(field) => value
                .get(field)
                .cloned()
                .ok_or_else(|| format!("Field '{}' not found", field)),

            VariableTransform::ArrayToSequence => {
                // TLA+ sequences are 1-indexed, but we keep as array internally
                if value.is_array() {
                    Ok(value.clone())
                } else {
                    Err("Expected array for sequence conversion".to_string())
                }
            }

            VariableTransform::ObjectToFunction => {
                // TLA+ functions from objects
                if value.is_object() {
                    Ok(value.clone())
                } else {
                    Err("Expected object for function conversion".to_string())
                }
            }

            VariableTransform::ObjectToRecord => {
                // TLA+ records from objects
                if value.is_object() {
                    Ok(value.clone())
                } else {
                    Err("Expected object for record conversion".to_string())
                }
            }

            VariableTransform::Custom(expr) => {
                // Custom expressions are evaluated elsewhere
                // For now, just return the value with the expression as metadata
                Ok(serde_json::json!({
                    "_transform": expr,
                    "_value": value
                }))
            }

            VariableTransform::Chain(transforms) => {
                let mut result = value.clone();
                for transform in transforms {
                    result = transform.apply(&result)?;
                }
                Ok(result)
            }
        }
    }
}

/// Convert a JSON value to TLA+ compatible representation.
fn convert_json_to_tla(value: &serde_json::Value) -> Result<serde_json::Value, String> {
    match value {
        serde_json::Value::Null => Ok(serde_json::json!("NULL")),
        serde_json::Value::Bool(b) => Ok(serde_json::json!(if *b { "TRUE" } else { "FALSE" })),
        serde_json::Value::Number(n) => Ok(serde_json::Value::Number(n.clone())),
        serde_json::Value::String(s) => Ok(serde_json::json!(s)),
        serde_json::Value::Array(arr) => {
            let converted: Result<Vec<_>, _> = arr.iter().map(convert_json_to_tla).collect();
            Ok(serde_json::Value::Array(converted?))
        }
        serde_json::Value::Object(obj) => {
            let mut converted = serde_json::Map::new();
            for (k, v) in obj {
                converted.insert(k.clone(), convert_json_to_tla(v)?);
            }
            Ok(serde_json::Value::Object(converted))
        }
    }
}

/// A mapping from a runtime operation to a TLA+ action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMapping {
    /// The name of the operation in the runtime/trace.
    pub runtime_name: String,

    /// The name of the action in the TLA+ specification.
    pub tla_action: String,

    /// Parameter mappings from runtime to TLA+.
    pub param_mappings: HashMap<String, ParamMapping>,

    /// Precondition for this action (TLA+ expression).
    pub precondition: Option<String>,

    /// Description of this mapping.
    pub description: Option<String>,
}

impl OperationMapping {
    /// Create a new operation mapping.
    pub fn new(runtime_name: impl Into<String>, tla_action: impl Into<String>) -> Self {
        OperationMapping {
            runtime_name: runtime_name.into(),
            tla_action: tla_action.into(),
            param_mappings: HashMap::new(),
            precondition: None,
            description: None,
        }
    }

    /// Add a simple parameter mapping (same name).
    pub fn with_param(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.param_mappings
            .insert(name.clone(), ParamMapping::simple(&name));
        self
    }

    /// Add a parameter mapping with renaming.
    pub fn with_param_mapping(
        mut self,
        runtime_name: impl Into<String>,
        tla_name: impl Into<String>,
    ) -> Self {
        self.param_mappings
            .insert(runtime_name.into(), ParamMapping::renamed(tla_name.into()));
        self
    }

    /// Add a parameter mapping with transformation.
    pub fn with_param_transform(
        mut self,
        runtime_name: impl Into<String>,
        mapping: ParamMapping,
    ) -> Self {
        self.param_mappings.insert(runtime_name.into(), mapping);
        self
    }

    /// Set the precondition.
    pub fn with_precondition(mut self, precondition: impl Into<String>) -> Self {
        self.precondition = Some(precondition.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Map runtime parameters to TLA+ parameters.
    pub fn map_params(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, BindingError> {
        let mut result = HashMap::new();

        for (runtime_name, mapping) in &self.param_mappings {
            let value = params
                .get(runtime_name)
                .ok_or_else(|| BindingError::ParamError {
                    operation: self.runtime_name.clone(),
                    param: runtime_name.clone(),
                    message: "Parameter not found in trace".to_string(),
                })?;

            let (tla_name, tla_value) =
                mapping
                    .apply(runtime_name, value)
                    .map_err(|msg| BindingError::ParamError {
                        operation: self.runtime_name.clone(),
                        param: runtime_name.clone(),
                        message: msg,
                    })?;

            result.insert(tla_name, tla_value);
        }

        Ok(result)
    }
}

/// A mapping for a single parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamMapping {
    /// The name in TLA+ (None = same as runtime).
    pub tla_name: Option<String>,

    /// Optional transformation.
    pub transform: Option<VariableTransform>,

    /// Default value if parameter is missing.
    pub default: Option<serde_json::Value>,
}

impl ParamMapping {
    /// Create a simple mapping (same name, no transform).
    pub fn simple(name: &str) -> Self {
        ParamMapping {
            tla_name: Some(name.to_string()),
            transform: None,
            default: None,
        }
    }

    /// Create a mapping with a different TLA+ name.
    pub fn renamed(tla_name: String) -> Self {
        ParamMapping {
            tla_name: Some(tla_name),
            transform: None,
            default: None,
        }
    }

    /// Create a mapping with a transformation.
    pub fn with_transform(mut self, transform: VariableTransform) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Create a mapping with a default value.
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }

    /// Apply this mapping to a parameter.
    pub fn apply(
        &self,
        runtime_name: &str,
        value: &serde_json::Value,
    ) -> Result<(String, serde_json::Value), String> {
        let tla_name = self
            .tla_name
            .as_ref()
            .map(|s| s.clone())
            .unwrap_or_else(|| runtime_name.to_string());

        let transformed = match &self.transform {
            Some(t) => t.apply(value)?,
            None => value.clone(),
        };

        Ok((tla_name, transformed))
    }
}

/// A mapping between runtime types and TLA+ types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMapping {
    /// The runtime type name (e.g., Rust type).
    pub runtime_type: String,

    /// The TLA+ type expression.
    pub tla_type: String,

    /// How to convert values of this type.
    pub conversion: TypeConversion,
}

impl TypeMapping {
    /// Create a new type mapping.
    pub fn new(
        runtime_type: impl Into<String>,
        tla_type: impl Into<String>,
        conversion: TypeConversion,
    ) -> Self {
        TypeMapping {
            runtime_type: runtime_type.into(),
            tla_type: tla_type.into(),
            conversion,
        }
    }
}

/// How to convert between runtime and TLA+ types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeConversion {
    /// Direct mapping (compatible types).
    Direct,

    /// Integer to Nat (non-negative).
    IntToNat,

    /// String to model value.
    StringToModelValue,

    /// Enum to model value set.
    EnumToModelValue(Vec<String>),

    /// Custom conversion function.
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_binding() {
        let binding = SpecBinding::new("KVStore")
            .with_module_name("KVSpec")
            .with_variable_mapping(VariableMapping::new("store", "kv_store"))
            .with_operation_mapping(
                OperationMapping::new("Put", "KVPut")
                    .with_param_mapping("key", "k")
                    .with_param_mapping("value", "v"),
            )
            .with_constant("MaxKeys", serde_json::json!(100));

        assert_eq!(binding.spec_id.0, "KVStore");
        assert_eq!(binding.module_name, Some("KVSpec".to_string()));
        assert_eq!(binding.variable_mappings.len(), 1);
        assert_eq!(binding.operation_mappings.len(), 1);

        // Test lookups
        let var_mapping = binding.find_variable_mapping("store").unwrap();
        assert_eq!(var_mapping.tla_name, "kv_store");

        let op_mapping = binding.find_operation_mapping("Put").unwrap();
        assert_eq!(op_mapping.tla_action, "KVPut");
    }

    #[test]
    fn test_variable_transform() {
        let value = serde_json::json!({
            "items": [1, 2, 3],
            "count": 3
        });

        // Test field extraction
        let transform = VariableTransform::ExtractField("items".to_string());
        let result = transform.apply(&value).unwrap();
        assert_eq!(result, serde_json::json!([1, 2, 3]));

        // Test chained transforms
        let chain = VariableTransform::Chain(vec![
            VariableTransform::ExtractField("items".to_string()),
            VariableTransform::ArrayToSequence,
        ]);
        let result = chain.apply(&value).unwrap();
        assert_eq!(result, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn test_operation_mapping() {
        let mapping = OperationMapping::new("Put", "KVPut")
            .with_param_mapping("key", "k")
            .with_param_mapping("value", "v");

        let mut params = HashMap::new();
        params.insert("key".to_string(), serde_json::json!("foo"));
        params.insert("value".to_string(), serde_json::json!(42));

        let tla_params = mapping.map_params(&params).unwrap();
        assert_eq!(tla_params.get("k"), Some(&serde_json::json!("foo")));
        assert_eq!(tla_params.get("v"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_binding_validation() {
        // Valid binding
        let binding = SpecBinding::new("Test")
            .with_variable_mapping(VariableMapping::new("a", "tla_a"))
            .with_variable_mapping(VariableMapping::new("b", "tla_b"));
        assert!(binding.validate().is_ok());

        // Duplicate runtime variable
        let binding = SpecBinding::new("Test")
            .with_variable_mapping(VariableMapping::new("a", "tla_a"))
            .with_variable_mapping(VariableMapping::new("a", "tla_b"));
        assert!(binding.validate().is_err());

        // Duplicate TLA+ variable
        let binding = SpecBinding::new("Test")
            .with_variable_mapping(VariableMapping::new("a", "tla_x"))
            .with_variable_mapping(VariableMapping::new("b", "tla_x"));
        assert!(binding.validate().is_err());
    }
}
