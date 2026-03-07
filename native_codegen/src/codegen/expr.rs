use super::CodeGenerator;
use core_types::{QResult, QType};
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
            self.output
                .push_str(&format!("{}print!(\"{{}}\", {});\n", indent, code));
        }
        if newline {
            self.output.push_str(&format!("{}println!();\n", indent));
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
                // Check parameters first
                if let Some(param_expr) = self.params.get(name) {
                    Ok(param_expr.clone())
                } else if var.type_suffix == Some('$') || name.ends_with('$') {
                    let idx = self.get_str_var_idx(name);
                    Ok(format!("get_str(&str_vars, {})", idx))
                } else {
                    let idx = self.get_num_var_idx(name);
                    Ok(format!("get_var(&num_vars, {})", idx))
                }
            }
            Expression::ArrayAccess { name, indices, .. } => {
                let name_upper = name.to_uppercase();
                let builtin = self.generate_builtin_call(name, indices)?;
                if builtin != "0.0" {
                    Ok(builtin)
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
                    let arr_idx = self.get_arr_var_idx(name);
                    let idx = self.generate_expression(idx_expr)?;
                    Ok(format!("arr_get(&arr_vars, {}, {})", arr_idx, idx))
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
            // User defined function call
            let args_code: Vec<String> = func
                .args
                .iter()
                .map(|arg| {
                    self.generate_expression(arg)
                        .unwrap_or_else(|_| "0.0".to_string())
                })
                .collect();
            let args_str = args_code.join(", ");

            let global_args = if self.is_in_sub {
                "global_num_vars, global_str_vars, global_arr_vars"
            } else {
                "&mut num_vars, &mut str_vars, &mut arr_vars"
            };

            Ok(format!(
                "{}({}, {})",
                self.rust_symbol("func", &func.name),
                global_args,
                args_str
            ))
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
            "VARPTR" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_varptr(&{})", arg))
            }
            "VARSEG" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_varseg(&{})", arg))
            }
            "SADD" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_sadd(&{})", arg))
            }
            "VARPTR$" => {
                let arg = self.generate_expression(&args[0])?;
                Ok(format!("qb_varptr_str(&{})", arg))
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

            "INKEY$" => Ok("inkey()".to_string()),
            "INPUT$" => {
                let count = self.generate_expression(&args[0])?;
                Ok(format!("qb_input_str({})", count))
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
            "LBOUND" => Ok("1.0".to_string()),
            "UBOUND" => {
                if let Some(Expression::Variable(v)) = args.first() {
                    let idx = self.get_arr_var_idx(&v.name);
                    Ok(format!("qb_ubound(&arr_vars[{}])", idx))
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

            _ => Ok("0.0".to_string()),
        }
    }
}
