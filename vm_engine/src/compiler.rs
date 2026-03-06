use crate::builtin_functions::is_builtin_function;
use crate::opcodes::OpCode;
use core_types::{QResult, QType};
use std::collections::HashMap;
use syntax_tree::ast_nodes::{BinaryOp, Expression, GotoTarget, Program, Statement, UnaryOp};

pub struct BytecodeCompiler {
    program: Program,
    bytecode: Vec<OpCode>,
    labels: HashMap<String, usize>,
    line_numbers: HashMap<String, usize>,
    pending_jumps: Vec<(usize, String)>, // (bytecode_index, label_name)
    pending_gosubs: Vec<(usize, String)>, // (bytecode_index, label_name)
    pending_on_errors: Vec<(usize, String)>, // (bytecode_index, label_name) for ON ERROR GOTO
    variable_map: HashMap<String, usize>,
    next_var_index: usize,
}

impl BytecodeCompiler {
    pub fn new(program: Program) -> Self {
        Self {
            program,
            bytecode: Vec::with_capacity(1024),
            labels: HashMap::with_capacity(64),
            line_numbers: HashMap::with_capacity(128),
            pending_jumps: Vec::with_capacity(32),
            pending_gosubs: Vec::with_capacity(32),
            pending_on_errors: Vec::with_capacity(16),
            variable_map: HashMap::with_capacity(64),
            next_var_index: 0,
        }
    }

    fn get_var_index(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.variable_map.get(name) {
            idx
        } else {
            let idx = self.next_var_index;
            self.variable_map.insert(name.to_string(), idx);
            self.next_var_index += 1;
            idx
        }
    }

    pub fn compile(&mut self) -> QResult<Vec<OpCode>> {
        // Reserve space for InitGlobals instruction
        self.bytecode.push(OpCode::NoOp);

        // First, collect all DATA statements from the program
        let mut all_data_stmts = Vec::new();
        self.collect_data_from_stmts(&self.program.statements, &mut all_data_stmts);
        for sub_def in self.program.subs.values() {
            self.collect_data_from_stmts(&sub_def.body, &mut all_data_stmts);
        }
        for func_def in self.program.functions.values() {
            self.collect_data_from_stmts(&func_def.body, &mut all_data_stmts);
        }

        // Emit Data opcodes at the beginning
        for data_values in all_data_stmts {
            self.bytecode.push(OpCode::Data(data_values));
        }

        let statements = std::mem::take(&mut self.program.statements);

        for stmt in statements {
            self.compile_statement(&stmt)?;
        }

        // Patch pending jumps
        self.patch_jumps();

        // Update the InitGlobals instruction
        if let Some(op) = self.bytecode.first_mut() {
            *op = OpCode::InitGlobals(self.next_var_index);
        }

        Ok(std::mem::take(&mut self.bytecode))
    }

    fn collect_data_from_stmts(&self, stmts: &[Statement], data: &mut Vec<Vec<QType>>) {
        for stmt in stmts {
            match stmt {
                Statement::Data { values } => {
                    data.push(values.iter().map(|v| QType::String(v.clone())).collect());
                }
                Statement::IfBlock {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.collect_data_from_stmts(then_branch, data);
                    if let Some(else_br) = else_branch {
                        self.collect_data_from_stmts(else_br, data);
                    }
                }
                Statement::ForLoop { body, .. } => self.collect_data_from_stmts(body, data),
                Statement::WhileLoop { body, .. } => self.collect_data_from_stmts(body, data),
                Statement::DoLoop { body, .. } => self.collect_data_from_stmts(body, data),
                Statement::Select { cases, .. } => {
                    for (_, body) in cases {
                        self.collect_data_from_stmts(body, data);
                    }
                }
                _ => {}
            }
        }
    }

    fn patch_jumps(&mut self) {
        for (idx, label) in &self.pending_jumps {
            if let Some(&addr) = self.labels.get(label) {
                if let Some(OpCode::Jump(_)) = self.bytecode.get_mut(*idx) {
                    self.bytecode[*idx] = OpCode::Jump(addr);
                }
            }
        }

        for (idx, label) in &self.pending_gosubs {
            if let Some(&addr) = self.labels.get(label) {
                if let Some(OpCode::Gosub(_)) = self.bytecode.get_mut(*idx) {
                    self.bytecode[*idx] = OpCode::Gosub(addr);
                }
            }
        }

        for (idx, label) in &self.pending_on_errors {
            if let Some(&addr) = self.labels.get(label) {
                if let Some(OpCode::OnError(_)) = self.bytecode.get_mut(*idx) {
                    self.bytecode[*idx] = OpCode::OnError(addr);
                }
            }
        }
    }

