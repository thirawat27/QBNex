use super::CodeGenerator;
use core_types::{QError, QResult, QType};
use syntax_tree::ast_nodes::*;

impl CodeGenerator {
    pub(super) fn generate_swap(
        &mut self,
        indent: &str,
        var1: &Expression,
        var2: &Expression,
    ) -> QResult<()> {
        let temp = self.next_temp_var();
        let other = self.next_temp_var();

        if let (Expression::Variable(v1), Expression::Variable(v2)) = (var1, var2) {
            let n1 = &v1.name;
            let n2 = &v2.name;

            if (v1.type_suffix == Some('$') || n1.ends_with('$'))
                && (v2.type_suffix == Some('$') || n2.ends_with('$'))
            {
                let idx1 = self.get_str_var_idx(n1);
                let idx2 = self.get_str_var_idx(n2);
                self.output.push_str(&format!(
                    "{}let {} = get_str(&str_vars, {});\n",
                    indent, temp, idx1
                ));
                self.output.push_str(&format!(
                    "{}let {} = get_str(&str_vars, {});\n",
                    indent, other, idx2
                ));
                self.output.push_str(&format!(
                    "{}set_str(&mut str_vars, {}, &{});\n",
                    indent, idx1, other
                ));
                self.output.push_str(&format!(
                    "{}set_str(&mut str_vars, {}, &{});\n",
                    indent, idx2, temp
                ));
            } else {
                let idx1 = self.get_num_var_idx(n1);
                let idx2 = self.get_num_var_idx(n2);
                self.output.push_str(&format!(
                    "{}let {} = get_var(&num_vars, {});\n",
                    indent, temp, idx1
                ));
                self.output.push_str(&format!(
                    "{}let {} = get_var(&num_vars, {});\n",
                    indent, other, idx2
                ));
                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, {});\n",
                    indent, idx1, other
                ));
                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, {});\n",
                    indent, idx2, temp
                ));
            }
        }
        Ok(())
    }

    pub(super) fn generate_print(
        &mut self,
        indent: &str,
        expressions: &[Expression],
        newline: bool,
    ) -> QResult<()> {
        for expr in expressions {
            let code = self.generate_expression(expr)?;
            self.output.push_str(&format!(
                "{}qb_print(&format!(\"{{}}\", {}));\n",
                indent, code
            ));
        }
        if newline {
            self.output
                .push_str(&format!("{}qb_print_newline();\n", indent));
        }
        Ok(())
    }

    pub(super) fn generate_condition(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::BinaryOp { op, left, right } => {
                let left_code = self
                    .generate_expression(left)
                    .unwrap_or_else(|_| "0.0".to_string());
                let right_code = self
                    .generate_expression(right)
                    .unwrap_or_else(|_| "0.0".to_string());

                // Check if string comparison
                let is_string_cmp = (left_code.contains("get_str")
                    || left_code.contains(".to_string()")
                    || left_code.contains("qb_chr")
                    || left_code.contains("inkey()")
                    || left_code.contains("qb_str"))
                    || (right_code.contains("get_str")
                        || right_code.contains(".to_string()")
                        || right_code.contains("qb_chr")
                        || right_code.contains("inkey()")
                        || right_code.contains("qb_str"))
                    || (left_code.starts_with('"') && left_code.ends_with(".to_string()"))
                    || (right_code.starts_with('"') && right_code.ends_with(".to_string()"))
                    || (left_code.contains("qb_left")
                        || right_code.contains("qb_left")
                        || left_code.contains("qb_right")
                        || right_code.contains("qb_right")
                        || left_code.contains("qb_mid")
                        || right_code.contains("qb_mid")
                        || left_code.contains("qb_string")
                        || right_code.contains("qb_string")
                        || left_code.contains("qb_space")
                        || right_code.contains("qb_space"));

                if is_string_cmp {
                    match op {
                        BinaryOp::Equal => format!("({} == {})", left_code, right_code),
                        BinaryOp::NotEqual => format!("({} != {})", left_code, right_code),
                        // Other string ops not fully supported in simple logic
                        _ => "false".to_string(),
                    }
                } else {
                    match op {
                        BinaryOp::LessThan => format!("({} < {})", left_code, right_code),
                        BinaryOp::GreaterThan => format!("({} > {})", left_code, right_code),
                        BinaryOp::LessOrEqual => format!("({} <= {})", left_code, right_code),
                        BinaryOp::GreaterOrEqual => format!("({} >= {})", left_code, right_code),
                        BinaryOp::Equal => format!("({} == {})", left_code, right_code),
                        BinaryOp::NotEqual => format!("({} != {})", left_code, right_code),
                        _ => format!(
                            "(({} {} {}) as i32 != 0)",
                            left_code,
                            self.op_to_str(op),
                            right_code
                        ),
                    }
                }
            }
            _ => {
                let code = self
                    .generate_expression(expr)
                    .unwrap_or_else(|_| "0.0".to_string());
                format!("({} as i32 != 0)", code)
            }
        }
    }

    pub(super) fn op_to_str(&self, op: &BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::And => "&",
            BinaryOp::Or => "|",
            BinaryOp::Xor => "^",
            _ => "+",
        }
    }

    pub(super) fn evaluate_const_expr(&self, expr: &Expression) -> Option<f64> {
        match expr {
            Expression::Literal(QType::Integer(i)) => Some(*i as f64),
            Expression::Literal(QType::Long(l)) => Some(*l as f64),
            Expression::Literal(QType::Single(s)) => Some(*s as f64),
            Expression::Literal(QType::Double(d)) => Some(*d),
            _ => None,
        }
    }

    pub(super) fn prepare_call_argument(
        &mut self,
        arg: &Expression,
        setup_lines: &mut Vec<String>,
        copy_back_lines: &mut Vec<String>,
    ) -> QResult<String> {
        match arg {
            Expression::Variable(var) => {
                let tmp_var = self.next_temp_var();
                if var.type_suffix == Some('$') || var.name.ends_with('$') {
                    let idx = self.get_str_var_idx(&var.name);
                    let width = self.fixed_string_width_for_name(&var.name);
                    setup_lines.push(format!("let mut {} = get_str(&str_vars, {});", tmp_var, idx));
                    if let Some(width) = width {
                        copy_back_lines.push(format!(
                            "set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}));",
                            idx, width, tmp_var
                        ));
                    } else {
                        copy_back_lines
                            .push(format!("set_str(&mut str_vars, {}, &{});", idx, tmp_var));
                    }
                } else {
                    let idx = self.get_num_var_idx(&var.name);
                    setup_lines.push(format!("let mut {} = get_var(&num_vars, {});", tmp_var, idx));
                    copy_back_lines.push(format!("set_var(&mut num_vars, {}, {});", idx, tmp_var));
                }
                Ok(format!("&mut {}", tmp_var))
            }
            Expression::ArrayAccess {
                name,
                indices,
                type_suffix,
            } => {
                if let Some(index_expr) = indices.first() {
                    let index_tmp = self.next_temp_var();
                    let value_tmp = self.next_temp_var();
                    let index_code = self.generate_expression(index_expr)?;
                    setup_lines.push(format!("let {} = {};", index_tmp, index_code));
                    if type_suffix == &Some('$') || name.ends_with('$') {
                        let arr_idx = self.get_str_arr_var_idx(name);
                        let width = self.fixed_string_width_for_name(name);
                        setup_lines.push(format!(
                            "let mut {} = str_arr_get(&str_arr_vars, {}, {});",
                            value_tmp, arr_idx, index_tmp
                        ));
                        if let Some(width) = width {
                            copy_back_lines.push(format!(
                                "str_arr_set(&mut str_arr_vars, {}, {}, &qb_fit_fixed_string({}, &{}));",
                                arr_idx, index_tmp, width, value_tmp
                            ));
                        } else {
                            copy_back_lines.push(format!(
                                "str_arr_set(&mut str_arr_vars, {}, {}, &{});",
                                arr_idx, index_tmp, value_tmp
                            ));
                        }
                    } else {
                        let arr_idx = self.get_arr_var_idx(name);
                        setup_lines.push(format!(
                            "let mut {} = arr_get(&arr_vars, {}, {});",
                            value_tmp, arr_idx, index_tmp
                        ));
                        copy_back_lines.push(format!(
                            "arr_set(&mut arr_vars, {}, {}, {});",
                            arr_idx, index_tmp, value_tmp
                        ));
                    }
                    Ok(format!("&mut {}", value_tmp))
                } else {
                    let tmp_var = self.next_temp_var();
                    let value_code = self.generate_expression(arg)?;
                    setup_lines.push(format!("let mut {} = {};", tmp_var, value_code));
                    Ok(format!("&mut {}", tmp_var))
                }
            }
            Expression::FieldAccess { .. } => {
                if let Some(field) = self.resolve_field_access_layout(arg) {
                    let value_tmp = self.next_temp_var();
                    match field.field_type {
                        QType::String(_) => {
                            if let Some(index_expr) = &field.array_index {
                                let index_tmp = self.next_temp_var();
                                let index_code = self.generate_expression(index_expr)?;
                                let arr_idx = self.get_str_arr_var_idx(&field.storage_name);
                                setup_lines.push(format!("let {} = {};", index_tmp, index_code));
                                setup_lines.push(format!(
                                    "let mut {} = str_arr_get(&str_arr_vars, {}, {});",
                                    value_tmp, arr_idx, index_tmp
                                ));
                                if let Some(width) = field.fixed_length {
                                    copy_back_lines.push(format!(
                                        "str_arr_set(&mut str_arr_vars, {}, {}, &qb_fit_fixed_string({}, &{}));",
                                        arr_idx, index_tmp, width, value_tmp
                                    ));
                                } else {
                                    copy_back_lines.push(format!(
                                        "str_arr_set(&mut str_arr_vars, {}, {}, &{});",
                                        arr_idx, index_tmp, value_tmp
                                    ));
                                }
                            } else {
                                let idx = self.get_str_var_idx(&field.storage_name);
                                setup_lines.push(format!(
                                    "let mut {} = get_str(&str_vars, {});",
                                    value_tmp, idx
                                ));
                                if let Some(width) = field.fixed_length {
                                    copy_back_lines.push(format!(
                                        "set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}));",
                                        idx, width, value_tmp
                                    ));
                                } else {
                                    copy_back_lines.push(format!(
                                        "set_str(&mut str_vars, {}, &{});",
                                        idx, value_tmp
                                    ));
                                }
                            }
                        }
                        QType::Integer(_)
                        | QType::Long(_)
                        | QType::Single(_)
                        | QType::Double(_) => {
                            if let Some(index_expr) = &field.array_index {
                                let index_tmp = self.next_temp_var();
                                let index_code = self.generate_expression(index_expr)?;
                                let arr_idx = self.get_arr_var_idx(&field.storage_name);
                                setup_lines.push(format!("let {} = {};", index_tmp, index_code));
                                setup_lines.push(format!(
                                    "let mut {} = arr_get(&arr_vars, {}, {});",
                                    value_tmp, arr_idx, index_tmp
                                ));
                                copy_back_lines.push(format!(
                                    "arr_set(&mut arr_vars, {}, {}, {});",
                                    arr_idx, index_tmp, value_tmp
                                ));
                            } else {
                                let idx = self.get_num_var_idx(&field.storage_name);
                                setup_lines.push(format!(
                                    "let mut {} = get_var(&num_vars, {});",
                                    value_tmp, idx
                                ));
                                copy_back_lines.push(format!(
                                    "set_var(&mut num_vars, {}, {});",
                                    idx, value_tmp
                                ));
                            }
                        }
                        _ => {
                            let tmp_var = self.next_temp_var();
                            let value_code = self.generate_expression(arg)?;
                            setup_lines.push(format!("let mut {} = {};", tmp_var, value_code));
                            return Ok(format!("&mut {}", tmp_var));
                        }
                    }
                    Ok(format!("&mut {}", value_tmp))
                } else {
                    let tmp_var = self.next_temp_var();
                    let value_code = self.generate_expression(arg)?;
                    setup_lines.push(format!("let mut {} = {};", tmp_var, value_code));
                    Ok(format!("&mut {}", tmp_var))
                }
            }
            _ => {
                let tmp_var = self.next_temp_var();
                let value_code = self.generate_expression(arg)?;
                setup_lines.push(format!("let mut {} = {};", tmp_var, value_code));
                Ok(format!("&mut {}", tmp_var))
            }
        }
    }

    fn generate_user_function_call(&mut self, name: &str, args: &[Expression]) -> QResult<String> {
        let mut setup_lines = Vec::new();
        let mut copy_back_lines = Vec::new();
        let mut arg_refs = Vec::new();

        for arg in args {
            arg_refs.push(self.prepare_call_argument(
                arg,
                &mut setup_lines,
                &mut copy_back_lines,
            )?);
        }

        let globals = if self.is_in_sub {
            "global_num_vars, global_str_vars, global_arr_vars, global_str_arr_vars"
        } else {
            "&mut num_vars, &mut str_vars, &mut arr_vars, &mut str_arr_vars"
        };
        let func_symbol = self.rust_symbol("func", name);
        let result_tmp = self.next_temp_var();

        let mut block = String::new();
        for line in setup_lines {
            block.push_str(&line);
            block.push(' ');
        }
        block.push_str(&format!("let {} = {}({}", result_tmp, func_symbol, globals));
        for arg_ref in arg_refs {
            block.push_str(", ");
            block.push_str(&arg_ref);
        }
        block.push_str("); ");
        for line in copy_back_lines {
            block.push_str(&line);
            block.push(' ');
        }
        block.push_str(&result_tmp);
        Ok(format!("({{ {} }})", block))
    }

    pub(super) fn generate_expression(&mut self, expr: &Expression) -> QResult<String> {
        match expr {
            Expression::Literal(qtype) => Ok(match qtype {
                QType::Integer(i) => format!("{}.0", i),
                QType::Long(l) => format!("{}.0", l),
                QType::Single(s) => s.to_string(),
                QType::Double(d) => d.to_string(),
                QType::String(s) => format!("\"{}\".to_string()", s.escape_default()),
                QType::Empty => "0.0".to_string(),
                _ => "0.0".to_string(),
            }),
            Expression::Variable(var) => {
                let name = &var.name;
                let name_upper = name.to_uppercase();
                // Check parameters first
                if let Some(param_expr) = self.params.get(name) {
                    Ok(param_expr.clone())
                } else if let Some(builtin_name) = Self::zero_arg_builtin_name(name) {
                    self.generate_builtin_call(builtin_name, &[])
                } else if self.functions.contains(&name_upper)
                    && self
                        .current_function_name
                        .as_ref()
                        .map_or(true, |current| !current.eq_ignore_ascii_case(name))
                {
                    self.generate_user_function_call(name, &[])
                } else if var.type_suffix == Some('$') || name.ends_with('$') {
                    let idx = self.get_str_var_idx(name);
                    Ok(format!("get_str(&str_vars, {})", idx))
                } else {
                    let idx = self.get_num_var_idx(name);
                    Ok(format!("get_var(&num_vars, {})", idx))
                }
            }
            Expression::ArrayAccess {
                name,
                indices,
                type_suffix,
            } => {
                let name_upper = name.to_uppercase();
                let builtin = self.generate_builtin_call(name, indices)?;
                if builtin != "0.0" {
                    Ok(builtin)
                } else if self.functions.contains(&name_upper) {
                    self.generate_user_function_call(name, indices)
                } else if name_upper.starts_with("FN") {
                    let args_code: Vec<String> = indices
                        .iter()
                        .map(|arg| {
                            self.generate_expression(arg)
                                .unwrap_or_else(|_| "0.0".to_string())
                        })
                        .collect();
                    Ok(format!(
                        "{}(&num_vars, &str_vars, {})",
                        self.rust_symbol("qbfn", name),
                        args_code.join(", ")
                    ))
                } else if let Some(idx_expr) = indices.first() {
                    let idx = self.generate_expression(idx_expr)?;
                    if name.ends_with('$') || matches!(type_suffix, Some('$')) {
                        let arr_idx = self.get_str_arr_var_idx(name);
                        Ok(format!("str_arr_get(&str_arr_vars, {}, {})", arr_idx, idx))
                    } else {
                        let arr_idx = self.get_arr_var_idx(name);
                        Ok(format!("arr_get(&arr_vars, {}, {})", arr_idx, idx))
                    }
                } else {
                    Ok("0.0".to_string())
                }
            }
            Expression::FieldAccess { .. } => {
                if let Some(field) = self.resolve_field_access_layout(expr) {
                    match field.field_type {
                        QType::String(_) => {
                            if let Some(index_expr) = &field.array_index {
                                let arr_idx = self.get_str_arr_var_idx(&field.storage_name);
                                let index_code = self.generate_expression(index_expr)?;
                                Ok(format!(
                                    "str_arr_get(&str_arr_vars, {}, {})",
                                    arr_idx, index_code
                                ))
                            } else {
                                let idx = self.get_str_var_idx(&field.storage_name);
                                Ok(format!("get_str(&str_vars, {})", idx))
                            }
                        }
                        QType::Integer(_)
                        | QType::Long(_)
                        | QType::Single(_)
                        | QType::Double(_) => {
                            if let Some(index_expr) = &field.array_index {
                                let arr_idx = self.get_arr_var_idx(&field.storage_name);
                                let index_code = self.generate_expression(index_expr)?;
                                Ok(format!("arr_get(&arr_vars, {}, {})", arr_idx, index_code))
                            } else {
                                let idx = self.get_num_var_idx(&field.storage_name);
                                Ok(format!("get_var(&num_vars, {})", idx))
                            }
                        }
                        _ => Ok("0.0".to_string()),
                    }
                } else {
                    Ok("0.0".to_string())
                }
            }
            Expression::BinaryOp { op, left, right } => {
                let left_code = self.generate_expression(left)?;
                let right_code = self.generate_expression(right)?;
                match op {
                    BinaryOp::Add => Ok(format!("({} + {})", left_code, right_code)),
                    BinaryOp::Subtract => Ok(format!("({} - {})", left_code, right_code)),
                    BinaryOp::Multiply => Ok(format!("({} * {})", left_code, right_code)),
                    BinaryOp::Divide => Ok(format!("({} / {})", left_code, right_code)),
                    BinaryOp::IntegerDivide => Ok(format!(
                        "((({}) as f64 / ({}) as f64).floor())",
                        left_code, right_code
                    )),
                    BinaryOp::Modulo => Ok(format!(
                        "((({} as i64) % ({} as i64)) as f64)",
                        left_code, right_code
                    )),
                    BinaryOp::Power => Ok(format!("({}).powf({})", left_code, right_code)),
                    BinaryOp::Equal => Ok(format!(
                        "(if ({} == {}) {{ -1.0 }} else {{ 0.0 }})",
                        left_code, right_code
                    )),
                    BinaryOp::NotEqual => Ok(format!(
                        "(if ({} != {}) {{ -1.0 }} else {{ 0.0 }})",
                        left_code, right_code
                    )),
                    BinaryOp::LessThan => Ok(format!(
                        "(if ({} < {}) {{ -1.0 }} else {{ 0.0 }})",
                        left_code, right_code
                    )),
                    BinaryOp::GreaterThan => Ok(format!(
                        "(if ({} > {}) {{ -1.0 }} else {{ 0.0 }})",
                        left_code, right_code
                    )),
                    BinaryOp::LessOrEqual => Ok(format!(
                        "(if ({} <= {}) {{ -1.0 }} else {{ 0.0 }})",
                        left_code, right_code
                    )),
                    BinaryOp::GreaterOrEqual => Ok(format!(
                        "(if ({} >= {}) {{ -1.0 }} else {{ 0.0 }})",
                        left_code, right_code
                    )),
                    BinaryOp::And => Ok(format!(
                        "((({} as i64) & ({} as i64)) as f64)",
                        left_code, right_code
                    )),
                    BinaryOp::Or => Ok(format!(
                        "((({} as i64) | ({} as i64)) as f64)",
                        left_code, right_code
                    )),
                    BinaryOp::Xor => Ok(format!(
                        "((({} as i64) ^ ({} as i64)) as f64)",
                        left_code, right_code
                    )),
                    BinaryOp::Eqv => Ok(format!(
                        "((!(({} as i64) ^ ({} as i64))) as f64)",
                        left_code, right_code
                    )),
                    BinaryOp::Imp => Ok(format!(
                        "((((!({} as i64)) | ({} as i64))) as f64)",
                        left_code, right_code
                    )),
                }
            }

            Expression::UnaryOp { op, operand } => {
                let operand_code = self.generate_expression(operand)?;
                match op {
                    UnaryOp::Negate => Ok(format!("(-{})", operand_code)),
                    UnaryOp::Not => Ok(format!("((!({} as i64)) as f64)", operand_code)),
                }
            }

            Expression::CaseRange { .. } | Expression::CaseIs { .. } | Expression::CaseElse => Err(
                core_types::QError::Internal("Case expression used in value context".to_string()),
            ),

            Expression::FunctionCall(func) => self.generate_function_call(func),

            _ => Ok("0.0".to_string()),
        }
    }

    pub(super) fn generate_function_call(&mut self, func: &FunctionCall) -> QResult<String> {
        let builtin = self.generate_builtin_call(&func.name, &func.args)?;
        if builtin != "0.0" {
            Ok(builtin)
        } else if func.name.to_uppercase().starts_with("FN") {
            let args_code: Vec<String> = func
                .args
                .iter()
                .map(|arg| {
                    self.generate_expression(arg)
                        .unwrap_or_else(|_| "0.0".to_string())
                })
                .collect();
            Ok(format!(
                "{}(&num_vars, &str_vars, {})",
                self.rust_symbol("qbfn", &func.name),
                args_code.join(", ")
            ))
        } else {
            self.generate_user_function_call(&func.name, &func.args)
        }
    }

    pub(super) fn generate_builtin_call(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> QResult<String> {
        let name_upper = name.to_uppercase();
        match name_upper.as_str() {
            "ABS" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_abs({})", arg))
            }
            "SGN" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_sgn({})", arg))
            }
            "INT" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_int({})", arg))
            }
            "FIX" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_fix({})", arg))
            }
            "SQR" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_sqr({})", arg))
            }
            "SIN" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_sin({})", arg))
            }
            "COS" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_cos({})", arg))
            }
            "TAN" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_tan({})", arg))
            }
            "ATN" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_atn({})", arg))
            }
            "EXP" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_exp({})", arg))
            }
            "LOG" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_log({})", arg))
            }
            "RND" => Ok("qb_rnd()".to_string()),
            "CINT" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_cint({})", arg))
            }
            "CLNG" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_clng({})", arg))
            }
            "CSNG" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_csng({})", arg))
            }
            "CDBL" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_cdbl({})", arg))
            }
            "MKI$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_mki({})", arg))
            }
            "MKL$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_mkl({})", arg))
            }
            "MKS$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_mks({})", arg))
            }
            "MKD$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_mkd({})", arg))
            }
            "CVI" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_cvi(&{})", arg))
            }
            "CVL" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_cvl(&{})", arg))
            }
            "CVS" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_cvs(&{})", arg))
            }
            "CVD" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_cvd(&{})", arg))
            }
            "FRE" => {
                let arg = self.generate_expression(&args[0])?;
                if arg.contains("to_string()") || arg.contains("get_str(") {
                    Ok(format!("qb_fre(&{})", arg))
                } else {
                    Ok(format!("qb_fre_num({})", arg))
                }
            }
            "PEEK" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_peek({})", arg))
            }
            "INP" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_inp({})", arg))
            }
            "VARPTR" => {
                let arg = self.generate_expression(&args[0])?;
                let name = self.pointer_ref_name(&args[0])?;
                Ok(format!("qb_varptr(\"{}\", {})", name, arg))
            }
            "VARSEG" => {
                let arg = self.generate_expression(&args[0])?;
                let name = self.pointer_ref_name(&args[0])?;
                Ok(format!("qb_varseg(\"{}\", {})", name, arg))
            }
            "SADD" => {
                let arg = self.generate_expression(&args[0])?;
                let name = self.pointer_ref_name(&args[0])?;
                Ok(format!("qb_sadd(\"{}\", {})", name, arg))
            }
            "VARPTR$" => {
                let arg = self.generate_expression(&args[0])?;
                let name = self.pointer_ref_name(&args[0])?;
                Ok(format!("qb_varptr_str(\"{}\", {})", name, arg))
            }
            "TIMER" => Ok("qb_timer()".to_string()),
            "DATE" => Ok("qb_date()".to_string()),
            "TIME" => Ok("qb_time()".to_string()),
            "COMMAND" => Ok("qb_command()".to_string()),
            "CSRLIN" => Ok("qb_csrlin()".to_string()),
            "ERR" => Ok("qb_err()".to_string()),
            "ERL" => Ok("qb_erl()".to_string()),
            "ERDEV" => Ok("qb_erdev()".to_string()),
            "ERDEVSTR" => Ok("qb_erdev_str()".to_string()),
            "FREEFILE" => Ok("qb_freefile()".to_string()),

            "INKEY$" => Ok("qb_inkey()".to_string()),
            "INPUT$" => {
                let count = self.generate_expression(&args[0])?;
                if let Some(file_number) = args.get(1) {
                    let file_number = self.generate_expression(file_number)?;
                    Ok(format!("qb_input_str({}, Some({}))", count, file_number))
                } else {
                    Ok(format!("qb_input_str({}, None)", count))
                }
            }
            "ENVIRON$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_environ(&{})", arg))
            }
            "LEN" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_len(&{})", arg))
            }
            "LTRIM$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_ltrim(&{})", arg))
            }
            "RTRIM$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_rtrim(&{})", arg))
            }
            "UCASE$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_ucase(&{})", arg))
            }
            "LCASE$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_lcase(&{})", arg))
            }
            "LEFT$" => {
                let s = self.generate_expression(&args[0])?;
                let n = self.generate_expression(&args[1])?;
                Ok(format!("qb_left(&{}, {})", s, n))
            }
            "RIGHT$" => {
                let s = self.generate_expression(&args[0])?;
                let n = self.generate_expression(&args[1])?;
                Ok(format!("qb_right(&{}, {})", s, n))
            }
            "MID$" => {
                let s = self.generate_expression(&args[0])?;
                let start = self.generate_expression(&args[1])?;
                if args.len() > 2 {
                    let len = self.generate_expression(&args[2])?;
                    Ok(format!("qb_mid(&{}, {}, {})", s, start, len))
                } else {
                    Ok(format!("qb_mid_no_len(&{}, {})", s, start))
                }
            }
            "INSTR" => {
                let s = self.generate_expression(&args[0])?;
                let substr = self.generate_expression(&args[1])?;
                Ok(format!("qb_instr(&{}, &{})", s, substr))
            }
            "STRING$" => {
                let n = self.generate_expression(&args[0])?;
                let c = self.generate_expression(&args[1])?;
                Ok(format!("qb_string({}, &{})", n, c))
            }
            "SPACE$" => {
                let n = self.generate_expression(&args[0])?;
                Ok(format!("qb_space({})", n))
            }
            "CHR$" => {
                let n = self.generate_expression(&args[0])?;
                Ok(format!("qb_chr({})", n))
            }
            "ASC" => {
                let s = self.generate_expression(&args[0])?;
                Ok(format!("qb_asc(&{})", s))
            }
            "VAL" => {
                let s = self.generate_expression(&args[0])?;
                Ok(format!("qb_val(&{})", s))
            }
            "STR$" => {
                let n = self.generate_expression(&args[0])?;
                Ok(format!("qb_str({})", n))
            }
            "HEX$" => {
                let n = self.generate_expression(&args[0])?;
                Ok(format!("qb_hex({})", n))
            }
            "OCT$" => {
                let n = self.generate_expression(&args[0])?;
                Ok(format!("qb_oct({})", n))
            }
            "LBOUND" => {
                if let Some(Expression::Variable(v)) = args.first() {
                    if v.type_suffix == Some('$') || v.name.ends_with('$') {
                        let idx = self.get_str_arr_var_idx(&v.name);
                        Ok(format!("qb_lbound(&str_arr_vars[{}])", idx))
                    } else {
                        let idx = self.get_arr_var_idx(&v.name);
                        Ok(format!("qb_lbound(&arr_vars[{}])", idx))
                    }
                } else {
                    Ok("0.0".to_string())
                }
            }
            "UBOUND" => {
                if let Some(Expression::Variable(v)) = args.first() {
                    if v.type_suffix == Some('$') || v.name.ends_with('$') {
                        let idx = self.get_str_arr_var_idx(&v.name);
                        Ok(format!("qb_ubound(&str_arr_vars[{}])", idx))
                    } else {
                        let idx = self.get_arr_var_idx(&v.name);
                        Ok(format!("qb_ubound(&arr_vars[{}])", idx))
                    }
                } else {
                    Ok("0.0".to_string())
                }
            }
            "POS" => {
                let arg = if args.is_empty() {
                    "0.0".to_string()
                } else {
                    self.generate_expression(&args[0])?
                };
                Ok(format!("qb_pos({})", arg))
            }
            "LPOS" => {
                let arg = if args.is_empty() {
                    "0.0".to_string()
                } else {
                    self.generate_expression(&args[0])?
                };
                Ok(format!("qb_pos({})", arg))
            }
            "EOF" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_eof({})", arg))
            }
            "LOF" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_lof({})", arg))
            }
            "LOC" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_loc({})", arg))
            }

            _ => Ok("0.0".to_string()),
        }
    }

    fn pointer_ref_name(&self, expr: &Expression) -> QResult<String> {
        match expr {
            Expression::Variable(var) => Ok(var.name.clone()),
            Expression::ArrayAccess { name, .. } => Ok(name.clone()),
            Expression::FieldAccess { field, .. } => Ok(field.clone()),
            _ => Err(QError::Syntax(
                "VARPTR/VARSEG/SADD/VARPTR$ require a variable-compatible reference".to_string(),
            )),
        }
    }

    fn zero_arg_builtin_name(name: &str) -> Option<&'static str> {
        match name.to_uppercase().as_str() {
            "TIMER" => Some("TIMER"),
            "RND" => Some("RND"),
            "DATE$" => Some("DATE"),
            "TIME$" => Some("TIME"),
            "INKEY$" => Some("INKEY$"),
            "CSRLIN" => Some("CSRLIN"),
            "FREEFILE" => Some("FREEFILE"),
            "COMMAND$" => Some("COMMAND"),
            "ERR" => Some("ERR"),
            "ERL" => Some("ERL"),
            "ERDEV" => Some("ERDEV"),
            "ERDEV$" => Some("ERDEVSTR"),
            _ => None,
        }
    }
}
