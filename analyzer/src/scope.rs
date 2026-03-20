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

    pub fn define_procedure(
        &mut self,
        name: String,
        var_type: QType,
        is_function: bool,
        is_sub: bool,
    ) {
        let entry = self
            .symbols
            .entry(name.to_lowercase())
            .or_insert_with(|| SymbolEntry::new(name.clone(), var_type.clone()));
        entry.name = name;
        entry.var_type = var_type;
        entry.is_function = entry.is_function || is_function;
        entry.is_sub = entry.is_sub || is_sub;
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

fn suffix_type(name: &str) -> Option<QType> {
    if name.ends_with('%') {
        Some(QType::Integer(0))
    } else if name.ends_with('&') {
        Some(QType::Long(0))
    } else if name.ends_with('!') {
        Some(QType::Single(0.0))
    } else if name.ends_with('#') {
        Some(QType::Double(0.0))
    } else if name.ends_with('$') {
        Some(QType::String(String::new()))
    } else {
        None
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
                    let var_type = if let Some(declared_type) = &var.declared_type {
                        QType::UserDefined(declared_type.clone().into_bytes())
                    } else if size.is_some() {
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
                letter_ranges,
                type_name,
            } => {
                for (start, end) in letter_ranges {
                    for c in *start..=*end {
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
            }
            syntax_tree::Statement::Declare {
                name,
                is_function,
                return_type,
                ..
            } => {
                let proc_type = if *is_function {
                    return_type
                        .clone()
                        .or_else(|| suffix_type(name))
                        .unwrap_or_else(|| table.get_default_type(name))
                } else {
                    QType::Empty
                };
                table.define_procedure(name.clone(), proc_type, *is_function, !*is_function);
            }
            _ => {}
        }
    }

    for func in program.functions.values() {
        if !table.contains(&func.name) {
            table.define_procedure(func.name.clone(), func.return_type.clone(), true, false);
        }
    }

    for sub in program.subs.values() {
        if !table.contains(&sub.name) {
            table.define_procedure(sub.name.clone(), QType::Empty, false, true);
        }
    }

    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::analyze_program;
    use core_types::QType;
    use syntax_tree::Parser;

    #[test]
    fn deftype_applies_multiple_letter_ranges_from_single_statement() {
        let source = "DEFLNG A, S\nDIM alpha, sigma, beta\n";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let table = analyze_program(&program).unwrap();

        assert!(matches!(table.get_type("alpha"), Ok(QType::Long(_))));
        assert!(matches!(table.get_type("sigma"), Ok(QType::Long(_))));
        assert!(matches!(table.get_type("beta"), Ok(QType::Single(_))));
    }

    #[test]
    fn analyze_program_registers_declared_function_signature_type() {
        let source = "DECLARE FUNCTION PAD(A$ AS STRING * 4) AS STRING * 6\n";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let table = analyze_program(&program).unwrap();
        let entry = table.lookup("PAD").unwrap();

        assert!(entry.is_function);
        assert!(matches!(entry.var_type, QType::String(_)));
    }

    #[test]
    fn analyze_program_registers_defined_function_type_without_declare() {
        let source = "FUNCTION TOTAL&()\nTOTAL& = 1\nEND FUNCTION\n";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let table = analyze_program(&program).unwrap();
        let entry = table.lookup("TOTAL&").unwrap();

        assert!(entry.is_function);
        assert!(matches!(entry.var_type, QType::Long(_)));
    }
}