    fn compile_statement(&mut self, stmt: &Statement) -> QResult<()> {
        match stmt {
            Statement::Print {
                expressions,
                newline,
            } => {
                for expr in expressions {
                    self.compile_expression(expr)?;
                    self.bytecode.push(OpCode::Print);
                }
                if *newline {
                    self.bytecode.push(OpCode::PrintNewline);
                }
            }

            Statement::PrintFile {
                file_number,
                expressions,
                newline,
            } => {
                // For each expression, we need to print it to the file
                // The file number is dynamic, so we need a different approach
                // We'll compile file_number once and use it for all prints

                for expr in expressions {
                    self.compile_expression(file_number)?; // Push file number
                    self.compile_expression(expr)?; // Push value
                    self.bytecode.push(OpCode::PrintFileDynamic);
                }

                if *newline {
                    self.compile_expression(file_number)?;
                    self.bytecode
                        .push(OpCode::LoadConstant(core_types::QType::String(
                            "\n".to_string(),
                        )));
                    self.bytecode.push(OpCode::PrintFileDynamic);
                }
            }

            Statement::Assignment { target, value } => {
                // Check if this is a MID$ assignment (MID$(var$, start, len) = value)
                // MID$ can be parsed as either FunctionCall or ArrayAccess
                let is_mid_assignment = match target {
                    Expression::FunctionCall(func) => {
                        func.name.to_uppercase() == "MID$" || func.name.to_uppercase() == "MID"
                    }
                    Expression::ArrayAccess { name, .. } => {
                        name.to_uppercase() == "MID$" || name.to_uppercase() == "MID"
                    }
                    _ => false,
                };

                if is_mid_assignment {
                    // Extract arguments from either FunctionCall or ArrayAccess
                    let (args, _type_suffix) = match target {
                        Expression::FunctionCall(func) => (func.args.clone(), func.type_suffix),
                        Expression::ArrayAccess {
                            indices,
                            type_suffix,
                            ..
                        } => (indices.clone(), *type_suffix),
                        _ => unreachable!(),
                    };

                    if args.len() >= 2 {
                        // Extract variable name from first argument
                        if let Expression::Variable(var) = &args[0] {
                            let var_name = var.name.clone();
                            let var_idx = self.get_var_index(&var_name);

                            // Load the original variable value using LoadFast
                            self.bytecode.push(OpCode::LoadFast(var_idx));

                            // Compile start position
                            self.compile_expression(&args[1])?;

                            // Compile length (if provided, otherwise use length of value)
                            if args.len() >= 3 {
                                self.compile_expression(&args[2])?;
                            } else {
                                // Use length of replacement string
                                self.compile_expression(value)?;
                                self.bytecode.push(OpCode::Len);
                            }

                            // Compile replacement value
                            self.compile_expression(value)?;

                            // Emit MidAssign opcode (will pop: original, start, length, value)
                            self.bytecode.push(OpCode::MidAssign {
                                var_name: var_name.clone(),
                                start: 0,
                                length: Some(0),
                            });

                            // Store the modified string back using StoreFast
                            self.bytecode.push(OpCode::StoreFast(var_idx));
                            return Ok(());
                        }
                    }
                }

                // Regular assignment
                // For array assignments, push indices first, then value
                if let Expression::ArrayAccess { indices, .. } = target {
                    for idx in indices {
                        self.compile_expression(idx)?;
                    }
                    self.compile_expression(value)?;
                    self.compile_store_target(target)?;
                } else {
                    self.compile_expression(value)?;
                    self.compile_store_target(target)?;
                }
            }

            Statement::IfBlock {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expression(condition)?;
                let else_idx = self.bytecode.len();
                self.bytecode.push(OpCode::JumpIfFalse(0));

                for s in then_branch {
                    self.compile_statement(s)?;
                }

                if else_branch.is_some() || else_branch.is_none() {
                    let end_idx = self.bytecode.len();
                    self.bytecode.push(OpCode::Jump(0));

                    let else_start = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(addr)) = self.bytecode.get_mut(else_idx) {
                        *addr = else_start;
                    }

                    if let Some(else_br) = else_branch {
                        for s in else_br {
                            self.compile_statement(s)?;
                        }
                    }

                    let end = self.bytecode.len();
                    if let Some(OpCode::Jump(addr)) = self.bytecode.get_mut(end_idx) {
                        *addr = end;
                    }
                } else {
                    let end = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(addr)) = self.bytecode.get_mut(else_idx) {
                        *addr = end;
                    }
                }
            }

            Statement::ForLoop {
                variable,
                start,
                end,
                step,
                body,
            } => {
                // Compile step
                let step_value: f64 = match step {
                    Some(expr) => {
                        match expr {
                            Expression::Literal(QType::Integer(i)) => *i as f64,
                            Expression::Literal(QType::Long(l)) => *l as f64,
                            Expression::Literal(QType::Double(d)) => *d,
                            Expression::Literal(QType::Single(s)) => *s as f64,
                            Expression::UnaryOp { op, operand } => {
                                // Handle negative numbers like -1, -2
                                if let Expression::Literal(lit) = &**operand {
                                    let val = match lit {
                                        QType::Integer(i) => *i as f64,
                                        QType::Long(l) => *l as f64,
                                        QType::Double(d) => *d,
                                        QType::Single(s) => *s as f64,
                                        _ => 1.0,
                                    };
                                    match op {
                                        syntax_tree::ast_nodes::UnaryOp::Negate => -val,
                                        _ => val,
                                    }
                                } else {
                                    1.0
                                }
                            }
                            _ => 1.0,
                        }
                    }
                    None => 1.0,
                };

                // Compile end expression and push it
                self.compile_expression(end)?;

                // Push start value and store in variable!
                self.compile_expression(start)?;
                let var_idx = self.get_var_index(&variable.name);
                self.bytecode.push(OpCode::StoreFast(var_idx));

                let for_init_idx = self.bytecode.len();
                self.bytecode.push(OpCode::ForInitFast {
                    var_index: var_idx,
                    end_label: 0,
                    step: step_value,
                });

                // Loop body starts here (ForInit will record this position)
                for s in body {
                    self.compile_statement(s)?;
                }

                self.bytecode.push(OpCode::ForStepFast {
                    var_index: var_idx,
                    step: step_value,
                });

                self.bytecode.push(OpCode::NextFast(var_idx));

                let end_addr = self.bytecode.len();

                // Patch ForInit end_label
                if let Some(OpCode::ForInitFast { end_label, .. }) =
                    self.bytecode.get_mut(for_init_idx)
                {
                    *end_label = end_addr;
                }
            }

