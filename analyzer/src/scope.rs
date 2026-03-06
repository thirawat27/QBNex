use core_types::{QError, QResult, QType};
use std::collections::HashMap;
use syntax_tree::Program;

#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub var_type: QType,
    pub is_constant: bool,
    pub is_function: bool,
    pub is_sub: bool,
    pub dimensions: Vec<Option<i32>>,
}

impl SymbolEntry {
    pub fn new(name: String, var_type: QType) -> Self {
        Self {
            name,
            var_type,
            is_constant: false,
            is_function: false,
            is_sub: false,
            dimensions: Vec::new(),
        }
    }
}

pub struct SymbolTable {
    symbols: HashMap<String, SymbolEntry>,
    def_types: HashMap<char, QType>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut def_types = HashMap::new();

        for c in 'a'..='z' {
            def_types.insert(c, QType::Single(0.0));
        }

        Self {
            symbols: HashMap::new(),
            def_types,
        }
    }

    pub fn define(&mut self, name: String, var_type: QType) -> QResult<()> {
        let name_lower = name.to_lowercase();

        if self.symbols.contains_key(&name_lower) {
            return Ok(());
        }

        self.symbols
            .insert(name_lower, SymbolEntry::new(name, var_type));
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> QResult<SymbolEntry> {
        let name_lower = name.to_lowercase();

        self.symbols
            .get(&name_lower)
            .cloned()
            .ok_or_else(|| QError::UndefinedVariable(name.to_string()))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(&name.to_lowercase())
    }

    pub fn get_type(&self, name: &str) -> QResult<QType> {
        let entry = self.lookup(name)?;
        Ok(entry.var_type)
    }

    pub fn set_type(&mut self, first_char: char, var_type: QType) {
        self.def_types
            .insert(first_char.to_ascii_lowercase(), var_type);
    }

    pub fn get_default_type(&self, name: &str) -> QType {
        let first_char = name.chars().next().unwrap_or('a').to_ascii_lowercase();

        self.def_types
            .get(&first_char)
            .cloned()
            .unwrap_or(QType::String(String::new()))
    }

    pub fn define_from_def_type(&mut self, name: &str, type_name: &str) -> QResult<()> {
        let var_type = match type_name.to_uppercase().as_str() {
            "INTEGER" => QType::Integer(0),
            "LONG" => QType::Long(0),
            "SINGLE" => QType::Single(0.0),
            "DOUBLE" => QType::Double(0.0),
            "STRING" => QType::String(String::new()),
            _ => return Err(QError::Syntax(format!("Unknown type: {}", type_name))),
        };

        let first_char = name.chars().next().unwrap_or('a').to_ascii_lowercase();
        self.set_type(first_char, var_type.clone());
        self.define(name.to_string(), var_type)
    }

    pub fn all_symbols(&self) -> Vec<&SymbolEntry> {
        self.symbols.values().collect()
    }

    pub fn clear(&mut self) {
        self.symbols.clear();
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

pub fn analyze_program(program: &Program) -> QResult<SymbolTable> {
    let mut table = SymbolTable::new();

    for stmt in &program.statements {
        match stmt {
            syntax_tree::Statement::Dim { variables, .. } => {
                for (var, size) in variables {
                    let var_type = if size.is_some() {
                        QType::Integer(0)
                    } else {
                        table.get_default_type(&var.name)
                    };

                    table.define(var.name.clone(), var_type)?;
                }
            }
            syntax_tree::Statement::Const { name, value } => {
                let value_type = match value {
                    syntax_tree::Expression::Literal(qtype) => qtype.clone(),
                    _ => QType::Empty,
                };

                let mut entry = SymbolEntry::new(name.clone(), value_type);
                entry.is_constant = true;
                table.symbols.insert(name.to_lowercase(), entry);
            }
            syntax_tree::Statement::DefType {
                letter_range,
                type_name,
            } => {
                for c in letter_range.0..=letter_range.1 {
                    let var_type = match type_name.as_str() {
                        "INTEGER" => QType::Integer(0),
                        "LONG" => QType::Long(0),
                        "SINGLE" => QType::Single(0.0),
                        "DOUBLE" => QType::Double(0.0),
                        "STRING" => QType::String(String::new()),
                        _ => QType::Empty,
                    };

                    table.set_type(c, var_type);
                }
            }
            _ => {}
        }
    }

    Ok(table)
}
