use crate::scope::SymbolTable;
use core_types::{QError, QResult, QType};
use std::collections::HashMap;
use syntax_tree::ast_nodes::{BinaryOp, Expression, Program, Statement, UnaryOp, Variable};

#[derive(Debug, Clone)]
struct ProcedureSignature {
    is_function: bool,
    param_types: Vec<QType>,
    return_type: Option<QType>,
}

#[derive(Clone, Copy)]
enum BuiltinArgRule {
    Any,
    Numeric,
    String,
    NumericOrString,
    VariableRef,
    ArrayName,
}

pub struct TypeChecker {
    symbol_table: SymbolTable,
    user_types: HashMap<String, syntax_tree::ast_nodes::UserType>,
    procedure_signatures: HashMap<String, ProcedureSignature>,
}

impl TypeChecker {
    pub fn new(symbol_table: SymbolTable) -> Self {
        Self {
            symbol_table,
            user_types: HashMap::new(),
            procedure_signatures: HashMap::new(),
        }
    }

    pub fn with_user_types(
        mut self,
        user_types: HashMap<String, syntax_tree::ast_nodes::UserType>,
    ) -> Self {
        self.user_types = user_types;
        self
    }

    pub fn check_program(&mut self, program: &Program) -> QResult<()> {
        // First, collect all user-defined types
        for (name, user_type) in &program.user_types {
            self.user_types.insert(name.clone(), user_type.clone());
        }

        // Process all DefType statements first to set up default types
        for stmt in &program.statements {
            if let Statement::DefType {
                letter_ranges,
                type_name,
            } = stmt
            {
                self.process_deftype(letter_ranges, type_name)?;
            }
        }

        self.collect_procedure_signatures(program)?;

        let mut top_level_scope = HashMap::new();
        self.check_statements_in_scope(&program.statements, &mut top_level_scope)?;

        for sub in program.subs.values() {
            let mut local_scope = HashMap::new();
            for param in &sub.params {
                local_scope.insert(param.name.to_lowercase(), self.get_variable_type(param));
            }
            self.check_statements_in_scope(&sub.body, &mut local_scope)?;
        }

        for func in program.functions.values() {
            let mut local_scope = HashMap::new();
            local_scope.insert(func.name.to_lowercase(), func.return_type.clone());
            for param in &func.params {
                local_scope.insert(param.name.to_lowercase(), self.get_variable_type(param));
            }
            self.check_statements_in_scope(&func.body, &mut local_scope)?;
        }

        Ok(())
    }

    fn process_deftype(&mut self, letter_ranges: &[(char, char)], type_name: &str) -> QResult<()> {
        let var_type = match type_name.to_uppercase().as_str() {
            "INTEGER" => QType::Integer(0),
            "LONG" => QType::Long(0),
            "SINGLE" => QType::Single(0.0),
            "DOUBLE" => QType::Double(0.0),
            "STRING" => QType::String(String::new()),
            _ => return Err(QError::Syntax(format!("Unknown DEF type: {}", type_name))),
        };

        for (start, end) in letter_ranges {
            for c in start.to_ascii_lowercase()..=end.to_ascii_lowercase() {
                self.symbol_table.set_type(c, var_type.clone());
            }
        }
        Ok(())
    }

    fn normalize_proc_name(name: &str) -> String {
        name.to_ascii_uppercase()
    }