            Statement::WhileLoop { condition, body } => {
                let loop_start = self.bytecode.len();

                self.compile_expression(condition)?;
                let jz_idx = self.bytecode.len();
                self.bytecode.push(OpCode::JumpIfFalse(0)); // placeholder

                for s in body {
                    self.compile_statement(s)?;
                }

                self.bytecode.push(OpCode::Jump(loop_start));
                let end_idx = self.bytecode.len();

                if let Some(OpCode::JumpIfFalse(target)) = self.bytecode.get_mut(jz_idx) {
                    *target = end_idx;
                }
            }

            Statement::DoLoop {
                condition,
                body,
                pre_condition,
            } => {
                let loop_start = self.bytecode.len();
                let mut jz_idx = None;

                if *pre_condition {
                    if let Some(cond) = condition {
                        self.compile_expression(cond)?;
                        jz_idx = Some(self.bytecode.len());
                        self.bytecode.push(OpCode::JumpIfTrue(0)); // For DO UNTIL: exit if true
                    }
                }

                for s in body {
                    self.compile_statement(s)?;
                }

                if !*pre_condition {
                    if let Some(cond) = condition {
                        self.compile_expression(cond)?;
                        self.bytecode.push(OpCode::JumpIfFalse(loop_start)); // For DO UNTIL: loop if false
                    } else {
                        self.bytecode.push(OpCode::Jump(loop_start));
                    }
                } else {
                    self.bytecode.push(OpCode::Jump(loop_start));
                }

                let end_idx = self.bytecode.len();

                if let Some(idx) = jz_idx {
                    if let Some(OpCode::JumpIfTrue(target)) = self.bytecode.get_mut(idx) {
                        *target = end_idx;
                    }
                }
            }

            Statement::Goto { target } => {
                let addr = match target {
                    GotoTarget::LineNumber(n) => {
                        self.line_numbers.get(&n.to_string()).copied().unwrap_or(0)
                    }
                    GotoTarget::Label(name) => {
                        if let Some(&addr) = self.labels.get(name) {
                            addr
                        } else {
                            // Label not yet defined, add to pending
                            let idx = self.bytecode.len();
                            self.pending_jumps.push((idx, name.clone()));
                            0 // placeholder
                        }
                    }
                };
                self.bytecode.push(OpCode::Jump(addr));
            }

            Statement::Gosub { target } => {
                let addr = match target {
                    GotoTarget::LineNumber(n) => {
                        self.line_numbers.get(&n.to_string()).copied().unwrap_or(0)
                    }
                    GotoTarget::Label(name) => {
                        if let Some(&addr) = self.labels.get(name) {
                            addr
                        } else {
                            // Label not yet defined, add to pending
                            let idx = self.bytecode.len();
                            self.pending_gosubs.push((idx, name.clone()));
                            0 // placeholder
                        }
                    }
                };
                self.bytecode.push(OpCode::Gosub(addr));
            }

            Statement::Return => {
                self.bytecode.push(OpCode::Return);
            }

            Statement::Label { name } => {
                self.labels.insert(name.clone(), self.bytecode.len());
            }

            Statement::LineNumber { number } => {
                self.line_numbers
                    .insert(number.to_string(), self.bytecode.len());
            }

            Statement::Screen { mode } => {
                if let Some(expr) = mode {
                    if let Expression::Literal(QType::Integer(i)) = expr {
                        self.bytecode.push(OpCode::Screen(*i as i32));
                    }
                } else {
                    self.bytecode.push(OpCode::Screen(0));
                }
            }

