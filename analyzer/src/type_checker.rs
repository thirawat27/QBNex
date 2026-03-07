use crate::scope::SymbolTable;
use core_types::{QError, QResult, QType};
use std::collections::HashMap;
use syntax_tree::ast_nodes::{BinaryOp, Expression, Program, Statement, UnaryOp, Variable};

pub struct TypeChecker {
    symbol_table: SymbolTable,
    user_types: HashMap<String, syntax_tree::ast_nodes::UserType>,
}

impl TypeChecker {
    pub fn new(symbol_table: SymbolTable) -> Self {
        Self {
            symbol_table,
            user_types: HashMap::new(),
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
                letter_range,
                type_name,
            } = stmt
            {
                self.process_deftype(letter_range, type_name)?;
            }
        }

        Ok(())
    }

    fn process_deftype(&mut self, letter_range: &(char, char), type_name: &str) -> QResult<()> {
        let var_type = match type_name.to_uppercase().as_str() {
            "INTEGER" => QType::Integer(0),
            "LONG" => QType::Long(0),
            "SINGLE" => QType::Single(0.0),
            "DOUBLE" => QType::Double(0.0),
            "STRING" => QType::String(String::new()),
            _ => return Err(QError::Syntax(format!("Unknown DEF type: {}", type_name))),
        };

        for c in letter_range.0.to_ascii_lowercase()..=letter_range.1.to_ascii_lowercase() {
            self.symbol_table.set_type(c, var_type.clone());
        }
        Ok(())
    }

    /// Get the default type for a variable based on DEFxxx rules and type suffix
    pub fn get_variable_type(&self, var: &Variable) -> QType {
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

    pub fn infer_type(&self, expr: &Expression) -> QResult<QType> {
        match expr {
            Expression::Literal(qtype) => Ok(qtype.clone()),

            Expression::Variable(var) => {
                // First try to get from symbol table (for explicitly declared vars)
                if let Ok(t) = self.symbol_table.get_type(&var.name) {
                    return Ok(t);
                }
                // Otherwise infer from DEFxxx rules and suffix
                Ok(self.get_variable_type(var))
            }

            Expression::ArrayAccess {
                name,
                indices: _,
                type_suffix,
            } => {
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
                let obj_type = self.infer_type(object)?;

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
                        // Field not found - try suffix-based inference as fallback
                        Self::infer_type_from_suffix(field)
                    }
                    _ => {
                        // For non-user-defined types, use suffix-based inference
                        Self::infer_type_from_suffix(field)
                    }
                }
            }

            Expression::FunctionCall(func) => {
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

            Expression::TypeCast { target_type, .. } => Ok(target_type.clone()),

            Expression::BinaryOp { op, left, right } => {
                let left_type = self.infer_type(left)?;
                let right_type = self.infer_type(right)?;
                self.binary_op_type(op, &left_type, &right_type)
            }

            Expression::UnaryOp { op, operand } => {
                let operand_type = self.infer_type(operand)?;
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
                let start_type = self.infer_type(start)?;
                let end_type = self.infer_type(end)?;
                if (start_type.is_numeric() && end_type.is_numeric())
                    || std::mem::discriminant(&start_type) == std::mem::discriminant(&end_type)
                {
                    Ok(QType::Integer(0))
                } else {
                    Err(QError::TypeMismatch("Case range types mismatch".into()))
                }
            }

            Expression::CaseIs { value, .. } => {
                self.infer_type(value)?;
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
        let upper = name.to_uppercase();
        match upper.as_str() {
            // String functions
            "LEFT$" | "RIGHT$" | "MID$" | "LCASE$" | "UCASE$" | "LTRIM$" | "RTRIM$" | "TRIM$"
            | "STR$" | "CHR$" | "SPACE$" | "STRING$" | "HEX$" | "OCT$" | "MKI$" | "MKL$"
            | "MKS$" | "MKD$" | "DATE$" | "TIME$" | "ENVIRON$" | "COMMAND$" | "INKEY$"
            | "ERDEV$" => QType::String(String::new()),

            // Integer functions
            "CINT" | "INT" | "FIX" | "LEN" | "ASC" | "INSTR" | "CSRLIN" | "POS" | "ERR" | "ERL"
            | "ERDEV" | "FREEFILE" => QType::Integer(0),

            // Long functions
            "CLNG" | "FRE" | "LOF" | "LOC" | "VARPTR" | "SADD" => QType::Long(0),

            // Single functions
            "CSNG" | "SIN" | "COS" | "TAN" | "ATN" | "EXP" | "LOG" | "SQR" | "ABS" | "SGN"
            | "RND" | "TIMER" => QType::Single(0.0),

            // Double functions
            "CDBL" | "CVD" => QType::Double(0.0),

            // Type conversion that depends on input
            "CVI" => QType::Integer(0),
            "CVL" => QType::Long(0),
            "CVS" => QType::Single(0.0),

            // Default
            _ => QType::Single(0.0),
        }
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

    pub fn check_statement(&self, stmt: &Statement) -> QResult<()> {
        match stmt {
            Statement::Assignment { target, value } => self.check_assignment(target, value),
            Statement::Print { expressions, .. } => {
                for expr in expressions {
                    self.infer_type(expr)?;
                }
                Ok(())
            }
            Statement::IfBlock {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_condition(condition)?;
                for s in then_branch {
                    self.check_statement(s)?;
                }
                if let Some(else_br) = else_branch {
                    for s in else_br {
                        self.check_statement(s)?;
                    }
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
                // Use get_variable_type for DEFxxx support
                let _ = self.get_variable_type(variable);
                self.infer_type(start)?;
                self.infer_type(end)?;
                if let Some(s) = step {
                    self.infer_type(s)?;
                }
                for b in body {
                    self.check_statement(b)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_condition(&self, expr: &Expression) -> QResult<()> {
        let cond_type = self.infer_type(expr)?;

        if cond_type.is_numeric() {
            Ok(())
        } else {
            Err(QError::TypeMismatch(
                "Condition must be numeric".to_string(),
            ))
        }
    }
}
