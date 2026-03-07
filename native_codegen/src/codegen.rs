use core_types::QResult;
use syntax_tree::ast_nodes::*;

#[path = "codegen/expr.rs"]
mod expr;
#[path = "codegen/graphics.rs"]
mod graphics;
#[path = "codegen/program.rs"]
mod program;
#[path = "codegen/state.rs"]
mod state;

pub use state::CodeGenerator;

impl CodeGenerator {
    fn generate_statement(&mut self, stmt: &Statement) -> QResult<()> {
        let indent = self.indent();

        if self.emit_graphics_statement(&indent, stmt)? {
            return Ok(());
        }

        match stmt {
            Statement::Print {
                expressions,
                newline,
            } => {
                self.generate_print(&indent, expressions, *newline)?;
            }

            Statement::Assignment { target, value } => {
                let value_code = self.generate_expression(value)?;
                match target {
                    Expression::Variable(var) => {
                        let name_upper = var.name.to_uppercase();
                        if var.type_suffix == Some('$') || name_upper.ends_with('$') {
                            let idx = self.get_str_var_idx(&name_upper);
                            // Avoid mutable borrow conflict
                            self.output
                                .push_str(&format!("{}let _tmp_str = {};\n", indent, value_code));
                            self.output.push_str(&format!(
                                "{}set_str(&mut str_vars, {}, &_tmp_str);\n",
                                indent, idx
                            ));
                        } else {
                            let idx = self.get_num_var_idx(&name_upper);
                            self.output.push_str(&format!(
                                "{}let _val = {} as f64;\n",
                                indent, value_code
                            ));
                            self.output.push_str(&format!(
                                "{}set_var(&mut num_vars, {}, _val);\n",
                                indent, idx
                            ));
                        }
                    }
                    Expression::ArrayAccess { name, indices, .. } => {
                        if name.eq_ignore_ascii_case("MID$") {
                            if let Some(Expression::Variable(var)) = indices.first() {
                                let start_code = indices
                                    .get(1)
                                    .map(|expr| self.generate_expression(expr))
                                    .transpose()?
                                    .unwrap_or_else(|| "1.0".to_string());
                                let len_code = indices
                                    .get(2)
                                    .map(|expr| self.generate_expression(expr))
                                    .transpose()?
                                    .unwrap_or_else(|| {
                                        format!("({}.chars().count() as f64)", value_code)
                                    });
                                let idx = self.get_str_var_idx(&var.name);
                                let src_tmp = self.next_temp_var();
                                let repl_tmp = self.next_temp_var();
                                let chars_tmp = self.next_temp_var();
                                let start_tmp = self.next_temp_var();
                                let len_tmp = self.next_temp_var();
                                let result_tmp = self.next_temp_var();
                                self.output.push_str(&format!(
                                    "{}let {} = get_str(&str_vars, {});\n",
                                    indent, src_tmp, idx
                                ));
                                self.output.push_str(&format!(
                                    "{}let {} = {};\n",
                                    indent, repl_tmp, value_code
                                ));
                                self.output.push_str(&format!(
                                    "{}let mut {}: Vec<char> = {}.chars().collect();\n",
                                    indent, chars_tmp, src_tmp
                                ));
                                self.output.push_str(&format!(
                                    "{}let {} = (({}) as i32 - 1).max(0) as usize;\n",
                                    indent, start_tmp, start_code
                                ));
                                self.output.push_str(&format!(
                                    "{}let {} = (({}) as i32).max(0) as usize;\n",
                                    indent, len_tmp, len_code
                                ));
                                self.output.push_str(&format!(
                                    "{}while {}.len() < {} {{ {}.push(' '); }}\n",
                                    indent, chars_tmp, start_tmp, chars_tmp
                                ));
                                self.output.push_str(&format!(
                                    "{}for (i, ch) in {}.chars().take({}).enumerate() {{\n",
                                    indent, repl_tmp, len_tmp
                                ));
                                self.output.push_str(&format!(
                                    "{}    let pos = {} + i;\n",
                                    indent, start_tmp
                                ));
                                self.output.push_str(&format!(
                                    "{}    if pos < {}.len() {{ {}[pos] = ch; }} else {{ {}.push(ch); }}\n",
                                    indent, chars_tmp, chars_tmp, chars_tmp
                                ));
                                self.output.push_str(&format!("{}}}\n", indent));
                                self.output.push_str(&format!(
                                    "{}let {}: String = {}.into_iter().collect();\n",
                                    indent, result_tmp, chars_tmp
                                ));
                                self.output.push_str(&format!(
                                    "{}set_str(&mut str_vars, {}, &{});\n",
                                    indent, idx, result_tmp
                                ));
                            }
                        } else if let Some(idx) = indices.first() {
                            let arr_idx = self.get_arr_var_idx(name);
                            let idx_code = self.generate_expression(idx)?;
                            self.output.push_str(&format!(
                                "{}let _val = {} as f64;\n",
                                indent, value_code
                            ));
                            self.output.push_str(&format!(
                                "{}arr_set(&mut arr_vars, {}, {}, _val);\n",
                                indent, arr_idx, idx_code
                            ));
                        }
                    }
                    _ => {}
                }
            }

            Statement::Dim { variables, .. } => {
                for (var, dim_expr) in variables {
                    let var_name = &var.name;
                    if var.type_suffix == Some('$') || var_name.ends_with('$') {
                        continue; // String array not fully supported
                    }

                    if let Some(expr) = dim_expr {
                        let size = self.evaluate_const_expr(expr).unwrap_or(10.0) as usize;
                        let idx = self.get_arr_var_idx(var_name);
                        // If idx >= arr_vars.len(), we have a problem because arr_vars is pre-allocated?
                        // But we can re-allocate if needed or check bounds.
                        // However, main() pre-allocates based on collected vars.
                        // So idx is valid.
                        // We just need to set the inner vector size.
                        self.output.push_str(&format!(
                            "{}if {} < arr_vars.len() {{ arr_vars[{}] = vec![0.0; {}]; }}\n",
                            indent, idx, idx, size
                        ));
                    }
                }
            }

            Statement::Redim { variables, .. } => {
                for (var, dim_expr) in variables {
                    if let Some(expr) = dim_expr {
                        let size = self.evaluate_const_expr(expr).unwrap_or(10.0) as usize;
                        let idx = self.get_arr_var_idx(&var.name);
                        self.output.push_str(&format!(
                            "{}if {} < arr_vars.len() {{ arr_vars[{}] = vec![0.0; {}]; }}\n",
                            indent, idx, idx, size
                        ));
                    }
                }
            }

            Statement::Erase { variables } => {
                for var in variables {
                    let idx = self.get_arr_var_idx(&var.name);
                    self.output.push_str(&format!(
                        "{}if {} < arr_vars.len() {{ arr_vars[{}] = Vec::new(); }}\n",
                        indent, idx, idx
                    ));
                }
            }

            Statement::IfBlock {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_code = self.generate_expression(condition)?;
                self.output
                    .push_str(&format!("{}if ({} as i32) != 0 {{\n", indent, cond_code));
                self.indent_level += 1;
                for stmt in then_branch {
                    self.generate_statement(stmt)?;
                }
                self.indent_level -= 1;
                if let Some(else_stmts) = else_branch {
                    self.output.push_str(&format!("{}}} else {{\n", indent));
                    self.indent_level += 1;
                    for stmt in else_stmts {
                        self.generate_statement(stmt)?;
                    }
                    self.indent_level -= 1;
                }
                self.output.push_str(&format!("{}}}\n", indent));
            }

            Statement::Select { expression, cases } => {
                let expr_code = self.generate_expression(expression)?;
                let temp = self.next_temp_var();
                self.output
                    .push_str(&format!("{}let {} = {};\n", indent, temp, expr_code));

                let mut first = true;
                for (case_val, stmts) in cases {
                    let condition = match case_val {
                        Expression::CaseRange { start, end } => {
                            let s = self.generate_expression(start)?;
                            let e = self.generate_expression(end)?;
                            format!("({} >= {} && {} <= {})", temp, s, temp, e)
                        }
                        Expression::CaseIs { op, value } => {
                            let v = self.generate_expression(value)?;
                            match op {
                                BinaryOp::LessThan => format!("({} < {})", temp, v),
                                BinaryOp::GreaterThan => format!("({} > {})", temp, v),
                                BinaryOp::LessOrEqual => format!("({} <= {})", temp, v),
                                BinaryOp::GreaterOrEqual => format!("({} >= {})", temp, v),
                                BinaryOp::Equal => format!("({} == {})", temp, v),
                                BinaryOp::NotEqual => format!("({} != {})", temp, v),
                                _ => "false".to_string(),
                            }
                        }
                        Expression::CaseElse => "true".to_string(),
                        _ => {
                            let v = self.generate_expression(case_val)?;
                            format!("(({} - {}).abs() < 0.0001)", temp, v)
                        }
                    };

                    if first {
                        self.output
                            .push_str(&format!("{}if {} {{\n", indent, condition));
                        first = false;
                    } else {
                        self.output
                            .push_str(&format!("{}}} else if {} {{\n", indent, condition));
                    }
                    self.indent_level += 1;
                    for stmt in stmts {
                        self.generate_statement(stmt)?;
                    }
                    self.indent_level -= 1;
                }
                if !first {
                    self.output.push_str(&format!("{}}}\n", indent));
                }
            }

            Statement::ForLoop {
                variable,
                start,
                end,
                step,
                body,
            } => {
                let var_name = &variable.name;
                let var_idx = self.get_num_var_idx(var_name);
                let start_code = self.generate_expression(start)?;
                let end_code = self.generate_expression(end)?;
                let step_code = step
                    .as_ref()
                    .map(|s| {
                        self.generate_expression(s)
                            .unwrap_or_else(|_| "1.0".to_string())
                    })
                    .unwrap_or_else(|| "1.0".to_string());

                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, {} as f64);\n",
                    indent, var_idx, start_code
                ));
                self.output.push_str(&format!(
                    "{}while get_var(&num_vars, {}) <= {} {{\n",
                    indent, var_idx, end_code
                ));
                self.indent_level += 1;
                for stmt in body {
                    self.generate_statement(stmt)?;
                }
                self.output.push_str(&format!(
                    "{}let _next = get_var(&num_vars, {}) + {};\n",
                    self.indent(),
                    var_idx,
                    step_code
                ));
                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, _next);\n",
                    self.indent(),
                    var_idx
                ));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }

            Statement::WhileLoop { condition, body } => {
                let cond_code = self.generate_condition(condition);
                self.output
                    .push_str(&format!("{}while {} {{\n", indent, cond_code));
                self.indent_level += 1;
                for stmt in body {
                    self.generate_statement(stmt)?;
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }

            Statement::DoLoop {
                condition,
                body,
                pre_condition,
            } => {
                if *pre_condition {
                    if let Some(cond) = condition {
                        let cond_code = self.generate_condition(cond);
                        self.output
                            .push_str(&format!("{}while {} {{\n", indent, cond_code));
                    } else {
                        self.output.push_str(&format!("{}loop {{\n", indent));
                    }
                } else {
                    self.output.push_str(&format!("{}loop {{\n", indent));
                }
                self.indent_level += 1;
                for stmt in body {
                    self.generate_statement(stmt)?;
                }
                if !*pre_condition {
                    if let Some(cond) = condition {
                        let cond_code = self.generate_condition(cond);
                        self.output.push_str(&format!(
                            "{}if !{} {{ break; }}\n",
                            self.indent(),
                            cond_code
                        ));
                    }
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }

            Statement::Call { name, args } => {
                let sub_name = self.rust_symbol("sub", name);

                let mut arg_vars = Vec::new();
                let mut copy_back_ops = Vec::new();

                for arg in args {
                    match arg {
                        Expression::Variable(v) => {
                            let n = &v.name;
                            let tmp_var = self.next_temp_var();

                            if v.type_suffix == Some('$') || n.ends_with('$') {
                                let idx = self.get_str_var_idx(n);
                                self.output.push_str(&format!(
                                    "{}let mut {} = get_str(&str_vars, {});\n",
                                    indent, tmp_var, idx
                                ));
                                arg_vars.push(format!("&mut {}", tmp_var));
                                copy_back_ops.push(format!(
                                    "set_str(&mut str_vars, {}, &{});",
                                    idx, tmp_var
                                ));
                            } else {
                                let idx = self.get_num_var_idx(n);
                                self.output.push_str(&format!(
                                    "{}let mut {} = get_var(&num_vars, {});\n",
                                    indent, tmp_var, idx
                                ));
                                arg_vars.push(format!("&mut {}", tmp_var));
                                copy_back_ops
                                    .push(format!("set_var(&mut num_vars, {}, {});", idx, tmp_var));
                            }
                        }
                        _ => {
                            let val_code = self.generate_expression(arg)?;
                            let tmp_var = self.next_temp_var();
                            self.output.push_str(&format!(
                                "{}let mut {} = {};\n",
                                indent, tmp_var, val_code
                            ));
                            arg_vars.push(format!("&mut {}", tmp_var));
                        }
                    }
                }

                let global_args = if self.is_in_sub {
                    "global_num_vars, global_str_vars, global_arr_vars"
                } else {
                    "&mut num_vars, &mut str_vars, &mut arr_vars"
                };

                self.output
                    .push_str(&format!("{}{}({}", indent, sub_name, global_args));
                for arg in arg_vars {
                    self.output.push_str(", ");
                    self.output.push_str(&arg);
                }
                self.output.push_str(");\n");

                for op in copy_back_ops {
                    self.output.push_str(&format!("{}{}\n", indent, op));
                }
            }

            Statement::Swap { var1, var2 } => {
                self.generate_swap(&indent, var1, var2)?;
            }

            Statement::Exit { exit_type } => match exit_type {
                ExitType::Sub => self.output.push_str(&format!("{}return;\n", indent)),
                ExitType::Function => {
                    let func_name = self.current_function_name.clone();
                    if let Some(name) = func_name {
                        if name.ends_with('$') {
                            let idx = self.get_str_var_idx(&name);
                            self.output.push_str(&format!(
                                "{}return get_str(&str_vars, {});\n",
                                indent, idx
                            ));
                        } else {
                            let idx = self.get_num_var_idx(&name);
                            self.output.push_str(&format!(
                                "{}return get_var(&num_vars, {});\n",
                                indent, idx
                            ));
                        }
                    } else {
                        self.output.push_str(&format!("{}return 0.0;\n", indent));
                    }
                }
                ExitType::For => self.output.push_str(&format!("{}break;\n", indent)),
                ExitType::Do => self.output.push_str(&format!("{}break;\n", indent)),
            },

            Statement::Screen { mode } => {
                if self.use_graphics {
                    let mode_code = if let Some(m) = mode {
                        self.generate_expression(m).unwrap_or("0.0".to_string())
                    } else {
                        "0.0".to_string()
                    };
                    self.output
                        .push_str(&format!("{}match {} as i32 {{\n", indent, mode_code));
                    self.output
                        .push_str(&format!("{}    13 => init_graphics(320, 200),\n", indent));
                    self.output
                        .push_str(&format!("{}    12 => init_graphics(640, 480),\n", indent));
                    self.output
                        .push_str(&format!("{}    _ => init_graphics(80, 25),\n", indent));
                    self.output.push_str(&format!("{}}}\n", indent));
                }
            }

            Statement::Pset { coords, color } => {
                if self.use_graphics {
                    let (x_expr, y_expr) = coords;
                    let x = self.generate_expression(x_expr)?;
                    let y = self.generate_expression(y_expr)?;
                    let c = if let Some(col) = color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    self.output
                        .push_str(&format!("{}pset({}, {}, {});\n", indent, x, y, c));
                    // Don't update screen after every pixel - let program control updates
                }
            }

            Statement::Preset { coords, color } => {
                if self.use_graphics {
                    let (x_expr, y_expr) = coords;
                    let x = self.generate_expression(x_expr)?;
                    let y = self.generate_expression(y_expr)?;
                    let c = if let Some(col) = color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    self.output
                        .push_str(&format!("{}preset({}, {}, {});\n", indent, x, y, c));
                }
            }

            Statement::Line { coords, color, .. } => {
                if self.use_graphics {
                    let ((x1_expr, y1_expr), (x2_expr, y2_expr)) = coords;
                    let x1 = self.generate_expression(x1_expr)?;
                    let y1 = self.generate_expression(y1_expr)?;
                    let x2 = self.generate_expression(x2_expr)?;
                    let y2 = self.generate_expression(y2_expr)?;
                    let c = if let Some(col) = color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    self.output.push_str(&format!(
                        "{}line({}, {}, {}, {}, {});\n",
                        indent, x1, y1, x2, y2, c
                    ));
                    // Update screen after line drawing
                    self.output
                        .push_str(&format!("{}update_screen();\n", indent));
                }
            }

            Statement::Circle {
                center,
                radius,
                color,
                ..
            } => {
                if self.use_graphics {
                    let x = self.generate_expression(&center.0)?;
                    let y = self.generate_expression(&center.1)?;
                    let r = self.generate_expression(radius)?;
                    let c = if let Some(col) = color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    self.output
                        .push_str(&format!("{}circle({}, {}, {}, {});\n", indent, x, y, r, c));
                    self.output
                        .push_str(&format!("{}update_screen();\n", indent));
                }
            }

            Statement::Paint {
                coords,
                paint_color,
                border_color,
            } => {
                if self.use_graphics {
                    let x = self.generate_expression(&coords.0)?;
                    let y = self.generate_expression(&coords.1)?;
                    let paint = if let Some(col) = paint_color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    let border = if let Some(col) = border_color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    self.output.push_str(&format!(
                        "{}paint({}, {}, {}, {});\n",
                        indent, x, y, paint, border
                    ));
                    self.output
                        .push_str(&format!("{}update_screen();\n", indent));
                }
            }

            Statement::Draw { commands } => {
                if self.use_graphics {
                    let commands = self.generate_expression(commands)?;
                    self.output
                        .push_str(&format!("{}draw_cmd(&{});\n", indent, commands));
                    self.output
                        .push_str(&format!("{}update_screen();\n", indent));
                }
            }

            Statement::Palette { attribute, color } => {
                if self.use_graphics {
                    let attr = self.generate_expression(attribute)?;
                    let color = if let Some(col) = color {
                        self.generate_expression(col)?
                    } else {
                        "0.0".to_string()
                    };
                    self.output
                        .push_str(&format!("{}palette_set({}, {});\n", indent, attr, color));
                    self.output
                        .push_str(&format!("{}update_screen();\n", indent));
                }
            }

            Statement::View {
                coords,
                fill_color,
                border_color,
            } => {
                if self.use_graphics {
                    let x1 = self.generate_expression(&coords.0 .0)?;
                    let y1 = self.generate_expression(&coords.0 .1)?;
                    let x2 = self.generate_expression(&coords.1 .0)?;
                    let y2 = self.generate_expression(&coords.1 .1)?;
                    let fill = if let Some(col) = fill_color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    let border = if let Some(col) = border_color {
                        self.generate_expression(col)?
                    } else {
                        "-1.0".to_string()
                    };
                    self.output.push_str(&format!(
                        "{}view_rect({}, {}, {}, {}, {}, {});\n",
                        indent, x1, y1, x2, y2, fill, border
                    ));
                    self.output
                        .push_str(&format!("{}update_screen();\n", indent));
                }
            }

            Statement::ViewReset => {
                if self.use_graphics {
                    self.output
                        .push_str(&format!("{}view_reset_qb();\n", indent));
                    self.output
                        .push_str(&format!("{}update_screen();\n", indent));
                }
            }

            Statement::Window { coords } => {
                if self.use_graphics {
                    let x1 = self.generate_expression(&coords.0 .0)?;
                    let y1 = self.generate_expression(&coords.0 .1)?;
                    let x2 = self.generate_expression(&coords.1 .0)?;
                    let y2 = self.generate_expression(&coords.1 .1)?;
                    self.output.push_str(&format!(
                        "{}window_set({}, {}, {}, {});\n",
                        indent, x1, y1, x2, y2
                    ));
                }
            }

            Statement::WindowReset => {
                if self.use_graphics {
                    self.output
                        .push_str(&format!("{}window_reset_qb();\n", indent));
                }
            }

            Statement::GetImage { coords, variable } => {
                if self.use_graphics {
                    if let Some(array_name) = self.expr_to_array_name(variable) {
                        let arr_idx = self.get_arr_var_idx(&array_name);
                        let x1 = self.generate_expression(&coords.0 .0)?;
                        let y1 = self.generate_expression(&coords.0 .1)?;
                        let x2 = self.generate_expression(&coords.1 .0)?;
                        let y2 = self.generate_expression(&coords.1 .1)?;
                        self.output.push_str(&format!(
                            "{}get_image_to_array({}, {}, {}, {}, {}, &mut arr_vars);\n",
                            indent, x1, y1, x2, y2, arr_idx
                        ));
                    }
                }
            }

            Statement::PutImage {
                coords,
                variable,
                action,
            } => {
                if self.use_graphics {
                    if let Some(array_name) = self.expr_to_array_name(variable) {
                        let arr_idx = self.get_arr_var_idx(&array_name);
                        let x = self.generate_expression(&coords.0)?;
                        let y = self.generate_expression(&coords.1)?;
                        let action = if let Some(expr) = action {
                            self.generate_expression(expr)?
                        } else {
                            "\"PSET\".to_string()".to_string()
                        };
                        self.output.push_str(&format!(
                            "{}put_image_from_array({}, {}, {}, &{}, &arr_vars);\n",
                            indent, x, y, arr_idx, action
                        ));
                        self.output
                            .push_str(&format!("{}update_screen();\n", indent));
                    }
                }
            }

            Statement::Read { variables } => {
                for var in variables {
                    let var_name = &var.name;
                    if var.type_suffix == Some('$') || var_name.ends_with('$') {
                        // String read not fully supported yet in this simple DATA model
                        self.output.push_str(&format!("{}data_idx += 1;\n", indent));
                    } else {
                        let var_idx = self.get_num_var_idx(var_name);
                        self.output
                            .push_str(&format!("{}if data_idx < DATA_VALUES.len() {{\n", indent));
                        self.output.push_str(&format!(
                            "{}    set_var(&mut num_vars, {}, DATA_VALUES[data_idx]);\n",
                            indent, var_idx
                        ));
                        self.output
                            .push_str(&format!("{}    data_idx += 1;\n", indent));
                        self.output.push_str(&format!("{}}}\n", indent));
                    }
                }
            }

            Statement::Restore { .. } => {
                self.output.push_str(&format!("{}data_idx = 0;\n", indent));
            }

            Statement::Cls => {
                self.output.push_str(&format!("{}cls();\n", indent));
            }

            Statement::Beep => {
                self.output
                    .push_str(&format!("{}print!(\"\\x07\");\n", indent));
            }

            Statement::Color {
                foreground,
                background: _,
            } => {
                if let Some(fg) = foreground {
                    let val = self.generate_expression(fg)?;
                    if self.use_graphics {
                        self.output
                            .push_str(&format!("{}qb_color({});\n", indent, val));
                    }
                }
                // Background color not fully supported in graphics mode yet (requires full redraw or next CLS)
            }

            Statement::Sleep { duration } => {
                let seconds = if let Some(d) = duration {
                    self.generate_expression(d)?
                } else {
                    "0.0".to_string()
                };
                self.output
                    .push_str(&format!("{}qb_sleep({});\n", indent, seconds));
            }

            Statement::End | Statement::Stop | Statement::System => {
                self.output.push_str(&format!("{}return;\n", indent));
            }

            Statement::Label { .. } => {}

            Statement::DefFn { name, params, body } => {
                let fn_name = self.rust_symbol("qbfn", name);
                let param_list = params
                    .iter()
                    .map(|p| format!("{}: f64", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                let body_code = self.generate_expression(body)?;

                self.output.push_str(&format!(
                    "{}fn {}(num_vars: &[f64], str_vars: &[String], {}) -> f64 {{\n",
                    indent, fn_name, param_list
                ));
                self.output
                    .push_str(&format!("{}    {}\n", indent, body_code));
                self.output.push_str(&format!("{}}}\n", indent));
            }

            Statement::Input {
                prompt, variables, ..
            } => {
                // In graphics mode, temporarily hide window for console input
                if self.use_graphics {
                    self.output.push_str(&format!("{}unsafe {{\n", indent));
                    self.output.push_str(&format!(
                        "{}    // Temporarily release window for console input\n",
                        indent
                    ));
                    self.output.push_str(&format!(
                        "{}    if let Some(mut window) = WINDOW.take() {{\n",
                        indent
                    ));
                    self.output
                        .push_str(&format!("{}        drop(window);\n", indent));
                    self.output.push_str(&format!("{}    }}\n", indent));
                    self.output.push_str(&format!("{}}}\n", indent));
                }

                // Print prompt if exists
                if let Some(p) = prompt {
                    let prompt_code = self.generate_expression(p)?;
                    self.output
                        .push_str(&format!("{}print!(\"{{}}\", {});\n", indent, prompt_code));
                }

                // Read input for each variable
                for var in variables {
                    if let Expression::Variable(v) = var {
                        let var_name = &v.name;
                        if v.type_suffix == Some('$') || var_name.ends_with('$') {
                            // String input
                            let idx = self.get_str_var_idx(var_name);
                            self.output
                                .push_str(&format!("{}io::stdout().flush().unwrap();\n", indent));
                            self.output
                                .push_str(&format!("{}let mut _input = String::new();\n", indent));
                            self.output.push_str(&format!(
                                "{}io::stdin().read_line(&mut _input).unwrap();\n",
                                indent
                            ));
                            self.output.push_str(&format!(
                                "{}set_str(&mut str_vars, {}, _input.trim());\n",
                                indent, idx
                            ));
                        } else {
                            // Numeric input
                            let idx = self.get_num_var_idx(var_name);
                            self.output
                                .push_str(&format!("{}io::stdout().flush().unwrap();\n", indent));
                            self.output
                                .push_str(&format!("{}let mut _input = String::new();\n", indent));
                            self.output.push_str(&format!(
                                "{}io::stdin().read_line(&mut _input).unwrap();\n",
                                indent
                            ));
                            self.output.push_str(&format!(
                                "{}let _val = _input.trim().parse::<f64>().unwrap_or(0.0);\n",
                                indent
                            ));
                            self.output.push_str(&format!(
                                "{}set_var(&mut num_vars, {}, _val);\n",
                                indent, idx
                            ));
                        }
                    }
                }

                // In graphics mode, recreate window after input
                if self.use_graphics {
                    self.output.push_str(&format!("{}unsafe {{\n", indent));
                    self.output.push_str(&format!(
                        "{}    // Recreate window after console input\n",
                        indent
                    ));
                    self.output
                        .push_str(&format!("{}    if WINDOW.is_none() {{\n", indent));
                    self.output.push_str(&format!(
                        "{}        let mut window = Window::new(\n",
                        indent
                    ));
                    self.output
                        .push_str(&format!("{}            \"QBNex Graphics\",\n", indent));
                    self.output
                        .push_str(&format!("{}            WINDOW_WIDTH,\n", indent));
                    self.output
                        .push_str(&format!("{}            WINDOW_HEIGHT,\n", indent));
                    self.output
                        .push_str(&format!("{}            WindowOptions {{\n", indent));
                    self.output
                        .push_str(&format!("{}                resize: true,\n", indent));
                    self.output
                        .push_str(&format!("{}                scale: Scale::X2,\n", indent));
                    self.output.push_str(&format!(
                        "{}                ..WindowOptions::default()\n",
                        indent
                    ));
                    self.output
                        .push_str(&format!("{}            }},\n", indent));
                    self.output
                        .push_str(&format!("{}        ).unwrap();\n", indent));
                    self.output
                        .push_str(&format!("{}        window.set_target_fps(60);\n", indent));
                    self.output
                        .push_str(&format!("{}        WINDOW = Some(window);\n", indent));
                    self.output
                        .push_str(&format!("{}        update_screen();\n", indent));
                    self.output.push_str(&format!("{}    }}\n", indent));
                    self.output.push_str(&format!("{}}}\n", indent));
                }
            }

            _ => {}
        }
        Ok(())
    }
}