            Statement::Pset { coords, color } => {
                self.compile_expression(&coords.0)?;
                self.compile_expression(&coords.1)?;
                let c = color
                    .as_ref()
                    .map(|e| {
                        if let Expression::Literal(QType::Integer(i)) = e {
                            *i as i32
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                self.bytecode.push(OpCode::Pset {
                    x: 0,
                    y: 0,
                    color: c,
                });
            }

            Statement::Preset { coords, color } => {
                self.compile_expression(&coords.0)?;
                self.compile_expression(&coords.1)?;
                let c = color
                    .as_ref()
                    .map(|e| {
                        if let Expression::Literal(QType::Integer(i)) = e {
                            *i as i32
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                self.bytecode.push(OpCode::Preset {
                    x: 0,
                    y: 0,
                    color: c,
                });
            }

            Statement::Line { coords, color, .. } => {
                self.compile_expression(&coords.0 .0)?;
                self.compile_expression(&coords.0 .1)?;
                self.compile_expression(&coords.1 .0)?;
                self.compile_expression(&coords.1 .1)?;
                let c = color
                    .as_ref()
                    .map(|e| {
                        if let Expression::Literal(QType::Integer(i)) = e {
                            *i as i32
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                self.bytecode.push(OpCode::Line {
                    x1: 0,
                    y1: 0,
                    x2: 0,
                    y2: 0,
                    color: c,
                });
            }

            Statement::Circle {
                center,
                radius,
                color,
                ..
            } => {
                self.compile_expression(&center.0)?;
                self.compile_expression(&center.1)?;
                self.compile_expression(radius)?;
                let c = color
                    .as_ref()
                    .map(|e| {
                        if let Expression::Literal(QType::Integer(i)) = e {
                            *i as i32
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                self.bytecode.push(OpCode::Circle {
                    x: 0,
                    y: 0,
                    radius: 0,
                    color: c,
                });
            }

            Statement::Sound {
                frequency,
                duration,
            } => {
                self.compile_expression(frequency)?;
                self.compile_expression(duration)?;
                self.bytecode.push(OpCode::Sound {
                    frequency: 0,
                    duration: 0,
                });
            }

            Statement::Play {
                melody: Expression::Literal(QType::String(s)),
            } => {
                self.bytecode.push(OpCode::Play(s.clone()));
            }
            Statement::Play { .. } => {}

            Statement::Beep => {
                self.bytecode.push(OpCode::Beep);
            }

            Statement::Cls => {
                self.bytecode.push(OpCode::PrintNewline);
            }

            Statement::End => {
                self.bytecode.push(OpCode::End);
            }

            Statement::Stop => {
                self.bytecode.push(OpCode::Stop);
            }

            Statement::Chain {
                filename,
                delete: _,
            } => {
                // Compile filename expression
                self.compile_expression(filename)?;
                // Push CHAIN opcode
                self.bytecode.push(OpCode::Chain);
            }

            Statement::Shell { command } => {
                if let Some(cmd) = command {
                    self.compile_expression(cmd)?;
                } else {
                    // Empty SHELL command opens a shell
                    self.bytecode
                        .push(OpCode::LoadConstant(QType::String(String::new())));
                }
                self.bytecode.push(OpCode::Shell);
            }

            Statement::Randomize { seed } => {
                if let Some(s) = seed {
                    self.compile_expression(s)?;
                } else {
                    self.bytecode.push(OpCode::LoadConstant(QType::Integer(0)));
                }
            }

            Statement::FunctionCall(func) => {
                for arg in &func.args {
                    self.compile_expression(arg)?;
                }
                self.bytecode.push(OpCode::CallNative(func.name.clone()));
                self.bytecode.push(OpCode::Pop);
            }

            Statement::Dim { variables, .. } => {
                for (var, size) in variables {
                    if let Some(dim_expr) = size {
                        // Array declaration - extract size from expression
                        let upper_bound = if let Expression::Literal(QType::Integer(i)) = dim_expr {
                            (*i as i32).clamp(0, 10000) // Limit to 10k
                        } else if let Expression::Literal(QType::Long(l)) = dim_expr {
                            (*l).clamp(0, 10000) // Limit to 10k
                        } else {
                            10 // Default
                        };
                        let dimensions = vec![(0, upper_bound)];
                        self.bytecode.push(OpCode::ArrayDim {
                            name: var.name.clone(),
                            dimensions,
                        });
                    } else {
                        // Simple variable
                        self.bytecode.push(OpCode::LoadConstant(QType::Integer(0)));
                        self.bytecode.push(OpCode::StoreVariable(var.name.clone()));
                    }
                }
            }

            Statement::Redim {
                variables,
                preserve,
            } => {
                for (var, _size) in variables {
                    let dimensions = vec![(0, 10)]; // Default 0 to 10
                    self.bytecode.push(OpCode::ArrayRedim {
                        name: var.name.clone(),
                        dimensions,
                        preserve: *preserve,
                    });
                }
            }

            Statement::Select { expression, cases } => {
                // Compile the expression to test
                self.compile_expression(expression)?;

                // Use a unique variable name based on current instruction pointer
                let select_var = format!("$SELECT_{}", self.bytecode.len());
                let select_var_idx = self.get_var_index(&select_var);
                self.bytecode.push(OpCode::StoreFast(select_var_idx));

                let mut end_jumps = Vec::new();

                for (case_expr, case_body) in cases {
                    match case_expr {
                        Expression::CaseRange { start, end } => {
                            // Check (val >= start) AND (val <= end)
                            self.bytecode.push(OpCode::LoadFast(select_var_idx));
                            self.compile_expression(start)?;
                            self.bytecode.push(OpCode::GreaterOrEqual);

                            self.bytecode.push(OpCode::LoadFast(select_var_idx));
                            self.compile_expression(end)?;
                            self.bytecode.push(OpCode::LessOrEqual);

                            self.bytecode.push(OpCode::And);
                        }
                        Expression::CaseIs { op, value } => {
                            // Check val OP value
                            self.bytecode.push(OpCode::LoadFast(select_var_idx));
                            self.compile_expression(value)?;
                            self.compile_binary_op(op)?;
                        }
                        Expression::CaseElse => {
                            // Always true (-1)
                            self.bytecode.push(OpCode::LoadConstant(QType::Integer(-1)));
                        }
                        _ => {
                            // Simple equality
                            self.bytecode.push(OpCode::LoadFast(select_var_idx));
                            self.compile_expression(case_expr)?;
                            self.bytecode.push(OpCode::Equal);
                        }
                    }

                    // Jump to next case if false
                    let jump_idx = self.bytecode.len();
                    self.bytecode.push(OpCode::JumpIfFalse(0));

                    // Compile case body
                    for stmt in case_body {
                        self.compile_statement(stmt)?;
                    }

                    // Jump to end of SELECT
                    let end_jump_idx = self.bytecode.len();
                    self.bytecode.push(OpCode::Jump(0));
                    end_jumps.push(end_jump_idx);

                    // Patch jump to next case
                    let next_case = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(addr)) = self.bytecode.get_mut(jump_idx) {
                        *addr = next_case;
                    }
                }

                self.bytecode.push(OpCode::EndSelect);

                // Patch all end jumps
                let end_addr = self.bytecode.len();
                for jump_idx in end_jumps {
                    if let Some(OpCode::Jump(addr)) = self.bytecode.get_mut(jump_idx) {
                        *addr = end_addr;
                    }
                }
            }

            Statement::PrintUsing {
                format,
                expressions,
                newline,
            } => {
                self.compile_expression(format)?;
                for expr in expressions {
                    self.compile_expression(expr)?;
                }
                self.bytecode.push(OpCode::PrintUsing(expressions.len()));
                if *newline {
                    self.bytecode.push(OpCode::PrintNewline);
                }
            }

            Statement::OnError { label } => {
                let addr = match label {
                    Some(name) => {
                        if let Some(&addr) = self.labels.get(name) {
                            addr
                        } else {
                            // Label not yet defined, add to pending list
                            let idx = self.bytecode.len();
                            self.pending_on_errors.push((idx, name.clone()));
                            0 // Placeholder
                        }
                    }
                    None => 0, // 0 = Disable error handler
                };
                self.bytecode.push(OpCode::OnError(addr));
            }

            Statement::OnErrorResumeNext => {
                self.bytecode.push(OpCode::OnErrorResumeNext);
            }

            Statement::Error { code } => {
                self.compile_expression(code)?;
                self.bytecode.push(OpCode::ErrorStmt);
            }

            Statement::Resume => {
                self.bytecode.push(OpCode::Resume);
            }

            Statement::ResumeNext => {
                self.bytecode.push(OpCode::ResumeNext);
            }

            Statement::ResumeLabel { label } => {
                let addr = self.labels.get(label).copied().unwrap_or(0);
                self.bytecode.push(OpCode::ResumeLabel(addr));
            }

            Statement::Paint {
                coords,
                paint_color,
                border_color,
            } => {
                self.compile_expression(&coords.0)?;
                self.compile_expression(&coords.1)?;
                let pc = paint_color
                    .as_ref()
                    .map(|e| self.expr_to_i32(e))
                    .unwrap_or(-1);
                let bc = border_color
                    .as_ref()
                    .map(|e| self.expr_to_i32(e))
                    .unwrap_or(-1);
                self.bytecode.push(OpCode::Paint {
                    x: 0,
                    y: 0,
                    paint_color: pc,
                    border_color: bc,
                });
            }

            Statement::Draw {
                commands: Expression::Literal(QType::String(s)),
            } => {
                self.bytecode.push(OpCode::Draw {
                    commands: s.clone(),
                });
            }

            Statement::Palette { attribute, color } => {
                let attr = self.expr_to_i32(attribute);
                let c = color.as_ref().map(|e| self.expr_to_i32(e)).unwrap_or(-1);
                self.bytecode.push(OpCode::Palette {
                    attribute: attr,
                    color: c,
                });
            }

            Statement::View {
                coords,
                fill_color,
                border_color,
            } => {
                self.compile_expression(&coords.0 .0)?;
                self.compile_expression(&coords.0 .1)?;
                self.compile_expression(&coords.1 .0)?;
                self.compile_expression(&coords.1 .1)?;
                let fc = fill_color
                    .as_ref()
                    .map(|e| self.expr_to_i32(e))
                    .unwrap_or(-1);
                let bc = border_color
                    .as_ref()
                    .map(|e| self.expr_to_i32(e))
                    .unwrap_or(-1);
                self.bytecode.push(OpCode::View {
                    x1: 0,
                    y1: 0,
                    x2: 0,
                    y2: 0,
                    fill_color: fc,
                    border_color: bc,
                });
            }

            Statement::ViewPrint { top, bottom } => {
                let t = top.as_ref().map(|e| self.expr_to_i32(e)).unwrap_or(1);
                let b = bottom.as_ref().map(|e| self.expr_to_i32(e)).unwrap_or(25);
                self.bytecode.push(OpCode::ViewPrint { top: t, bottom: b });
            }

            Statement::ViewReset => {
                self.bytecode.push(OpCode::ViewReset);
            }

            Statement::Window { coords } => {
                self.compile_expression(&coords.0 .0)?;
                self.compile_expression(&coords.0 .1)?;
                self.compile_expression(&coords.1 .0)?;
                self.compile_expression(&coords.1 .1)?;
                self.bytecode.push(OpCode::Window {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 0.0,
                });
            }

            Statement::WindowReset => {
                self.bytecode.push(OpCode::WindowReset);
            }

            Statement::Data { .. } => {
                // DATA statements are handled in pre-pass
            }

            Statement::Read { variables } => {
                for var in variables {
                    let var_idx = self.get_var_index(&var.name);
                    self.bytecode.push(OpCode::ReadFast(var_idx));
                }
            }

            Statement::Restore { label } => {
                self.bytecode.push(OpCode::Restore(label.clone()));
            }

            Statement::Swap { var1, var2 } => {
                let name1 = self.expr_to_var_name(var1);
                let name2 = self.expr_to_var_name(var2);
                if let (Some(n1), Some(n2)) = (name1, name2) {
                    let idx1 = self.get_var_index(&n1);
                    let idx2 = self.get_var_index(&n2);
                    self.bytecode.push(OpCode::SwapFast(idx1, idx2));
                }
            }

            Statement::Sleep { duration } => {
                if let Some(dur) = duration {
                    self.compile_expression(dur)?;
                    self.bytecode.push(OpCode::Sleep);
                } else {
                    // Sleep indefinitely (wait for keypress)
                    self.bytecode.push(OpCode::LoadConstant(QType::Integer(-1)));
                    self.bytecode.push(OpCode::Sleep);
                }
            }

            Statement::Open {
                filename,
                mode,
                file_number,
                ..
            } => {
                // Push filename and file number onto stack
                self.compile_expression(filename)?;
                self.compile_expression(file_number)?;

                // Convert mode to string
                let mode_str = match mode {
                    syntax_tree::ast_nodes::OpenMode::Input => "INPUT",
                    syntax_tree::ast_nodes::OpenMode::Output => "OUTPUT",
                    syntax_tree::ast_nodes::OpenMode::Append => "APPEND",
                    syntax_tree::ast_nodes::OpenMode::Binary => "BINARY",
                    syntax_tree::ast_nodes::OpenMode::Random => "RANDOM",
                };

                self.bytecode.push(OpCode::Open {
                    mode: mode_str.to_string(),
                });
            }

            Statement::Close { file_numbers } => {
                for file_num in file_numbers {
                    self.compile_expression(file_num)?;
                    self.bytecode.push(OpCode::Close);
                }
            }

            Statement::LineInputFile {
                file_number,
                variable,
            } => {
                // Push file number onto stack
                self.compile_expression(file_number)?;
                // LineInputDynamic will pop file_number, read line, and we need to store it
                self.bytecode.push(OpCode::LineInputDynamic);
                // Store the result in the variable
                self.compile_store_target(variable)?;
            }

            Statement::Input {
                prompt, variables, ..
            } => {
                if let Some(p) = prompt {
                    self.compile_expression(p)?;
                    self.bytecode.push(OpCode::Print);
                }
                for var in variables {
                    self.bytecode.push(OpCode::Input);
                    self.compile_store_target(var)?;
                }
            }

            Statement::LineInput { prompt, variable } => {
                if let Some(p) = prompt {
                    self.compile_expression(p)?;
                    self.bytecode.push(OpCode::Print);
                }
                self.bytecode.push(OpCode::LineInput(
                    self.expr_to_var_name(variable).unwrap_or_default(),
                ));
            }

            Statement::Write { expressions } => {
                for expr in expressions {
                    self.compile_expression(expr)?;
                    // Write needs commas between values
                    // For prototype, just print
                    self.bytecode.push(OpCode::Print);
                }
                self.bytecode.push(OpCode::PrintNewline);
            }

            Statement::Get {
                file_number,
                record,
                variable,
            } => {
                self.compile_expression(file_number)?;
                if let Some(rec) = record {
                    self.compile_expression(rec)?;
                } else {
                    self.bytecode.push(OpCode::LoadConstant(QType::Integer(0)));
                }
                self.bytecode.push(OpCode::Get);
                if let Some(var) = variable {
                    self.compile_store_target(var)?;
                } else {
                    self.bytecode.push(OpCode::Pop);
                }
            }

            Statement::Put {
                file_number,
                record,
                variable,
            } => {
                self.compile_expression(file_number)?;
                if let Some(rec) = record {
                    self.compile_expression(rec)?;
                } else {
                    self.bytecode.push(OpCode::LoadConstant(QType::Integer(0)));
                }
                if let Some(var) = variable {
                    self.compile_expression(var)?;
                } else {
                    self.bytecode.push(OpCode::LoadConstant(QType::Empty));
                }
                self.bytecode.push(OpCode::Put);
            }

            Statement::GetImage { coords, variable } => {
                let array = self.expr_to_var_name(variable).unwrap_or_default();
                self.bytecode.push(OpCode::GetImage {
                    x1: self.expr_to_i32(&coords.0 .0),
                    y1: self.expr_to_i32(&coords.0 .1),
                    x2: self.expr_to_i32(&coords.1 .0),
                    y2: self.expr_to_i32(&coords.1 .1),
                    array,
                });
            }

            Statement::PutImage {
                coords,
                variable,
                action,
            } => {
                let array = self.expr_to_var_name(variable).unwrap_or_default();
                let action = action
                    .as_ref()
                    .and_then(|expr| self.expr_to_string(expr))
                    .unwrap_or_else(|| "PSET".to_string());
                self.bytecode.push(OpCode::PutImage {
                    x: self.expr_to_i32(&coords.0),
                    y: self.expr_to_i32(&coords.1),
                    array,
                    action,
                });
            }

            Statement::Seek {
                file_number,
                position,
            } => {
                self.compile_expression(file_number)?;
                self.compile_expression(position)?;
                self.bytecode.push(OpCode::SeekDynamic);
            }

            Statement::Call { name, args } => {
                for arg in args {
                    self.compile_expression(arg)?;
                }
                self.bytecode.push(OpCode::CallSub(name.clone()));
            }

            Statement::Declare { .. } => {}
            Statement::OptionBase { .. } => {}
            Statement::Erase { .. } => {}
            Statement::Field { .. } => {} // TODO: Implement Field
            Statement::LSet { .. } => {}  // TODO: Implement LSet
            Statement::RSet { .. } => {}  // TODO: Implement RSet

            Statement::Const { name, value } => {
                self.compile_expression(value)?;
                self.compile_store_target(&Expression::Variable(
                    syntax_tree::ast_nodes::Variable::new(name.clone()),
                ))?;
            }

            Statement::DefType { .. } => {
                // DefType is a compile-time directive, no runtime code needed
                // The type information should be handled by the parser/analyzer
            }

            Statement::DefFn { name, params, body } => {
                // Save current variable mapping
                let saved_var_map = self.variable_map.clone();
                let saved_var_counter = self.next_var_index;

                // Create new variable mapping for DEF FN parameters
                self.variable_map.clear();
                self.next_var_index = 0;
                for param in params {
                    self.variable_map.insert(param.clone(), self.next_var_index);
                    self.next_var_index += 1;
                }

                let start_idx = self.bytecode.len();
                self.compile_expression(body)?;
                let body_ops = self.bytecode.drain(start_idx..).collect();

                // Restore variable mapping
                self.variable_map = saved_var_map;
                self.next_var_index = saved_var_counter;

                self.bytecode.push(OpCode::DefFn {
                    name: name.clone(),
                    params: params.clone(),
                    body: body_ops,
                });
            }

            _ => {}
        }

        Ok(())
    }

    fn expr_to_i32(&self, expr: &Expression) -> i32 {
        match expr {
            Expression::Literal(QType::Integer(i)) => *i as i32,
            Expression::Literal(QType::Long(l)) => *l,
            Expression::Literal(QType::Single(s)) => *s as i32,
            Expression::Literal(QType::Double(d)) => *d as i32,
            _ => 0,
        }
    }

    fn expr_to_var_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(var) => Some(var.name.clone()),
            _ => None,
        }
    }

    fn expr_to_string(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Literal(QType::String(s)) => Some(s.clone()),
            Expression::Variable(var) => Some(var.name.clone()),
            _ => None,
        }
    }

    fn compile_expression(&mut self, expr: &Expression) -> QResult<()> {
        match expr {
            Expression::Literal(qtype) => {
                self.bytecode.push(OpCode::LoadConstant(qtype.clone()));
            }

            Expression::Variable(var) => {
                // Check if this is actually a zero-argument builtin function
                if is_builtin_function(&var.name) {
                    // Treat as function call with no arguments
                    let func = syntax_tree::ast_nodes::FunctionCall {
                        name: var.name.clone(),
                        args: Vec::new(),
                        type_suffix: var.type_suffix,
                    };
                    self.compile_expression(&Expression::FunctionCall(func))?;
                } else {
                    let var_idx = self.get_var_index(&var.name);
                    self.bytecode.push(OpCode::LoadFast(var_idx));
                }
            }

            Expression::ArrayAccess { name, indices, .. } => {
                if is_builtin_function(name) {
                    // Treat as Function Call if it's a built-in function
                    let func = syntax_tree::ast_nodes::FunctionCall {
                        name: name.clone(),
                        args: indices.clone(),
                        type_suffix: None,
                    };
                    self.compile_expression(&Expression::FunctionCall(func))?;
                } else {
                    // Push indices onto stack
                    for idx in indices {
                        self.compile_expression(idx)?;
                    }
                    // Load array element
                    self.bytecode
                        .push(OpCode::ArrayLoad(name.clone(), indices.len()));
                }
            }

            Expression::BinaryOp { op, left, right } => {
                self.compile_expression(left.as_ref())?;
                self.compile_expression(right.as_ref())?;
                self.compile_binary_op(op)?;
            }

            Expression::UnaryOp { op, operand } => {
                self.compile_expression(operand.as_ref())?;
                match op {
                    UnaryOp::Negate => self.bytecode.push(OpCode::Negate),
                    UnaryOp::Not => self.bytecode.push(OpCode::Not),
                }
            }

            Expression::FunctionCall(func) => {
                // Check if it's a built-in function
                if is_builtin_function(&func.name) {
                    // Compile arguments
                    for arg in &func.args {
                        self.compile_expression(arg)?;
                    }

                    // Generate appropriate opcode for built-in function
                    match func.name.to_uppercase().as_str() {
                        "LEFT$" => self.bytecode.push(OpCode::Left),
                        "RIGHT$" => self.bytecode.push(OpCode::Right),
                        "MID$" => self.bytecode.push(OpCode::Mid),
                        "LEN" => self.bytecode.push(OpCode::Len),
                        "INSTR" => self.bytecode.push(OpCode::InStr),
                        "LCASE$" => self.bytecode.push(OpCode::LCase),
                        "UCASE$" => self.bytecode.push(OpCode::UCase),
                        "LTRIM$" => self.bytecode.push(OpCode::LTrim),
                        "RTRIM$" => self.bytecode.push(OpCode::RTrim),
                        "TRIM$" => self.bytecode.push(OpCode::Trim),
                        "STR$" => self.bytecode.push(OpCode::StrFunc),
                        "VAL" => self.bytecode.push(OpCode::ValFunc),
                        "CHR$" => self.bytecode.push(OpCode::ChrFunc),
                        "ASC" => self.bytecode.push(OpCode::AscFunc),
                        "SPACE$" => self.bytecode.push(OpCode::SpaceFunc),
                        "STRING$" => self.bytecode.push(OpCode::StringFunc),
                        "HEX$" => self.bytecode.push(OpCode::HexFunc),
                        "OCT$" => self.bytecode.push(OpCode::OctFunc),
                        "ABS" => self.bytecode.push(OpCode::Abs),
                        "SGN" => self.bytecode.push(OpCode::Sgn),
                        "SIN" => self.bytecode.push(OpCode::Sin),
                        "COS" => self.bytecode.push(OpCode::Cos),
                        "TAN" => self.bytecode.push(OpCode::Tan),
                        "ATN" => self.bytecode.push(OpCode::Atn),
                        "EXP" => self.bytecode.push(OpCode::ExpFunc),
                        "LOG" => self.bytecode.push(OpCode::LogFunc),
                        "SQR" => self.bytecode.push(OpCode::Sqr),
                        "INT" => self.bytecode.push(OpCode::IntFunc),
                        "FIX" => self.bytecode.push(OpCode::Fix),
                        "RND" => self.bytecode.push(OpCode::Rnd),
                        "CINT" => self.bytecode.push(OpCode::CInt),
                        "CLNG" => self.bytecode.push(OpCode::CLng),
                        "CSNG" => self.bytecode.push(OpCode::CSng),
                        "CDBL" => self.bytecode.push(OpCode::CDbl),
                        "CSTR" => self.bytecode.push(OpCode::CStr),
                        "TIMER" => self.bytecode.push(OpCode::Timer),
                        "DATE$" => self.bytecode.push(OpCode::Date),
                        "TIME$" => self.bytecode.push(OpCode::Time),
                        // Binary conversion functions
                        "MKI$" => self.bytecode.push(OpCode::MkiFunc),
                        "MKL$" => self.bytecode.push(OpCode::MklFunc),
                        "MKS$" => self.bytecode.push(OpCode::MksFunc),
                        "MKD$" => self.bytecode.push(OpCode::MkdFunc),
                        "CVI" => self.bytecode.push(OpCode::CviFunc),
                        "CVL" => self.bytecode.push(OpCode::CvlFunc),
                        "CVS" => self.bytecode.push(OpCode::CvsFunc),
                        "CVD" => self.bytecode.push(OpCode::CvdFunc),
                        // System functions
                        "FRE" => {
                            // Argument already compiled, add opcode
                            self.bytecode.push(OpCode::FreFunc(0));
                        }
                        "CSRLIN" => self.bytecode.push(OpCode::CsrLinFunc),
                        "POS" => self.bytecode.push(OpCode::PosFunc(0)),
                        "ENVIRON$" => self.bytecode.push(OpCode::EnvironFunc),
                        "COMMAND$" => self.bytecode.push(OpCode::CommandFunc),
                        "INKEY$" => self.bytecode.push(OpCode::InKeyFunc),
                        "FREEFILE" => self.bytecode.push(OpCode::FreeFile),
                        // Error functions
                        "ERR" => self.bytecode.push(OpCode::Err),
                        "ERL" => self.bytecode.push(OpCode::Erl),
                        "ERDEV" => self.bytecode.push(OpCode::ErDev),
                        "ERDEV$" => self.bytecode.push(OpCode::ErDevStr),
                        // Array functions
                        "LBOUND" => {
                            // Get array name from first argument
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                let dim = if func.args.len() > 1 { 1 } else { 0 };
                                self.bytecode.push(OpCode::LBound(var.name.clone(), dim));
                            }
                        }
                        "UBOUND" => {
                            // Get array name from first argument
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                let dim = if func.args.len() > 1 { 1 } else { 0 };
                                self.bytecode.push(OpCode::UBound(var.name.clone(), dim));
                            }
                        }
                        _ => {}
                    }
                } else {
                    // User-defined function or unknown
                    for arg in &func.args {
                        self.compile_expression(arg)?;
                    }

                    if self.program.functions.contains_key(&func.name) {
                        self.bytecode.push(OpCode::CallFunction(func.name.clone()));
                    } else if func.name.to_uppercase().starts_with("FN") {
                        self.bytecode.push(OpCode::CallDefFn(func.name.clone()));
                    } else {
                        self.bytecode.push(OpCode::CallNative(func.name.clone()));
                    }
                }
            }

            Expression::FieldAccess { object, .. } => {
                // For now, compile the object expression
                self.compile_expression(object.as_ref())?;
            }
            Expression::TypeCast { expression, .. } => {
                // Compile the inner expression, ignore type cast for now
                self.compile_expression(expression.as_ref())?;
            }
            Expression::CaseRange { .. } | Expression::CaseIs { .. } | Expression::CaseElse => {
                return Err(core_types::QError::Syntax(
                    "CASE expression used outside SELECT CASE".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn compile_binary_op(&mut self, op: &BinaryOp) -> QResult<()> {
        let opcode = match op {
            BinaryOp::Add => OpCode::Add,
            BinaryOp::Subtract => OpCode::Subtract,
            BinaryOp::Multiply => OpCode::Multiply,
            BinaryOp::Divide => OpCode::Divide,
            BinaryOp::IntegerDivide => OpCode::IntegerDivide,
            BinaryOp::Modulo => OpCode::Modulo,
            BinaryOp::Power => OpCode::Power,
            BinaryOp::Equal => OpCode::Equal,
            BinaryOp::NotEqual => OpCode::NotEqual,
            BinaryOp::LessThan => OpCode::LessThan,
            BinaryOp::GreaterThan => OpCode::GreaterThan,
            BinaryOp::LessOrEqual => OpCode::LessOrEqual,
            BinaryOp::GreaterOrEqual => OpCode::GreaterOrEqual,
            BinaryOp::And => OpCode::And,
            BinaryOp::Or => OpCode::Or,
            BinaryOp::Xor => OpCode::Xor,
            BinaryOp::Eqv => OpCode::Eqv,
            BinaryOp::Imp => OpCode::Imp,
        };

        self.bytecode.push(opcode);
        Ok(())
    }

    fn compile_store_target(&mut self, target: &Expression) -> QResult<()> {
        match target {
            Expression::Variable(var) => {
                let var_idx = self.get_var_index(&var.name);
                self.bytecode.push(OpCode::StoreFast(var_idx));
            }
            Expression::ArrayAccess { name, indices, .. } => {
                // For array store, value is already on stack
                // We need to push indices after value
                // But value was pushed before this function was called
                // So we need to save value, push indices, then restore value

                // Actually, let's change the order: push indices first
                // This requires changing how Assignment works
                self.bytecode
                    .push(OpCode::ArrayStore(name.clone(), indices.len()));
            }
            Expression::FieldAccess { object, field: _ } => {
                self.compile_expression(object.as_ref())?;
            }
            _ => {}
        }

        Ok(())
    }
}