    fn type_name(qtype: &QType) -> &'static str {
        match qtype {
            QType::Integer(_) => "INTEGER",
            QType::Long(_) => "LONG",
            QType::Single(_) => "SINGLE",
            QType::Double(_) => "DOUBLE",
            QType::String(_) => "STRING",
            QType::UserDefined(_) => "USERDEFINED",
            QType::Empty => "EMPTY",
        }
    }

    fn declared_type_to_qtype(type_name: &str) -> QType {
        match Self::normalize_qb64_type_name(type_name).as_str() {
            "INTEGER" => QType::Integer(0),
            "LONG" => QType::Long(0),
            "SINGLE" => QType::Single(0.0),
            "DOUBLE" | "_FLOAT" | "FLOAT" => QType::Double(0.0),
            "STRING" => QType::String(String::new()),
            "_BYTE" | "BYTE" | "_BIT" | "BIT" => QType::Integer(0),
            "_UNSIGNED _BYTE" | "UNSIGNED _BYTE" | "_UNSIGNED BYTE" | "UNSIGNED BYTE"
            | "_UNSIGNED _BIT" | "UNSIGNED _BIT" | "_UNSIGNED BIT" | "UNSIGNED BIT" => {
                QType::Integer(0)
            }
            "_UNSIGNED INTEGER" | "UNSIGNED INTEGER" => QType::Long(0),
            "_UNSIGNED LONG"
            | "UNSIGNED LONG"
            | "_INTEGER64"
            | "INTEGER64"
            | "_UNSIGNED _INTEGER64"
            | "UNSIGNED _INTEGER64"
            | "_UNSIGNED INTEGER64"
            | "UNSIGNED INTEGER64"
            | "_OFFSET"
            | "OFFSET"
            | "_UNSIGNED _OFFSET"
            | "UNSIGNED _OFFSET"
            | "_UNSIGNED OFFSET"
            | "UNSIGNED OFFSET" => QType::Double(0.0),
            _ => QType::UserDefined(type_name.as_bytes().to_vec()),
        }
    }

    fn infer_qualified_variable_type(
        &self,
        name: &str,
        local_scope: Option<&HashMap<String, QType>>,
    ) -> QResult<QType> {
        if let Some(scope) = local_scope {
            if let Some(var_type) = scope.get(&name.to_lowercase()) {
                return Ok(var_type.clone());
            }
        }

        if let Ok(var_type) = self.symbol_table.get_type(name) {
            return Ok(var_type);
        }

        if matches!(
            name.chars().last(),
            Some('$') | Some('%') | Some('&') | Some('!') | Some('#')
        ) {
            return Self::infer_type_from_suffix(name);
        }

        Ok(self.symbol_table.get_default_type(name))
    }

    fn same_signature_type(left: &QType, right: &QType) -> bool {
        match (left, right) {
            (QType::Integer(_), QType::Integer(_))
            | (QType::Long(_), QType::Long(_))
            | (QType::Single(_), QType::Single(_))
            | (QType::Double(_), QType::Double(_))
            | (QType::String(_), QType::String(_))
            | (QType::Empty, QType::Empty) => true,
            (QType::UserDefined(left_name), QType::UserDefined(right_name)) => {
                left_name == right_name
            }
            _ => false,
        }
    }

    fn canonical_builtin_name(name: &str) -> Option<&'static str> {
        match name.to_ascii_uppercase().as_str() {
            "LEFT$" => Some("LEFT$"),
            "RIGHT$" => Some("RIGHT$"),
            "MID$" => Some("MID$"),
            "LEN" => Some("LEN"),
            "INSTR" => Some("INSTR"),
            "LCASE$" => Some("LCASE$"),
            "UCASE$" => Some("UCASE$"),
            "LTRIM$" => Some("LTRIM$"),
            "RTRIM$" => Some("RTRIM$"),
            "TRIM$" => Some("TRIM$"),
            "_TRIM$" => Some("TRIM$"),
            "STR$" => Some("STR$"),
            "VAL" => Some("VAL"),
            "CHR$" => Some("CHR$"),
            "ASC" => Some("ASC"),
            "SPACE$" => Some("SPACE$"),
            "STRING$" => Some("STRING$"),
            "HEX$" => Some("HEX$"),
            "OCT$" => Some("OCT$"),
            "ABS" => Some("ABS"),
            "SGN" => Some("SGN"),
            "SIN" => Some("SIN"),
            "COS" => Some("COS"),
            "TAN" => Some("TAN"),
            "ATN" => Some("ATN"),
            "EXP" => Some("EXP"),
            "LOG" => Some("LOG"),
            "SQR" => Some("SQR"),
            "INT" => Some("INT"),
            "FIX" => Some("FIX"),
            "RND" => Some("RND"),
            "CINT" => Some("CINT"),
            "CLNG" => Some("CLNG"),
            "CSNG" => Some("CSNG"),
            "CDBL" => Some("CDBL"),
            "CSTR" => Some("CSTR"),
            "MKI$" => Some("MKI$"),
            "MKL$" => Some("MKL$"),
            "MKS$" => Some("MKS$"),
            "MKD$" => Some("MKD$"),
            "CVI" => Some("CVI"),
            "CVL" => Some("CVL"),
            "CVS" => Some("CVS"),
            "CVD" => Some("CVD"),
            "_CV" => Some("_CV"),
            "TIMER" => Some("TIMER"),
            "DATE" | "DATE$" => Some("DATE$"),
            "TIME" | "TIME$" => Some("TIME$"),
            "LBOUND" => Some("LBOUND"),
            "UBOUND" => Some("UBOUND"),
            "_FILEEXISTS" => Some("_FILEEXISTS"),
            "_DIREXISTS" => Some("_DIREXISTS"),
            "EOF" => Some("EOF"),
            "LOF" => Some("LOF"),
            "FREEFILE" => Some("FREEFILE"),
            "LOC" => Some("LOC"),
            "INPUT$" => Some("INPUT$"),
            "FRE" => Some("FRE"),
            "CSRLIN" => Some("CSRLIN"),
            "POS" => Some("POS"),
            "LPOS" => Some("LPOS"),
            "ENVIRON$" => Some("ENVIRON$"),
            "COMMAND" | "COMMAND$" => Some("COMMAND$"),
            "INKEY" | "INKEY$" => Some("INKEY$"),
            "PEEK" => Some("PEEK"),
            "VARPTR" => Some("VARPTR"),
            "VARSEG" => Some("VARSEG"),
            "SADD" => Some("SADD"),
            "VARPTR$" | "VARPTRSTR" => Some("VARPTR$"),
            "INP" => Some("INP"),
            "POINT" => Some("POINT"),
            "PMAP" => Some("PMAP"),
            "SCREEN" => Some("SCREEN"),
            "PLAY" => Some("PLAY"),
            "ERR" => Some("ERR"),
            "ERL" => Some("ERL"),
            "ERDEV" => Some("ERDEV"),
            "ERDEV$" | "ERDEVSTR" => Some("ERDEV$"),
            _ => None,
        }
    }

    fn builtin_arity(name: &str) -> Option<(usize, usize)> {
        match Self::canonical_builtin_name(name)? {
            "_CV" => Some((2, 2)),
            "LEN" | "STR$" | "VAL" | "CHR$" | "LCASE$" | "UCASE$" | "LTRIM$" | "RTRIM$"
            | "TRIM$" | "HEX$" | "OCT$" | "ABS" | "SGN" | "SIN" | "COS" | "TAN" | "ATN" | "EXP"
            | "LOG" | "SQR" | "INT" | "FIX" | "CINT" | "CLNG" | "CSNG" | "CDBL" | "CSTR"
            | "MKI$" | "MKL$" | "MKS$" | "MKD$" | "CVI" | "CVL" | "CVS" | "CVD" | "FRE" | "POS"
            | "ENVIRON$" | "PEEK" | "VARPTR" | "VARSEG" | "SADD" | "VARPTR$" | "INP" | "LPOS" => {
                Some((1, 1))
            }
            "ASC" => Some((1, 2)),
            "TIMER" | "DATE$" | "TIME$" | "CSRLIN" | "COMMAND$" | "INKEY$" | "FREEFILE" | "ERR"
            | "ERL" | "ERDEV" | "ERDEV$" => Some((0, 0)),
            "POINT" | "PMAP" => Some((2, 2)),
            "SCREEN" => Some((2, 3)),
            "PLAY" => Some((1, 1)),
            "LEFT$" | "RIGHT$" | "STRING$" => Some((2, 2)),
            "SPACE$" => Some((1, 1)),
            "MID$" | "INSTR" => Some((2, 3)),
            "LBOUND" | "UBOUND" | "INPUT$" => Some((1, 2)),
            "_FILEEXISTS" | "_DIREXISTS" => Some((1, 1)),
            "EOF" | "LOF" | "LOC" => Some((1, 1)),
            "RND" | "RANDOMIZE" => Some((0, 1)),
            _ => None,
        }
    }

    fn normalize_qb64_type_name(type_name: &str) -> String {
        type_name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_uppercase()
    }

    fn cv_type_name_from_expr(expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(var) => Some(Self::normalize_qb64_type_name(&var.name)),
            Expression::FieldAccess { .. } => expr
                .flattened_qb64_name()
                .map(|name| Self::normalize_qb64_type_name(&name)),
            _ => None,
        }
    }

    fn cv_return_type_from_name(type_name: &str) -> Option<QType> {
        match Self::normalize_qb64_type_name(type_name).as_str() {
            "_BYTE" | "BYTE" | "_BIT" | "BIT" => Some(QType::Integer(0)),
            "_UNSIGNED _BYTE" | "UNSIGNED _BYTE" | "_UNSIGNED BYTE" | "UNSIGNED BYTE"
            | "_UNSIGNED _BIT" | "UNSIGNED _BIT" | "_UNSIGNED BIT" | "UNSIGNED BIT" => {
                Some(QType::Integer(0))
            }
            "INTEGER" => Some(QType::Integer(0)),
            "_UNSIGNED INTEGER" | "UNSIGNED INTEGER" => Some(QType::Long(0)),
            "LONG" => Some(QType::Long(0)),
            "_UNSIGNED LONG" | "UNSIGNED LONG" => Some(QType::Double(0.0)),
            "SINGLE" => Some(QType::Single(0.0)),
            "DOUBLE" | "_FLOAT" | "FLOAT" => Some(QType::Double(0.0)),
            "_INTEGER64" | "INTEGER64" | "_OFFSET" | "OFFSET" => Some(QType::Double(0.0)),
            "_UNSIGNED _INTEGER64"
            | "UNSIGNED _INTEGER64"
            | "_UNSIGNED INTEGER64"
            | "UNSIGNED INTEGER64"
            | "_UNSIGNED _OFFSET"
            | "UNSIGNED _OFFSET"
            | "_UNSIGNED OFFSET"
            | "UNSIGNED OFFSET" => Some(QType::Double(0.0)),
            _ => None,
        }
    }

    fn builtin_return_type(name: &str) -> Option<QType> {
        match Self::canonical_builtin_name(name)? {
            "LEFT$" | "RIGHT$" | "MID$" | "LCASE$" | "UCASE$" | "LTRIM$" | "RTRIM$" | "TRIM$"
            | "STR$" | "CHR$" | "SPACE$" | "STRING$" | "HEX$" | "OCT$" | "MKI$" | "MKL$"
            | "MKS$" | "MKD$" | "DATE$" | "TIME$" | "ENVIRON$" | "COMMAND$" | "INPUT$"
            | "INKEY$" | "VARPTR$" | "ERDEV$" | "CSTR" => Some(QType::String(String::new())),
            "LEN" | "ASC" | "INSTR" | "SGN" | "INT" | "FIX" | "CINT" | "LBOUND" | "UBOUND"
            | "EOF" | "LOC" | "CSRLIN" | "POS" | "LPOS" | "CVI" | "ERR" | "ERL" | "ERDEV"
            | "_FILEEXISTS" | "_DIREXISTS" | "PEEK" | "VARSEG" | "INP" | "POINT" | "SCREEN"
            | "PLAY" => Some(QType::Integer(0)),
            "ABS" | "SIN" | "COS" | "TAN" | "ATN" | "EXP" | "LOG" | "SQR" | "CLNG" | "CVL"
            | "FRE" | "LOF" | "VARPTR" | "SADD" => Some(QType::Long(0)),
            "TIMER" | "RND" | "CSNG" | "CVS" | "PMAP" | "FREEFILE" => Some(QType::Single(0.0)),
            "CDBL" | "CVD" | "VAL" => Some(QType::Double(0.0)),
            _ => None,
        }
    }

    fn is_zero_arg_builtin(name: &str) -> bool {
        matches!(Self::builtin_arity(name), Some((0, 0)))
    }

    fn builtin_arg_rule(name: &str, arg_count: usize, index: usize) -> Option<BuiltinArgRule> {
        match Self::canonical_builtin_name(name)? {
            "LEFT$" | "RIGHT$" => Some(if index == 0 {
                BuiltinArgRule::String
            } else {
                BuiltinArgRule::Numeric
            }),
            "MID$" => Some(match index {
                0 => BuiltinArgRule::String,
                1 | 2 => BuiltinArgRule::Numeric,
                _ => return None,
            }),
            "ASC" => Some(if index == 0 {
                BuiltinArgRule::String
            } else {
                BuiltinArgRule::Numeric
            }),
            "LCASE$" | "UCASE$" | "LTRIM$" | "RTRIM$" | "TRIM$" | "VAL" | "CVI" | "CVL" | "CVS"
            | "CVD" | "_FILEEXISTS" | "_DIREXISTS" => Some(BuiltinArgRule::String),
            "STR$" | "CHR$" | "SPACE$" | "HEX$" | "OCT$" | "ABS" | "SGN" | "SIN" | "COS"
            | "TAN" | "ATN" | "EXP" | "LOG" | "SQR" | "INT" | "FIX" | "CINT" | "CLNG" | "CSNG"
            | "CDBL" | "MKI$" | "MKL$" | "MKS$" | "MKD$" | "PEEK" | "INP" | "POS" | "LPOS"
            | "EOF" | "LOF" | "LOC" | "POINT" | "PMAP" | "SCREEN" | "PLAY" => {
                Some(BuiltinArgRule::Numeric)
            }
            "INSTR" => Some(if arg_count > 2 {
                match index {
                    0 => BuiltinArgRule::Numeric,
                    1 | 2 => BuiltinArgRule::String,
                    _ => return None,
                }
            } else {
                BuiltinArgRule::String
            }),
            "STRING$" => Some(if index == 0 {
                BuiltinArgRule::Numeric
            } else {
                BuiltinArgRule::Any
            }),
            "LEN" | "CSTR" => Some(BuiltinArgRule::Any),
            "FRE" | "ENVIRON$" => Some(BuiltinArgRule::NumericOrString),
            "RND" => Some(BuiltinArgRule::Numeric),
            "INPUT$" => Some(BuiltinArgRule::Numeric),
            "VARPTR" | "VARSEG" | "SADD" | "VARPTR$" => Some(BuiltinArgRule::VariableRef),
            "LBOUND" | "UBOUND" => Some(if index == 0 {
                BuiltinArgRule::ArrayName
            } else {
                BuiltinArgRule::Numeric
            }),
            _ => Some(BuiltinArgRule::Any),
        }
    }

    fn builtin_arg_label(rule: BuiltinArgRule) -> &'static str {
        match rule {
            BuiltinArgRule::Any => "ANY",
            BuiltinArgRule::Numeric => "NUMERIC",
            BuiltinArgRule::String => "STRING",
            BuiltinArgRule::NumericOrString => "NUMERIC OR STRING",
            BuiltinArgRule::VariableRef => "VARIABLE REFERENCE",
            BuiltinArgRule::ArrayName => "ARRAY NAME",
        }
    }

    fn is_variable_reference_expr(expr: &Expression) -> bool {
        match expr {
            Expression::Variable(_) | Expression::FieldAccess { .. } => true,
            Expression::ArrayAccess { name, .. } => Self::canonical_builtin_name(name).is_none(),
            _ => false,
        }
    }

    fn is_array_name_expr(expr: &Expression) -> bool {
        match expr {
            Expression::Variable(_) => true,
            Expression::ArrayAccess { name, indices, .. } => {
                indices.is_empty() && Self::canonical_builtin_name(name).is_none()
            }
            _ => false,
        }
    }

    fn validate_builtin_argument(
        &self,
        builtin_name: &str,
        args: &[Expression],
        arg_index: usize,
        arg: &Expression,
        local_scope: Option<&HashMap<String, QType>>,
    ) -> QResult<()> {
        let Some(rule) = Self::builtin_arg_rule(builtin_name, args.len(), arg_index) else {
            self.infer_type_in_scope(arg, local_scope)?;
            return Ok(());
        };

        match rule {
            BuiltinArgRule::Any => {
                self.infer_type_in_scope(arg, local_scope)?;
                Ok(())
            }
            BuiltinArgRule::Numeric => {
                let arg_type = self.infer_type_in_scope(arg, local_scope)?;
                if arg_type.is_numeric() {
                    Ok(())
                } else {
                    Err(QError::TypeMismatch(format!(
                        "Built-in function {} argument {} expects {}, got {}",
                        builtin_name,
                        arg_index + 1,
                        Self::builtin_arg_label(rule),
                        Self::type_name(&arg_type)
                    )))
                }
            }
            BuiltinArgRule::String => {
                let arg_type = self.infer_type_in_scope(arg, local_scope)?;
                if matches!(arg_type, QType::String(_)) {
                    Ok(())
                } else {
                    Err(QError::TypeMismatch(format!(
                        "Built-in function {} argument {} expects {}, got {}",
                        builtin_name,
                        arg_index + 1,
                        Self::builtin_arg_label(rule),
                        Self::type_name(&arg_type)
                    )))
                }
            }
            BuiltinArgRule::NumericOrString => {
                let arg_type = self.infer_type_in_scope(arg, local_scope)?;
                if arg_type.is_numeric() || matches!(arg_type, QType::String(_)) {
                    Ok(())
                } else {
                    Err(QError::TypeMismatch(format!(
                        "Built-in function {} argument {} expects {}, got {}",
                        builtin_name,
                        arg_index + 1,
                        Self::builtin_arg_label(rule),
                        Self::type_name(&arg_type)
                    )))
                }
            }
            BuiltinArgRule::VariableRef => {
                if Self::is_variable_reference_expr(arg) {
                    self.infer_type_in_scope(arg, local_scope)?;
                    Ok(())
                } else {
                    Err(QError::InvalidProcedure(format!(
                        "Built-in function {} argument {} expects {}",
                        builtin_name,
                        arg_index + 1,
                        Self::builtin_arg_label(rule)
                    )))
                }
            }
            BuiltinArgRule::ArrayName => {
                if Self::is_array_name_expr(arg) {
                    Ok(())
                } else {
                    Err(QError::InvalidProcedure(format!(
                        "Built-in function {} argument {} expects {}",
                        builtin_name,
                        arg_index + 1,
                        Self::builtin_arg_label(rule)
                    )))
                }
            }
        }
    }

    fn validate_builtin_function_call(
        &self,
        name: &str,
        args: &[Expression],
        local_scope: Option<&HashMap<String, QType>>,
    ) -> Option<QResult<QType>> {
        let builtin_name = Self::canonical_builtin_name(name)?;
        let Some((min_args, max_args)) = Self::builtin_arity(builtin_name) else {
            return Some(Ok(
                Self::builtin_return_type(builtin_name).unwrap_or(QType::Single(0.0))
            ));
        };

        if args.len() < min_args || args.len() > max_args {
            let expected = if min_args == max_args {
                format!("{} argument(s)", min_args)
            } else {
                format!("{} to {} argument(s)", min_args, max_args)
            };
            return Some(Err(QError::InvalidProcedure(format!(
                "Built-in function {} expects {}, got {}",
                builtin_name,
                expected,
                args.len()
            ))));
        }

        if builtin_name == "_CV" {
            let Some(type_name) = Self::cv_type_name_from_expr(&args[0]) else {
                return Some(Err(QError::InvalidProcedure(
                    "Built-in function _CV argument 1 expects a QB64 numeric type".to_string(),
                )));
            };
            let Some(return_type) = Self::cv_return_type_from_name(&type_name) else {
                return Some(Err(QError::InvalidProcedure(format!(
                    "Built-in function _CV does not support type {}",
                    type_name
                ))));
            };
            let value_type = match self.infer_type_in_scope(&args[1], local_scope) {
                Ok(value_type) => value_type,
                Err(err) => return Some(Err(err)),
            };
            if !matches!(value_type, QType::String(_)) {
                return Some(Err(QError::TypeMismatch(format!(
                    "Built-in function _CV argument 2 expects STRING, got {}",
                    Self::type_name(&value_type)
                ))));
            }
            return Some(Ok(return_type));
        }

        for (index, arg) in args.iter().enumerate() {
            if let Err(err) =
                self.validate_builtin_argument(builtin_name, args, index, arg, local_scope)
            {
                return Some(Err(err));
            }
        }

        Some(Ok(
            Self::builtin_return_type(builtin_name).unwrap_or(QType::Single(0.0))
        ))
    }

    fn register_procedure_signature(
        &mut self,
        name: &str,
        is_function: bool,
        param_types: Vec<QType>,
        return_type: Option<QType>,
        source: &str,
    ) -> QResult<()> {
        let key = Self::normalize_proc_name(name);
        if let Some(existing) = self.procedure_signatures.get(&key) {
            if existing.is_function != is_function {
                return Err(QError::InvalidProcedure(format!(
                    "{} {} conflicts with existing procedure kind",
                    source, name
                )));
            }
            if existing.param_types.len() != param_types.len() {
                return Err(QError::InvalidProcedure(format!(
                    "{} {} expects {} argument(s), but another signature has {}",
                    source,
                    name,
                    param_types.len(),
                    existing.param_types.len()
                )));
            }
            for (index, (existing_type, new_type)) in existing
                .param_types
                .iter()
                .zip(param_types.iter())
                .enumerate()
            {
                if !Self::same_signature_type(existing_type, new_type) {
                    return Err(QError::InvalidProcedure(format!(
                        "{} {} argument {} type mismatch: {} vs {}",
                        source,
                        name,
                        index + 1,
                        Self::type_name(existing_type),
                        Self::type_name(new_type)
                    )));
                }
            }
            match (&existing.return_type, &return_type) {
                (Some(existing_type), Some(new_type))
                    if !Self::same_signature_type(existing_type, new_type) =>
                {
                    return Err(QError::InvalidProcedure(format!(
                        "{} {} return type mismatch: {} vs {}",
                        source,
                        name,
                        Self::type_name(existing_type),
                        Self::type_name(new_type)
                    )));
                }
                (None, Some(_)) | (Some(_), None) => {
                    return Err(QError::InvalidProcedure(format!(
                        "{} {} conflicts with existing return type",
                        source, name
                    )));
                }
                _ => {}
            }
            return Ok(());
        }

        self.procedure_signatures.insert(
            key,
            ProcedureSignature {
                is_function,
                param_types,
                return_type,
            },
        );
        Ok(())
    }

    fn collect_procedure_signatures(&mut self, program: &Program) -> QResult<()> {
        self.procedure_signatures.clear();

        for stmt in &program.statements {
            if let Statement::Declare {
                name,
                is_function,
                params,
                return_type,
                ..
            } = stmt
            {
                self.register_procedure_signature(
                    name,
                    *is_function,
                    params
                        .iter()
                        .map(|param| self.get_variable_type(param))
                        .collect(),
                    return_type.clone(),
                    "DECLARE",
                )?;
            }
        }

        for func in program.functions.values() {
            self.register_procedure_signature(
                &func.name,
                true,
                func.params
                    .iter()
                    .map(|param| self.get_variable_type(param))
                    .collect(),
                Some(func.return_type.clone()),
                "FUNCTION",
            )?;
        }

        for sub in program.subs.values() {
            self.register_procedure_signature(
                &sub.name,
                false,
                sub.params
                    .iter()
                    .map(|param| self.get_variable_type(param))
                    .collect(),
                None,
                "SUB",
            )?;
        }

        Ok(())
    }

    /// Get the default type for a variable based on DEFxxx rules and type suffix
    pub fn get_variable_type(&self, var: &Variable) -> QType {
        if let Some(declared_type) = &var.declared_type {
            return Self::declared_type_to_qtype(declared_type);
        }

        // First check for explicit type suffix
        if let Some(suffix) = var.type_suffix {
            return match suffix {
                '%' => QType::Integer(0),
                '&' => QType::Long(0),
                '!' => QType::Single(0.0),
                '#' => QType::Double(0.0),
                '$' => QType::String(String::new()),
                _ => QType::Empty,
            };
        }

        // Otherwise use DEFxxx rules from symbol table
        self.symbol_table.get_default_type(&var.name)
    }

    fn get_variable_type_in_scope(
        &self,
        var: &Variable,
        local_scope: Option<&HashMap<String, QType>>,
    ) -> QType {
        if let Some(local_scope) = local_scope {
            if let Some(var_type) = local_scope.get(&var.name.to_lowercase()) {
                return var_type.clone();
            }
        }
        self.get_variable_type(var)
    }

    pub fn infer_type(&self, expr: &Expression) -> QResult<QType> {
        self.infer_type_in_scope(expr, None)
    }

    fn infer_type_in_scope(
        &self,
        expr: &Expression,
        local_scope: Option<&HashMap<String, QType>>,
    ) -> QResult<QType> {
        match expr {
            Expression::Literal(qtype) => Ok(qtype.clone()),

            Expression::Variable(var) => {
                if Self::is_zero_arg_builtin(&var.name) {
                    if let Some(return_type) = Self::builtin_return_type(&var.name) {
                        return Ok(return_type);
                    }
                }
                if let Some(local_scope) = local_scope {
                    if let Some(var_type) = local_scope.get(&var.name.to_lowercase()) {
                        return Ok(var_type.clone());
                    }
                }
                // First try to get from symbol table (for explicitly declared vars)
                if let Ok(t) = self.symbol_table.get_type(&var.name) {
                    return Ok(t);
                }
                // Otherwise infer from DEFxxx rules and suffix
                Ok(self.get_variable_type_in_scope(var, local_scope))
            }

            Expression::ArrayAccess {
                name,
                indices,
                type_suffix,
            } => {
                if let Some(result) =
                    self.validate_builtin_function_call(name, indices, local_scope)
                {
                    return result;
                }
                if let Some(local_scope) = local_scope {
                    if let Some(var_type) = local_scope.get(&name.to_lowercase()) {
                        return Ok(var_type.clone());
                    }
                }
                // First try to get from symbol table
                if let Ok(t) = self.symbol_table.get_type(name) {
                    return Ok(t);
                }
                // Otherwise infer from suffix or DEFxxx rules
                if let Some(suffix) = type_suffix {
                    return Ok(match suffix {
                        '%' => QType::Integer(0),
                        '&' => QType::Long(0),
                        '!' => QType::Single(0.0),
                        '#' => QType::Double(0.0),
                        '$' => QType::String(String::new()),
                        _ => QType::Empty,
                    });
                }
                // Use DEFxxx default
                Ok(self.symbol_table.get_default_type(name))
            }

            Expression::FieldAccess { object, field } => {
                let obj_type = self.infer_type_in_scope(object, local_scope)?;

                // Check if object is a user-defined type
                match &obj_type {
                    QType::UserDefined(type_name_bytes) => {
                        // Convert Vec<u8> to String for lookup
                        let type_name = String::from_utf8_lossy(type_name_bytes);
                        // Look up the field in the type definition
                        if let Some(user_type) = self.user_types.get(type_name.as_ref()) {
                            if let Some(type_field) = user_type
                                .fields
                                .iter()
                                .find(|f| f.name.eq_ignore_ascii_case(field))
                            {
                                return Ok(type_field.field_type.clone());
                            }
                        }
                        if let Some(name) = expr.flattened_qb64_name() {
                            return self.infer_qualified_variable_type(&name, local_scope);
                        }
                        Self::infer_type_from_suffix(field)
                    }
                    _ => {
                        if let Some(name) = expr.flattened_qb64_name() {
                            return self.infer_qualified_variable_type(&name, local_scope);
                        }
                        Self::infer_type_from_suffix(field)
                    }
                }
            }

            Expression::FunctionCall(func) => self.validate_function_call(func, local_scope),

            Expression::TypeCast { target_type, .. } => Ok(target_type.clone()),

            Expression::BinaryOp { op, left, right } => {
                let left_type = self.infer_type_in_scope(left, local_scope)?;
                let right_type = self.infer_type_in_scope(right, local_scope)?;
                self.binary_op_type(op, &left_type, &right_type)
            }

            Expression::UnaryOp { op, operand } => {
                let operand_type = self.infer_type_in_scope(operand, local_scope)?;
                match op {
                    UnaryOp::Negate => {
                        if operand_type.is_numeric() {
                            Ok(operand_type)
                        } else {
                            Err(QError::TypeMismatch(
                                "Cannot negate non-numeric type".to_string(),
                            ))
                        }
                    }
                    UnaryOp::Not => Ok(QType::Integer(0)),
                }
            }

            Expression::CaseRange { start, end } => {
                let start_type = self.infer_type_in_scope(start, local_scope)?;
                let end_type = self.infer_type_in_scope(end, local_scope)?;
                if (start_type.is_numeric() && end_type.is_numeric())
                    || std::mem::discriminant(&start_type) == std::mem::discriminant(&end_type)
                {
                    Ok(QType::Integer(0))
                } else {
                    Err(QError::TypeMismatch("Case range types mismatch".into()))
                }
            }

            Expression::CaseIs { value, .. } => {
                self.infer_type_in_scope(value, local_scope)?;
                Ok(QType::Integer(0))
            }

            Expression::CaseElse => Ok(QType::Integer(0)),
        }
    }

    fn infer_type_from_suffix(name: &str) -> QResult<QType> {
        if name.ends_with('$') {
            Ok(QType::String(String::new()))
        } else if name.ends_with('%') {
            Ok(QType::Integer(0))
        } else if name.ends_with('&') {
            Ok(QType::Long(0))
        } else if name.ends_with('!') {
            Ok(QType::Single(0.0))
        } else if name.ends_with('#') {
            Ok(QType::Double(0.0))
        } else {
            // Default to Single for untyped fields (QBASIC default)
            Ok(QType::Single(0.0))
        }
    }

    fn infer_function_return_type(&self, name: &str) -> QType {
        if let Some(builtin_type) = Self::builtin_return_type(name) {
            return builtin_type;
        }

        let upper = name.to_uppercase();
        match upper.as_str() {
            // String functions
            "LEFT$" | "RIGHT$" | "MID$" | "LCASE$" | "UCASE$" | "LTRIM$" | "RTRIM$" | "TRIM$"
            | "STR$" | "CHR$" | "SPACE$" | "STRING$" | "HEX$" | "OCT$" | "MKI$" | "MKL$"
            | "MKS$" | "MKD$" | "DATE$" | "TIME$" | "ENVIRON$" | "COMMAND$" | "INKEY$"
            | "ERDEV$" => QType::String(String::new()),

            // Integer functions
            "CINT" | "INT" | "FIX" | "LEN" | "ASC" | "INSTR" | "CSRLIN" | "POS" | "LPOS"
            | "ERR" | "ERL" | "ERDEV" | "FREEFILE" | "INP" | "SCREEN" | "PLAY" => QType::Integer(0),

            // Long functions
            "CLNG" | "FRE" | "LOF" | "LOC" | "VARPTR" | "SADD" => QType::Long(0),

            // Single functions
            "CSNG" | "SIN" | "COS" | "TAN" | "ATN" | "EXP" | "LOG" | "SQR" | "ABS" | "SGN"
            | "RND" | "TIMER" => QType::Single(0.0),

            // Double functions
            "CDBL" | "CVD" => QType::Double(0.0),
            "_CV" => QType::Double(0.0),

            // Type conversion that depends on input
            "CVI" => QType::Integer(0),
            "CVL" => QType::Long(0),
            "CVS" => QType::Single(0.0),

            // Default
            _ => QType::Single(0.0),
        }
    }

    fn validate_argument_list(
        &self,
        proc_name: &str,
        proc_kind: &str,
        expected_types: &[QType],
        args: &[Expression],
        local_scope: Option<&HashMap<String, QType>>,
    ) -> QResult<()> {
        if expected_types.len() != args.len() {
            return Err(QError::InvalidProcedure(format!(
                "{} {} expects {} argument(s), got {}",
                proc_kind,
                proc_name,
                expected_types.len(),
                args.len()
            )));
        }

        for (index, (expected_type, arg)) in expected_types.iter().zip(args.iter()).enumerate() {
            let arg_type = self.infer_type_in_scope(arg, local_scope)?;
            if !self.is_compatible(expected_type, &arg_type) {
                return Err(QError::TypeMismatch(format!(
                    "{} {} argument {} expects {}, got {}",
                    proc_kind,
                    proc_name,
                    index + 1,
                    Self::type_name(expected_type),
                    Self::type_name(&arg_type)
                )));
            }
        }

        Ok(())
    }

    fn validate_function_call(
        &self,
        func: &syntax_tree::ast_nodes::FunctionCall,
        local_scope: Option<&HashMap<String, QType>>,
    ) -> QResult<QType> {
        if let Some(result) =
            self.validate_builtin_function_call(&func.name, &func.args, local_scope)
        {
            return result;
        }

        if let Some(signature) = self
            .procedure_signatures
            .get(&Self::normalize_proc_name(&func.name))
        {
            if !signature.is_function {
                return Err(QError::InvalidProcedure(format!(
                    "{} is declared as SUB, not FUNCTION",
                    func.name
                )));
            }
            self.validate_argument_list(
                &func.name,
                "FUNCTION",
                &signature.param_types,
                &func.args,
                local_scope,
            )?;
            return Ok(signature
                .return_type
                .clone()
                .unwrap_or_else(|| self.infer_function_return_type(&func.name)));
        }

        // Try to get from symbol table first
        if let Ok(t) = self.symbol_table.get_type(&func.name) {
            return Ok(t);
        }
        // Otherwise infer from suffix
        if let Some(suffix) = func.type_suffix {
            return Ok(match suffix {
                '%' => QType::Integer(0),
                '&' => QType::Long(0),
                '!' => QType::Single(0.0),
                '#' => QType::Double(0.0),
                '$' => QType::String(String::new()),
                _ => QType::Empty,
            });
        }
        // Default return type based on function name patterns
        Ok(self.infer_function_return_type(&func.name))
    }

    fn binary_op_type(&self, op: &BinaryOp, left: &QType, right: &QType) -> QResult<QType> {
        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                if left.is_numeric() && right.is_numeric() {
                    self.promote_types(left, right)
                } else if matches!(op, BinaryOp::Add)
                    && matches!(left, QType::String(_))
                    && matches!(right, QType::String(_))
                {
                    Ok(QType::String(String::new()))
                } else {
                    Err(QError::TypeMismatch(format!(
                        "Cannot perform arithmetic on {:?} and {:?}",
                        left, right
                    )))
                }
            }
            BinaryOp::IntegerDivide | BinaryOp::Modulo => {
                if left.is_numeric() && right.is_numeric() {
                    Ok(QType::Integer(0))
                } else {
                    Err(QError::TypeMismatch(
                        "Integer operations require numeric types".to_string(),
                    ))
                }
            }
            BinaryOp::Power => {
                if left.is_numeric() && right.is_numeric() {
                    Ok(QType::Double(0.0))
                } else {
                    Err(QError::TypeMismatch(
                        "Power requires numeric types".to_string(),
                    ))
                }
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::LessThan
            | BinaryOp::GreaterThan
            | BinaryOp::LessOrEqual
            | BinaryOp::GreaterOrEqual => Ok(QType::Integer(0)),
            BinaryOp::And | BinaryOp::Or | BinaryOp::Xor | BinaryOp::Eqv | BinaryOp::Imp => {
                if left.is_numeric() && right.is_numeric() {
                    Ok(QType::Integer(0))
                } else {
                    Err(QError::TypeMismatch(
                        "Logical operations require numeric types".to_string(),
                    ))
                }
            }
        }
    }

    fn promote_types(&self, left: &QType, right: &QType) -> QResult<QType> {
        let priority = |q: &QType| -> u8 {
            match q {
                QType::Integer(_) => 1,
                QType::Long(_) => 2,
                QType::Single(_) => 3,
                QType::Double(_) => 4,
                _ => 0,
            }
        };

        let lp = priority(left);
        let rp = priority(right);

        if lp >= rp {
            Ok(left.clone())
        } else {
            Ok(right.clone())
        }
    }

    pub fn check_assignment(&self, target: &Expression, value: &Expression) -> QResult<()> {
        let target_type = self.infer_type(target)?;
        let value_type = self.infer_type(value)?;

        if self.is_compatible(&target_type, &value_type) {
            Ok(())
        } else {
            Err(QError::TypeMismatch(format!(
                "Cannot assign {:?} to {:?}",
                value_type, target_type
            )))
        }
    }

    fn is_compatible(&self, target: &QType, value: &QType) -> bool {
        matches!(
            (target, value),
            (QType::Integer(_), QType::Integer(_))
                | (QType::Integer(_), QType::Long(_))
                | (QType::Integer(_), QType::Single(_))
                | (QType::Integer(_), QType::Double(_))
                | (QType::Long(_), QType::Integer(_))
                | (QType::Long(_), QType::Long(_))
                | (QType::Long(_), QType::Single(_))
                | (QType::Long(_), QType::Double(_))
                | (QType::Single(_), QType::Integer(_))
                | (QType::Single(_), QType::Long(_))
                | (QType::Single(_), QType::Single(_))
                | (QType::Single(_), QType::Double(_))
                | (QType::Double(_), QType::Integer(_))
                | (QType::Double(_), QType::Long(_))
                | (QType::Double(_), QType::Single(_))
                | (QType::Double(_), QType::Double(_))
                | (QType::String(_), QType::String(_))
                | (QType::UserDefined(_), QType::UserDefined(_))
        )
    }

    fn check_statements_in_scope(
        &self,
        statements: &[Statement],
        local_scope: &mut HashMap<String, QType>,
    ) -> QResult<()> {
        for stmt in statements {
            self.check_statement_in_scope(stmt, local_scope)?;
        }
        Ok(())
    }

    pub fn check_statement(&self, stmt: &Statement) -> QResult<()> {
        let mut local_scope = HashMap::new();
        self.check_statement_in_scope(stmt, &mut local_scope)
    }

    fn check_statement_in_scope(
        &self,
        stmt: &Statement,
        local_scope: &mut HashMap<String, QType>,
    ) -> QResult<()> {
        match stmt {
            Statement::Assignment { target, value } => {
                let target_type = self.infer_type_in_scope(target, Some(local_scope))?;
                let value_type = self.infer_type_in_scope(value, Some(local_scope))?;

                if self.is_compatible(&target_type, &value_type) {
                    if let Expression::Variable(var) = target {
                        local_scope
                            .entry(var.name.to_lowercase())
                            .or_insert(target_type);
                    }
                    Ok(())
                } else {
                    Err(QError::TypeMismatch(format!(
                        "Cannot assign {:?} to {:?}",
                        value_type, target_type
                    )))
                }
            }
            Statement::Print { expressions, .. } => {
                for expr in expressions {
                    self.infer_type_in_scope(expr, Some(local_scope))?;
                }
                Ok(())
            }
            Statement::FunctionCall(func) => {
                self.validate_function_call(func, Some(local_scope))?;
                Ok(())
            }
            Statement::Call { name, args } => {
                let Some(signature) = self
                    .procedure_signatures
                    .get(&Self::normalize_proc_name(name))
                else {
                    return Err(QError::InvalidProcedure(format!(
                        "SUB {} is not defined or declared",
                        name
                    )));
                };
                if signature.is_function {
                    return Err(QError::InvalidProcedure(format!(
                        "{} is declared as FUNCTION, not SUB",
                        name
                    )));
                }
                self.validate_argument_list(
                    name,
                    "SUB",
                    &signature.param_types,
                    args,
                    Some(local_scope),
                )
            }
            Statement::IfBlock {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_condition_in_scope(condition, Some(local_scope))?;
                self.check_statements_in_scope(then_branch, local_scope)?;
                if let Some(else_br) = else_branch {
                    self.check_statements_in_scope(else_br, local_scope)?;
                }
                Ok(())
            }
            Statement::IfElseBlock {
                condition,
                then_branch,
                else_ifs,
                else_branch,
            } => {
                self.check_condition_in_scope(condition, Some(local_scope))?;
                self.check_statements_in_scope(then_branch, local_scope)?;
                for (else_if_cond, branch) in else_ifs {
                    self.check_condition_in_scope(else_if_cond, Some(local_scope))?;
                    self.check_statements_in_scope(branch, local_scope)?;
                }
                if let Some(else_branch) = else_branch {
                    self.check_statements_in_scope(else_branch, local_scope)?;
                }
                Ok(())
            }
            Statement::ForLoop {
                variable,
                start,
                end,
                step,
                body,
            } => {
                let var_type = self.get_variable_type_in_scope(variable, Some(local_scope));
                local_scope
                    .entry(variable.name.to_lowercase())
                    .or_insert(var_type);
                self.infer_type_in_scope(start, Some(local_scope))?;
                self.infer_type_in_scope(end, Some(local_scope))?;
                if let Some(s) = step {
                    self.infer_type_in_scope(s, Some(local_scope))?;
                }
                self.check_statements_in_scope(body, local_scope)?;
                Ok(())
            }
            Statement::WhileLoop { condition, body } => {
                self.check_condition_in_scope(condition, Some(local_scope))?;
                self.check_statements_in_scope(body, local_scope)?;
                Ok(())
            }
            Statement::DoLoop {
                condition, body, ..
            } => {
                if let Some(condition) = condition {
                    self.check_condition_in_scope(condition, Some(local_scope))?;
                }
                self.check_statements_in_scope(body, local_scope)?;
                Ok(())
            }
            Statement::Select { expression, cases } => {
                self.infer_type_in_scope(expression, Some(local_scope))?;
                for (case_expr, branch) in cases {
                    self.infer_type_in_scope(case_expr, Some(local_scope))?;
                    self.check_statements_in_scope(branch, local_scope)?;
                }
                Ok(())
            }
            Statement::ForEach { array, body, .. } => {
                self.infer_type_in_scope(array, Some(local_scope))?;
                self.check_statements_in_scope(body, local_scope)?;
                Ok(())
            }
            Statement::Dim { variables, .. } | Statement::Redim { variables, .. } => {
                for (var, dimensions) in variables {
                    local_scope.insert(var.name.to_lowercase(), self.get_variable_type(var));
                    if let Some(dimensions) = dimensions {
                        for dimension in dimensions {
                            if let Some(lower_bound) = &dimension.lower_bound {
                                self.infer_type_in_scope(lower_bound, Some(local_scope))?;
                            }
                            self.infer_type_in_scope(&dimension.upper_bound, Some(local_scope))?;
                        }
                    }
                }
                Ok(())
            }
            Statement::Const { name, value } => {
                let value_type = self.infer_type_in_scope(value, Some(local_scope))?;
                local_scope.insert(name.to_lowercase(), value_type);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_condition_in_scope(
        &self,
        expr: &Expression,
        local_scope: Option<&HashMap<String, QType>>,
    ) -> QResult<()> {
        let cond_type = self.infer_type_in_scope(expr, local_scope)?;

        if cond_type.is_numeric() {
            Ok(())
        } else {
            Err(QError::TypeMismatch(
                "Condition must be numeric".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypeChecker;
    use crate::scope::analyze_program;
    use core_types::QType;
    use syntax_tree::{Expression, Parser, Statement};

    #[test]
    fn infer_type_uses_declared_function_return_type() {
        let source = "DECLARE FUNCTION TOTAL(A$ AS STRING * 4) AS LONG\nPRINT TOTAL(\"AB\")\n";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let type_checker = TypeChecker::new(symbol_table);

        let expr = match &program.statements[1] {
            Statement::Print { expressions, .. } => &expressions[0],
            _ => panic!("expected PRINT statement"),
        };

        assert!(matches!(expr, Expression::FunctionCall(_)));
        assert!(matches!(type_checker.infer_type(expr), Ok(QType::Long(_))));
    }

    #[test]
    fn check_program_rejects_function_argument_type_mismatch() {
        let source = "\
DECLARE FUNCTION TOTAL%(X%)
PRINT TOTAL%(\"AB\")
FUNCTION TOTAL%(X%)
    TOTAL% = X%
END FUNCTION";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(matches!(
            result,
            Err(core_types::QError::TypeMismatch(message))
                if message.contains("FUNCTION TOTAL% argument 1 expects INTEGER, got STRING")
        ));
    }

    #[test]
    fn check_program_uses_parameter_types_inside_procedure_body() {
        let source = "\
DECLARE SUB INNER(X%)
SUB OUTER(NAME$)
    CALL INNER(NAME$)
END SUB
SUB INNER(X%)
END SUB";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(matches!(
            result,
            Err(core_types::QError::TypeMismatch(message))
                if message.contains("SUB INNER argument 1 expects INTEGER, got STRING")
        ));
    }

    #[test]
    fn check_program_rejects_declare_definition_type_mismatch() {
        let source = "\
DECLARE FUNCTION FOO(X AS STRING) AS INTEGER
FUNCTION FOO(X AS LONG) AS INTEGER
    FOO = X
END FUNCTION";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(matches!(
            result,
            Err(core_types::QError::InvalidProcedure(message))
                if message.contains("FUNCTION FOO argument 1 type mismatch: STRING vs LONG")
        ));
    }

    #[test]
    fn infer_type_handles_builtin_array_access_return_types() {
        let source = "\
PRINT POINT(1, 2)
PRINT PMAP(1, 0)
PRINT INPUT$(1, 1)
PRINT TIMER";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let type_checker = TypeChecker::new(symbol_table);

        let point_expr = match &program.statements[0] {
            Statement::Print { expressions, .. } => &expressions[0],
            _ => panic!("expected PRINT statement"),
        };
        let pmap_expr = match &program.statements[1] {
            Statement::Print { expressions, .. } => &expressions[0],
            _ => panic!("expected PRINT statement"),
        };
        let input_expr = match &program.statements[2] {
            Statement::Print { expressions, .. } => &expressions[0],
            _ => panic!("expected PRINT statement"),
        };
        let timer_expr = match &program.statements[3] {
            Statement::Print { expressions, .. } => &expressions[0],
            _ => panic!("expected PRINT statement"),
        };

        assert!(matches!(
            type_checker.infer_type(point_expr),
            Ok(QType::Integer(_))
        ));
        assert!(matches!(
            type_checker.infer_type(pmap_expr),
            Ok(QType::Single(_))
        ));
        assert!(matches!(
            type_checker.infer_type(input_expr),
            Ok(QType::String(_))
        ));
        assert!(matches!(
            type_checker.infer_type(timer_expr),
            Ok(QType::Single(_))
        ));
    }

    #[test]
    fn check_program_rejects_builtin_argument_type_mismatch() {
        let source = "PRINT LEFT$(123, 2)";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(matches!(
            result,
            Err(core_types::QError::TypeMismatch(message))
                if message.contains("Built-in function LEFT$ argument 1 expects STRING, got INTEGER")
        ));
    }

    #[test]
    fn check_program_rejects_builtin_arity_mismatch() {
        let source = "PRINT TIMER(1)";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(matches!(
            result,
            Err(core_types::QError::InvalidProcedure(message))
                if message.contains("Built-in function TIMER expects 0 argument(s), got 1")
        ));
    }

    #[test]
    fn check_program_accepts_space_with_single_argument() {
        let source = "PRINT \"[\"; SPACE$(2); \"]\"";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(result.is_ok());
    }

    #[test]
    fn infer_type_recognizes_bare_zero_arg_string_builtins() {
        let source = "k$ = INKEY$\nPRINT DATE$\nPRINT TIME$\nPRINT COMMAND$";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(result.is_ok());
    }

    #[test]
    fn check_program_rejects_builtin_array_name_mismatch() {
        let source = "PRINT LBOUND(1)";
        let mut parser = Parser::new(source.to_string()).unwrap();
        let program = parser.parse().unwrap();
        let symbol_table = analyze_program(&program).unwrap();
        let mut type_checker = TypeChecker::new(symbol_table);

        let result = type_checker.check_program(&program);

        assert!(matches!(
            result,
            Err(core_types::QError::InvalidProcedure(message))
                if message.contains("Built-in function LBOUND argument 1 expects ARRAY NAME")
        ));
    }
}
