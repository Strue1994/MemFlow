use crate::error::ExecError;
use serde_json::Value;
use std::collections::HashMap;

pub struct Environment {
    variables: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Result<&Value, ExecError> {
        self.variables
            .get(name)
            .ok_or_else(|| ExecError::VariableNotFound(name.to_string()))
    }

    pub fn get_mut(&mut self, name: &str) -> Result<&mut Value, ExecError> {
        self.variables
            .get_mut(name)
            .ok_or_else(|| ExecError::VariableNotFound(name.to_string()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.variables.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Value)> {
        self.variables.iter_mut()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
