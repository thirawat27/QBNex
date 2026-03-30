use super::CodeGenerator;
use core_types::{QError, QResult, QType};
use syntax_tree::ast_nodes::*;
use syntax_tree::{validate_program, Backend};

#[derive(Clone)]
struct TopLevelBlock {
    statements: Vec<Statement>,
    line_number: Option<u16>,
}

impl CodeGenerator {
    fn normalize_data_label(name: &str) -> String {
        name.to_ascii_uppercase()
    }

    pub(super) fn normalize_restore_target(name: &str) -> String {
        if name.parse::<u16>().is_ok() {
            name.to_string()
        } else {
            Self::normalize_data_label(name)
        }
    }

    fn collect_data_layout_from_stmts(
        stmts: &[Statement],
        data_values: &mut Vec<String>,
        pending_restore_targets: &mut Vec<String>,
        restore_targets: &mut std::collections::HashMap<String, usize>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::Label { name } => {
                    pending_restore_targets.push(Self::normalize_data_label(name));
                }
                Statement::LineNumber { number } => {
                    pending_restore_targets.push(number.to_string());
                }
                Statement::Data { values } => {
                    let value_index = data_values.len();
                    for target in pending_restore_targets.drain(..) {
                        restore_targets.entry(target).or_insert(value_index);
                    }
                    data_values.extend(values.iter().cloned());
                }
                Statement::IfBlock {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    Self::collect_data_layout_from_stmts(
                        then_branch,
                        data_values,
                        pending_restore_targets,
                        restore_targets,
                    );
                    if let Some(else_branch) = else_branch {
                        Self::collect_data_layout_from_stmts(
                            else_branch,
                            data_values,
                            pending_restore_targets,
                            restore_targets,
                        );
                    }
                }
                Statement::IfElseBlock {
                    then_branch,
                    else_ifs,
                    else_branch,
                    ..
                } => {
                    Self::collect_data_layout_from_stmts(
                        then_branch,
                        data_values,
                        pending_restore_targets,
                        restore_targets,
                    );
                    for (_, branch) in else_ifs {
                        Self::collect_data_layout_from_stmts(
                            branch,
                            data_values,
                            pending_restore_targets,
                            restore_targets,
                        );
                    }
                    if let Some(branch) = else_branch {
                        Self::collect_data_layout_from_stmts(
                            branch,
                            data_values,
                            pending_restore_targets,
                            restore_targets,
                        );
                    }
                }
                Statement::ForLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::DoLoop { body, .. }
                | Statement::ForEach { body, .. } => Self::collect_data_layout_from_stmts(
                    body,
                    data_values,
                    pending_restore_targets,
                    restore_targets,
                ),
                Statement::Select { cases, .. } => {
                    for (_, body) in cases {
                        Self::collect_data_layout_from_stmts(
                            body,
                            data_values,
                            pending_restore_targets,
                            restore_targets,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_program_data_layout(&mut self, program: &Program) -> Vec<String> {
        self.restore_targets.clear();

        let mut data_values = Vec::new();
        let mut pending_restore_targets = Vec::new();

        Self::collect_data_layout_from_stmts(
            &program.statements,
            &mut data_values,
            &mut pending_restore_targets,
            &mut self.restore_targets,
        );

        let mut sub_names = program.subs.keys().cloned().collect::<Vec<_>>();
        sub_names.sort();
        for name in sub_names {
            if let Some(sub_def) = program.subs.get(&name) {
                Self::collect_data_layout_from_stmts(
                    &sub_def.body,
                    &mut data_values,
                    &mut pending_restore_targets,
                    &mut self.restore_targets,
                );
            }
        }

        let mut function_names = program.functions.keys().cloned().collect::<Vec<_>>();
        function_names.sort();
        for name in function_names {
            if let Some(func_def) = program.functions.get(&name) {
                Self::collect_data_layout_from_stmts(
                    &func_def.body,
                    &mut data_values,
                    &mut pending_restore_targets,
                    &mut self.restore_targets,
                );
            }
        }

        let end_of_data = data_values.len();
        for target in pending_restore_targets.drain(..) {
            self.restore_targets.entry(target).or_insert(end_of_data);
        }

        data_values
    }

    pub fn generate(&mut self, program: &Program) -> QResult<String> {
        validate_program(program, Backend::Native)?;

        self.output.clear();
        self.user_types = program
            .user_types
            .iter()
            .map(|(name, user_type)| (name.to_uppercase(), user_type.clone()))
            .collect();
        self.functions = program
            .functions
            .keys()
            .map(|name| name.to_uppercase())
            .collect();
        self.function_return_types.clear();
        self.shared_num_vars.clear();
        self.shared_str_vars.clear();
        self.shared_arr_vars.clear();
        self.shared_str_arr_vars.clear();
        self.shared_udt_vars.clear();
        self.shared_udt_array_vars.clear();
        self.sub_param_modes.clear();
        self.function_param_modes.clear();
        for stmt in &program.statements {
            if let Statement::Declare {
                name,
                is_function: true,
                return_type,
                ..
            } = stmt
            {
                let inferred = return_type.clone().unwrap_or_else(|| {
                    if name.ends_with('$') {
                        QType::String(String::new())
                    } else {
                        QType::Double(0.0)
                    }
                });
                self.function_return_types
                    .insert(name.to_uppercase(), inferred);
            }
            if let Statement::Declare {
                name,
                is_function,
                params,
                ..
            } = stmt
            {
                if *is_function {
                    self.register_function_param_modes(name, params);
                } else {
                    self.register_sub_param_modes(name, params);
                }
            }
        }
        for sub_def in program.subs.values() {
            self.register_sub_param_modes(&sub_def.name, &sub_def.params);
        }
        for func_def in program.functions.values() {
            self.register_function_param_modes(&func_def.name, &func_def.params);
            self.function_return_types
                .insert(func_def.name.to_uppercase(), func_def.return_type.clone());
        }
        self.udt_vars.clear();
        self.write_prelude();

        // Collect variables for main program
        self.collect_vars(&program.statements);
        self.register_shared_globals_from_statements(&program.statements);

        let data_values = self.collect_program_data_layout(program);

        if !data_values.is_empty() {
            self.output.push_str("static DATA_VALUES: &[&str] = &[\n");
            for val in &data_values {
                self.output.push_str(&format!("    {:?},\n", val));
            }
            self.output.push_str("];\n\n");
        } else if !program.data_statements.is_empty() {
            // Fallback for pre-populated data_statements (if any)
            self.generate_data_array(&program.data_statements);
        }

        // Generate main function
        self.output.push_str("fn main() {\n");
        self.output.push_str("    qb_install_panic_hook();\n");
        // Pre-allocate variables
        self.output.push_str(&format!(
            "    let mut num_vars = vec![0.0; {}];\n",
            self.num_vars.len().max(1)
        ));
        self.output.push_str(&format!(
            "    let mut str_vars = vec![String::new(); {}];\n",
            self.str_vars.len().max(1)
        ));
        self.output.push_str(&format!(
            "    let mut arr_vars: Vec<Vec<f64>> = vec![Vec::new(); {}];\n",
            self.arr_vars.len().max(1)
        ));
        self.output.push_str(&format!(
            "    let mut arr_bounds: Vec<Vec<(i32, i32)>> = vec![Vec::new(); {}];\n",
            self.arr_vars.len().max(1)
        ));
        self.output.push_str(&format!(
            "    let mut str_arr_vars: Vec<Vec<String>> = vec![Vec::new(); {}];\n",
            self.str_arr_vars.len().max(1)
        ));
        self.output.push_str(&format!(
            "    let mut str_arr_bounds: Vec<Vec<(i32, i32)>> = vec![Vec::new(); {}];\n",
            self.str_arr_vars.len().max(1)
        ));

        if self.has_data_ops(&program.statements) {
            self.output.push_str("    let mut data_idx: usize = 0;\n\n");
        } else {
            self.output.push('\n');
        }

        if self.has_top_level_control_flow(&program.statements) {
            self.generate_top_level_control_flow(&program.statements)?;
        } else {
            for stmt in &program.statements {
                self.generate_statement(stmt)?;
            }
        }

        self.write_epilogue();

        // Generate Subs
        for sub_def in program.subs.values() {
            self.generate_sub(sub_def)?;
        }

        // Generate Functions
        for func_def in program.functions.values() {
            self.generate_function(func_def)?;
        }

        Ok(self.output.clone())
    }

    pub(super) fn collect_vars(&mut self, stmts: &[Statement]) {
        for stmt in stmts {
            match stmt {
                Statement::Assignment { target, .. } => self.collect_var_from_expr(target),
                Statement::Input { variables, .. } => {
                    for var in variables {
                        self.collect_var_from_expr(var);
                    }
                }
                Statement::InputFile { variables, .. } => {
                    for var in variables {
                        self.collect_var_from_expr(var);
                    }
                }
                Statement::Field { fields, .. } => {
                    for (_, var) in fields {
                        self.collect_var_from_expr(var);
                    }
                }
                Statement::LSet { target, .. } | Statement::RSet { target, .. } => {
                    self.collect_var_from_expr(target);
                }
                Statement::Get {
                    variable: Some(variable),
                    ..
                } => {
                    self.collect_var_from_expr(variable);
                }
                Statement::Get { .. } => {}
                Statement::LineInput { variable, .. }
                | Statement::LineInputFile { variable, .. } => {
                    self.collect_var_from_expr(variable);
                }
                Statement::Read { variables } => {
                    for var in variables {
                        // Read takes Variable struct, not Expression
                        if self.variable_is_string(var) {
                            self.get_str_var_idx(&var.name);
                        } else {
                            self.get_num_var_idx(&var.name);
                        }
                    }
                }
                Statement::ForLoop { variable, body, .. } => {
                    self.get_num_var_idx(&variable.name);
                    self.collect_vars(body);
                }
                Statement::ForEach {
                    variable,
                    array,
                    body,
                } => {
                    self.get_num_var_idx(&variable.name);
                    self.collect_var_from_expr(array);
                    self.collect_vars(body);
                }
                Statement::IfBlock {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.collect_vars(then_branch);
                    if let Some(else_br) = else_branch {
                        self.collect_vars(else_br);
                    }
                }
                Statement::WhileLoop { body, .. } => self.collect_vars(body),
                Statement::DoLoop { body, .. } => self.collect_vars(body),
                Statement::Select { cases, .. } => {
                    for (_, body) in cases {
                        self.collect_vars(body);
                    }
                }
                Statement::Dim { variables, .. } => {
                    for (var, dimensions) in variables {
                        if let Some(declared_type) = &var.declared_type {
                            match Self::declared_type_to_qtype(declared_type) {
                                QType::UserDefined(_) => {
                                    if dimensions.is_some() || !var.indices.is_empty() {
                                        self.ensure_udt_array_storage(&var.name, declared_type);
                                    } else {
                                        self.ensure_udt_storage(&var.name, declared_type);
                                    }
                                }
                                QType::String(_) => {
                                    if let Some(width) = var.fixed_length {
                                        self.field_widths.insert(var.name.to_uppercase(), width);
                                    }
                                    if dimensions.is_some() || !var.indices.is_empty() {
                                        self.get_str_arr_var_idx(&var.name);
                                    } else {
                                        self.get_str_var_idx(&var.name);
                                    }
                                }
                                qtype if Self::qtype_is_numeric(&qtype) => {
                                    if dimensions.is_some() || !var.indices.is_empty() {
                                        self.get_arr_var_idx(&var.name);
                                    } else {
                                        self.get_num_var_idx(&var.name);
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }
                        if var.indices.is_empty() && !self.variable_is_string(var) {
                            // Scalar variable declared with DIM
                            self.get_num_var_idx(&var.name);
                        } else if var.indices.is_empty() && self.variable_is_string(var) {
                            // String variable
                            self.get_str_var_idx(&var.name);
                        } else if self.array_is_string(&var.name, var.type_suffix) {
                            self.get_str_arr_var_idx(&var.name);
                        } else {
                            // Array
                            self.get_arr_var_idx(&var.name);
                        }
                    }
                }
                Statement::Redim { variables, .. } => {
                    for (var, dimensions) in variables {
                        if let Some(declared_type) = &var.declared_type {
                            match Self::declared_type_to_qtype(declared_type) {
                                QType::UserDefined(_) => {
                                    if dimensions.is_some() || !var.indices.is_empty() {
                                        self.ensure_udt_array_storage(&var.name, declared_type);
                                    } else {
                                        self.ensure_udt_storage(&var.name, declared_type);
                                    }
                                }
                                QType::String(_) => {
                                    if let Some(width) = var.fixed_length {
                                        self.field_widths.insert(var.name.to_uppercase(), width);
                                    }
                                    if dimensions.is_some() || !var.indices.is_empty() {
                                        self.get_str_arr_var_idx(&var.name);
                                    } else {
                                        self.get_str_var_idx(&var.name);
                                    }
                                }
                                qtype if Self::qtype_is_numeric(&qtype) => {
                                    if dimensions.is_some() || !var.indices.is_empty() {
                                        self.get_arr_var_idx(&var.name);
                                    } else {
                                        self.get_num_var_idx(&var.name);
                                    }
                                }
                                _ => {}
                            }
                            continue;
                        }
                        if self.array_is_string(&var.name, var.type_suffix) {
                            self.get_str_arr_var_idx(&var.name);
                        } else {
                            self.get_arr_var_idx(&var.name);
                        }
                    }
                }
                Statement::GetImage { variable, .. } | Statement::PutImage { variable, .. } => {
                    if let Some(name) = self.expr_to_array_name(variable) {
                        self.get_arr_var_idx(&name);
                    }
                }
                _ => {}
            }
        }
    }

    fn register_shared_globals_from_statements(&mut self, stmts: &[Statement]) {
        for stmt in stmts {
            let Statement::Dim {
                variables,
                is_shared,
                is_common,
                ..
            } = stmt
            else {
                continue;
            };

            if !(*is_shared || *is_common) {
                continue;
            }

            for (var, dimensions) in variables {
                let has_dimensions = dimensions.is_some() || !var.indices.is_empty();
                let name_upper = var.name.to_uppercase();

                if let Some(declared_type) = &var.declared_type {
                    match Self::declared_type_to_qtype(declared_type) {
                        QType::UserDefined(_) => {
                            if has_dimensions {
                                if let Some(type_name) = self
                                    .udt_array_vars
                                    .get(&Self::normalize_udt_name(&var.name))
                                {
                                    self.shared_udt_array_vars.insert(
                                        Self::normalize_udt_name(&var.name),
                                        type_name.clone(),
                                    );
                                }
                            } else if let Some(type_name) =
                                self.udt_vars.get(&Self::normalize_udt_name(&var.name))
                            {
                                self.shared_udt_vars
                                    .insert(Self::normalize_udt_name(&var.name), type_name.clone());
                            }
                        }
                        QType::String(_) => {
                            if has_dimensions {
                                if let Some(idx) = self.str_arr_vars.get(&name_upper) {
                                    self.shared_str_arr_vars.insert(name_upper.clone(), *idx);
                                }
                            } else if let Some(idx) = self.str_vars.get(&name_upper) {
                                self.shared_str_vars.insert(name_upper.clone(), *idx);
                            }
                        }
                        qtype if Self::qtype_is_numeric(&qtype) => {
                            if has_dimensions {
                                if let Some(idx) = self.arr_vars.get(&name_upper) {
                                    self.shared_arr_vars.insert(name_upper.clone(), *idx);
                                }
                            } else if let Some(idx) = self.num_vars.get(&name_upper) {
                                self.shared_num_vars.insert(name_upper.clone(), *idx);
                            }
                        }
                        _ => {}
                    }
                    continue;
                }

                if has_dimensions {
                    if self.array_is_string(&var.name, var.type_suffix) {
                        if let Some(idx) = self.str_arr_vars.get(&name_upper) {
                            self.shared_str_arr_vars.insert(name_upper.clone(), *idx);
                        }
                    } else if let Some(idx) = self.arr_vars.get(&name_upper) {
                        self.shared_arr_vars.insert(name_upper.clone(), *idx);
                    }
                } else if self.variable_is_string(var) {
                    if let Some(idx) = self.str_vars.get(&name_upper) {
                        self.shared_str_vars.insert(name_upper.clone(), *idx);
                    }
                } else if let Some(idx) = self.num_vars.get(&name_upper) {
                    self.shared_num_vars.insert(name_upper.clone(), *idx);
                }
            }
        }
    }

    pub(super) fn has_data_ops(&self, stmts: &[Statement]) -> bool {
        for stmt in stmts {
            match stmt {
                Statement::Read { .. } | Statement::Restore { .. } => return true,
                Statement::IfBlock {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    if self.has_data_ops(then_branch)
                        || else_branch
                            .as_ref()
                            .is_some_and(|branch| self.has_data_ops(branch))
                    {
                        return true;
                    }
                }
                Statement::IfElseBlock {
                    then_branch,
                    else_ifs,
                    else_branch,
                    ..
                } => {
                    if self.has_data_ops(then_branch)
                        || else_ifs.iter().any(|(_, branch)| self.has_data_ops(branch))
                        || else_branch
                            .as_ref()
                            .is_some_and(|branch| self.has_data_ops(branch))
                    {
                        return true;
                    }
                }
                Statement::ForLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::DoLoop { body, .. } => {
                    if self.has_data_ops(body) {
                        return true;
                    }
                }
                Statement::Select { cases, .. } => {
                    if cases.iter().any(|(_, branch)| self.has_data_ops(branch)) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn has_top_level_control_flow(&self, statements: &[Statement]) -> bool {
        for statement in statements {
            match statement {
                Statement::Goto { .. }
                | Statement::Gosub { .. }
                | Statement::Return
                | Statement::OnGotoGosub { .. }
                | Statement::OnError { .. }
                | Statement::OnErrorResumeNext
                | Statement::Resume
                | Statement::ResumeNext
                | Statement::ResumeLabel { .. }
                | Statement::Error { .. }
                | Statement::OnTimer { .. }
                | Statement::OnPlay { .. }
                | Statement::TimerOn
                | Statement::TimerOff
                | Statement::TimerStop
                | Statement::PlayOn
                | Statement::PlayOff
                | Statement::PlayStop => return true,
                Statement::IfBlock {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    if self.has_top_level_control_flow(then_branch)
                        || else_branch
                            .as_ref()
                            .is_some_and(|branch| self.has_top_level_control_flow(branch))
                    {
                        return true;
                    }
                }
                Statement::IfElseBlock {
                    then_branch,
                    else_ifs,
                    else_branch,
                    ..
                } => {
                    if self.has_top_level_control_flow(then_branch)
                        || else_ifs
                            .iter()
                            .any(|(_, branch)| self.has_top_level_control_flow(branch))
                        || else_branch
                            .as_ref()
                            .is_some_and(|branch| self.has_top_level_control_flow(branch))
                    {
                        return true;
                    }
                }
                Statement::ForLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::DoLoop { body, .. }
                | Statement::ForEach { body, .. } => {
                    if self.has_top_level_control_flow(body) {
                        return true;
                    }
                }
                Statement::Select { cases, .. } => {
                    if cases
                        .iter()
                        .any(|(_, branch)| self.has_top_level_control_flow(branch))
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn normalize_label(name: &str) -> String {
        name.to_ascii_uppercase()
    }

    fn build_top_level_blocks(
        &self,
        statements: &[Statement],
    ) -> (
        Vec<TopLevelBlock>,
        std::collections::HashMap<String, usize>,
        std::collections::HashMap<u16, usize>,
    ) {
        let mut blocks = Vec::new();
        let mut labels = std::collections::HashMap::new();
        let mut line_numbers = std::collections::HashMap::new();
        let mut current = Vec::new();
        let mut pending_labels = Vec::new();
        let mut pending_line_numbers = Vec::new();
        let mut pending_block_line = None;

        let assign_pending =
            |blocks_len: usize,
             pending_labels: &mut Vec<String>,
             pending_line_numbers: &mut Vec<u16>,
             pending_block_line: &mut Option<u16>,
             labels: &mut std::collections::HashMap<String, usize>,
             line_numbers: &mut std::collections::HashMap<u16, usize>| {
                for label in pending_labels.drain(..) {
                    labels.insert(Self::normalize_label(&label), blocks_len);
                }
                *pending_block_line = pending_line_numbers.last().copied();
                for line in pending_line_numbers.drain(..) {
                    line_numbers.insert(line, blocks_len);
                }
            };

        for statement in statements {
            match statement {
                Statement::Label { name } => {
                    if !current.is_empty() {
                        blocks.push(TopLevelBlock {
                            statements: std::mem::take(&mut current),
                            line_number: pending_block_line.take(),
                        });
                    }
                    pending_labels.push(name.clone());
                }
                Statement::LineNumber { number } => {
                    if !current.is_empty() {
                        blocks.push(TopLevelBlock {
                            statements: std::mem::take(&mut current),
                            line_number: pending_block_line.take(),
                        });
                    }
                    pending_line_numbers.push(*number);
                }
                _ => {
                    if current.is_empty() {
                        assign_pending(
                            blocks.len(),
                            &mut pending_labels,
                            &mut pending_line_numbers,
                            &mut pending_block_line,
                            &mut labels,
                            &mut line_numbers,
                        );
                    }
                    current.push(statement.clone());
                    blocks.push(TopLevelBlock {
                        statements: std::mem::take(&mut current),
                        line_number: pending_block_line.take(),
                    });
                }
            }
        }

        if !current.is_empty() {
            assign_pending(
                blocks.len(),
                &mut pending_labels,
                &mut pending_line_numbers,
                &mut pending_block_line,
                &mut labels,
                &mut line_numbers,
            );
            blocks.push(TopLevelBlock {
                statements: current,
                line_number: pending_block_line.take(),
            });
        } else if !pending_labels.is_empty() || !pending_line_numbers.is_empty() {
            assign_pending(
                blocks.len(),
                &mut pending_labels,
                &mut pending_line_numbers,
                &mut pending_block_line,
                &mut labels,
                &mut line_numbers,
            );
            blocks.push(TopLevelBlock {
                statements: Vec::new(),
                line_number: pending_block_line.take(),
            });
        }

        (blocks, labels, line_numbers)
    }

    fn resolve_top_level_target(
        &self,
        target: &GotoTarget,
        labels: &std::collections::HashMap<String, usize>,
        line_numbers: &std::collections::HashMap<u16, usize>,
    ) -> QResult<usize> {
        match target {
            GotoTarget::Label(name) => labels
                .get(&Self::normalize_label(name))
                .copied()
                .ok_or_else(|| QError::LabelNotFound(name.clone())),
            GotoTarget::LineNumber(number) => line_numbers
                .get(number)
                .copied()
                .ok_or_else(|| QError::LabelNotFound(number.to_string())),
        }
    }

    fn generate_top_level_control_flow(&mut self, statements: &[Statement]) -> QResult<()> {
        let (blocks, labels, line_numbers) = self.build_top_level_blocks(statements);
        let mut loop_blocks = Vec::new();
        for (index, block) in blocks.iter().enumerate() {
            self.collect_loop_resume_blocks(&block.statements, index, &mut loop_blocks);
        }
        self.loop_state_counter = 0;

        self.output.push_str("    let mut qb_pc: usize = 0;\n");
        self.output
            .push_str("    let mut qb_gosub_stack: Vec<(usize, usize)> = Vec::new();\n");
        if loop_blocks.is_empty() {
            self.output
                .push_str("    let qb_loop_block: Vec<usize> = Vec::new();\n");
            self.output
                .push_str("    let mut qb_loop_active: Vec<bool> = Vec::new();\n");
            self.output
                .push_str("    let mut qb_loop_pc: Vec<usize> = Vec::new();\n");
            self.output
                .push_str("    let mut qb_loop_end: Vec<f64> = Vec::new();\n");
            self.output
                .push_str("    let mut qb_loop_step: Vec<f64> = Vec::new();\n");
            self.output
                .push_str("    let mut qb_loop_index: Vec<usize> = Vec::new();\n");
        } else {
            self.output.push_str(&format!(
                "    let qb_loop_block: Vec<usize> = vec![{}];\n",
                loop_blocks
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            self.output.push_str(&format!(
                "    let mut qb_loop_active: Vec<bool> = vec![false; {}];\n",
                loop_blocks.len()
            ));
            self.output.push_str(&format!(
                "    let mut qb_loop_pc: Vec<usize> = vec![0; {}];\n",
                loop_blocks.len()
            ));
            self.output.push_str(&format!(
                "    let mut qb_loop_end: Vec<f64> = vec![0.0; {}];\n",
                loop_blocks.len()
            ));
            self.output.push_str(&format!(
                "    let mut qb_loop_step: Vec<f64> = vec![0.0; {}];\n",
                loop_blocks.len()
            ));
            self.output.push_str(&format!(
                "    let mut qb_loop_index: Vec<usize> = vec![0; {}];\n",
                loop_blocks.len()
            ));
        }
        self.output
            .push_str("    let mut qb_error_handler: Option<usize> = None;\n");
        self.output
            .push_str("    let mut qb_error_resume_next = false;\n");
        self.output
            .push_str("    let mut qb_error_resume_kind: usize = 0;\n");
        self.output
            .push_str("    let mut qb_error_resume_pc: usize = 0;\n");
        self.output
            .push_str("    let mut qb_error_resume_next_kind: usize = 0;\n");
        self.output
            .push_str("    let mut qb_error_resume_next_pc: usize = 0;\n");
        self.output
            .push_str("    let mut qb_in_error_handler = false;\n");
        self.output
            .push_str("    let mut qb_timer_handler: Option<usize> = None;\n");
        self.output
            .push_str("    let mut qb_timer_interval = std::time::Duration::from_secs_f64(0.0);\n");
        self.output
            .push_str("    let mut qb_timer_enabled = false;\n");
        self.output
            .push_str("    let mut qb_timer_active = false;\n");
        self.output
            .push_str("    let mut qb_timer_next_tick = std::time::Instant::now();\n");
        self.output
            .push_str("    let mut qb_play_handler: Option<usize> = None;\n");
        self.output
            .push_str("    let mut qb_play_queue_limit: usize = 1;\n");
        self.output
            .push_str("    let mut qb_play_trap_state: i32 = 0;\n");
        self.output
            .push_str("    let mut qb_play_pending_event = false;\n");
        self.output
            .push_str("    let mut qb_play_active = false;\n");
        self.output.push_str("    'qb_main: loop {\n");
        self.output.push_str(
            "        if qb_timer_enabled && !qb_timer_active && qb_timer_handler.is_some() && std::time::Instant::now() >= qb_timer_next_tick {\n",
        );
        self.output.push_str(
            "            qb_timer_next_tick = std::time::Instant::now() + qb_timer_interval;\n",
        );
        self.output
            .push_str("            qb_gosub_stack.push((0, qb_pc));\n");
        self.output
            .push_str("            qb_pc = qb_timer_handler.unwrap();\n");
        self.output
            .push_str("            qb_timer_active = true;\n");
        self.output.push_str("            continue 'qb_main;\n");
        self.output.push_str("        }\n");
        self.output.push_str(
            "        qb_update_play_queue(qb_play_queue_limit, qb_play_trap_state, &mut qb_play_pending_event);\n",
        );
        self.output.push_str(
            "        if qb_play_trap_state == 1 && !qb_play_active && qb_play_handler.is_some() && qb_play_pending_event {\n",
        );
        self.output
            .push_str("            qb_gosub_stack.push((0, qb_pc));\n");
        self.output
            .push_str("            qb_pc = qb_play_handler.unwrap();\n");
        self.output.push_str("            qb_play_active = true;\n");
        self.output
            .push_str("            qb_set_play_handler_active(true);\n");
        self.output
            .push_str("            qb_play_trap_state = 2;\n");
        self.output
            .push_str("            qb_play_pending_event = false;\n");
        self.output.push_str("            continue 'qb_main;\n");
        self.output.push_str("        }\n");
        self.output.push_str("        match qb_pc {\n");

        let saved_indent = self.indent_level;
        for (index, block) in blocks.iter().enumerate() {
            self.output
                .push_str(&format!("            {} => {{\n", index));
            self.indent_level = 2;

            if let Some(line_number) = block.line_number {
                self.output.push_str(&format!(
                    "{}qb_set_current_line({});\n",
                    self.indent(),
                    line_number
                ));
            }

            let mut terminated = false;
            for statement in &block.statements {
                if self.generate_top_level_flow_statement(
                    statement,
                    index,
                    index,
                    index + 1,
                    block.line_number,
                    &labels,
                    &line_numbers,
                    None,
                )? {
                    terminated = true;
                    break;
                }
            }

            if !terminated {
                if index + 1 < blocks.len() {
                    self.output
                        .push_str(&format!("{}qb_pc = {};\n", self.indent(), index + 1));
                    self.output
                        .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                } else {
                    self.output.push_str(&format!("{}break;\n", self.indent()));
                }
            }

            self.output.push_str("            }\n");
        }
        self.indent_level = saved_indent;
        self.output.push_str("            _ => break,\n");
        self.output.push_str("        }\n");
        self.output.push_str("    }\n");
        Ok(())
    }

    fn collect_loop_resume_blocks(
        &self,
        statements: &[Statement],
        block_index: usize,
        out: &mut Vec<usize>,
    ) {
        for statement in statements {
            match statement {
                Statement::IfBlock {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.collect_loop_resume_blocks(then_branch, block_index, out);
                    if let Some(branch) = else_branch {
                        self.collect_loop_resume_blocks(branch, block_index, out);
                    }
                }
                Statement::IfElseBlock {
                    then_branch,
                    else_ifs,
                    else_branch,
                    ..
                } => {
                    self.collect_loop_resume_blocks(then_branch, block_index, out);
                    for (_, branch) in else_ifs {
                        self.collect_loop_resume_blocks(branch, block_index, out);
                    }
                    if let Some(branch) = else_branch {
                        self.collect_loop_resume_blocks(branch, block_index, out);
                    }
                }
                Statement::ForLoop { body, .. }
                | Statement::ForEach { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::DoLoop { body, .. } => {
                    out.push(block_index);
                    self.collect_loop_resume_blocks(body, block_index, out);
                }
                Statement::Select { cases, .. } => {
                    for (_, branch) in cases {
                        self.collect_loop_resume_blocks(branch, block_index, out);
                    }
                }
                _ => {}
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_top_level_branch(
        &mut self,
        statements: &[Statement],
        block_index: usize,
        resume_index: usize,
        next_index: usize,
        line_number: Option<u16>,
        labels: &std::collections::HashMap<String, usize>,
        line_numbers: &std::collections::HashMap<u16, usize>,
        loop_context: Option<(usize, usize)>,
    ) -> QResult<bool> {
        for statement in statements {
            if self.generate_top_level_flow_statement(
                statement,
                block_index,
                resume_index,
                next_index,
                line_number,
                labels,
                line_numbers,
                loop_context,
            )? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_top_level_flow_statement(
        &mut self,
        statement: &Statement,
        block_index: usize,
        resume_index: usize,
        next_index: usize,
        line_number: Option<u16>,
        labels: &std::collections::HashMap<String, usize>,
        line_numbers: &std::collections::HashMap<u16, usize>,
        loop_context: Option<(usize, usize)>,
    ) -> QResult<bool> {
        match statement {
            Statement::Goto { target } => {
                let target_index = self.resolve_top_level_target(target, labels, line_numbers)?;
                self.output
                    .push_str(&format!("{}qb_pc = {};\n", self.indent(), target_index));
                self.output
                    .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                Ok(true)
            }
            Statement::Gosub { target } => {
                let target_index = self.resolve_top_level_target(target, labels, line_numbers)?;
                if let Some((loop_id, _)) = loop_context {
                    self.output.push_str(&format!(
                        "{}qb_gosub_stack.push(({}, {}));\n",
                        self.indent(),
                        loop_id + 1,
                        next_index
                    ));
                } else {
                    self.output.push_str(&format!(
                        "{}qb_gosub_stack.push((0, {}));\n",
                        self.indent(),
                        next_index
                    ));
                }
                self.output
                    .push_str(&format!("{}qb_pc = {};\n", self.indent(), target_index));
                self.output
                    .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                Ok(true)
            }
            Statement::Return => {
                self.output.push_str(&format!(
                    "{}if let Some((qb_return_kind, qb_return_pc)) = qb_gosub_stack.pop() {{\n",
                    self.indent()
                ));
                self.indent_level += 1;
                self.emit_control_flow_resume_dispatch("qb_return_kind", "qb_return_pc", true);
                self.indent_level -= 1;
                self.output
                    .push_str(&format!("{}}} else {{\n", self.indent()));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}let qb_error = QBRuntimeError {{ code: 17, message: \"RETURN without GOSUB\".to_string() }};\n",
                    self.indent()
                ));
                self.emit_top_level_runtime_error_dispatch(
                    "qb_error",
                    resume_index,
                    next_index,
                    line_number,
                    loop_context,
                );
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                Ok(true)
            }
            Statement::OnGotoGosub {
                expression,
                targets,
                is_gosub,
            } => {
                let selector = self.generate_expression(expression)?;
                let selector_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {} = (({}) as f64).round() as i32;\n",
                    self.indent(),
                    selector_tmp,
                    selector
                ));
                self.output
                    .push_str(&format!("{}match {} {{\n", self.indent(), selector_tmp));
                self.indent_level += 1;
                for (target_index, target) in targets.iter().enumerate() {
                    let resolved = self.resolve_top_level_target(target, labels, line_numbers)?;
                    self.output
                        .push_str(&format!("{}{} => {{\n", self.indent(), target_index + 1));
                    self.indent_level += 1;
                    if *is_gosub {
                        if let Some((loop_id, _)) = loop_context {
                            self.output.push_str(&format!(
                                "{}qb_gosub_stack.push(({}, {}));\n",
                                self.indent(),
                                loop_id + 1,
                                next_index
                            ));
                        } else {
                            self.output.push_str(&format!(
                                "{}qb_gosub_stack.push((0, {}));\n",
                                self.indent(),
                                next_index
                            ));
                        }
                    }
                    self.output
                        .push_str(&format!("{}qb_pc = {};\n", self.indent(), resolved));
                    self.output
                        .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                }
                self.output.push_str(&format!(
                    "{}_ => {{ qb_pc = {}; continue 'qb_main; }}\n",
                    self.indent(),
                    next_index
                ));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                Ok(true)
            }
            Statement::OnError { target } => {
                if let Some(target) = target {
                    let handler = self.resolve_top_level_target(target, labels, line_numbers)?;
                    self.output.push_str(&format!(
                        "{}qb_error_handler = Some({});\n",
                        self.indent(),
                        handler
                    ));
                } else {
                    self.output
                        .push_str(&format!("{}qb_error_handler = None;\n", self.indent()));
                }
                self.output
                    .push_str(&format!("{}qb_error_resume_next = false;\n", self.indent()));
                Ok(false)
            }
            Statement::OnErrorResumeNext => {
                self.output
                    .push_str(&format!("{}qb_error_resume_next = true;\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_error_handler = None;\n", self.indent()));
                Ok(false)
            }
            Statement::Resume => {
                self.output.push_str(&format!(
                    "{}if !qb_in_error_handler && qb_err() == 0.0 {{\n",
                    self.indent()
                ));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}qb_report_unhandled_error(&QBRuntimeError {{ code: 255, message: \"RESUME without error\".to_string() }});\n",
                    self.indent()
                ));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_in_error_handler = false;\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_clear_error_state();\n", self.indent()));
                self.emit_control_flow_resume_dispatch(
                    "qb_error_resume_kind",
                    "qb_error_resume_pc",
                    false,
                );
                Ok(true)
            }
            Statement::ResumeNext => {
                self.output.push_str(&format!(
                    "{}if !qb_in_error_handler && qb_err() == 0.0 {{\n",
                    self.indent()
                ));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}qb_report_unhandled_error(&QBRuntimeError {{ code: 255, message: \"RESUME NEXT without error\".to_string() }});\n",
                    self.indent()
                ));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_in_error_handler = false;\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_clear_error_state();\n", self.indent()));
                self.emit_control_flow_resume_dispatch(
                    "qb_error_resume_next_kind",
                    "qb_error_resume_next_pc",
                    false,
                );
                Ok(true)
            }
            Statement::ResumeLabel { label } => {
                let target = labels
                    .get(&Self::normalize_label(label))
                    .copied()
                    .ok_or_else(|| QError::LabelNotFound(label.clone()))?;
                self.output.push_str(&format!(
                    "{}if !qb_in_error_handler && qb_err() == 0.0 {{\n",
                    self.indent()
                ));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}qb_report_unhandled_error(&QBRuntimeError {{ code: 255, message: \"RESUME without error\".to_string() }});\n",
                    self.indent()
                ));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_pc = {};\n", self.indent(), target));
                self.output
                    .push_str(&format!("{}qb_in_error_handler = false;\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_clear_error_state();\n", self.indent()));
                self.output
                    .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                Ok(true)
            }
            Statement::OnTimer { interval, label } => {
                let interval = self.generate_expression(interval)?;
                let handler = labels
                    .get(&Self::normalize_label(label))
                    .copied()
                    .ok_or_else(|| QError::LabelNotFound(label.clone()))?;
                self.output.push_str(&format!(
                    "{}qb_timer_interval = std::time::Duration::from_secs_f64((({}) as f64).max(0.0));\n",
                    self.indent(),
                    interval
                ));
                self.output.push_str(&format!(
                    "{}qb_timer_handler = Some({});\n",
                    self.indent(),
                    handler
                ));
                self.output.push_str(&format!(
                    "{}qb_timer_next_tick = std::time::Instant::now() + qb_timer_interval;\n",
                    self.indent()
                ));
                Ok(false)
            }
            Statement::OnPlay { queue_limit, label } => {
                let queue_limit = self.generate_expression(queue_limit)?;
                let handler = labels
                    .get(&Self::normalize_label(label))
                    .copied()
                    .ok_or_else(|| QError::LabelNotFound(label.clone()))?;
                self.output.push_str(&format!(
                    "{}qb_play_queue_limit = (({}) as i32).clamp(1, 32) as usize;\n",
                    self.indent(),
                    queue_limit
                ));
                self.output.push_str(&format!(
                    "{}qb_play_handler = Some({});\n",
                    self.indent(),
                    handler
                ));
                self.output.push_str(&format!(
                    "{}qb_play_pending_event = false;\n",
                    self.indent()
                ));
                Ok(false)
            }
            Statement::TimerOn => {
                self.output
                    .push_str(&format!("{}qb_timer_enabled = true;\n", self.indent()));
                self.output.push_str(&format!(
                    "{}qb_timer_next_tick = std::time::Instant::now() + qb_timer_interval;\n",
                    self.indent()
                ));
                Ok(false)
            }
            Statement::TimerOff | Statement::TimerStop => {
                self.output
                    .push_str(&format!("{}qb_timer_enabled = false;\n", self.indent()));
                self.output
                    .push_str(&format!("{}qb_timer_active = false;\n", self.indent()));
                Ok(false)
            }
            Statement::PlayOn => {
                self.output
                    .push_str(&format!("{}qb_play_trap_state = 1;\n", self.indent()));
                Ok(false)
            }
            Statement::PlayOff => {
                self.output
                    .push_str(&format!("{}qb_play_trap_state = 0;\n", self.indent()));
                self.output.push_str(&format!(
                    "{}qb_play_pending_event = false;\n",
                    self.indent()
                ));
                self.output
                    .push_str(&format!("{}qb_play_active = false;\n", self.indent()));
                self.output.push_str(&format!(
                    "{}qb_set_play_handler_active(false);\n",
                    self.indent()
                ));
                Ok(false)
            }
            Statement::PlayStop => {
                self.output
                    .push_str(&format!("{}qb_play_trap_state = 2;\n", self.indent()));
                Ok(false)
            }
            Statement::Exit { exit_type } => {
                match exit_type {
                    ExitType::For | ExitType::Do | ExitType::While => {
                        if let Some((loop_id, loop_exit_block)) = loop_context {
                            self.output.push_str(&format!(
                                "{}qb_loop_active[{}] = false;\n",
                                self.indent(),
                                loop_id
                            ));
                            self.output.push_str(&format!(
                                "{}qb_loop_pc[{}] = 0;\n",
                                self.indent(),
                                loop_id
                            ));
                            self.output.push_str(&format!(
                                "{}qb_pc = {};\n",
                                self.indent(),
                                loop_exit_block
                            ));
                            self.output
                                .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                        } else {
                            self.output.push_str(&format!("{}break;\n", self.indent()));
                        }
                    }
                    ExitType::Sub | ExitType::Function => {
                        self.output
                            .push_str(&format!("{}break 'qb_proc;\n", self.indent()));
                    }
                }
                Ok(true)
            }
            Statement::End | Statement::Stop | Statement::System => {
                if self.is_in_sub {
                    self.output
                        .push_str(&format!("{}break 'qb_proc;\n", self.indent()));
                } else {
                    self.output.push_str(&format!("{}return;\n", self.indent()));
                }
                Ok(true)
            }
            Statement::Label { .. } | Statement::LineNumber { .. } => Ok(false),
            Statement::IfBlock {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_code = self.generate_expression(condition)?;
                self.output.push_str(&format!(
                    "{}if ({} as i32) != 0 {{\n",
                    self.indent(),
                    cond_code
                ));
                self.indent_level += 1;
                let _ = self.generate_top_level_branch(
                    then_branch,
                    block_index,
                    resume_index,
                    next_index,
                    line_number,
                    labels,
                    line_numbers,
                    loop_context,
                )?;
                self.indent_level -= 1;
                if let Some(else_branch) = else_branch {
                    self.output
                        .push_str(&format!("{}}} else {{\n", self.indent()));
                    self.indent_level += 1;
                    let _ = self.generate_top_level_branch(
                        else_branch,
                        block_index,
                        resume_index,
                        next_index,
                        line_number,
                        labels,
                        line_numbers,
                        loop_context,
                    )?;
                    self.indent_level -= 1;
                }
                self.output.push_str(&format!("{}}}\n", self.indent()));
                Ok(false)
            }
            Statement::IfElseBlock {
                condition,
                then_branch,
                else_ifs,
                else_branch,
            } => {
                let cond_code = self.generate_expression(condition)?;
                self.output.push_str(&format!(
                    "{}if ({} as i32) != 0 {{\n",
                    self.indent(),
                    cond_code
                ));
                self.indent_level += 1;
                let _ = self.generate_top_level_branch(
                    then_branch,
                    block_index,
                    resume_index,
                    next_index,
                    line_number,
                    labels,
                    line_numbers,
                    loop_context,
                )?;
                self.indent_level -= 1;

                for (else_if_condition, branch) in else_ifs {
                    let cond_code = self.generate_expression(else_if_condition)?;
                    self.output.push_str(&format!(
                        "{}}} else if ({} as i32) != 0 {{\n",
                        self.indent(),
                        cond_code
                    ));
                    self.indent_level += 1;
                    let _ = self.generate_top_level_branch(
                        branch,
                        block_index,
                        resume_index,
                        next_index,
                        line_number,
                        labels,
                        line_numbers,
                        loop_context,
                    )?;
                    self.indent_level -= 1;
                }

                if let Some(branch) = else_branch {
                    self.output
                        .push_str(&format!("{}}} else {{\n", self.indent()));
                    self.indent_level += 1;
                    let _ = self.generate_top_level_branch(
                        branch,
                        block_index,
                        resume_index,
                        next_index,
                        line_number,
                        labels,
                        line_numbers,
                        loop_context,
                    )?;
                    self.indent_level -= 1;
                }

                self.output.push_str(&format!("{}}}\n", self.indent()));
                Ok(false)
            }
            Statement::Select { expression, cases } => {
                let expr_code = self.generate_expression(expression)?;
                let temp = self.next_temp_var();
                let select_is_string = self.is_string_expression(expression);
                self.output
                    .push_str(&format!("{}let {} = {};\n", self.indent(), temp, expr_code));

                let mut first = true;
                for (case_val, branch) in cases {
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
                            if select_is_string || self.is_string_expression(case_val) {
                                format!("({} == {})", temp, v)
                            } else {
                                format!(
                                    "(((({}) as f64) - (({}) as f64)).abs() < 0.0001_f64)",
                                    temp, v
                                )
                            }
                        }
                    };

                    if first {
                        self.output
                            .push_str(&format!("{}if {} {{\n", self.indent(), condition));
                        first = false;
                    } else {
                        self.output.push_str(&format!(
                            "{}}} else if {} {{\n",
                            self.indent(),
                            condition
                        ));
                    }
                    self.indent_level += 1;
                    let _ = self.generate_top_level_branch(
                        branch,
                        block_index,
                        resume_index,
                        next_index,
                        line_number,
                        labels,
                        line_numbers,
                        loop_context,
                    )?;
                    self.indent_level -= 1;
                }
                if !first {
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                }
                Ok(false)
            }
            Statement::ForLoop {
                variable,
                start,
                end,
                step,
                body,
            } => {
                let loop_id = self.next_loop_state_id();
                let loop_label = format!("'qb_loop_{}", loop_id);
                let var_idx = self.get_num_var_idx(&variable.name);
                let start_code = self.generate_expression(start)?;
                let end_code = self.generate_expression(end)?;
                let step_code = step
                    .as_ref()
                    .map(|s| {
                        self.generate_expression(s)
                            .unwrap_or_else(|_| "1.0".to_string())
                    })
                    .unwrap_or_else(|| "1.0".to_string());
                let next_tmp = self.next_temp_var();

                self.output.push_str(&format!(
                    "{}if !qb_loop_active[{}] {{\n",
                    self.indent(),
                    loop_id
                ));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, {} as f64);\n",
                    self.indent(),
                    var_idx,
                    start_code
                ));
                self.output.push_str(&format!(
                    "{}qb_loop_end[{}] = {} as f64;\n",
                    self.indent(),
                    loop_id,
                    end_code
                ));
                self.output.push_str(&format!(
                    "{}qb_loop_step[{}] = {} as f64;\n",
                    self.indent(),
                    loop_id,
                    step_code
                ));
                self.output
                    .push_str(&format!("{}qb_loop_pc[{}] = 0;\n", self.indent(), loop_id));
                self.output.push_str(&format!(
                    "{}qb_loop_active[{}] = true;\n",
                    self.indent(),
                    loop_id
                ));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.output.push_str(&format!("{}: loop {{\n", loop_label));
                self.indent_level += 1;
                self.emit_loop_timer_poll(loop_id);
                self.output.push_str(&format!(
                    "{}if if qb_loop_step[{}] >= 0.0 {{ get_var(&num_vars, {}) <= qb_loop_end[{}] }} else {{ get_var(&num_vars, {}) >= qb_loop_end[{}] }} {{\n",
                    self.indent(),
                    loop_id,
                    var_idx,
                    loop_id,
                    var_idx,
                    loop_id
                ));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}match qb_loop_pc[{}] {{\n",
                    self.indent(),
                    loop_id
                ));
                self.indent_level += 1;
                for (stmt_index, stmt) in body.iter().enumerate() {
                    self.output
                        .push_str(&format!("{}{} => {{\n", self.indent(), stmt_index));
                    self.indent_level += 1;
                    let terminated = self.generate_top_level_flow_statement(
                        stmt,
                        block_index,
                        stmt_index,
                        stmt_index + 1,
                        line_number,
                        labels,
                        line_numbers,
                        Some((loop_id, next_index)),
                    )?;
                    if !terminated {
                        self.output.push_str(&format!(
                            "{}qb_loop_pc[{}] = {};\n",
                            self.indent(),
                            loop_id,
                            stmt_index + 1
                        ));
                        self.output.push_str(&format!(
                            "{}continue {};\n",
                            self.indent(),
                            loop_label
                        ));
                    }
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                }
                self.output.push_str(&format!("{}_ => {{\n", self.indent()));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}let {} = get_var(&num_vars, {}) + qb_loop_step[{}];\n",
                    self.indent(),
                    next_tmp,
                    var_idx,
                    loop_id
                ));
                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, {});\n",
                    self.indent(),
                    var_idx,
                    next_tmp
                ));
                self.output
                    .push_str(&format!("{}qb_loop_pc[{}] = 0;\n", self.indent(), loop_id));
                self.output
                    .push_str(&format!("{}continue {};\n", self.indent(), loop_label));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output
                    .push_str(&format!("{}}} else {{\n", self.indent()));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}qb_loop_active[{}] = false;\n",
                    self.indent(),
                    loop_id
                ));
                self.output
                    .push_str(&format!("{}qb_loop_pc[{}] = 0;\n", self.indent(), loop_id));
                self.output
                    .push_str(&format!("{}qb_pc = {};\n", self.indent(), next_index));
                self.output
                    .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                Ok(false)
            }
            Statement::ForEach {
                variable,
                array,
                body,
            } => {
                if let Some(array_name) = self.expr_to_array_name(array) {
                    let loop_id = self.next_loop_state_id();
                    let loop_label = format!("'qb_loop_{}", loop_id);
                    let arr_idx = self.get_arr_var_idx(&array_name);
                    let var_idx = self.get_num_var_idx(&variable.name);
                    self.output.push_str(&format!(
                        "{}if !qb_loop_active[{}] {{ qb_loop_index[{}] = 0; qb_loop_pc[{}] = 0; qb_loop_active[{}] = true; }}\n",
                        self.indent(),
                        loop_id,
                        loop_id,
                        loop_id,
                        loop_id
                    ));
                    self.output.push_str(&format!("{}: loop {{\n", loop_label));
                    self.indent_level += 1;
                    self.emit_loop_timer_poll(loop_id);
                    self.output.push_str(&format!(
                        "{}if qb_loop_index[{}] >= arr_vars[{}].len() {{\n",
                        self.indent(),
                        loop_id,
                        arr_idx
                    ));
                    self.indent_level += 1;
                    self.output.push_str(&format!(
                        "{}qb_loop_active[{}] = false;\n",
                        self.indent(),
                        loop_id
                    ));
                    self.output.push_str(&format!(
                        "{}qb_loop_pc[{}] = 0;\n",
                        self.indent(),
                        loop_id
                    ));
                    self.output
                        .push_str(&format!("{}qb_pc = {};\n", self.indent(), next_index));
                    self.output
                        .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                    self.output.push_str(&format!(
                        "{}set_var(&mut num_vars, {}, arr_vars[{}][qb_loop_index[{}]]);\n",
                        self.indent(),
                        var_idx,
                        arr_idx,
                        loop_id
                    ));
                    self.output.push_str(&format!(
                        "{}match qb_loop_pc[{}] {{\n",
                        self.indent(),
                        loop_id
                    ));
                    self.indent_level += 1;
                    for (stmt_index, stmt) in body.iter().enumerate() {
                        self.output
                            .push_str(&format!("{}{} => {{\n", self.indent(), stmt_index));
                        self.indent_level += 1;
                        let terminated = self.generate_top_level_flow_statement(
                            stmt,
                            block_index,
                            stmt_index,
                            stmt_index + 1,
                            line_number,
                            labels,
                            line_numbers,
                            Some((loop_id, next_index)),
                        )?;
                        if !terminated {
                            self.output.push_str(&format!(
                                "{}qb_loop_pc[{}] = {};\n",
                                self.indent(),
                                loop_id,
                                stmt_index + 1
                            ));
                            self.output.push_str(&format!(
                                "{}continue {};\n",
                                self.indent(),
                                loop_label
                            ));
                        }
                        self.indent_level -= 1;
                        self.output.push_str(&format!("{}}}\n", self.indent()));
                    }
                    self.output.push_str(&format!("{}_ => {{\n", self.indent()));
                    self.indent_level += 1;
                    self.output.push_str(&format!(
                        "{}qb_loop_index[{}] += 1;\n",
                        self.indent(),
                        loop_id
                    ));
                    self.output.push_str(&format!(
                        "{}qb_loop_pc[{}] = 0;\n",
                        self.indent(),
                        loop_id
                    ));
                    self.output
                        .push_str(&format!("{}continue {};\n", self.indent(), loop_label));
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                }
                Ok(false)
            }
            Statement::WhileLoop { condition, body } => {
                let loop_id = self.next_loop_state_id();
                let loop_label = format!("'qb_loop_{}", loop_id);
                let cond_code = self.generate_condition(condition);
                self.output.push_str(&format!(
                    "{}if !qb_loop_active[{}] {{ qb_loop_pc[{}] = 0; qb_loop_active[{}] = true; }}\n",
                    self.indent(),
                    loop_id,
                    loop_id,
                    loop_id
                ));
                self.output.push_str(&format!("{}: loop {{\n", loop_label));
                self.indent_level += 1;
                self.emit_loop_timer_poll(loop_id);
                self.output
                    .push_str(&format!("{}if {} {{\n", self.indent(), cond_code));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}match qb_loop_pc[{}] {{\n",
                    self.indent(),
                    loop_id
                ));
                self.indent_level += 1;
                for (stmt_index, stmt) in body.iter().enumerate() {
                    self.output
                        .push_str(&format!("{}{} => {{\n", self.indent(), stmt_index));
                    self.indent_level += 1;
                    let terminated = self.generate_top_level_flow_statement(
                        stmt,
                        block_index,
                        stmt_index,
                        stmt_index + 1,
                        line_number,
                        labels,
                        line_numbers,
                        Some((loop_id, next_index)),
                    )?;
                    if !terminated {
                        self.output.push_str(&format!(
                            "{}qb_loop_pc[{}] = {};\n",
                            self.indent(),
                            loop_id,
                            stmt_index + 1
                        ));
                        self.output.push_str(&format!(
                            "{}continue {};\n",
                            self.indent(),
                            loop_label
                        ));
                    }
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                }
                self.output.push_str(&format!("{}_ => {{\n", self.indent()));
                self.indent_level += 1;
                self.output
                    .push_str(&format!("{}qb_loop_pc[{}] = 0;\n", self.indent(), loop_id));
                self.output
                    .push_str(&format!("{}continue {};\n", self.indent(), loop_label));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output
                    .push_str(&format!("{}}} else {{\n", self.indent()));
                self.indent_level += 1;
                self.output.push_str(&format!(
                    "{}qb_loop_active[{}] = false;\n",
                    self.indent(),
                    loop_id
                ));
                self.output
                    .push_str(&format!("{}qb_loop_pc[{}] = 0;\n", self.indent(), loop_id));
                self.output
                    .push_str(&format!("{}qb_pc = {};\n", self.indent(), next_index));
                self.output
                    .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                Ok(false)
            }
            Statement::DoLoop {
                condition,
                body,
                pre_condition,
            } => {
                let loop_id = self.next_loop_state_id();
                let loop_label = format!("'qb_loop_{}", loop_id);
                let cond_code = condition.as_ref().map(|cond| self.generate_condition(cond));
                self.output.push_str(&format!(
                    "{}if !qb_loop_active[{}] {{ qb_loop_pc[{}] = 0; qb_loop_active[{}] = true; }}\n",
                    self.indent(),
                    loop_id,
                    loop_id,
                    loop_id
                ));
                self.output.push_str(&format!("{}: loop {{\n", loop_label));
                self.indent_level += 1;
                self.emit_loop_timer_poll(loop_id);
                if *pre_condition {
                    if let Some(cond_code) = &cond_code {
                        self.output
                            .push_str(&format!("{}if {} {{\n", self.indent(), cond_code));
                        self.indent_level += 1;
                        self.output.push_str(&format!(
                            "{}qb_loop_active[{}] = false;\n",
                            self.indent(),
                            loop_id
                        ));
                        self.output.push_str(&format!(
                            "{}qb_loop_pc[{}] = 0;\n",
                            self.indent(),
                            loop_id
                        ));
                        self.output.push_str(&format!(
                            "{}qb_pc = {};\n",
                            self.indent(),
                            next_index
                        ));
                        self.output
                            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                        self.indent_level -= 1;
                        self.output.push_str(&format!("{}}}\n", self.indent()));
                    }
                }
                self.output.push_str(&format!(
                    "{}match qb_loop_pc[{}] {{\n",
                    self.indent(),
                    loop_id
                ));
                self.indent_level += 1;
                for (stmt_index, stmt) in body.iter().enumerate() {
                    self.output
                        .push_str(&format!("{}{} => {{\n", self.indent(), stmt_index));
                    self.indent_level += 1;
                    let terminated = self.generate_top_level_flow_statement(
                        stmt,
                        block_index,
                        stmt_index,
                        stmt_index + 1,
                        line_number,
                        labels,
                        line_numbers,
                        Some((loop_id, next_index)),
                    )?;
                    if !terminated {
                        self.output.push_str(&format!(
                            "{}qb_loop_pc[{}] = {};\n",
                            self.indent(),
                            loop_id,
                            stmt_index + 1
                        ));
                        self.output.push_str(&format!(
                            "{}continue {};\n",
                            self.indent(),
                            loop_label
                        ));
                    }
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                }
                self.output.push_str(&format!("{}_ => {{\n", self.indent()));
                self.indent_level += 1;
                self.output
                    .push_str(&format!("{}qb_loop_pc[{}] = 0;\n", self.indent(), loop_id));
                if !*pre_condition {
                    if let Some(cond_code) = &cond_code {
                        self.output
                            .push_str(&format!("{}if {} {{\n", self.indent(), cond_code));
                        self.indent_level += 1;
                        self.output.push_str(&format!(
                            "{}qb_loop_active[{}] = false;\n",
                            self.indent(),
                            loop_id
                        ));
                        self.output.push_str(&format!(
                            "{}qb_pc = {};\n",
                            self.indent(),
                            next_index
                        ));
                        self.output
                            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
                        self.indent_level -= 1;
                        self.output.push_str(&format!("{}}}\n", self.indent()));
                    }
                }
                self.output
                    .push_str(&format!("{}continue {};\n", self.indent(), loop_label));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                Ok(false)
            }
            _ => {
                self.generate_top_level_checked_statement(
                    statement,
                    resume_index,
                    next_index,
                    line_number,
                    loop_context,
                )?;
                Ok(false)
            }
        }
    }

    fn generate_top_level_checked_statement(
        &mut self,
        statement: &Statement,
        resume_index: usize,
        next_index: usize,
        line_number: Option<u16>,
        loop_context: Option<(usize, usize)>,
    ) -> QResult<()> {
        self.output.push_str(&format!(
            "{}let qb_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{\n",
            self.indent()
        ));
        self.indent_level += 1;
        self.generate_statement(statement)?;
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}));\n", self.indent()));
        self.output.push_str(&format!(
            "{}if let Err(qb_panic) = qb_result {{\n",
            self.indent()
        ));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}let qb_error = qb_take_runtime_error(qb_panic);\n",
            self.indent()
        ));
        self.emit_top_level_runtime_error_dispatch(
            "qb_error",
            resume_index,
            next_index,
            line_number,
            loop_context,
        );
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", self.indent()));
        Ok(())
    }

    fn emit_top_level_runtime_error_dispatch(
        &mut self,
        error_var: &str,
        resume_index: usize,
        next_index: usize,
        line_number: Option<u16>,
        loop_context: Option<(usize, usize)>,
    ) {
        let line_number = line_number.unwrap_or(0);
        let (resume_kind, resume_next_kind) = if let Some((loop_id, _)) = loop_context {
            (loop_id + 1, loop_id + 1)
        } else {
            (0, 0)
        };
        self.output.push_str(&format!(
            "{}qb_set_error_state({}.code, {}, &{}.message);\n",
            self.indent(),
            error_var,
            line_number,
            error_var
        ));
        self.output
            .push_str(&format!("{}if qb_error_resume_next {{\n", self.indent()));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}qb_error_resume_kind = {};\n",
            self.indent(),
            resume_kind
        ));
        self.output.push_str(&format!(
            "{}qb_error_resume_pc = {};\n",
            self.indent(),
            resume_index
        ));
        self.output.push_str(&format!(
            "{}qb_error_resume_next_kind = {};\n",
            self.indent(),
            resume_next_kind
        ));
        self.output.push_str(&format!(
            "{}qb_error_resume_next_pc = {};\n",
            self.indent(),
            next_index
        ));
        if resume_next_kind == 0 {
            self.output
                .push_str(&format!("{}qb_pc = {};\n", self.indent(), next_index));
        } else {
            self.output.push_str(&format!(
                "{}qb_loop_pc[{}] = {};\n",
                self.indent(),
                resume_next_kind - 1,
                next_index
            ));
            self.output.push_str(&format!(
                "{}qb_pc = qb_loop_block[{}];\n",
                self.indent(),
                resume_next_kind - 1
            ));
        }
        self.output
            .push_str(&format!("{}qb_in_error_handler = false;\n", self.indent()));
        self.output
            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
        self.indent_level -= 1;
        self.output.push_str(&format!(
            "{}}} else if qb_in_error_handler {{\n",
            self.indent()
        ));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}qb_report_unhandled_error(&{});\n",
            self.indent(),
            error_var
        ));
        self.indent_level -= 1;
        self.output.push_str(&format!(
            "{}}} else if let Some(qb_handler) = qb_error_handler {{\n",
            self.indent()
        ));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}qb_error_resume_kind = {};\n",
            self.indent(),
            resume_kind
        ));
        self.output.push_str(&format!(
            "{}qb_error_resume_pc = {};\n",
            self.indent(),
            resume_index
        ));
        self.output.push_str(&format!(
            "{}qb_error_resume_next_kind = {};\n",
            self.indent(),
            resume_next_kind
        ));
        self.output.push_str(&format!(
            "{}qb_error_resume_next_pc = {};\n",
            self.indent(),
            next_index
        ));
        self.output
            .push_str(&format!("{}qb_pc = qb_handler;\n", self.indent()));
        self.output
            .push_str(&format!("{}qb_in_error_handler = true;\n", self.indent()));
        self.output
            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
        self.indent_level -= 1;
        self.output
            .push_str(&format!("{}}} else {{\n", self.indent()));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}qb_report_unhandled_error(&{});\n",
            self.indent(),
            error_var
        ));
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", self.indent()));
    }

    fn emit_control_flow_resume_dispatch(
        &mut self,
        kind_expr: &str,
        pc_expr: &str,
        clear_timer_active: bool,
    ) {
        self.output
            .push_str(&format!("{}if {} == 0 {{\n", self.indent(), kind_expr));
        self.indent_level += 1;
        self.output
            .push_str(&format!("{}qb_pc = {};\n", self.indent(), pc_expr));
        if clear_timer_active {
            self.output
                .push_str(&format!("{}qb_timer_active = false;\n", self.indent()));
            self.output
                .push_str(&format!("{}if qb_play_active {{\n", self.indent()));
            self.indent_level += 1;
            self.output
                .push_str(&format!("{}qb_play_active = false;\n", self.indent()));
            self.output.push_str(&format!(
                "{}qb_set_play_handler_active(false);\n",
                self.indent()
            ));
            self.output
                .push_str(&format!("{}if qb_play_trap_state != 0 {{\n", self.indent()));
            self.indent_level += 1;
            self.output
                .push_str(&format!("{}qb_play_trap_state = 1;\n", self.indent()));
            self.indent_level -= 1;
            self.output.push_str(&format!("{}}}\n", self.indent()));
            self.indent_level -= 1;
            self.output.push_str(&format!("{}}}\n", self.indent()));
        }
        self.output
            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
        self.indent_level -= 1;
        self.output
            .push_str(&format!("{}}} else {{\n", self.indent()));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}let qb_loop_id = {} - 1;\n",
            self.indent(),
            kind_expr
        ));
        self.output.push_str(&format!(
            "{}if qb_loop_id < qb_loop_pc.len() {{\n",
            self.indent()
        ));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}qb_loop_pc[qb_loop_id] = {};\n",
            self.indent(),
            pc_expr
        ));
        self.output.push_str(&format!(
            "{}qb_pc = qb_loop_block[qb_loop_id];\n",
            self.indent()
        ));
        if clear_timer_active {
            self.output
                .push_str(&format!("{}qb_timer_active = false;\n", self.indent()));
            self.output
                .push_str(&format!("{}if qb_play_active {{\n", self.indent()));
            self.indent_level += 1;
            self.output
                .push_str(&format!("{}qb_play_active = false;\n", self.indent()));
            self.output.push_str(&format!(
                "{}qb_set_play_handler_active(false);\n",
                self.indent()
            ));
            self.output
                .push_str(&format!("{}if qb_play_trap_state != 0 {{\n", self.indent()));
            self.indent_level += 1;
            self.output
                .push_str(&format!("{}qb_play_trap_state = 1;\n", self.indent()));
            self.indent_level -= 1;
            self.output.push_str(&format!("{}}}\n", self.indent()));
            self.indent_level -= 1;
            self.output.push_str(&format!("{}}}\n", self.indent()));
        }
        self.output
            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", self.indent()));
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", self.indent()));
    }

    fn emit_loop_timer_poll(&mut self, loop_id: usize) {
        self.output.push_str(&format!(
            "{}if qb_timer_enabled && !qb_timer_active && qb_timer_handler.is_some() && std::time::Instant::now() >= qb_timer_next_tick {{\n",
            self.indent()
        ));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}qb_timer_next_tick = std::time::Instant::now() + qb_timer_interval;\n",
            self.indent()
        ));
        self.output.push_str(&format!(
            "{}qb_gosub_stack.push(({}, qb_loop_pc[{}]));\n",
            self.indent(),
            loop_id + 1,
            loop_id
        ));
        self.output.push_str(&format!(
            "{}qb_pc = qb_timer_handler.unwrap();\n",
            self.indent()
        ));
        self.output
            .push_str(&format!("{}qb_timer_active = true;\n", self.indent()));
        self.output
            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", self.indent()));
        self.output.push_str(&format!(
            "{}qb_update_play_queue(qb_play_queue_limit, qb_play_trap_state, &mut qb_play_pending_event);\n",
            self.indent()
        ));
        self.output.push_str(&format!(
            "{}if qb_play_trap_state == 1 && !qb_play_active && qb_play_handler.is_some() && qb_play_pending_event {{\n",
            self.indent()
        ));
        self.indent_level += 1;
        self.output.push_str(&format!(
            "{}qb_gosub_stack.push(({}, qb_loop_pc[{}]));\n",
            self.indent(),
            loop_id + 1,
            loop_id
        ));
        self.output.push_str(&format!(
            "{}qb_pc = qb_play_handler.unwrap();\n",
            self.indent()
        ));
        self.output
            .push_str(&format!("{}qb_play_active = true;\n", self.indent()));
        self.output.push_str(&format!(
            "{}qb_set_play_handler_active(true);\n",
            self.indent()
        ));
        self.output
            .push_str(&format!("{}qb_play_trap_state = 2;\n", self.indent()));
        self.output.push_str(&format!(
            "{}qb_play_pending_event = false;\n",
            self.indent()
        ));
        self.output
            .push_str(&format!("{}continue 'qb_main;\n", self.indent()));
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", self.indent()));
    }

    pub(super) fn collect_var_from_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::Variable(var) => {
                if self.is_in_sub && self.shared_global_scalar_name(&var.name) {
                    return;
                }
                if self.variable_is_string(var) {
                    self.get_str_var_idx(&var.name);
                } else {
                    self.get_num_var_idx(&var.name);
                }
            }
            Expression::ArrayAccess {
                name, type_suffix, ..
            } => {
                if self.is_in_sub && self.shared_global_array_name(name) {
                    return;
                }
                if self.array_udt_type(name).is_some() {
                    return;
                }
                if self.array_is_string(name, *type_suffix) {
                    self.get_str_arr_var_idx(name);
                } else {
                    self.get_arr_var_idx(name);
                }
            }
            Expression::FieldAccess { .. } => {
                if let Some(field) = self.resolve_field_access_layout(expr) {
                    match field.field_type {
                        QType::String(_) => {
                            if field.array_indices.is_some() {
                                self.get_str_arr_var_idx(&field.storage_name);
                            } else {
                                self.get_str_var_idx(&field.storage_name);
                            }
                        }
                        QType::Integer(_)
                        | QType::Long(_)
                        | QType::Single(_)
                        | QType::Double(_) => {
                            if field.array_indices.is_some() {
                                self.get_arr_var_idx(&field.storage_name);
                            } else {
                                self.get_num_var_idx(&field.storage_name);
                            }
                        }
                        _ => {}
                    }
                } else if let Some(name) = Self::qualified_field_name(expr) {
                    if self.is_in_sub && self.shared_global_scalar_name(&name) {
                        return;
                    }
                    if self.name_is_string(&name) {
                        self.get_str_var_idx(&name);
                    } else {
                        self.get_num_var_idx(&name);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn expr_to_array_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(var) => Some(var.name.clone()),
            Expression::ArrayAccess { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    pub(super) fn generate_data_array(&mut self, data_statements: &[Vec<Expression>]) {
        let mut all_values: Vec<String> = Vec::new();
        for stmt in data_statements {
            for expr in stmt {
                if let Expression::Literal(qtype) = expr {
                    match qtype {
                        QType::Integer(i) => all_values.push(i.to_string()),
                        QType::Long(l) => all_values.push(l.to_string()),
                        QType::Single(s) => all_values.push(s.to_string()),
                        QType::Double(d) => all_values.push(d.to_string()),
                        QType::String(s) => all_values.push(s.clone()),
                        _ => all_values.push(String::new()),
                    }
                }
            }
        }

        if !all_values.is_empty() {
            self.output.push_str("static DATA_VALUES: &[&str] = &[\n");
            for val in &all_values {
                self.output.push_str(&format!("    {:?},\n", val));
            }
            self.output.push_str("];\n\n");
        }
    }

    pub(super) fn write_prelude(&mut self) {
        self.output.push_str(
            r#"// Auto-generated by QBNex
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_parens)]
#![allow(static_mut_refs)]
#![allow(unreachable_code)]
#![allow(unused_assignments)]

use std::io::{self, Read, Seek, SeekFrom, Write};

"#,
        );
        self.output.push_str(
            r#"
thread_local! {
    static QB_CURSOR_STATE: std::cell::RefCell<(i32, i32)> = std::cell::RefCell::new((1, 1));
    static QB_CURSOR_VISIBLE: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static QB_CURSOR_SHAPE: std::cell::RefCell<(Option<i32>, Option<i32>)> =
        std::cell::RefCell::new((None, None));
    static QB_PENDING_VIEW_PRINT_SCROLL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static QB_SCREEN_SIZE: std::cell::RefCell<(i32, i32)> = std::cell::RefCell::new((80, 25));
    static QB_VIEW_PRINT_REGION: std::cell::RefCell<Option<(i32, i32)>> = std::cell::RefCell::new(None);
    static QB_TEXT_CHARS: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![b' '; 80 * 25]);
    static QB_TEXT_ATTRS: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![7; 80 * 25]);
    static QB_TEXT_FOREGROUND: std::cell::Cell<u8> = const { std::cell::Cell::new(7) };
    static QB_TEXT_BACKGROUND: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    static QB_NONINTERACTIVE_INKEY_EMITTED: std::cell::RefCell<bool> = std::cell::RefCell::new(false);
}

fn qb_text_columns() -> i32 {
    QB_SCREEN_SIZE.with(|state| state.borrow().0.max(1))
}

fn qb_text_rows() -> i32 {
    QB_SCREEN_SIZE.with(|state| state.borrow().1.max(1))
}

fn qb_view_print_bounds() -> (i32, i32) {
    let rows = qb_text_rows();
    QB_VIEW_PRINT_REGION.with(|region| {
        if let Some((top, bottom)) = *region.borrow() {
            let top = top.clamp(1, rows);
            let bottom = bottom.clamp(top, rows);
            (top, bottom)
        } else {
            (1, rows)
        }
    })
}

fn qb_current_text_attr() -> u8 {
    QB_TEXT_FOREGROUND.with(|fg| {
        QB_TEXT_BACKGROUND.with(|bg| fg.get().saturating_add(bg.get().saturating_mul(16)))
    })
}

fn qb_resize_text_buffer(columns: i32, rows: i32, preserve: bool) {
    let columns = columns.max(1) as usize;
    let rows = rows.max(1) as usize;
    let attr = qb_current_text_attr();
    QB_TEXT_CHARS.with(|chars| {
        QB_TEXT_ATTRS.with(|attrs| {
            let old_chars = chars.borrow().clone();
            let old_attrs = attrs.borrow().clone();
            let old_columns = qb_text_columns().max(1) as usize;
            let old_rows = qb_text_rows().max(1) as usize;
            let mut new_chars = vec![b' '; columns * rows];
            let mut new_attrs = vec![attr; columns * rows];
            if preserve {
                let copy_rows = old_rows.min(rows);
                let copy_cols = old_columns.min(columns);
                for row in 0..copy_rows {
                    let old_start = row * old_columns;
                    let new_start = row * columns;
                    new_chars[new_start..new_start + copy_cols]
                        .copy_from_slice(&old_chars[old_start..old_start + copy_cols]);
                    new_attrs[new_start..new_start + copy_cols]
                        .copy_from_slice(&old_attrs[old_start..old_start + copy_cols]);
                }
            }
            *chars.borrow_mut() = new_chars;
            *attrs.borrow_mut() = new_attrs;
        });
    });
}

fn qb_text_index(row: i32, col: i32) -> Option<usize> {
    let row = row.clamp(1, qb_text_rows()) as usize - 1;
    let col = col.clamp(1, qb_text_columns()) as usize - 1;
    let width = qb_text_columns() as usize;
    let idx = row * width + col;
    QB_TEXT_CHARS.with(|chars| (idx < chars.borrow().len()).then_some(idx))
}

fn qb_put_text_cell(row: i32, col: i32, ch: u8) {
    if let Some(idx) = qb_text_index(row, col) {
        QB_TEXT_CHARS.with(|chars| chars.borrow_mut()[idx] = ch);
        QB_TEXT_ATTRS.with(|attrs| attrs.borrow_mut()[idx] = qb_current_text_attr());
    }
}

fn qb_clear_text_rows(top: i32, bottom: i32) {
    let width = qb_text_columns().max(1) as usize;
    let top = top.clamp(1, qb_text_rows()) as usize;
    let bottom = bottom.clamp(top as i32, qb_text_rows()) as usize;
    let attr = qb_current_text_attr();
    QB_TEXT_CHARS.with(|chars| {
        QB_TEXT_ATTRS.with(|attrs| {
            let mut chars = chars.borrow_mut();
            let mut attrs = attrs.borrow_mut();
            for row in top..=bottom {
                let start = (row - 1) * width;
                let end = start + width;
                chars[start..end].fill(b' ');
                attrs[start..end].fill(attr);
            }
        });
    });
}

fn qb_scroll_text_rows_up(top: i32, bottom: i32) {
    let width = qb_text_columns().max(1) as usize;
    let top = top.clamp(1, qb_text_rows()) as usize;
    let bottom = bottom.clamp(top as i32, qb_text_rows()) as usize;
    if top >= bottom {
        qb_clear_text_rows(top as i32, bottom as i32);
        return;
    }

    QB_TEXT_CHARS.with(|chars| {
        QB_TEXT_ATTRS.with(|attrs| {
            let mut chars = chars.borrow_mut();
            let mut attrs = attrs.borrow_mut();
            for row in top..bottom {
                let dst_start = (row - 1) * width;
                let src_start = row * width;
                let src_end = src_start + width;
                chars.copy_within(src_start..src_end, dst_start);
                attrs.copy_within(src_start..src_end, dst_start);
            }
        });
    });

    qb_clear_text_rows(bottom as i32, bottom as i32);
}

fn qb_screen(row: f64, col: f64, color_flag: f64) -> f64 {
    let row = row.round() as i32;
    let col = col.round() as i32;
    if let Some(idx) = qb_text_index(row, col) {
        if color_flag.round() as i32 != 0 {
            QB_TEXT_ATTRS.with(|attrs| attrs.borrow()[idx] as f64)
        } else {
            QB_TEXT_CHARS.with(|chars| chars.borrow()[idx] as f64)
        }
    } else {
        0.0
    }
}

fn qb_ensure_cursor_in_window() {
    let columns = qb_text_columns();
    let rows = qb_text_rows();
    let (top, bottom) = qb_view_print_bounds();
    let has_view_print = QB_VIEW_PRINT_REGION.with(|region| region.borrow().is_some());
    QB_CURSOR_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if has_view_print && (state.0 < top || state.0 > bottom) {
            state.0 = top;
            state.1 = 1;
        } else {
            state.0 = state.0.clamp(1, rows);
            state.1 = state.1.clamp(1, columns);
        }
    });
}

fn qb_consume_pending_view_print_scroll() {
    if QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.get()) {
        let (top, bottom) = if QB_VIEW_PRINT_REGION.with(|region| region.borrow().is_some()) {
            qb_view_print_bounds()
        } else {
            (1, qb_text_rows())
        };
        qb_scroll_text_rows_up(top, bottom);
        QB_CURSOR_STATE.with(|state| *state.borrow_mut() = (bottom, 1));
        QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
    }
}

fn qb_advance_console_line() {
    let rows = qb_text_rows();
    let has_view_print = QB_VIEW_PRINT_REGION.with(|region| region.borrow().is_some());
    let (top, bottom) = qb_view_print_bounds();
    QB_CURSOR_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if has_view_print {
            let current = state.0.clamp(top, bottom);
            state.0 = if current < bottom {
                QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
                current + 1
            } else {
                QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(true));
                bottom
            };
        } else {
            state.0 = if state.0 < rows {
                QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
                state.0 + 1
            } else {
                QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(true));
                rows
            };
        }
        state.1 = 1;
    });
}

fn qb_emit_console_text(text: &str) {
    if text.is_empty() {
        return;
    }

    qb_ensure_cursor_in_window();
    let columns = qb_text_columns();
    let mut rendered = String::with_capacity(text.len());
    let (mut row, mut col) = QB_CURSOR_STATE.with(|state| *state.borrow());
    let has_view_print = QB_VIEW_PRINT_REGION.with(|region| region.borrow().is_some());
    let (top, bottom) = qb_view_print_bounds();
    let rows = qb_text_rows();
    for ch in text.chars() {
        qb_consume_pending_view_print_scroll();
        match ch {
            '\n' => {
                rendered.push('\n');
                if has_view_print {
                    let current = row.clamp(top, bottom);
                    row = if current < bottom {
                        QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
                        current + 1
                    } else {
                        QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(true));
                        bottom
                    };
                } else {
                    row = if row < rows {
                        QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
                        row + 1
                    } else {
                        QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(true));
                        rows
                    };
                }
                col = 1;
            }
            '\r' => {
                rendered.push('\r');
                col = 1;
            }
            _ => {
                rendered.push(ch);
                qb_put_text_cell(row, col, ch as u8);
                col += 1;
                if col > columns {
                    rendered.push('\n');
                    if has_view_print {
                        let current = row.clamp(top, bottom);
                        row = if current < bottom {
                            QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
                            current + 1
                        } else {
                            QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(true));
                            bottom
                        };
                    } else {
                        row = if row < rows {
                            QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
                            row + 1
                        } else {
                            QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(true));
                            rows
                        };
                    }
                    col = 1;
                }
            }
        }
    }
    QB_CURSOR_STATE.with(|state| *state.borrow_mut() = (row, col));
    print!("{}", rendered);
    let _ = io::stdout().flush();
}

fn qb_next_print_zone(column: i32, line_width: i32) -> Option<i32> {
    let normalized = column.max(1);
    let target = ((normalized - 1) / 14 + 1) * 14 + 1;
    if target > line_width.max(1) {
        None
    } else {
        Some(target)
    }
}

fn qb_print(text: &str) {
    qb_emit_console_text(text);
}

fn qb_trim_float_text(mut text: String) -> String {
    if let Some(exponent_index) = text.find(['e', 'E']) {
        let exponent = text.split_off(exponent_index);
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        if text == "-0" {
            text = "0".to_string();
        }
        text.push_str(&exponent);
        return text;
    }

    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        "0".to_string()
    } else {
        text
    }
}

fn qb_format_significant_digits(value: f64, digits: usize) -> String {
    if value == 0.0 {
        return "0".to_string();
    }

    let exponent = value.abs().log10().floor() as i32;
    let decimals = (digits as i32 - exponent - 1).max(0) as usize;
    qb_trim_float_text(format!("{:.*}", decimals, value))
}

fn qb_format_number(value: f64) -> String {
    if !value.is_finite() {
        return format!("{}", value);
    }

    if value == 0.0 {
        return "0".to_string();
    }

    let rounded = value.round();
    if (value - rounded).abs() <= f64::EPSILON * value.abs().max(1.0) {
        if value.abs() <= i64::MAX as f64 {
            return format!("{}", rounded as i64);
        }
        return format!("{:.0}", rounded);
    }

    let single = value as f32 as f64;
    let single_tolerance = f32::EPSILON as f64 * value.abs().max(1.0) * 2.0;
    if single.is_finite() && (value - single).abs() <= single_tolerance {
        return qb_format_significant_digits(value, 7);
    }

    qb_trim_float_text(format!("{}", value))
}

fn qb_concat<L: std::fmt::Display, R: std::fmt::Display>(left: L, right: R) -> String {
    format!("{}{}", left, right)
}

fn qb_write_numeric_field(value: f64) -> String {
    qb_format_number(value)
}

fn qb_write_string_field(value: &str) -> String {
    format!("\"{}\"", value)
}

fn qb_print_spc(count: f64) {
    let count = count.round().max(0.0) as usize;
    if count == 0 {
        return;
    }
    let spaces = " ".repeat(count);
    qb_print(&spaces);
}

fn qb_print_tab(target_col: f64) {
    let target_col = target_col.round().max(1.0) as i32;
    qb_ensure_cursor_in_window();
    let target_col = target_col.min(qb_text_columns());
    let current_col = QB_CURSOR_STATE.with(|state| state.borrow().1);
    if target_col <= current_col {
        qb_emit_console_text("\n");
    }
    let current_col = QB_CURSOR_STATE.with(|state| state.borrow().1);
    let spaces = (target_col - current_col).max(0) as usize;
    if spaces > 0 {
        qb_print(&" ".repeat(spaces));
    }
}

fn qb_print_comma() {
    let current_col = QB_CURSOR_STATE.with(|state| state.borrow().1);
    if let Some(target_col) = qb_next_print_zone(current_col, qb_text_columns()) {
        qb_print_tab(target_col as f64);
    } else {
        qb_print_newline();
    }
}

fn qb_print_newline() {
    qb_emit_console_text("\n");
}

fn qb_width(columns: f64, rows: f64) {
    let columns = columns.round().max(1.0) as i32;
    let rows = rows.round().max(1.0) as i32;
    qb_resize_text_buffer(columns, rows, true);
    QB_SCREEN_SIZE.with(|state| *state.borrow_mut() = (columns, rows));
    QB_VIEW_PRINT_REGION.with(|region| {
        let current = *region.borrow();
        if let Some((top, bottom)) = current {
            let top = top.clamp(1, rows);
            *region.borrow_mut() = Some((top, bottom.clamp(top, rows)));
        }
    });
    QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
    qb_ensure_cursor_in_window();
}

fn qb_default_text_geometry_for_screen_mode(mode: i32) -> (i32, i32) {
    match mode {
        1 | 4 | 5 | 7 | 13 => (40, 25),
        2 | 6 | 8 | 9 | 10 => (80, 25),
        11 | 12 => (80, 30),
        _ => (80, 25),
    }
}

fn qb_apply_screen_mode(mode: i32) {
    let (columns, rows) = qb_default_text_geometry_for_screen_mode(mode);
    qb_resize_text_buffer(columns, rows, false);
    QB_SCREEN_SIZE.with(|state| *state.borrow_mut() = (columns, rows));
    QB_VIEW_PRINT_REGION.with(|region| *region.borrow_mut() = None);
    QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
    QB_CURSOR_STATE.with(|state| *state.borrow_mut() = (1, 1));
}

fn qb_view_print(top: f64, bottom: f64) {
    let rows = qb_text_rows();
    let top = (top.round() as i32).clamp(1, rows);
    let bottom = (bottom.round() as i32).clamp(top, rows);
    QB_VIEW_PRINT_REGION.with(|region| *region.borrow_mut() = Some((top, bottom)));
    QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
    QB_CURSOR_STATE.with(|state| *state.borrow_mut() = (top, 1));
}

fn qb_view_print_reset() {
    QB_VIEW_PRINT_REGION.with(|region| *region.borrow_mut() = None);
    QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
    qb_ensure_cursor_in_window();
}

fn qb_set_cursor_state(visible: f64, start: f64, stop: f64) {
    let visible = visible.round() as i32;
    let start = start.round() as i32;
    let stop = stop.round() as i32;

    if visible >= 0 {
        let visible = visible != 0;
        QB_CURSOR_VISIBLE.with(|state| {
            if state.get() != visible {
                print!("{}", if visible { "\x1B[?25h" } else { "\x1B[?25l" });
                let _ = io::stdout().flush();
            }
            state.set(visible);
        });
    }

    if start >= 0 || stop >= 0 {
        QB_CURSOR_SHAPE.with(|shape| {
            let mut shape = shape.borrow_mut();
            if start >= 0 {
                shape.0 = Some(start);
            }
            if stop >= 0 {
                shape.1 = Some(stop);
            }
        });
    }
}
thread_local! {
    static QB_PRINTER_COL: std::cell::Cell<i16> = const { std::cell::Cell::new(1) };
}
fn qb_lprint(text: &str) {
    QB_PRINTER_COL.with(|col| {
        let mut current = col.get();
        for ch in text.chars() {
            match ch {
                '\n' | '\r' => current = 1,
                _ => current = current.saturating_add(1),
            }
        }
        col.set(current);
    });
}
fn qb_lprint_spc(count: f64) {
    let count = count.round().max(0.0) as i16;
    QB_PRINTER_COL.with(|col| col.set(col.get().saturating_add(count)));
}
fn qb_lprint_tab(target_col: f64) {
    QB_PRINTER_COL.with(|col| col.set(target_col.round().max(1.0) as i16));
}
fn qb_lprint_comma() {
    QB_PRINTER_COL.with(|col| {
        let current = col.get();
        if let Some(target) = qb_next_print_zone(current as i32, 80) {
            col.set(target as i16);
        } else {
            col.set(1);
        }
    });
}
fn qb_lprint_newline() {
    QB_PRINTER_COL.with(|col| col.set(1));
}
fn qb_lprint_using(pattern: &str, values: &[String], comma_after: &[bool], newline: bool) {
    let chunks = qb_format_using_values(pattern, values);
    for (index, chunk) in chunks.iter().enumerate() {
        qb_lprint(chunk);
        if comma_after.get(index).copied().unwrap_or(false) {
            qb_lprint_comma();
        }
    }
    if newline {
        qb_lprint_newline();
    }
}
"#
        );
        if self.use_graphics {
            self.write_graphics_prelude();
        } else {
            self.output.push_str(
                r#"
fn qb_cls(mode: f64) {
     let mode = mode.round() as i32;
     let (top, bottom) = if mode == 0 {
         (1, qb_text_rows())
     } else {
         qb_view_print_bounds()
     };
     qb_clear_text_rows(top, bottom);
     QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
     QB_CURSOR_STATE.with(|state| *state.borrow_mut() = (top, 1));
}

fn locate(row: f64, col: f64) {
     let row = row.round() as i32;
     let col = col.round() as i32;
     qb_ensure_cursor_in_window();
     let rows = qb_text_rows();
     let cols = qb_text_columns();
     let has_view_print = QB_VIEW_PRINT_REGION.with(|region| region.borrow().is_some());
     let (top, bottom) = qb_view_print_bounds();
     let (current_row, current_col) = QB_CURSOR_STATE.with(|state| *state.borrow());
     let row = if row == 0 {
         current_row
     } else if has_view_print {
         row.clamp(top, bottom)
     } else {
         row.clamp(1, rows)
     };
     let col = if col == 0 { current_col } else { col.clamp(1, cols) };
     if row != current_row || col != current_col {
         print!("\x1B[{};{}H", row, col);
         let _ = io::stdout().flush();
     }
     QB_PENDING_VIEW_PRINT_SCROLL.with(|pending| pending.set(false));
     QB_CURSOR_STATE.with(|state| *state.borrow_mut() = (row, col));
}

fn locate_ex(row: f64, col: f64, cursor: f64, start: f64, stop: f64) {
     locate(row, col);
     qb_set_cursor_state(cursor, start, stop);
}

fn qb_color(foreground: f64, background: f64) {
     let color = (foreground.round() as i32).clamp(0, 15);
     let bg = (background.round() as i32).clamp(0, 15);
     QB_TEXT_FOREGROUND.with(|fg| fg.set(color as u8));
     QB_TEXT_BACKGROUND.with(|cell| cell.set(bg as u8));
     print!("\x1B[{}m", 30 + color);
}

fn qb_sleep(seconds: f64) {
     if seconds <= 0.0 {
          // Check if stdin is available (not piped)
          use std::io::IsTerminal;
          if std::io::stdin().is_terminal() {
             let _ = std::io::stdin().read_line(&mut String::new());
         }
     } else {
         std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
     }
}
fn qb_should_pause_on_exit() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}
"#,
            );
        }

        self.output.push_str(r#"fn get_var(vars: &[f64], idx: usize) -> f64 {
    if idx < vars.len() { vars[idx] } else { 0.0 }
}

fn set_var(vars: &mut [f64], idx: usize, val: f64) {
    if idx < vars.len() { vars[idx] = val; }
}

fn get_str(str_vars: &[String], idx: usize) -> String {
    if idx < str_vars.len() { str_vars[idx].clone() } else { String::new() }
}

fn set_str(str_vars: &mut [String], idx: usize, val: &str) {
    if idx < str_vars.len() { str_vars[idx] = val.to_string(); }
}

fn qb_fit_fixed_string(width: usize, val: &str) -> String {
    let mut text = val.to_string();
    if text.len() > width {
        text.truncate(width);
    } else if text.len() < width {
        text.push_str(&" ".repeat(width - text.len()));
    }
    text
}

fn qb_data_to_num(value: &str) -> f64 {
    value.trim().parse::<f64>().unwrap_or(0.0)
}

// Math functions
fn qb_abs(x: f64) -> f64 { x.abs() }
fn qb_sgn(x: f64) -> f64 { if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 } }
fn qb_int(x: f64) -> f64 { x.floor() }
fn qb_fix(x: f64) -> f64 { if x >= 0.0 { x.floor() } else { x.ceil() } }
fn qb_sqr(x: f64) -> f64 { x.sqrt() }
fn qb_sin(x: f64) -> f64 { x.sin() }
fn qb_cos(x: f64) -> f64 { x.cos() }
fn qb_tan(x: f64) -> f64 { x.tan() }
fn qb_atn(x: f64) -> f64 { x.atan() }
fn qb_exp(x: f64) -> f64 { x.exp() }
fn qb_log(x: f64) -> f64 { x.ln() }
thread_local! {
    static QB_RNG_STATE: std::cell::RefCell<u32> = std::cell::RefCell::new(0x1234_5678);
}
fn qb_randomize(seed: Option<f64>) {
    let seed = seed.unwrap_or_else(qb_timer) as u32;
    QB_RNG_STATE.with(|state| *state.borrow_mut() = seed.wrapping_add(1));
    QB_RNG_LAST.with(|last| *last.borrow_mut() = None);
}
thread_local! {
    static QB_RNG_LAST: std::cell::RefCell<Option<f64>> = std::cell::RefCell::new(None);
}
fn qb_rnd(arg: Option<f64>) -> f64 {
    if let Some(value) = arg {
        if value < 0.0 {
            qb_randomize(Some(value));
        } else if value == 0.0 {
            if let Some(last) = QB_RNG_LAST.with(|last| *last.borrow()) {
                return last;
            }
            return qb_rnd(None);
        }
    }

    let random = QB_RNG_STATE.with(|state| {
        let mut state = state.borrow_mut();
        *state = state.wrapping_mul(1103515245).wrapping_add(12345);
        ((*state / 65536) % 32768) as f64 / 32768.0
    });
    QB_RNG_LAST.with(|last| *last.borrow_mut() = Some(random));
    random
}
fn qb_cint(x: f64) -> f64 { x.round() }
fn qb_clng(x: f64) -> f64 { x.round() }
fn qb_csng(x: f64) -> f64 { x }
fn qb_cdbl(x: f64) -> f64 { x }
fn qb_mki(x: f64) -> String { format!("__BIN:I16:{:04X}", (x as i16) as u16) }
fn qb_mkl(x: f64) -> String { format!("__BIN:I32:{:08X}", (x as i32) as u32) }
fn qb_mks(x: f64) -> String { format!("__BIN:F32:{:08X}", (x as f32).to_bits()) }
fn qb_mkd(x: f64) -> String { format!("__BIN:F64:{:016X}", x.to_bits()) }
fn qb_binary_bytes(s: &str) -> Vec<u8> {
    if let Some(hex) = s.strip_prefix("__BIN:I16:") {
        if let Ok(bits) = u16::from_str_radix(hex, 16) {
            return bits.to_le_bytes().to_vec();
        }
    }
    if let Some(hex) = s.strip_prefix("__BIN:I32:") {
        if let Ok(bits) = u32::from_str_radix(hex, 16) {
            return bits.to_le_bytes().to_vec();
        }
    }
    if let Some(hex) = s.strip_prefix("__BIN:F32:") {
        if let Ok(bits) = u32::from_str_radix(hex, 16) {
            return bits.to_le_bytes().to_vec();
        }
    }
    if let Some(hex) = s.strip_prefix("__BIN:F64:") {
        if let Ok(bits) = u64::from_str_radix(hex, 16) {
            return bits.to_le_bytes().to_vec();
        }
    }
    s.chars().map(|ch| ch as u32 as u8).collect()
}
fn qb_binary_prefix<const N: usize>(s: &str) -> Option<[u8; N]> {
    let bytes = qb_binary_bytes(s);
    if bytes.len() < N {
        return None;
    }
    let mut prefix = [0u8; N];
    prefix.copy_from_slice(&bytes[..N]);
    Some(prefix)
}
fn qb_parse_number(s: &str) -> f64 { s.trim().parse().unwrap_or(0.0) }
fn qb_cvi(s: &str) -> f64 {
    qb_binary_prefix::<2>(s)
        .map(i16::from_le_bytes)
        .map(|value| value as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cvl(s: &str) -> f64 {
    qb_binary_prefix::<4>(s)
        .map(i32::from_le_bytes)
        .map(|value| value as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cvs(s: &str) -> f64 {
    qb_binary_prefix::<4>(s)
        .map(f32::from_le_bytes)
        .map(|value| value as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cvd(s: &str) -> f64 {
    qb_binary_prefix::<8>(s)
        .map(f64::from_le_bytes)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cv_i8(s: &str) -> f64 {
    qb_binary_prefix::<1>(s)
        .map(|bytes| bytes[0] as i8 as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cv_u8(s: &str) -> f64 {
    qb_binary_prefix::<1>(s)
        .map(|bytes| bytes[0] as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cv_u16(s: &str) -> f64 {
    qb_binary_prefix::<2>(s)
        .map(u16::from_le_bytes)
        .map(|value| value as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cv_u32(s: &str) -> f64 {
    qb_binary_prefix::<4>(s)
        .map(u32::from_le_bytes)
        .map(|value| value as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cv_i64(s: &str) -> f64 {
    qb_binary_prefix::<8>(s)
        .map(i64::from_le_bytes)
        .map(|value| value as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_cv_u64(s: &str) -> f64 {
    qb_binary_prefix::<8>(s)
        .map(u64::from_le_bytes)
        .map(|value| value as f64)
        .unwrap_or_else(|| qb_parse_number(s))
}
fn qb_normalize_qb64_type_name(type_name: &str) -> String {
    type_name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}
fn qb_cv(type_name: &str, s: &str) -> f64 {
    match qb_normalize_qb64_type_name(type_name).as_str() {
        "_BYTE" | "BYTE" | "_BIT" | "BIT" => qb_cv_i8(s),
        "_UNSIGNED _BYTE" | "UNSIGNED _BYTE" | "_UNSIGNED BYTE" | "UNSIGNED BYTE"
        | "_UNSIGNED _BIT" | "UNSIGNED _BIT" | "_UNSIGNED BIT" | "UNSIGNED BIT" => qb_cv_u8(s),
        "INTEGER" => qb_cvi(s),
        "_UNSIGNED INTEGER" | "UNSIGNED INTEGER" => qb_cv_u16(s),
        "LONG" => qb_cvl(s),
        "_UNSIGNED LONG" | "UNSIGNED LONG" => qb_cv_u32(s),
        "SINGLE" => qb_cvs(s),
        "DOUBLE" | "_FLOAT" | "FLOAT" => qb_cvd(s),
        "_INTEGER64" | "INTEGER64" | "_OFFSET" | "OFFSET" => qb_cv_i64(s),
        "_UNSIGNED _INTEGER64" | "UNSIGNED _INTEGER64" | "_UNSIGNED INTEGER64"
        | "UNSIGNED INTEGER64" | "_UNSIGNED _OFFSET" | "UNSIGNED _OFFSET"
        | "_UNSIGNED OFFSET" | "UNSIGNED OFFSET" => qb_cv_u64(s),
        _ => qb_parse_number(s),
    }
}
fn qb_fre(_arg: &str) -> f64 { 524288.0 }
fn qb_fre_num(_arg: f64) -> f64 { 1048576.0 }
const QB_PSEUDO_SEGMENT: u16 = 0x6000;
const QB_PSEUDO_MEMORY_SIZE: usize = 1_048_576;

#[derive(Default)]
struct QBPseudoMemoryState {
    memory: Vec<u8>,
    offsets: std::collections::HashMap<String, u16>,
    sizes: std::collections::HashMap<String, usize>,
    next_offset: u16,
    current_segment: u16,
}

trait QBPseudoValueBytes {
    fn qb_pseudo_bytes(&self) -> Vec<u8>;
}

impl QBPseudoValueBytes for f64 {
    fn qb_pseudo_bytes(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl QBPseudoValueBytes for String {
    fn qb_pseudo_bytes(&self) -> Vec<u8> {
        let mut bytes = self.as_bytes().to_vec();
        bytes.push(0);
        bytes
    }
}

impl QBPseudoValueBytes for &str {
    fn qb_pseudo_bytes(&self) -> Vec<u8> {
        let mut bytes = self.as_bytes().to_vec();
        bytes.push(0);
        bytes
    }
}

thread_local! {
    static QB_PSEUDO_MEMORY: std::cell::RefCell<QBPseudoMemoryState> =
        std::cell::RefCell::new(QBPseudoMemoryState {
            memory: vec![0; QB_PSEUDO_MEMORY_SIZE],
            offsets: std::collections::HashMap::new(),
            sizes: std::collections::HashMap::new(),
            next_offset: 1,
            current_segment: 0,
        });
    static QB_PORTS: std::cell::RefCell<std::collections::HashMap<u16, u8>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn qb_set_def_seg(segment: f64) {
    QB_PSEUDO_MEMORY.with(|state| {
        state.borrow_mut().current_segment = segment.round().clamp(0.0, u16::MAX as f64) as u16;
    });
}

fn qb_absolute_address(segment: u16, offset: u16) -> usize {
    (segment as usize) * 16 + offset as usize
}

fn qb_ensure_pseudo_slot<T: QBPseudoValueBytes>(name: &str, value: T) -> (u16, u16) {
    let bytes = value.qb_pseudo_bytes();
    QB_PSEUDO_MEMORY.with(|state| {
        let mut state = state.borrow_mut();
        let needed = bytes.len().max(1);
        let needs_realloc = state.sizes.get(name).map_or(true, |size| *size < needed);
        if needs_realloc {
            let offset = state.next_offset.max(1);
            let next = offset.saturating_add((needed as u16).saturating_add(1));
            state.offsets.insert(name.to_string(), offset);
            state.sizes.insert(name.to_string(), needed);
            state.next_offset = next;
        }
        let offset = *state.offsets.get(name).unwrap_or(&1);
        let capacity = *state.sizes.get(name).unwrap_or(&needed);
        for i in 0..capacity {
            let addr = qb_absolute_address(QB_PSEUDO_SEGMENT, offset.saturating_add(i as u16));
            if addr < state.memory.len() {
                state.memory[addr] = bytes.get(i).copied().unwrap_or(0);
            }
        }
        (QB_PSEUDO_SEGMENT, offset)
    })
}

fn qb_peek(addr: f64) -> f64 {
    QB_PSEUDO_MEMORY.with(|state| {
        let state = state.borrow();
        let offset = addr.floor().clamp(0.0, u16::MAX as f64) as u16;
        let addr = qb_absolute_address(state.current_segment, offset);
        state.memory.get(addr).copied().unwrap_or(0) as f64
    })
}

fn qb_poke(addr: f64, value: f64) {
    QB_PSEUDO_MEMORY.with(|state| {
        let mut state = state.borrow_mut();
        let offset = addr.floor().clamp(0.0, u16::MAX as f64) as u16;
        let byte = value.floor().clamp(0.0, u8::MAX as f64) as u8;
        let addr = qb_absolute_address(state.current_segment, offset);
        if addr < state.memory.len() {
            state.memory[addr] = byte;
        }
    });
}

fn qb_wait(addr: f64, and_mask: f64, xor_mask: Option<f64>) {
    let offset = addr.floor().clamp(0.0, u16::MAX as f64) as u16;
    let and_mask = and_mask.floor().clamp(0.0, u8::MAX as f64) as u8;
    let xor_mask = xor_mask.unwrap_or(0.0).floor().clamp(0.0, u8::MAX as f64) as u8;
    loop {
        let matched = QB_PSEUDO_MEMORY.with(|state| {
            let state = state.borrow();
            let addr = qb_absolute_address(state.current_segment, offset);
            let value = state.memory.get(addr).copied().unwrap_or(0);
            ((value ^ xor_mask) & and_mask) != 0
        });
        if matched {
            break;
        }
        std::thread::yield_now();
    }
}

fn qb_bload(path: &str, offset: Option<f64>) {
    let data = std::fs::read(path)
        .unwrap_or_else(|err| qb_runtime_fail(format!("BLOAD failed: {}", err)));
    let base = offset.unwrap_or(0.0).floor().clamp(0.0, u16::MAX as f64) as u16;
    QB_PSEUDO_MEMORY.with(|state| {
        let mut state = state.borrow_mut();
        for (i, byte) in data.iter().enumerate() {
            let addr = qb_absolute_address(
                state.current_segment,
                base.saturating_add(i.min(u16::MAX as usize) as u16),
            );
            if addr < state.memory.len() {
                state.memory[addr] = *byte;
            }
        }
    });
}

fn qb_bsave(path: &str, offset: f64, length: f64) {
    let base = offset.floor().clamp(0.0, u16::MAX as f64) as u16;
    let length = length.max(0.0) as usize;
    let data = QB_PSEUDO_MEMORY.with(|state| {
        let state = state.borrow();
        let mut data = Vec::with_capacity(length);
        for i in 0..length {
            let addr = qb_absolute_address(
                state.current_segment,
                base.saturating_add(i.min(u16::MAX as usize) as u16),
            );
            data.push(state.memory.get(addr).copied().unwrap_or(0));
        }
        data
    });
    std::fs::write(path, data)
        .unwrap_or_else(|err| qb_runtime_fail(format!("BSAVE failed: {}", err)));
}

fn qb_inp(port: f64) -> f64 {
    let port = port.floor().clamp(0.0, u16::MAX as f64) as u16;
    QB_PORTS.with(|ports| ports.borrow().get(&port).copied().unwrap_or(0) as f64)
}

fn qb_out(port: f64, value: f64) {
    let port = port.floor().clamp(0.0, u16::MAX as f64) as u16;
    let value = value.floor().clamp(0.0, u8::MAX as f64) as u8;
    QB_PORTS.with(|ports| {
        ports.borrow_mut().insert(port, value);
    });
}

fn qb_varptr<T: QBPseudoValueBytes>(name: &str, value: T) -> f64 {
    let (_segment, offset) = qb_ensure_pseudo_slot(name, value);
    offset as f64
}

fn qb_varseg<T: QBPseudoValueBytes>(name: &str, value: T) -> f64 {
    let (segment, _offset) = qb_ensure_pseudo_slot(name, value);
    segment as f64
}

fn qb_sadd<T: QBPseudoValueBytes>(name: &str, value: T) -> f64 {
    let (segment, offset) = qb_ensure_pseudo_slot(name, value);
    qb_absolute_address(segment, offset) as f64
}

fn qb_varptr_str<T: QBPseudoValueBytes>(name: &str, value: T) -> String {
    let (segment, offset) = qb_ensure_pseudo_slot(name, value);
    let address = qb_absolute_address(segment, offset) as u32;
    address
        .to_le_bytes()
        .iter()
        .map(|byte| *byte as char)
        .collect()
}
fn qb_timer() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() % 86_400) as f64 + d.subsec_nanos() as f64 / 1_000_000_000.0)
        .unwrap_or(0.0)
}
#[cfg(windows)]
#[repr(C)]
struct QBSystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetLocalTime(system_time: *mut QBSystemTime);
}

#[cfg(windows)]
fn qb_local_time() -> QBSystemTime {
    let mut time = QBSystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    unsafe {
        GetLocalTime(&mut time as *mut QBSystemTime);
    }
    time
}

#[cfg(windows)]
fn qb_date() -> String {
    let now = qb_local_time();
    format!("{:02}-{:02}-{:04}", now.month, now.day, now.year)
}

#[cfg(not(windows))]
fn qb_date() -> String {
    std::process::Command::new("date")
        .arg("+%m-%d-%Y")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "01-01-1980".to_string())
}

#[cfg(windows)]
fn qb_time() -> String {
    let now = qb_local_time();
    format!("{:02}:{:02}:{:02}", now.hour, now.minute, now.second)
}

#[cfg(not(windows))]
fn qb_time() -> String {
    std::process::Command::new("date")
        .arg("+%H:%M:%S")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "00:00:00".to_string())
}
fn qb_command() -> String {
    std::env::var("QBNEX_COMMAND_LINE")
        .unwrap_or_else(|_| std::env::args().skip(1).collect::<Vec<_>>().join(" "))
}
fn qb_environ(key: &str) -> String { std::env::var(key).unwrap_or_default() }
fn qb_sorted_environ_entries() -> Vec<String> {
    let mut entries = std::env::vars()
        .map(|(name, value)| format!("{}={}", name, value))
        .collect::<Vec<_>>();
    entries.sort();
    entries
}
fn qb_environ_index(index: f64) -> String {
    if index < 1.0 {
        return String::new();
    }
    qb_sorted_environ_entries()
        .get(index.round() as usize - 1)
        .cloned()
        .unwrap_or_default()
}
fn qb_csrlin() -> f64 {
    QB_CURSOR_STATE.with(|state| state.borrow().0 as f64)
}
fn qb_pos(_dummy: f64) -> f64 {
    QB_CURSOR_STATE.with(|state| state.borrow().1 as f64)
}
fn qb_lpos(_dummy: f64) -> f64 {
    QB_PRINTER_COL.with(|col| col.get() as f64)
}
thread_local! {
    static QB_PLAY_NOTE_DEADLINES: std::cell::RefCell<Vec<std::time::Instant>> =
        std::cell::RefCell::new(Vec::new());
    static QB_PLAY_HANDLER_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static QB_CURRENT_LINE: std::cell::Cell<i16> = const { std::cell::Cell::new(0) };
    static QB_TRACE_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static QB_KEY_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static QB_KEY_ASSIGNMENTS: std::cell::RefCell<std::collections::BTreeMap<i16, String>> =
        std::cell::RefCell::new(std::collections::BTreeMap::new());
}
fn qb_set_play_handler_active(active: bool) {
    QB_PLAY_HANDLER_ACTIVE.with(|state| state.set(active));
}
fn qb_set_current_line(line: u16) {
    QB_CURRENT_LINE.with(|state| state.set(line as i16));
    QB_TRACE_ENABLED.with(|trace| {
        if trace.get() {
            println!("[{}]", line);
        }
    });
}
fn qb_tron() {
    QB_TRACE_ENABLED.with(|trace| trace.set(true));
}
fn qb_troff() {
    QB_TRACE_ENABLED.with(|trace| trace.set(false));
}
fn qb_key_set(key_num: f64, key_string: &str) {
    QB_KEY_ASSIGNMENTS.with(|map| {
        map.borrow_mut().insert(key_num.round() as i16, key_string.to_string());
    });
}
fn qb_key_on() {
    QB_KEY_ENABLED.with(|state| state.set(true));
}
fn qb_key_off() {
    QB_KEY_ENABLED.with(|state| state.set(false));
}
fn qb_format_key_assignment(text: &str) -> String {
    let mut output = String::new();
    for ch in text.chars() {
        match ch {
            '\r' => output.push_str("<CR>"),
            '\n' => output.push_str("<LF>"),
            '\t' => output.push_str("<TAB>"),
            '\0' => output.push_str("<NUL>"),
            _ => output.push(ch),
        }
    }
    output
}
fn qb_key_list() {
    if !QB_KEY_ENABLED.with(|state| state.get()) {
        return;
    }
    QB_KEY_ASSIGNMENTS.with(|map| {
        for (key, value) in map.borrow().iter() {
            println!("F{} {}", key, qb_format_key_assignment(value));
        }
    });
}
fn qb_noninteractive_inkey() -> String {
    QB_NONINTERACTIVE_INKEY_EMITTED.with(|flag| {
        let mut flag = flag.borrow_mut();
        if *flag {
            String::new()
        } else {
            *flag = true;
            "\r".to_string()
        }
    })
}
#[cfg(windows)]
fn qb_inkey() -> String {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return qb_noninteractive_inkey();
    }

    unsafe extern "C" {
        fn _kbhit() -> i32;
        fn _getch() -> i32;
    }

    unsafe {
        if _kbhit() == 0 {
            return String::new();
        }
        let ch = _getch();
        if ch == 0 || ch == 224 {
            let ext = _getch();
            let mut text = String::with_capacity(2);
            text.push('\0');
            text.push(char::from(ext.clamp(0, u8::MAX as i32) as u8));
            text
        } else {
            char::from(ch.clamp(0, u8::MAX as i32) as u8).to_string()
        }
    }
}
#[cfg(not(windows))]
fn qb_inkey() -> String {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        qb_noninteractive_inkey()
    } else {
        String::new()
    }
}
#[derive(Debug)]
struct QBRuntimeError {
    code: i16,
    message: String,
}
#[derive(Debug, Clone, Default)]
struct QBRuntimeErrorState {
    code: i16,
    line: i16,
    device_code: i16,
    device_message: String,
}
thread_local! {
    static QB_ERROR_STATE: std::cell::RefCell<QBRuntimeErrorState> =
        std::cell::RefCell::new(QBRuntimeErrorState::default());
}
fn qb_set_error_state(code: i16, line: i16, message: &str) {
    QB_ERROR_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.code = code;
        state.line = line;
        state.device_code = 0;
        state.device_message = message.to_string();
    });
}
fn qb_clear_error_state() {
    QB_ERROR_STATE.with(|state| *state.borrow_mut() = QBRuntimeErrorState::default());
}
fn qb_err() -> f64 {
    QB_ERROR_STATE.with(|state| state.borrow().code as f64)
}
fn qb_erl() -> f64 {
    QB_ERROR_STATE.with(|state| state.borrow().line as f64)
}
fn qb_erdev() -> f64 {
    QB_ERROR_STATE.with(|state| state.borrow().device_code as f64)
}
fn qb_erdev_str() -> String {
    QB_ERROR_STATE.with(|state| state.borrow().device_message.clone())
}
fn qb_runtime_raise(code: i16, message: impl Into<String>) -> ! {
    std::panic::panic_any(QBRuntimeError {
        code,
        message: message.into(),
    });
}
fn qb_runtime_fail_code(code: i16, message: impl Into<String>) -> ! {
    qb_runtime_raise(code, message);
}
fn qb_runtime_fail(message: impl AsRef<str>) -> ! {
    qb_runtime_fail_code(255, message.as_ref().to_string());
}
fn qb_take_runtime_error(
    panic: Box<dyn std::any::Any + Send>,
) -> QBRuntimeError {
    match panic.downcast::<QBRuntimeError>() {
        Ok(error) => *error,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}
fn qb_report_unhandled_error(error: &QBRuntimeError) -> ! {
    eprintln!("{}", error.message);
    std::process::exit(1);
}
fn qb_install_panic_hook() {
    static QB_PANIC_HOOK: std::sync::Once = std::sync::Once::new();
    QB_PANIC_HOOK.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<QBRuntimeError>().is_some() {
                return;
            }
            default_hook(info);
        }));
    });
}
fn qb_clear(num_vars: &mut [f64], str_vars: &mut [String], arr_vars: &mut [Vec<f64>], str_arr_vars: &mut [Vec<String>]) {
    for value in num_vars.iter_mut() {
        *value = 0.0;
    }
    for value in str_vars.iter_mut() {
        value.clear();
    }
    for value in arr_vars.iter_mut() {
        value.clear();
    }
    for value in str_arr_vars.iter_mut() {
        value.clear();
    }
    qb_close_all();
}
fn qb_chdir(path: &str) {
    let _ = std::env::set_current_dir(path);
}
fn qb_mkdir(path: &str) {
    let _ = std::fs::create_dir_all(path);
}
fn qb_rmdir(path: &str) {
    let _ = std::fs::remove_dir(path);
}
fn qb_kill(path: &str) {
    let _ = std::fs::remove_file(path);
}
fn qb_rename(old_name: &str, new_name: &str) {
    let _ = std::fs::rename(old_name, new_name);
}
fn qb_normalize_dos_path(path: &str) -> std::path::PathBuf {
    let trimmed = path.trim().trim_matches('"');
    if trimmed.is_empty() {
        return std::path::PathBuf::from(".");
    }

    let mut normalized = std::path::PathBuf::new();
    let mut rest = trimmed;
    if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
        normalized.push(&trimmed[..2]);
        rest = &trimmed[2..];
    }

    for segment in rest.split(['\\', '/']) {
        if !segment.is_empty() {
            normalized.push(segment);
        }
    }

    if normalized.as_os_str().is_empty() {
        std::path::PathBuf::from(".")
    } else {
        normalized
    }
}
fn qb_files_query(pattern: Option<&str>) -> (std::path::PathBuf, Option<String>) {
    let Some(pattern) = pattern.map(str::trim).filter(|pattern| !pattern.is_empty()) else {
        return (std::path::PathBuf::from("."), None);
    };

    if pattern == "*" {
        return (std::path::PathBuf::from("."), None);
    }

    let normalized = qb_normalize_dos_path(pattern);
    let has_wildcards = pattern.contains('*') || pattern.contains('?');

    if has_wildcards {
        let mask = normalized
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|mask| !mask.is_empty())
            .unwrap_or("*")
            .to_string();
        let directory = normalized
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        return (directory, Some(mask));
    }

    if normalized.is_dir() {
        return (normalized, None);
    }

    let mask = normalized
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|mask| !mask.is_empty())
        .map(str::to_string);
    let directory = normalized
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    (directory, mask)
}
fn qb_files(pattern: Option<&str>) {
    let (directory, mask) = qb_files_query(pattern);
    if let Ok(entries) = std::fs::read_dir(directory) {
        let mut names = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                let matched = mask
                    .as_deref()
                    .is_none_or(|mask| qb_wildcard_match(mask, &name));
                matched.then_some(name)
            })
            .collect::<Vec<_>>();
        names.sort();
        for name in names {
            println!("{}", name);
        }
    }
}
fn qb_wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_ascii_uppercase().chars().collect::<Vec<_>>();
    let text = text.to_ascii_uppercase().chars().collect::<Vec<_>>();
    let mut dp = vec![vec![false; text.len() + 1]; pattern.len() + 1];
    dp[0][0] = true;

    for (index, ch) in pattern.iter().enumerate() {
        if *ch == '*' {
            dp[index + 1][0] = dp[index][0];
        }
    }

    for (i, pat) in pattern.iter().enumerate() {
        for (j, txt) in text.iter().enumerate() {
            dp[i + 1][j + 1] = match pat {
                '*' => dp[i][j + 1] || dp[i + 1][j] || dp[i][j],
                '?' => dp[i][j],
                ch => dp[i][j] && ch == txt,
            };
        }
    }

    dp[pattern.len()][text.len()]
}
fn qb_expand_dos_paths(pattern: &str) -> std::io::Result<Vec<std::path::PathBuf>> {
    let trimmed = pattern.trim();
    if trimmed.contains('*') || trimmed.contains('?') {
        let (directory, mask) = qb_files_query(Some(trimmed));
        let Some(mask) = mask else {
            return Ok(vec![directory]);
        };
        let mut matches = Vec::new();
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if qb_wildcard_match(&mask, &name) {
                matches.push(entry.path());
            }
        }
        matches.sort();
        Ok(matches)
    } else {
        Ok(vec![qb_normalize_dos_path(trimmed)])
    }
}
fn qb_shell_split_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for ch in command.chars() {
        match quote {
            Some(active) if ch == active => quote = None,
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}
fn qb_try_execute_dos_shell_builtin(command: &str) -> Result<Option<String>, String> {
    let words = qb_shell_split_words(command);
    let Some(head) = words.first() else {
        return Ok(Some(String::new()));
    };

    let verb = head.to_ascii_uppercase();
    match verb.as_str() {
        "CLS" => Ok(Some(String::new())),
        "ECHO" => Ok(Some(format!(
            "{}\n",
            words.iter().skip(1).cloned().collect::<Vec<_>>().join(" ")
        ))),
        "DIR" => {
            let pattern = words
                .iter()
                .skip(1)
                .find(|arg| !arg.starts_with('/'))
                .map(String::as_str);
            let (directory, mask) = qb_files_query(pattern);
            let mut names = std::fs::read_dir(directory)
                .map_err(|err| err.to_string())?
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let matched = mask
                        .as_deref()
                        .is_none_or(|mask| qb_wildcard_match(mask, &name));
                    matched.then_some(name)
                })
                .collect::<Vec<_>>();
            names.sort();
            Ok(Some(names.join("\n") + if names.is_empty() { "" } else { "\n" }))
        }
        "TYPE" => {
            let Some(path) = words.get(1) else {
                return Err("TYPE expects a file path".to_string());
            };
            std::fs::read_to_string(qb_normalize_dos_path(path))
                .map(Some)
                .map_err(|err| err.to_string())
        }
        "COPY" => {
            let Some(source) = words.get(1) else {
                return Err("COPY expects a source path".to_string());
            };
            let Some(destination) = words.get(2) else {
                return Err("COPY expects a destination path".to_string());
            };
            std::fs::copy(qb_normalize_dos_path(source), qb_normalize_dos_path(destination))
                .map_err(|err| err.to_string())?;
            Ok(Some(String::new()))
        }
        "REN" | "RENAME" => {
            let Some(source) = words.get(1) else {
                return Err("REN expects a source path".to_string());
            };
            let Some(destination) = words.get(2) else {
                return Err("REN expects a destination path".to_string());
            };
            std::fs::rename(qb_normalize_dos_path(source), qb_normalize_dos_path(destination))
                .map_err(|err| err.to_string())?;
            Ok(Some(String::new()))
        }
        "DEL" | "ERASE" => {
            for path in words.iter().skip(1) {
                for expanded in qb_expand_dos_paths(path).map_err(|err| err.to_string())? {
                    if expanded.is_file() {
                        std::fs::remove_file(expanded).map_err(|err| err.to_string())?;
                    }
                }
            }
            Ok(Some(String::new()))
        }
        "MD" | "MKDIR" => {
            for path in words.iter().skip(1) {
                std::fs::create_dir_all(qb_normalize_dos_path(path))
                    .map_err(|err| err.to_string())?;
            }
            Ok(Some(String::new()))
        }
        "RD" | "RMDIR" => {
            for path in words.iter().skip(1) {
                std::fs::remove_dir(qb_normalize_dos_path(path)).map_err(|err| err.to_string())?;
            }
            Ok(Some(String::new()))
        }
        "CD" | "CHDIR" => {
            if words.len() == 1 {
                return Ok(Some(format!(
                    "{}\n",
                    std::env::current_dir()
                        .map_err(|err| err.to_string())?
                        .display()
                )));
            }
            let path = qb_normalize_dos_path(&words[1]);
            if path.is_dir() {
                Ok(Some(String::new()))
            } else {
                Err(format!("directory not found: {}", path.display()))
            }
        }
        _ => Ok(None),
    }
}
#[derive(Debug)]
enum QBFileMode {
    Input,
    Output,
    Append,
    Binary,
    Random,
}
#[derive(Debug)]
struct QBFileHandle {
    file: std::fs::File,
    mode: QBFileMode,
}
thread_local! {
    static QB_FILES: std::cell::RefCell<std::collections::HashMap<i32, QBFileHandle>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    static QB_RANDOM_FIELDS: std::cell::RefCell<std::collections::HashMap<i32, Vec<(usize, usize)>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    static QB_FILE_PRINT_COLS: std::cell::RefCell<std::collections::HashMap<i32, i32>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}
fn qb_open(path: &str, mode: &str, file_number: f64) {
    let file_number = file_number as i32;
    let mode_upper = mode.to_ascii_uppercase();
    let mut options = std::fs::OpenOptions::new();
    let file_mode = match mode_upper.as_str() {
        "INPUT" => {
            options.read(true);
            QBFileMode::Input
        }
        "OUTPUT" => {
            options.write(true).create(true).truncate(true);
            QBFileMode::Output
        }
        "APPEND" => {
            options.append(true).create(true).read(true);
            QBFileMode::Append
        }
        "RANDOM" => {
            options.read(true).write(true).create(true);
            QBFileMode::Random
        }
        _ => {
            options.read(true).write(true).create(true);
            QBFileMode::Binary
        }
    };
    let file = options
        .open(path)
        .unwrap_or_else(|err| qb_runtime_fail(format!("file open failed for '{}': {}", path, err)));
    QB_FILES.with(|files| {
        files.borrow_mut().insert(
            file_number,
            QBFileHandle {
                file,
                mode: file_mode,
            },
        );
    });
    QB_FILE_PRINT_COLS.with(|cols| {
        cols.borrow_mut().insert(file_number, 1);
    });
}
fn qb_close(file_number: f64) {
    let removed = QB_FILES.with(|files| files.borrow_mut().remove(&(file_number as i32)));
    if removed.is_none() {
        qb_runtime_fail(format!("close failed: file #{} is not open", file_number as i32));
    }
    QB_RANDOM_FIELDS.with(|fields| {
        fields.borrow_mut().remove(&(file_number as i32));
    });
    QB_FILE_PRINT_COLS.with(|cols| {
        cols.borrow_mut().remove(&(file_number as i32));
    });
}
fn qb_close_all() {
    QB_FILES.with(|files| files.borrow_mut().clear());
    QB_RANDOM_FIELDS.with(|fields| fields.borrow_mut().clear());
    QB_FILE_PRINT_COLS.with(|cols| cols.borrow_mut().clear());
}
fn qb_freefile() -> f64 {
    QB_FILES.with(|files| {
        let files = files.borrow();
        let mut file_number = 1;
        while files.contains_key(&file_number) {
            file_number += 1;
        }
        file_number as f64
    })
}
fn qb_lof(file_number: f64) -> f64 {
    QB_FILES.with(|files| {
        files.borrow_mut()
            .get_mut(&(file_number as i32))
            .map(|handle| {
                handle
                    .file
                    .metadata()
                    .unwrap_or_else(|err| {
                        qb_runtime_fail(format!(
                            "LOF failed for file #{}: {}",
                            file_number as i32,
                            err
                        ))
                    })
                    .len() as f64
            })
            .unwrap_or_else(|| qb_runtime_fail(format!("LOF failed: file #{} is not open", file_number as i32)))
    })
}
fn qb_loc(file_number: f64) -> f64 {
    QB_FILES.with(|files| {
        files.borrow_mut()
            .get_mut(&(file_number as i32))
            .map(|handle| {
                handle
                    .file
                    .stream_position()
                    .unwrap_or_else(|err| {
                        qb_runtime_fail(format!(
                            "LOC failed for file #{}: {}",
                            file_number as i32,
                            err
                        ))
                    }) as f64
                    + 1.0
            })
            .unwrap_or_else(|| qb_runtime_fail(format!("LOC failed: file #{} is not open", file_number as i32)))
    })
}
fn qb_eof(file_number: f64) -> f64 {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!("EOF failed: file #{} is not open", file_number as i32));
        };
        let position = handle.file.stream_position().unwrap_or_else(|err| {
            qb_runtime_fail(format!("EOF failed for file #{}: {}", file_number as i32, err))
        });
        let length = handle.file.metadata().map(|metadata| metadata.len()).unwrap_or_else(|err| {
            qb_runtime_fail(format!("EOF failed for file #{}: {}", file_number as i32, err))
        });
        if position >= length {
            -1.0
        } else {
            0.0
        }
    })
}
fn qb_seek(file_number: f64, position: f64) {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!("SEEK failed: file #{} is not open", file_number as i32));
        };
        let offset = position.max(1.0) as u64 - 1;
        handle
            .file
            .seek(SeekFrom::Start(offset))
            .unwrap_or_else(|err| {
                qb_runtime_fail(format!("SEEK failed for file #{}: {}", file_number as i32, err))
            });
    });
    QB_FILE_PRINT_COLS.with(|cols| {
        cols.borrow_mut().remove(&(file_number as i32));
    });
}
fn qb_define_fields(file_number: f64, fields: Vec<(usize, usize)>) {
    QB_RANDOM_FIELDS.with(|map| {
        map.borrow_mut().insert(file_number as i32, fields);
    });
}
fn qb_lset_field(str_vars: &mut [String], var_index: usize, width: usize, value: &str) {
    if var_index >= str_vars.len() {
        return;
    }
    let mut text = value.to_string();
    if text.len() > width {
        text.truncate(width);
    } else if text.len() < width {
        text.push_str(&" ".repeat(width - text.len()));
    }
    str_vars[var_index] = text;
}
fn qb_rset_field(str_vars: &mut [String], var_index: usize, width: usize, value: &str) {
    if var_index >= str_vars.len() {
        return;
    }
    let text = value.to_string();
    let formatted = if text.len() >= width {
        text[text.len() - width..].to_string()
    } else {
        format!("{}{}", " ".repeat(width - text.len()), text)
    };
    str_vars[var_index] = formatted;
}
fn qb_random_record_len(file_number: i32) -> Option<usize> {
    QB_RANDOM_FIELDS.with(|map| {
        map.borrow()
            .get(&file_number)
            .map(|fields| fields.iter().map(|(width, _)| *width).sum())
    })
}
fn qb_get_record_to_fields(file_number: f64, record_number: f64, str_vars: &mut [String]) {
    let file_number = file_number as i32;
    let Some((fields, record_len)) = QB_RANDOM_FIELDS.with(|map| {
        map.borrow()
            .get(&file_number)
            .cloned()
            .map(|fields| {
                let record_len = fields.iter().map(|(width, _)| *width).sum();
                (fields, record_len)
            })
    }) else {
        qb_runtime_fail(format!("GET failed: no FIELD layout defined for file #{}", file_number));
    };
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&file_number) else {
            qb_runtime_fail(format!("GET failed: file #{} is not open", file_number));
        };
        let offset = (record_number.max(1.0) as u64 - 1) * record_len as u64;
        handle.file.seek(SeekFrom::Start(offset)).unwrap_or_else(|err| {
            qb_runtime_fail(format!("GET failed for file #{}: {}", file_number, err))
        });
        let mut buffer = vec![0u8; record_len];
        let read = handle.file.read(&mut buffer).unwrap_or_else(|err| {
            qb_runtime_fail(format!("GET failed for file #{}: {}", file_number, err))
        });
        if read < buffer.len() {
            buffer[read..].fill(b' ');
        }
        let mut start = 0usize;
        for (width, var_index) in fields {
            let end = start + width;
            if var_index < str_vars.len() {
                str_vars[var_index] = String::from_utf8_lossy(&buffer[start..end]).to_string();
            }
            start = end;
        }
    });
}
fn qb_put_record_from_fields(file_number: f64, record_number: f64, str_vars: &[String]) {
    let file_number = file_number as i32;
    let Some((fields, record_len)) = QB_RANDOM_FIELDS.with(|map| {
        map.borrow()
            .get(&file_number)
            .cloned()
            .map(|fields| {
                let record_len = fields.iter().map(|(width, _)| *width).sum();
                (fields, record_len)
            })
    }) else {
        qb_runtime_fail(format!("PUT failed: no FIELD layout defined for file #{}", file_number));
    };
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&file_number) else {
            qb_runtime_fail(format!("PUT failed: file #{} is not open", file_number));
        };
        let offset = (record_number.max(1.0) as u64 - 1) * record_len as u64;
        handle.file.seek(SeekFrom::Start(offset)).unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number, err))
        });
        let mut buffer = Vec::with_capacity(record_len);
        for (width, var_index) in fields {
            let value = str_vars.get(var_index).cloned().unwrap_or_default();
            let mut text = value;
            if text.len() > width {
                text.truncate(width);
            } else if text.len() < width {
                text.push_str(&" ".repeat(width - text.len()));
            }
            buffer.extend_from_slice(text.as_bytes());
        }
        handle.file.write_all(&buffer).unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number, err))
        });
        handle.file.flush().unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number, err))
        });
    });
}
fn qb_get_string_from_file(file_number: f64, record_number: f64, size_hint: f64) -> String {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!("GET failed: file #{} is not open", file_number as i32));
        };
        let offset = if record_number > 0.0 {
            record_number as u64 - 1
        } else {
            handle.file.stream_position().unwrap_or_else(|err| {
                qb_runtime_fail(format!("GET failed for file #{}: {}", file_number as i32, err))
            })
        };
        handle.file.seek(SeekFrom::Start(offset)).unwrap_or_else(|err| {
            qb_runtime_fail(format!("GET failed for file #{}: {}", file_number as i32, err))
        });
        let read_len = if size_hint > 0.0 {
            size_hint as usize
        } else {
            let end = handle.file.metadata().map(|m| m.len()).unwrap_or_else(|err| {
                qb_runtime_fail(format!("GET failed for file #{}: {}", file_number as i32, err))
            });
            end.saturating_sub(offset) as usize
        };
        let mut buffer = vec![0u8; read_len];
        let read = handle.file.read(&mut buffer).unwrap_or_else(|err| {
            qb_runtime_fail(format!("GET failed for file #{}: {}", file_number as i32, err))
        });
        buffer.truncate(read);
        String::from_utf8_lossy(&buffer).to_string()
    })
}
fn qb_get_bytes(file_number: f64, record_number: f64, size: usize) -> Vec<u8> {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!("GET failed: file #{} is not open", file_number as i32));
        };
        let offset = if record_number > 0.0 {
            record_number as u64 - 1
        } else {
            handle.file.stream_position().unwrap_or_else(|err| {
                qb_runtime_fail(format!("GET failed for file #{}: {}", file_number as i32, err))
            })
        };
        handle.file.seek(SeekFrom::Start(offset)).unwrap_or_else(|err| {
            qb_runtime_fail(format!("GET failed for file #{}: {}", file_number as i32, err))
        });
        let mut buffer = vec![0u8; size];
        let read = handle.file.read(&mut buffer).unwrap_or_else(|err| {
            qb_runtime_fail(format!("GET failed for file #{}: {}", file_number as i32, err))
        });
        if read < buffer.len() {
            buffer[read..].fill(0);
        }
        buffer
    })
}
fn qb_get_i16(file_number: f64, record_number: f64) -> f64 {
    let bytes = qb_get_bytes(file_number, record_number, 2);
    i16::from_le_bytes([bytes[0], bytes[1]]) as f64
}
fn qb_get_i32(file_number: f64, record_number: f64) -> f64 {
    let bytes = qb_get_bytes(file_number, record_number, 4);
    i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
}
fn qb_get_f32(file_number: f64, record_number: f64) -> f64 {
    let bytes = qb_get_bytes(file_number, record_number, 4);
    f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
}
fn qb_get_f64(file_number: f64, record_number: f64) -> f64 {
    let bytes = qb_get_bytes(file_number, record_number, 8);
    f64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}
fn qb_put_string(file_number: f64, record_number: f64, value: &str) {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!("PUT failed: file #{} is not open", file_number as i32));
        };
        let offset = if record_number > 0.0 {
            record_number as u64 - 1
        } else {
            handle.file.stream_position().unwrap_or_else(|err| {
                qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
            })
        };
        handle.file.seek(SeekFrom::Start(offset)).unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
        });
        handle.file.write_all(value.as_bytes()).unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
        });
        handle.file.flush().unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
        });
    });
}
fn qb_put_fixed_string(file_number: f64, record_number: f64, width: usize, value: &str) {
    let mut text = value.to_string();
    if text.len() > width {
        text.truncate(width);
    } else if text.len() < width {
        text.push_str(&" ".repeat(width - text.len()));
    }
    qb_put_string(file_number, record_number, &text);
}
fn qb_put_bytes(file_number: f64, record_number: f64, bytes: &[u8]) {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!("PUT failed: file #{} is not open", file_number as i32));
        };
        let offset = if record_number > 0.0 {
            record_number as u64 - 1
        } else {
            handle.file.stream_position().unwrap_or_else(|err| {
                qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
            })
        };
        handle.file.seek(SeekFrom::Start(offset)).unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
        });
        handle.file.write_all(bytes).unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
        });
        handle.file.flush().unwrap_or_else(|err| {
            qb_runtime_fail(format!("PUT failed for file #{}: {}", file_number as i32, err))
        });
    });
}
fn qb_put_i16(file_number: f64, record_number: f64, value: f64) {
    qb_put_bytes(file_number, record_number, &(value as i16).to_le_bytes());
}
fn qb_put_i32(file_number: f64, record_number: f64, value: f64) {
    qb_put_bytes(file_number, record_number, &(value as i32).to_le_bytes());
}
fn qb_put_f32(file_number: f64, record_number: f64, value: f64) {
    qb_put_bytes(file_number, record_number, &(value as f32).to_le_bytes());
}
fn qb_put_f64(file_number: f64, record_number: f64, value: f64) {
    qb_put_bytes(file_number, record_number, &value.to_le_bytes());
}
fn qb_read_line_from_file(file_number: f64) -> String {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!(
                "LINE INPUT # failed: file #{} is not open",
                file_number as i32
            ));
        };
        let mut bytes = Vec::new();
        let mut single = [0u8; 1];
        loop {
            match handle.file.read(&mut single) {
                Ok(0) => break,
                Ok(_) => {
                    if single[0] == b'\n' {
                        break;
                    }
                    if single[0] != b'\r' {
                        bytes.push(single[0]);
                    }
                }
                Err(err) => {
                    qb_runtime_fail(format!(
                        "LINE INPUT # failed for file #{}: {}",
                        file_number as i32,
                        err
                    ))
                }
            }
        }
        String::from_utf8_lossy(&bytes).to_string()
    })
}
fn qb_read_chars_from_file(file_number: f64, count: f64) -> String {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!(
                "INPUT$ failed: file #{} is not open",
                file_number as i32
            ));
        };
        let mut buffer = vec![0u8; count.max(0.0) as usize];
        let read = handle.file.read(&mut buffer).unwrap_or_else(|err| {
            qb_runtime_fail(format!("INPUT$ failed for file #{}: {}", file_number as i32, err))
        });
        buffer.truncate(read);
        String::from_utf8_lossy(&buffer).to_string()
    })
}
fn qb_input_str(count: f64, file_number: Option<f64>) -> String {
    if let Some(file_number) = file_number {
        return qb_read_chars_from_file(file_number, count);
    }
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    input.chars().take(count.max(0.0) as usize).collect()
}

fn qb_advance_file_print_column(file_number: i32, text: &str) {
    QB_FILE_PRINT_COLS.with(|cols| {
        let mut cols = cols.borrow_mut();
        let col = cols.entry(file_number).or_insert(1);
        for ch in text.chars() {
            match ch {
                '\n' | '\r' => *col = 1,
                _ => *col += 1,
            }
        }
    });
}

fn qb_file_write(file_number: f64, text: &str, newline: bool) {
    QB_FILES.with(|files| {
        let mut files = files.borrow_mut();
        let Some(handle) = files.get_mut(&(file_number as i32)) else {
            qb_runtime_fail(format!("write failed: file #{} is not open", file_number as i32));
        };
        match handle.mode {
            QBFileMode::Input => qb_runtime_fail(format!(
                "write failed: file #{} is opened for INPUT",
                file_number as i32
            )),
            _ => {
                if newline {
                    writeln!(handle.file, "{}", text)
                } else {
                    write!(handle.file, "{}", text)
                }
                .unwrap_or_else(|err| {
                    qb_runtime_fail(format!(
                        "write failed for file #{}: {}",
                        file_number as i32,
                        err
                    ))
                });
                handle.file.flush().unwrap_or_else(|err| {
                    qb_runtime_fail(format!(
                        "flush failed for file #{}: {}",
                        file_number as i32,
                        err
                    ))
                });
                qb_advance_file_print_column(file_number as i32, text);
                if newline {
                    qb_advance_file_print_column(file_number as i32, "\n");
                }
            }
        };
    });
}
fn qb_file_print_newline(file_number: f64) {
    qb_file_write(file_number, "", true);
}
fn qb_file_print_comma(file_number: f64) {
    let current_col = QB_FILE_PRINT_COLS.with(|cols| {
        cols.borrow()
            .get(&(file_number as i32))
            .copied()
            .unwrap_or(1)
    });
    if let Some(target_col) = qb_next_print_zone(current_col, 80) {
        let spaces = (target_col - current_col).max(0) as usize;
        if spaces > 0 {
            qb_file_write(file_number, &" ".repeat(spaces), false);
        }
    } else {
        qb_file_print_newline(file_number);
    }
}
fn qb_file_print_using(
    file_number: f64,
    pattern: &str,
    values: &[String],
    comma_after: &[bool],
    newline: bool,
) {
    let chunks = qb_format_using_values(pattern, values);
    for (index, chunk) in chunks.iter().enumerate() {
        qb_file_write(file_number, chunk, false);
        if comma_after.get(index).copied().unwrap_or(false) {
            qb_file_print_comma(file_number);
        }
    }
    if newline {
        qb_file_print_newline(file_number);
    }
}
fn qb_file_write_csv(file_number: f64, fields: &[String]) {
    qb_file_write(file_number, &fields.join(","), true);
}
fn qb_insert_commas(int_part: &str) -> String {
    let chars: Vec<char> = int_part.chars().collect();
    let mut out = String::new();
    for (idx, ch) in chars.iter().enumerate() {
        if idx > 0 && (chars.len() - idx) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    out
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum QbUsingStringMode {
    FirstChar,
    Whole,
    FixedWidth(usize),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum QbUsingPrefixKind {
    None,
    Stars,
    Dollars,
    StarDollars,
}
#[derive(Clone)]
struct QbUsingField {
    core: String,
    suffix: String,
}
fn qb_decode_using_pattern(pattern: &str) -> Vec<(char, bool)> {
    let mut decoded = Vec::new();
    let mut chars = pattern.chars();
    while let Some(ch) = chars.next() {
        if ch == '_' {
            if let Some(next) = chars.next() {
                decoded.push((next, true));
            } else {
                decoded.push(('_', true));
            }
        } else {
            decoded.push((ch, false));
        }
    }
    decoded
}
fn qb_collect_using_chars(slice: &[(char, bool)]) -> String {
    slice.iter().map(|(ch, _)| *ch).collect()
}
fn qb_find_using_string_field(decoded: &[(char, bool)]) -> Option<(usize, usize, QbUsingStringMode)> {
    for (index, (ch, literal)) in decoded.iter().enumerate() {
        if *literal {
            continue;
        }
        match ch {
            '!' => return Some((index, index, QbUsingStringMode::FirstChar)),
            '&' => return Some((index, index, QbUsingStringMode::Whole)),
            '\\' => {
                for end in index + 1..decoded.len() {
                    if !decoded[end].1 && decoded[end].0 == '\\' {
                        return Some((index, end, QbUsingStringMode::FixedWidth(end - index + 1)));
                    }
                }
            }
            _ => {}
        }
    }
    None
}
fn qb_find_using_numeric_span(decoded: &[(char, bool)]) -> Option<(usize, usize)> {
    let mut cursor = 0;
    while let Some((start, end)) = qb_next_using_field_span(decoded, cursor) {
        if matches!(decoded[start].0, '#' | '.' | '+' | '-' | '*' | '$') {
            return Some((start, end));
        }
        cursor = end + 1;
    }
    None
}
fn qb_using_field_span_at(decoded: &[(char, bool)], start: usize) -> Option<(usize, usize)> {
    let Some((ch, literal)) = decoded.get(start).copied() else {
        return None;
    };
    if literal {
        return None;
    }
    match ch {
        '!' | '&' => Some((start, start)),
        '\\' => {
            for end in start + 1..decoded.len() {
                if !decoded[end].1 && decoded[end].0 == '\\' {
                    return Some((start, end));
                }
            }
            None
        }
        '#' | '.' | '+' | '-' | '*' | '$' => {
            let mut end = start;
            while end + 1 < decoded.len()
                && !decoded[end + 1].1
                && matches!(decoded[end + 1].0, '#' | '.' | '+' | '-' | '*' | '$' | ',' | '^')
            {
                end += 1;
            }
            decoded[start..=end]
                .iter()
                .any(|(ch, literal)| !*literal && *ch == '#')
                .then_some((start, end))
        }
        _ => None,
    }
}
fn qb_next_using_field_span(decoded: &[(char, bool)], mut start: usize) -> Option<(usize, usize)> {
    while start < decoded.len() {
        if let Some(span) = qb_using_field_span_at(decoded, start) {
            return Some(span);
        }
        start += 1;
    }
    None
}
fn qb_parse_using_fields(pattern: &str) -> (String, Vec<QbUsingField>) {
    let decoded = qb_decode_using_pattern(pattern);
    let mut cursor = 0;
    let mut leading = String::new();
    let mut fields: Vec<QbUsingField> = Vec::new();

    while let Some((start, end)) = qb_next_using_field_span(&decoded, cursor) {
        let between = qb_collect_using_chars(&decoded[cursor..start]);
        if fields.is_empty() {
            leading.push_str(&between);
        } else if let Some(last) = fields.last_mut() {
            last.suffix.push_str(&between);
        }

        fields.push(QbUsingField {
            core: qb_collect_using_chars(&decoded[start..=end]),
            suffix: String::new(),
        });
        cursor = end + 1;
    }

    let trailing = qb_collect_using_chars(&decoded[cursor..]);
    if fields.is_empty() {
        leading.push_str(&trailing);
    } else if let Some(last) = fields.last_mut() {
        last.suffix.push_str(&trailing);
        if last.suffix.is_empty() && last.core.ends_with(',') {
            last.core.pop();
            last.suffix.push(',');
        }
    }

    (leading, fields)
}
fn qb_format_using_string_value(mode: QbUsingStringMode, value: &str) -> String {
    let text = value.trim_matches('"');
    match mode {
        QbUsingStringMode::FirstChar => text.chars().next().unwrap_or(' ').to_string(),
        QbUsingStringMode::Whole => text.to_string(),
        QbUsingStringMode::FixedWidth(width) => {
            let truncated = text.chars().take(width).collect::<String>();
            format!("{:<width$}", truncated, width = width)
        }
    }
}
fn qb_using_extra_integer_positions(prefix_kind: QbUsingPrefixKind) -> usize {
    match prefix_kind {
        QbUsingPrefixKind::None => 0,
        QbUsingPrefixKind::Stars => 2,
        QbUsingPrefixKind::Dollars => 1,
        QbUsingPrefixKind::StarDollars => 2,
    }
}
fn qb_scientific_digits(value: f64, significant_digits: usize) -> (String, i32) {
    if significant_digits == 0 {
        return (String::new(), 0);
    }
    if value.abs() < f64::EPSILON {
        return ("0".repeat(significant_digits), 0);
    }
    let mut exponent = value.abs().log10().floor() as i32;
    let scaled = value.abs() / 10f64.powi(exponent);
    let rounded = (scaled * 10f64.powi((significant_digits as i32) - 1)).round();
    let mut digits = rounded as i128;
    let limit = 10_i128.pow(significant_digits as u32);
    if digits >= limit {
        digits /= 10;
        exponent += 1;
    }
    (format!("{:0width$}", digits, width = significant_digits), exponent)
}
fn qb_compose_scientific_mantissa(
    digits: &str,
    digits_before_decimal: usize,
    digits_after_decimal: usize,
) -> String {
    if digits_before_decimal == 0 {
        return format!(".{}", digits);
    }
    let split = digits_before_decimal.min(digits.len());
    let left = &digits[..split];
    if digits_after_decimal == 0 {
        return left.to_string();
    }
    let right = &digits[split..];
    format!("{}.{}", left, right)
}
fn qb_format_using_fixed(
    core: &str,
    mantissa: &str,
    value: f64,
    leading_plus: bool,
    trailing_plus: bool,
    trailing_minus: bool,
    prefix_kind: QbUsingPrefixKind,
    target_width: usize,
) -> String {
    let parts: Vec<&str> = mantissa.splitn(2, '.').collect();
    let int_pattern = parts[0];
    let frac_pattern = parts.get(1).copied().unwrap_or("");
    let comma_slots = int_pattern.chars().filter(|ch| *ch == ',').count();
    let int_hashes = int_pattern.chars().filter(|ch| *ch == '#').count();
    let frac_hashes = frac_pattern.chars().filter(|ch| *ch == '#').count();
    let extra_integer_positions = qb_using_extra_integer_positions(prefix_kind);
    if int_hashes + frac_hashes + extra_integer_positions > 24 {
        qb_runtime_fail("Illegal function call".to_string());
    }

    let rounded = if frac_hashes > 0 {
        format!("{:.*}", frac_hashes, value.abs())
    } else {
        format!("{:.0}", value.abs())
    };
    let mut rounded_parts = rounded.split('.');
    let rounded_int = rounded_parts.next().unwrap_or("0");
    let rounded_frac = rounded_parts.next().unwrap_or("");

    let integer_capacity = int_hashes + extra_integer_positions + comma_slots;
    if integer_capacity == 0 && rounded_int != "0" {
        return format!("%{}", rounded);
    }

    let show_zero_before_decimal = int_hashes + extra_integer_positions > 0;
    let int_digits = if rounded_int == "0" && !show_zero_before_decimal {
        String::new()
    } else {
        rounded_int.to_string()
    };

    let grouped_int = if comma_slots > 0 {
        qb_insert_commas(&int_digits)
    } else {
        int_digits
    };
    if grouped_int.chars().count() > integer_capacity {
        return format!("%{}", rounded);
    }

    let mut number = grouped_int;
    if frac_hashes > 0 {
        if number.is_empty() {
            number.push('.');
            number.push_str(rounded_frac);
        } else {
            number.push('.');
            number.push_str(rounded_frac);
        }
    }

    let sign_prefix = if leading_plus {
        Some(if value.is_sign_negative() { '-' } else { '+' })
    } else if !trailing_plus && !trailing_minus && value.is_sign_negative() {
        Some('-')
    } else {
        None
    };
    let sign_suffix = if trailing_plus {
        Some(if value.is_sign_negative() { '-' } else { '+' })
    } else if trailing_minus && value.is_sign_negative() {
        Some('-')
    } else {
        None
    };

    let mut body = String::new();
    match prefix_kind {
        QbUsingPrefixKind::Dollars | QbUsingPrefixKind::StarDollars => {
            if let Some(sign) = sign_prefix {
                body.push(sign);
            }
            body.push('$');
            body.push_str(&number);
        }
        _ => {
            if let Some(sign) = sign_prefix {
                body.push(sign);
            }
            body.push_str(&number);
        }
    }

    let total_len = body.chars().count() + usize::from(sign_suffix.is_some());
    if total_len > target_width {
        let mut overflow = String::from("%");
        overflow.push_str(&body);
        if let Some(sign) = sign_suffix {
            overflow.push(sign);
        }
        return overflow;
    }

    let pad_char = match prefix_kind {
        QbUsingPrefixKind::Stars | QbUsingPrefixKind::StarDollars => '*',
        _ => ' ',
    };
    let mut output = String::new();
    output.push_str(&pad_char.to_string().repeat(target_width - total_len));
    output.push_str(&body);
    if let Some(sign) = sign_suffix {
        output.push(sign);
    }
    let _ = core;
    output
}
fn qb_format_using_exponential(
    core: &str,
    mantissa: &str,
    value: f64,
    leading_plus: bool,
    trailing_plus: bool,
    trailing_minus: bool,
    exponent_digits: usize,
) -> String {
    let parts: Vec<&str> = mantissa.splitn(2, '.').collect();
    let int_hashes = parts[0].chars().filter(|ch| *ch == '#').count();
    let frac_hashes = parts
        .get(1)
        .copied()
        .unwrap_or("")
        .chars()
        .filter(|ch| *ch == '#')
        .count();

    let explicit_sign = leading_plus || trailing_plus || trailing_minus;
    let digits_before_decimal = if explicit_sign {
        int_hashes
    } else {
        int_hashes.saturating_sub(1)
    };
    let significant_digits = digits_before_decimal + frac_hashes;
    if significant_digits == 0 || significant_digits > 24 {
        qb_runtime_fail("Illegal function call".to_string());
    }

    let (digits, exponent) = qb_scientific_digits(value, significant_digits);
    let adjusted_exponent = exponent + 1 - digits_before_decimal as i32;
    let mantissa_text =
        qb_compose_scientific_mantissa(&digits, digits_before_decimal, frac_hashes);
    let exponent_text = format!(
        "E{}{abs_exp:0width$}",
        if adjusted_exponent < 0 { '-' } else { '+' },
        abs_exp = adjusted_exponent.abs(),
        width = exponent_digits
    );

    let mut body = String::new();
    if leading_plus {
        body.push(if value.is_sign_negative() { '-' } else { '+' });
    } else if !trailing_plus && !trailing_minus {
        body.push(if value.is_sign_negative() { '-' } else { ' ' });
    }
    body.push_str(&mantissa_text);
    body.push_str(&exponent_text);

    let sign_suffix = if trailing_plus {
        Some(if value.is_sign_negative() { '-' } else { '+' })
    } else if trailing_minus && value.is_sign_negative() {
        Some('-')
    } else {
        None
    };

    let total_len = body.chars().count() + usize::from(sign_suffix.is_some());
    if total_len > core.chars().count() {
        let mut overflow = String::from("%");
        overflow.push_str(&body);
        if let Some(sign) = sign_suffix {
            overflow.push(sign);
        }
        return overflow;
    }

    let mut output = String::new();
    output.push_str(&" ".repeat(core.chars().count() - total_len));
    output.push_str(&body);
    if let Some(sign) = sign_suffix {
        output.push(sign);
    }
    output
}
fn qb_format_using_numeric_value(core: &str, value: f64) -> String {
    let target_width = core.chars().count();
    let mut mantissa = core;
    let mut trailing_plus = false;
    let mut trailing_minus = false;
    if let Some(stripped) = mantissa.strip_suffix('+') {
        trailing_plus = true;
        mantissa = stripped;
    } else if let Some(stripped) = mantissa.strip_suffix('-') {
        trailing_minus = true;
        mantissa = stripped;
    }

    let exponent_digits = if let Some(stripped) = mantissa.strip_suffix("^^^^^") {
        mantissa = stripped;
        Some(3usize)
    } else if let Some(stripped) = mantissa.strip_suffix("^^^^") {
        mantissa = stripped;
        Some(2usize)
    } else {
        None
    };

    let mut leading_plus = false;
    if let Some(stripped) = mantissa.strip_prefix('+') {
        leading_plus = true;
        mantissa = stripped;
    }

    let (prefix_kind, mantissa) = if let Some(stripped) = mantissa.strip_prefix("**$") {
        (QbUsingPrefixKind::StarDollars, stripped)
    } else if let Some(stripped) = mantissa.strip_prefix("$$") {
        (QbUsingPrefixKind::Dollars, stripped)
    } else if let Some(stripped) = mantissa.strip_prefix("**") {
        (QbUsingPrefixKind::Stars, stripped)
    } else {
        (QbUsingPrefixKind::None, mantissa)
    };

    if let Some(exp_digits) = exponent_digits {
        qb_format_using_exponential(
            core,
            mantissa,
            value,
            leading_plus,
            trailing_plus,
            trailing_minus,
            exp_digits,
        )
    } else {
        qb_format_using_fixed(
            core,
            mantissa,
            value,
            leading_plus,
            trailing_plus,
            trailing_minus,
            prefix_kind,
            target_width,
        )
    }
}
fn qb_format_using_value(pattern: &str, value: &str) -> String {
    let decoded = qb_decode_using_pattern(pattern);
    if let Some((start, end, mode)) = qb_find_using_string_field(&decoded) {
        let prefix = qb_collect_using_chars(&decoded[..start]);
        let suffix = qb_collect_using_chars(&decoded[end + 1..]);
        return format!(
            "{}{}{}",
            prefix,
            qb_format_using_string_value(mode, value),
            suffix
        );
    }
    if let Some((start, end)) = qb_find_using_numeric_span(&decoded) {
        let prefix = qb_collect_using_chars(&decoded[..start]);
        let suffix = qb_collect_using_chars(&decoded[end + 1..]);
        let core = qb_collect_using_chars(&decoded[start..=end]);
        if let Ok(number) = value.trim_matches('"').parse::<f64>() {
            return format!(
                "{}{}{}",
                prefix,
                qb_format_using_numeric_value(&core, number),
                suffix
            );
        }
    }
    value.trim_matches('"').to_string()
}
fn qb_format_using_values(pattern: &str, values: &[String]) -> Vec<String> {
    let (leading, fields) = qb_parse_using_fields(pattern);
    if fields.is_empty() {
        if values.is_empty() {
            return if leading.is_empty() {
                Vec::new()
            } else {
                vec![leading]
            };
        }
        return values.iter().map(|_| leading.clone()).collect();
    }

    let mut chunks = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let field_index = index % fields.len();
        let field = &fields[field_index];
        let mut chunk = String::new();
        if field_index == 0 {
            chunk.push_str(&leading);
        }
        chunk.push_str(&qb_format_using_value(&field.core, value));
        chunk.push_str(&field.suffix);
        chunks.push(chunk);
    }
    chunks
}
fn qb_print_using(pattern: &str, values: &[String], comma_after: &[bool], newline: bool) {
    let chunks = qb_format_using_values(pattern, values);
    for (index, chunk) in chunks.iter().enumerate() {
        qb_print(chunk);
        if comma_after.get(index).copied().unwrap_or(false) {
            qb_print_comma();
        }
    }
    if newline {
        qb_print_newline();
    }
}
fn qb_parse_input_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' => in_quotes = true,
            ',' => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current.trim().to_string());
    fields
}
fn qb_input_file_fields(file_number: f64, expected_fields: usize) -> Vec<String> {
    let line = qb_read_line_from_file(file_number);
    let mut fields = qb_parse_input_fields(&line);
    while fields.len() < expected_fields {
        fields.push(String::new());
    }
    fields
}
#[derive(Clone, Copy)]
struct QbSoundNote {
    frequency: f64,
    duration_ms: u32,
}
#[derive(Clone, Copy)]
enum QbPlayArticulation {
    Normal,
    Legato,
    Staccato,
}
fn qb_emit_bell() {
    let _ = io::stdout().flush();
    print!("\x07");
    let _ = io::stdout().flush();
}
fn qb_render_sound_notes(notes: &[QbSoundNote]) {
    if notes.iter().any(|note| note.frequency > 0.0) {
        qb_emit_bell();
    }
    let total_ms: u64 = notes.iter().map(|note| note.duration_ms as u64).sum();
    if total_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(total_ms));
    }
}
fn qb_play_parse_number(chars: &[char], index: &mut usize) -> Option<u32> {
    let start = *index;
    let mut digits = String::new();
    while *index < chars.len() && chars[*index].is_ascii_digit() {
        digits.push(chars[*index]);
        *index += 1;
    }
    if *index == start {
        None
    } else {
        digits.parse::<u32>().ok()
    }
}
fn qb_play_note_length(chars: &[char], index: &mut usize, default_length: u32) -> u32 {
    match qb_play_parse_number(chars, index) {
        Some(0) | None => default_length,
        Some(value) => value,
    }
}
fn qb_play_duration_ms(tempo: u32, length: u32) -> u32 {
    if length == 0 {
        return 0;
    }
    240000 / (tempo.max(1) * length.max(1))
}
fn qb_play_dotted_duration(tempo: u32, length: u32, chars: &[char], index: &mut usize) -> u32 {
    let base_duration = qb_play_duration_ms(tempo, length);
    let mut total = base_duration as f64;
    let mut extra = base_duration as f64 / 2.0;
    while *index < chars.len() && chars[*index] == '.' {
        total += extra;
        extra /= 2.0;
        *index += 1;
    }
    total.round() as u32
}
fn qb_push_rest_note(notes: &mut Vec<QbSoundNote>, duration_ms: u32) {
    if duration_ms == 0 {
        return;
    }
    notes.push(QbSoundNote {
        frequency: 0.0,
        duration_ms,
    });
}
fn qb_push_articulated_note(
    notes: &mut Vec<QbSoundNote>,
    frequency: f64,
    duration_ms: u32,
    articulation: QbPlayArticulation,
) {
    if duration_ms == 0 {
        return;
    }
    let sound_duration = match articulation {
        QbPlayArticulation::Legato => duration_ms,
        QbPlayArticulation::Normal => duration_ms.saturating_mul(7) / 8,
        QbPlayArticulation::Staccato => duration_ms.saturating_mul(3) / 4,
    }
    .min(duration_ms);
    let rest_duration = duration_ms.saturating_sub(sound_duration);

    if sound_duration > 0 {
        notes.push(QbSoundNote {
            frequency,
            duration_ms: sound_duration,
        });
    }
    if rest_duration > 0 {
        qb_push_rest_note(notes, rest_duration);
    }
}
fn qb_parse_play(mml: &str) -> (Vec<QbSoundNote>, bool) {
    let mut notes = Vec::new();
    let mut tempo: u32 = 120;
    let mut octave: u32 = 4;
    let mut length: u32 = 4;
    let mut articulation = QbPlayArticulation::Normal;
    let mut background = false;
    let chars: Vec<char> = mml.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i].to_ascii_uppercase();
        match ch {
            ' ' | '\t' | '\r' | '\n' | ';' => {
                i += 1;
            }
            'T' => {
                i += 1;
                if let Some(value) = qb_play_parse_number(&chars, &mut i) {
                    tempo = value.max(1);
                }
            }
            'O' => {
                i += 1;
                if let Some(value) = qb_play_parse_number(&chars, &mut i) {
                    octave = value.clamp(1, 8);
                }
            }
            'L' => {
                i += 1;
                if let Some(value) = qb_play_parse_number(&chars, &mut i) {
                    length = value.max(1);
                }
            }
            'M' => {
                i += 1;
                if i < chars.len() {
                    match chars[i].to_ascii_uppercase() {
                        'L' => articulation = QbPlayArticulation::Legato,
                        'N' => articulation = QbPlayArticulation::Normal,
                        'S' => articulation = QbPlayArticulation::Staccato,
                        'B' => background = true,
                        'F' => background = false,
                        _ => {
                            continue;
                        }
                    }
                    i += 1;
                }
            }
            '>' => {
                octave = (octave + 1).min(8);
                i += 1;
            }
            '<' => {
                octave = octave.saturating_sub(1).max(1);
                i += 1;
            }
            'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' => {
                let mut frequency = qb_play_note_frequency(ch, octave);
                i += 1;
                if i < chars.len() {
                    match chars[i] {
                        '#' | '+' => {
                            frequency *= 1.0594630943592953;
                            i += 1;
                        }
                        '-' => {
                            frequency /= 1.0594630943592953;
                            i += 1;
                        }
                        _ => {}
                    }
                }

                let note_length = qb_play_note_length(&chars, &mut i, length);
                let duration_ms = qb_play_dotted_duration(tempo, note_length, &chars, &mut i);
                qb_push_articulated_note(&mut notes, frequency, duration_ms, articulation);
            }
            'N' => {
                i += 1;
                let note_number = qb_play_parse_number(&chars, &mut i).unwrap_or(0);
                let duration_ms = qb_play_dotted_duration(tempo, length, &chars, &mut i);
                if note_number == 0 {
                    qb_push_rest_note(&mut notes, duration_ms);
                } else {
                    qb_push_articulated_note(
                        &mut notes,
                        qb_play_note_number_frequency(note_number),
                        duration_ms,
                        articulation,
                    );
                }
            }
            'P' => {
                i += 1;
                let pause_length = match qb_play_parse_number(&chars, &mut i) {
                    Some(0) => 0,
                    Some(value) => value,
                    None => length,
                };
                let duration_ms = qb_play_dotted_duration(tempo, pause_length, &chars, &mut i);
                qb_push_rest_note(&mut notes, duration_ms);
            }
            'R' => {
                i += 1;
                let rest_length = qb_play_note_length(&chars, &mut i, length);
                let duration_ms = qb_play_dotted_duration(tempo, rest_length, &chars, &mut i);
                qb_push_rest_note(&mut notes, duration_ms);
            }
            _ => {
                i += 1;
            }
        }
    }

    (notes, background)
}
fn qb_play_note_frequency(note: char, octave: u32) -> f64 {
    let note_val = match note {
        'C' => 0u32,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => 0,
    };
    let semitones = octave.saturating_sub(1) * 12 + note_val;
    440.0 * 2.0_f64.powf((semitones as f64 - 9.0) / 12.0)
}
fn qb_play_note_number_frequency(note_number: u32) -> f64 {
    let semitones = note_number.saturating_sub(1);
    440.0 * 2.0_f64.powf((semitones as f64 - 9.0) / 12.0)
}
fn qb_update_play_queue(play_queue_limit: usize, play_trap_state: i32, play_pending_event: &mut bool) {
    if *play_pending_event || QB_PLAY_HANDLER_ACTIVE.with(|state| state.get()) {
        return;
    }
    QB_PLAY_NOTE_DEADLINES.with(|deadlines| {
        let now = std::time::Instant::now();
        let deadlines = &mut *deadlines.borrow_mut();
        while deadlines
            .first()
            .copied()
            .is_some_and(|deadline| deadline <= now)
        {
            let previous = deadlines.len();
            deadlines.remove(0);
            let current = deadlines.len();
            if previous == play_queue_limit && current + 1 == previous {
                if play_trap_state == 0 {
                    *play_pending_event = false;
                } else {
                    *play_pending_event = true;
                    break;
                }
            }
        }
    });
}
fn qb_play_queue_len(
    play_queue_limit: usize,
    play_trap_state: i32,
    play_pending_event: &mut bool,
) -> usize {
    if QB_PLAY_HANDLER_ACTIVE.with(|state| state.get()) {
        return QB_PLAY_NOTE_DEADLINES.with(|deadlines| deadlines.borrow().len());
    }
    qb_update_play_queue(play_queue_limit, play_trap_state, play_pending_event);
    QB_PLAY_NOTE_DEADLINES.with(|deadlines| {
        deadlines.borrow().len()
    })
}
fn qb_enqueue_play_notes(notes: &[QbSoundNote]) {
    if notes.iter().any(|note| note.frequency > 0.0) {
        qb_emit_bell();
    }
    QB_PLAY_NOTE_DEADLINES.with(|deadlines| {
        let now = std::time::Instant::now();
        let deadlines = &mut *deadlines.borrow_mut();
        let mut next_deadline = deadlines.last().copied().unwrap_or(now).max(now);
        for note in notes {
            next_deadline += std::time::Duration::from_millis(note.duration_ms as u64);
            deadlines.push(next_deadline);
        }
    });
}
fn qb_beep() {
    qb_emit_bell();
    std::thread::sleep(std::time::Duration::from_millis(200));
}
fn qb_sound(frequency: f64, duration_ticks: f64) {
    if frequency > 0.0 {
        qb_emit_bell();
    }
    if duration_ticks > 0.0 {
        std::thread::sleep(std::time::Duration::from_secs_f64(duration_ticks / 18.2));
    }
}
fn qb_play(melody: &str) {
    let (notes, background) = qb_parse_play(melody);
    if background {
        qb_enqueue_play_notes(&notes);
    } else {
        qb_render_sound_notes(&notes);
    }
}
fn qb_play_count(
    _dummy: f64,
    play_queue_limit: usize,
    play_trap_state: i32,
    play_pending_event: &mut bool,
) -> f64 {
    qb_play_queue_len(play_queue_limit, play_trap_state, play_pending_event) as f64
}
fn qb_shell(command: Option<&str>) {
    if let Some(command) = command {
        if command.is_empty() {
            return;
        }

        match qb_try_execute_dos_shell_builtin(command) {
            Ok(Some(stdout)) => {
                if !stdout.is_empty() {
                    print!("{}", stdout);
                }
                return;
            }
            Ok(None) => {}
            Err(err) => qb_runtime_fail(format!("SHELL failed: {}", err)),
        }

        #[cfg(target_os = "windows")]
        let status = std::process::Command::new("cmd")
            .args(["/C", command])
            .status()
            .unwrap_or_else(|err| qb_runtime_fail(format!("SHELL failed: {}", err)));

        #[cfg(not(target_os = "windows"))]
        let status = std::process::Command::new("sh")
            .args(["-c", command])
            .status()
            .unwrap_or_else(|err| qb_runtime_fail(format!("SHELL failed: {}", err)));

        if !status.success() {
            qb_runtime_fail(format!(
                "SHELL command exited with status {}",
                status.code().unwrap_or(1)
            ));
        }
    }
}
fn qb_chain(path: &str) -> ! {
    let launcher = std::env::var("QBNEX_COMPILER_EXE")
        .map(std::path::PathBuf::from)
        .ok()
        .filter(|path| path.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("qb"));

    let mut command = std::process::Command::new(launcher);
    if std::env::var("QBNEX_QUIET").is_ok_and(|value| value == "1") {
        command.arg("-q");
    }
    let status = command
        .args(["-x", path])
        .status()
        .unwrap_or_else(|err| qb_runtime_fail(format!("CHAIN failed to launch qb: {}", err)));
    std::process::exit(status.code().unwrap_or(if status.success() { 0 } else { 1 }));
}

// String functions
fn qb_len(s: &str) -> f64 { s.len() as f64 }
fn qb_ltrim(s: &str) -> String { s.trim_start().to_string() }
fn qb_rtrim(s: &str) -> String { s.trim_end().to_string() }
fn qb_trim(s: &str) -> String { s.trim().to_string() }
fn qb_fileexists(path: &str) -> f64 {
    if std::path::Path::new(path).is_file() { -1.0 } else { 0.0 }
}
fn qb_direxists(path: &str) -> f64 {
    if std::path::Path::new(path).is_dir() { -1.0 } else { 0.0 }
}
fn qb_ucase(s: &str) -> String { s.to_uppercase() }
fn qb_lcase(s: &str) -> String { s.to_lowercase() }
fn qb_left(s: &str, n: f64) -> String { s.chars().take(n as usize).collect() }
fn qb_right(s: &str, n: f64) -> String { s.chars().rev().take(n as usize).collect::<String>().chars().rev().collect() }
fn qb_mid(s: &str, start: f64, len: f64) -> String { 
    s.chars().skip((start - 1.0) as usize).take(len as usize).collect() 
}
fn qb_mid_no_len(s: &str, start: f64) -> String { 
    s.chars().skip((start - 1.0) as usize).collect() 
}
fn qb_instr(s: &str, substr: &str) -> f64 { 
    match s.find(substr) {
        Some(i) => (i + 1) as f64,
        None => 0.0
    }
}
fn qb_instr_from(start: f64, s: &str, substr: &str) -> f64 {
    let start = start.round().max(1.0) as usize;
    let chars: Vec<char> = s.chars().collect();
    let offset = start.saturating_sub(1);
    let haystack: String = chars.iter().skip(offset).collect();
    match haystack.find(substr) {
        Some(i) => (offset + i + 1) as f64,
        None => 0.0
    }
}
fn qb_string_str(n: f64, c: &str) -> String {
    let ch = c.chars().next().unwrap_or(' ');
    ch.to_string().repeat(n as usize)
}
fn qb_string_chr(n: f64, c: f64) -> String {
    let ch = ((c as u8) as char).to_string();
    ch.repeat(n as usize)
}
fn qb_space(n: f64) -> String { " ".repeat(n as usize) }
fn qb_asc(s: &str) -> f64 { s.chars().next().unwrap_or('\0') as u8 as f64 }
fn qb_set_asc(s: &str, position: f64, value: f64) -> String {
    let index = (position.round() as i32 - 1).max(0) as usize;
    let ascii = value.round().clamp(0.0, 255.0) as u8;
    let mut chars: Vec<char> = s.chars().collect();
    while chars.len() < index {
        chars.push(' ');
    }
    let new_char = char::from(ascii);
    if index < chars.len() {
        chars[index] = new_char;
    } else {
        chars.push(new_char);
    }
    chars.into_iter().collect()
}
fn qb_chr(n: f64) -> String { ((n as u8) as char).to_string() }
fn qb_cstr(n: f64) -> String { qb_format_number(n) }
fn qb_val(s: &str) -> f64 {
    let trimmed = s.trim_start();
    if trimmed.is_empty() {
        return 0.0;
    }

    let bytes = trimmed.as_bytes();
    let mut index = 0usize;
    let mut sign = 1.0f64;
    if let Some(byte) = bytes.first() {
        match byte {
            b'+' => index = 1,
            b'-' => {
                index = 1;
                sign = -1.0;
            }
            _ => {}
        }
    }

    let rest = &trimmed[index..];
    if rest.len() >= 2 && rest.as_bytes()[0] == b'&' {
        let radix = match rest.as_bytes()[1].to_ascii_uppercase() {
            b'H' => 16,
            b'O' => 8,
            _ => 0,
        };
        if radix != 0 {
            let mut end = 2usize;
            while end < rest.len() {
                let ch = rest.as_bytes()[end] as char;
                let valid = match radix {
                    16 => ch.is_ascii_hexdigit(),
                    8 => matches!(ch, '0'..='7'),
                    _ => false,
                };
                if !valid {
                    break;
                }
                end += 1;
            }
            if end > 2 {
                let value = i64::from_str_radix(&rest[2..end], radix).unwrap_or(0) as f64;
                return sign * value;
            }
            return 0.0;
        }
    }

    let mut end = index;
    let mut has_digits = false;
    while end < trimmed.len() && trimmed.as_bytes()[end].is_ascii_digit() {
        end += 1;
        has_digits = true;
    }
    if end < trimmed.len() && trimmed.as_bytes()[end] == b'.' {
        end += 1;
        while end < trimmed.len() && trimmed.as_bytes()[end].is_ascii_digit() {
            end += 1;
            has_digits = true;
        }
    }
    if !has_digits {
        return 0.0;
    }
    if end < trimmed.len() && matches!(trimmed.as_bytes()[end], b'E' | b'e' | b'D' | b'd') {
        let mut exp_end = end + 1;
        if exp_end < trimmed.len() && matches!(trimmed.as_bytes()[exp_end], b'+' | b'-') {
            exp_end += 1;
        }
        let exp_start = exp_end;
        while exp_end < trimmed.len() && trimmed.as_bytes()[exp_end].is_ascii_digit() {
            exp_end += 1;
        }
        if exp_end > exp_start {
            end = exp_end;
        }
    }

    trimmed[..end]
        .replace('D', "E")
        .replace('d', "E")
        .parse()
        .unwrap_or(0.0)
}
fn qb_str(n: f64) -> String {
    let mut text = qb_format_number(n);
    if n >= 0.0 {
        text.insert(0, ' ');
    }
    text
}
fn qb_hex(n: f64) -> String { format!("{:X}", n as i64) }
fn qb_oct(n: f64) -> String { format!("{:o}", n as i64) }

// Array functions
thread_local! {
    static QB_OPTION_BASE: std::cell::RefCell<i32> = const { std::cell::RefCell::new(0) };
}

fn qb_set_option_base(base: i32) {
    QB_OPTION_BASE.with(|cell| *cell.borrow_mut() = base.clamp(0, 1));
}

fn qb_current_option_base() -> i32 {
    QB_OPTION_BASE.with(|cell| *cell.borrow())
}

fn qb_array_bound_value(value: f64) -> i32 {
    value.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

fn qb_validate_array_dimensions(dimensions: &[(i32, i32)]) -> usize {
    let mut total_size = 1usize;
    for (lower, upper) in dimensions {
        if upper < lower {
            qb_runtime_fail(format!("Invalid array bounds: {} TO {}", lower, upper));
        }

        let dim_size = (*upper - *lower + 1) as i64;
        if !(1..=10_000).contains(&dim_size) {
            qb_runtime_fail(format!("Array dimension too large: {} TO {}", lower, upper));
        }

        total_size = total_size
            .checked_mul(dim_size as usize)
            .unwrap_or_else(|| qb_runtime_fail("Array size exceeds limit (max 100,000 elements)"));
        if total_size > 100_000 {
            qb_runtime_fail("Array size exceeds limit (max 100,000 elements)");
        }
    }
    total_size
}

fn qb_create_implicit_array_dimensions(indices: &[f64]) -> Vec<(i32, i32)> {
    let lower = qb_current_option_base();
    let mut dimensions = Vec::with_capacity(indices.len());
    for index in indices {
        let idx = qb_array_bound_value(*index);
        let upper = (idx + 5).max(lower + 5).min(lower + 100);
        dimensions.push((lower, upper));
    }
    let _ = qb_validate_array_dimensions(&dimensions);
    dimensions
}

fn qb_linear_array_index(dimensions: &[(i32, i32)], indices: &[f64]) -> Option<usize> {
    if dimensions.len() != indices.len() {
        return None;
    }

    let mut linear_index = 0usize;
    let mut multiplier = 1usize;
    for ((lower, upper), index) in dimensions.iter().zip(indices.iter()).rev() {
        let index = qb_array_bound_value(*index);
        if index < *lower || index > *upper {
            return None;
        }
        linear_index += (index - *lower) as usize * multiplier;
        multiplier = multiplier.checked_mul((*upper - *lower + 1) as usize)?;
    }
    Some(linear_index)
}

fn qb_dim_num_array(
    arrs: &mut [Vec<f64>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    dimensions: &[(i32, i32)],
    preserve: bool,
) {
    if arr_idx >= arrs.len() || arr_idx >= bounds_store.len() {
        qb_runtime_fail("Numeric array slot out of range");
    }

    let total_size = qb_validate_array_dimensions(dimensions);
    let mut new_array = vec![0.0; total_size];
    if preserve {
        let copy_size = arrs[arr_idx].len().min(total_size);
        new_array[..copy_size].copy_from_slice(&arrs[arr_idx][..copy_size]);
    }
    arrs[arr_idx] = new_array;
    bounds_store[arr_idx] = dimensions.to_vec();
}

fn qb_dim_str_array(
    arrs: &mut [Vec<String>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    dimensions: &[(i32, i32)],
    preserve: bool,
) {
    if arr_idx >= arrs.len() || arr_idx >= bounds_store.len() {
        qb_runtime_fail("String array slot out of range");
    }

    let total_size = qb_validate_array_dimensions(dimensions);
    let mut new_array = vec![String::new(); total_size];
    if preserve {
        let copy_size = arrs[arr_idx].len().min(total_size);
        new_array[..copy_size].clone_from_slice(&arrs[arr_idx][..copy_size]);
    }
    arrs[arr_idx] = new_array;
    bounds_store[arr_idx] = dimensions.to_vec();
}

fn qb_ensure_num_array(
    arrs: &mut [Vec<f64>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    indices: &[f64],
) -> usize {
    if arr_idx >= arrs.len() || arr_idx >= bounds_store.len() {
        qb_runtime_fail("Numeric array slot out of range");
    }
    if bounds_store[arr_idx].is_empty() {
        let dimensions = qb_create_implicit_array_dimensions(indices);
        let total_size = qb_validate_array_dimensions(&dimensions);
        arrs[arr_idx] = vec![0.0; total_size];
        bounds_store[arr_idx] = dimensions;
    }

    qb_linear_array_index(&bounds_store[arr_idx], indices)
        .unwrap_or_else(|| qb_runtime_fail("Subscript out of range"))
}

fn qb_ensure_str_array(
    arrs: &mut [Vec<String>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    indices: &[f64],
) -> usize {
    if arr_idx >= arrs.len() || arr_idx >= bounds_store.len() {
        qb_runtime_fail("String array slot out of range");
    }
    if bounds_store[arr_idx].is_empty() {
        let dimensions = qb_create_implicit_array_dimensions(indices);
        let total_size = qb_validate_array_dimensions(&dimensions);
        arrs[arr_idx] = vec![String::new(); total_size];
        bounds_store[arr_idx] = dimensions;
    }

    qb_linear_array_index(&bounds_store[arr_idx], indices)
        .unwrap_or_else(|| qb_runtime_fail("Subscript out of range"))
}

fn arr_get(
    arrs: &mut [Vec<f64>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    indices: &[f64],
) -> f64 {
    let linear = qb_ensure_num_array(arrs, bounds_store, arr_idx, indices);
    arrs[arr_idx][linear]
}

fn arr_set(
    arrs: &mut [Vec<f64>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    indices: &[f64],
    val: f64,
) {
    let linear = qb_ensure_num_array(arrs, bounds_store, arr_idx, indices);
    arrs[arr_idx][linear] = val;
}

fn str_arr_get(
    arrs: &mut [Vec<String>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    indices: &[f64],
) -> String {
    let linear = qb_ensure_str_array(arrs, bounds_store, arr_idx, indices);
    arrs[arr_idx][linear].clone()
}

fn str_arr_set(
    arrs: &mut [Vec<String>],
    bounds_store: &mut [Vec<(i32, i32)>],
    arr_idx: usize,
    indices: &[f64],
    val: &str,
) {
    let linear = qb_ensure_str_array(arrs, bounds_store, arr_idx, indices);
    arrs[arr_idx][linear] = val.to_string();
}

fn qb_lbound(arr_bounds: &[Vec<(i32, i32)>], arr_idx: usize, dim: Option<f64>) -> f64 {
    let dimension = qb_array_bound_value(dim.unwrap_or(1.0)).max(1) as usize - 1;
    arr_bounds
        .get(arr_idx)
        .and_then(|bounds| bounds.get(dimension))
        .map(|(lower, _)| *lower as f64)
        .unwrap_or(qb_current_option_base() as f64)
}

fn qb_ubound(arr_bounds: &[Vec<(i32, i32)>], arr_idx: usize, dim: Option<f64>) -> f64 {
    let dimension = qb_array_bound_value(dim.unwrap_or(1.0)).max(1) as usize - 1;
    arr_bounds
        .get(arr_idx)
        .and_then(|bounds| bounds.get(dimension))
        .map(|(_, upper)| *upper as f64)
        .unwrap_or(0.0)
}

"#);
    }

    pub(super) fn write_epilogue(&mut self) {
        if self.use_graphics {
            // Auto-close graphics window after program ends
            self.output.push_str("    unsafe {\n");
            self.output
                .push_str("        if let Some(window) = WINDOW.as_mut() {\n");
            self.output.push_str("            // Final screen update\n");
            self.output.push_str("            update_screen();\n");
            self.output
                .push_str("            // Brief pause to show final result\n");
            self.output
                .push_str("            std::thread::sleep(Duration::from_millis(500));\n");
            self.output.push_str("        }\n");
            self.output.push_str("    }\n");
            // Don't wait for Enter in graphics mode
        } else {
            self.output.push_str("    if qb_should_pause_on_exit() {\n");
            self.output
                .push_str("        println!(\"\\nPress Enter to exit...\");\n");
            self.output.push_str("        qb_sleep(0.0);\n");
            self.output.push_str("    }\n");
        }
        self.output.push_str("}\n");
    }

    pub(super) fn generate_sub(&mut self, sub: &SubDef) -> QResult<()> {
        let old_num = self.num_vars.clone();
        let old_str = self.str_vars.clone();
        let old_arr = self.arr_vars.clone();
        let old_str_arr = self.str_arr_vars.clone();
        let old_field_widths = self.field_widths.clone();
        let old_udt_vars = self.udt_vars.clone();
        let old_udt_array_vars = self.udt_array_vars.clone();
        let old_params = self.params.clone();
        let old_in_sub = self.is_in_sub;
        let old_proc_is_static = self.current_proc_is_static;

        self.num_vars.clear();
        self.str_vars.clear();
        self.arr_vars.clear();
        self.str_arr_vars.clear();
        self.field_widths.clear();
        self.udt_vars.clear();
        self.udt_array_vars.clear();
        self.params.clear();
        self.is_in_sub = true;
        self.current_proc_is_static = sub.is_static;

        let mut rust_params = Vec::new();
        for param in &sub.params {
            let param_name = &param.name;
            // Create a safe rust variable name for the parameter
            let rust_name = format!(
                "arg_{}",
                param_name
                    .replace("$", "_S")
                    .replace("%", "_I")
                    .replace("&", "_L")
                    .replace("!", "_F")
                    .replace("#", "_D")
            );

            if self.variable_is_string(param) {
                rust_params.push(format!("{}: &mut String", rust_name));
                let _ = self.get_str_var_idx(param_name);
                if let Some(width) = param.fixed_length {
                    self.field_widths.insert(param_name.to_uppercase(), width);
                }
            } else {
                rust_params.push(format!("{}: &mut f64", rust_name));
                let _ = self.get_num_var_idx(param_name);
            }
        }

        self.collect_vars(&sub.body);

        let fn_name = self.rust_symbol("sub", &sub.name);
        let static_num_name = format!("{}_static_num", fn_name);
        let static_str_name = format!("{}_static_str", fn_name);
        let static_arr_name = format!("{}_static_arr", fn_name);
        let static_arr_bounds_name = format!("{}_static_arr_bounds", fn_name);
        let static_str_arr_name = format!("{}_static_str_arr", fn_name);
        let static_str_arr_bounds_name = format!("{}_static_str_arr_bounds", fn_name);

        if sub.is_static {
            self.output.push_str("thread_local! {\n");
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<f64>> = std::cell::RefCell::new(vec![0.0; {}]);\n",
                static_num_name.to_uppercase(),
                self.num_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(vec![String::new(); {}]);\n",
                static_str_name.to_uppercase(),
                self.str_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<f64>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_arr_name.to_uppercase(),
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<(i32, i32)>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_arr_bounds_name.to_uppercase(),
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<String>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_str_arr_name.to_uppercase(),
                self.str_arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<(i32, i32)>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_str_arr_bounds_name.to_uppercase(),
                self.str_arr_vars.len().max(1)
            ));
            self.output.push_str("}\n");
        }

        self.output.push_str(&format!("\nfn {}(", fn_name));
        self.output.push_str("global_num_vars: &mut Vec<f64>, global_str_vars: &mut Vec<String>, global_arr_vars: &mut Vec<Vec<f64>>, global_arr_bounds: &mut Vec<Vec<(i32, i32)>>, global_str_arr_vars: &mut Vec<Vec<String>>, global_str_arr_bounds: &mut Vec<Vec<(i32, i32)>>");
        if !rust_params.is_empty() {
            self.output.push_str(", ");
            self.output.push_str(&rust_params.join(", "));
        }
        self.output.push_str(") {\n");

        if sub.is_static {
            self.output.push_str(&format!(
                "    let mut num_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_num_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut str_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_str_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut arr_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut arr_bounds = {}.with(|cell| cell.borrow().clone());\n",
                static_arr_bounds_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_str_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_bounds = {}.with(|cell| cell.borrow().clone());\n",
                static_str_arr_bounds_name.to_uppercase()
            ));
        } else {
            self.output.push_str(&format!(
                "    let mut num_vars = vec![0.0; {}];\n",
                self.num_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut str_vars = vec![String::new(); {}];\n",
                self.str_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut arr_vars: Vec<Vec<f64>> = vec![Vec::new(); {}];\n",
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut arr_bounds: Vec<Vec<(i32, i32)>> = vec![Vec::new(); {}];\n",
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_vars: Vec<Vec<String>> = vec![Vec::new(); {}];\n",
                self.str_arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_bounds: Vec<Vec<(i32, i32)>> = vec![Vec::new(); {}];\n",
                self.str_arr_vars.len().max(1)
            ));
        }

        // Initialize local vars from args
        for param in &sub.params {
            let param_name = &param.name;
            let rust_name = format!(
                "arg_{}",
                param_name
                    .replace("$", "_S")
                    .replace("%", "_I")
                    .replace("&", "_L")
                    .replace("!", "_F")
                    .replace("#", "_D")
            );
            if self.variable_is_string(param) {
                let idx = self.get_str_var_idx(param_name);
                if let Some(width) = param.fixed_length {
                    self.output.push_str(&format!(
                        "    set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, {}));\n",
                        idx, width, rust_name
                    ));
                } else {
                    self.output.push_str(&format!(
                        "    set_str(&mut str_vars, {}, {});\n",
                        idx, rust_name
                    ));
                }
            } else {
                let idx = self.get_num_var_idx(param_name);
                self.output.push_str(&format!(
                    "    set_var(&mut num_vars, {}, *{});\n",
                    idx, rust_name
                ));
            }
        }

        self.output.push_str("    'qb_proc: {\n");
        if self.has_top_level_control_flow(&sub.body) {
            self.generate_top_level_control_flow(&sub.body)?;
        } else {
            for stmt in &sub.body {
                self.generate_statement(stmt)?;
            }
        }
        self.output.push_str("    }\n");

        if sub.is_static {
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = num_vars.clone());\n",
                static_num_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = str_vars.clone());\n",
                static_str_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = arr_vars.clone());\n",
                static_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = arr_bounds.clone());\n",
                static_arr_bounds_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = str_arr_vars.clone());\n",
                static_str_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = str_arr_bounds.clone());\n",
                static_str_arr_bounds_name.to_uppercase()
            ));
        }

        // Copy back to args (pass by reference simulation)
        for param in &sub.params {
            if param.by_val {
                continue;
            }
            let param_name = &param.name;
            let rust_name = format!(
                "arg_{}",
                param_name
                    .replace("$", "_S")
                    .replace("%", "_I")
                    .replace("&", "_L")
                    .replace("!", "_F")
                    .replace("#", "_D")
            );
            if self.variable_is_string(param) {
                let idx = self.get_str_var_idx(param_name);
                if let Some(width) = param.fixed_length {
                    self.output.push_str(&format!(
                        "    *{} = qb_fit_fixed_string({}, &get_str(&str_vars, {}));\n",
                        rust_name, width, idx
                    ));
                } else {
                    self.output.push_str(&format!(
                        "    *{} = get_str(&str_vars, {});\n",
                        rust_name, idx
                    ));
                }
            } else {
                let idx = self.get_num_var_idx(param_name);
                self.output.push_str(&format!(
                    "    *{} = get_var(&num_vars, {});\n",
                    rust_name, idx
                ));
            }
        }

        self.output.push_str("}\n");

        self.num_vars = old_num;
        self.str_vars = old_str;
        self.arr_vars = old_arr;
        self.str_arr_vars = old_str_arr;
        self.field_widths = old_field_widths;
        self.udt_vars = old_udt_vars;
        self.udt_array_vars = old_udt_array_vars;
        self.params = old_params;
        self.is_in_sub = old_in_sub;
        self.current_proc_is_static = old_proc_is_static;
        Ok(())
    }

    pub(super) fn generate_function(&mut self, func: &FunctionDef) -> QResult<()> {
        let old_num = self.num_vars.clone();
        let old_str = self.str_vars.clone();
        let old_arr = self.arr_vars.clone();
        let old_str_arr = self.str_arr_vars.clone();
        let old_field_widths = self.field_widths.clone();
        let old_udt_vars = self.udt_vars.clone();
        let old_udt_array_vars = self.udt_array_vars.clone();
        let old_params = self.params.clone();
        let old_in_sub = self.is_in_sub;
        let old_proc_is_static = self.current_proc_is_static;
        let old_func_name = self.current_function_name.clone();

        self.num_vars.clear();
        self.str_vars.clear();
        self.arr_vars.clear();
        self.str_arr_vars.clear();
        self.field_widths.clear();
        self.udt_vars.clear();
        self.udt_array_vars.clear();
        self.params.clear();
        self.is_in_sub = true;
        self.current_proc_is_static = func.is_static;
        self.current_function_name = Some(func.name.clone());

        let mut rust_params = Vec::new();
        for param in &func.params {
            let param_name = &param.name;
            let rust_name = format!(
                "arg_{}",
                param_name
                    .replace("$", "_S")
                    .replace("%", "_I")
                    .replace("&", "_L")
                    .replace("!", "_F")
                    .replace("#", "_D")
            );

            if self.variable_is_string(param) {
                rust_params.push(format!("{}: &mut String", rust_name));
                let _ = self.get_str_var_idx(param_name);
                if let Some(width) = param.fixed_length {
                    self.field_widths.insert(param_name.to_uppercase(), width);
                }
            } else {
                rust_params.push(format!("{}: &mut f64", rust_name));
                let _ = self.get_num_var_idx(param_name);
            }
        }

        // Reserve return value
        let ret_name = func.name.clone();
        if matches!(func.return_type, QType::String(_)) {
            self.get_str_var_idx(&ret_name);
            if let Some(width) = func.return_fixed_length {
                self.field_widths.insert(ret_name.to_uppercase(), width);
            }
        } else {
            self.get_num_var_idx(&ret_name);
        }

        self.collect_vars(&func.body);

        let fn_name = self.rust_symbol("func", &func.name);
        let ret_type = if matches!(func.return_type, QType::String(_)) {
            "String"
        } else {
            "f64"
        };
        let static_num_name = format!("{}_static_num", fn_name);
        let static_str_name = format!("{}_static_str", fn_name);
        let static_arr_name = format!("{}_static_arr", fn_name);
        let static_arr_bounds_name = format!("{}_static_arr_bounds", fn_name);
        let static_str_arr_name = format!("{}_static_str_arr", fn_name);
        let static_str_arr_bounds_name = format!("{}_static_str_arr_bounds", fn_name);

        if func.is_static {
            self.output.push_str("thread_local! {\n");
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<f64>> = std::cell::RefCell::new(vec![0.0; {}]);\n",
                static_num_name.to_uppercase(),
                self.num_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(vec![String::new(); {}]);\n",
                static_str_name.to_uppercase(),
                self.str_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<f64>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_arr_name.to_uppercase(),
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<(i32, i32)>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_arr_bounds_name.to_uppercase(),
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<String>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_str_arr_name.to_uppercase(),
                self.str_arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    static {}: std::cell::RefCell<Vec<Vec<(i32, i32)>>> = std::cell::RefCell::new(vec![Vec::new(); {}]);\n",
                static_str_arr_bounds_name.to_uppercase(),
                self.str_arr_vars.len().max(1)
            ));
            self.output.push_str("}\n");
        }

        self.output.push_str(&format!("\nfn {}(", fn_name));
        self.output.push_str("global_num_vars: &mut Vec<f64>, global_str_vars: &mut Vec<String>, global_arr_vars: &mut Vec<Vec<f64>>, global_arr_bounds: &mut Vec<Vec<(i32, i32)>>, global_str_arr_vars: &mut Vec<Vec<String>>, global_str_arr_bounds: &mut Vec<Vec<(i32, i32)>>");
        if !rust_params.is_empty() {
            self.output.push_str(", ");
            self.output.push_str(&rust_params.join(", "));
        }
        self.output.push_str(&format!(") -> {} {{\n", ret_type));

        if func.is_static {
            self.output.push_str(&format!(
                "    let mut num_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_num_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut str_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_str_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut arr_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut arr_bounds = {}.with(|cell| cell.borrow().clone());\n",
                static_arr_bounds_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_vars = {}.with(|cell| cell.borrow().clone());\n",
                static_str_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_bounds = {}.with(|cell| cell.borrow().clone());\n",
                static_str_arr_bounds_name.to_uppercase()
            ));
        } else {
            self.output.push_str(&format!(
                "    let mut num_vars = vec![0.0; {}];\n",
                self.num_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut str_vars = vec![String::new(); {}];\n",
                self.str_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut arr_vars: Vec<Vec<f64>> = vec![Vec::new(); {}];\n",
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut arr_bounds: Vec<Vec<(i32, i32)>> = vec![Vec::new(); {}];\n",
                self.arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_vars: Vec<Vec<String>> = vec![Vec::new(); {}];\n",
                self.str_arr_vars.len().max(1)
            ));
            self.output.push_str(&format!(
                "    let mut str_arr_bounds: Vec<Vec<(i32, i32)>> = vec![Vec::new(); {}];\n",
                self.str_arr_vars.len().max(1)
            ));
        }

        // Initialize local vars from args
        for param in &func.params {
            let param_name = &param.name;
            let rust_name = format!(
                "arg_{}",
                param_name
                    .replace("$", "_S")
                    .replace("%", "_I")
                    .replace("&", "_L")
                    .replace("!", "_F")
                    .replace("#", "_D")
            );
            if self.variable_is_string(param) {
                let idx = self.get_str_var_idx(param_name);
                if let Some(width) = param.fixed_length {
                    self.output.push_str(&format!(
                        "    set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, {}));\n",
                        idx, width, rust_name
                    ));
                } else {
                    self.output.push_str(&format!(
                        "    set_str(&mut str_vars, {}, {});\n",
                        idx, rust_name
                    ));
                }
            } else {
                let idx = self.get_num_var_idx(param_name);
                self.output.push_str(&format!(
                    "    set_var(&mut num_vars, {}, *{});\n",
                    idx, rust_name
                ));
            }
        }

        self.output.push_str("    'qb_proc: {\n");
        if self.has_top_level_control_flow(&func.body) {
            self.generate_top_level_control_flow(&func.body)?;
        } else {
            for stmt in &func.body {
                self.generate_statement(stmt)?;
            }
        }
        self.output.push_str("    }\n");

        if func.is_static {
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = num_vars.clone());\n",
                static_num_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = str_vars.clone());\n",
                static_str_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = arr_vars.clone());\n",
                static_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = arr_bounds.clone());\n",
                static_arr_bounds_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = str_arr_vars.clone());\n",
                static_str_arr_name.to_uppercase()
            ));
            self.output.push_str(&format!(
                "    {}.with(|cell| *cell.borrow_mut() = str_arr_bounds.clone());\n",
                static_str_arr_bounds_name.to_uppercase()
            ));
        }

        // Copy back to args
        for param in &func.params {
            if param.by_val {
                continue;
            }
            let param_name = &param.name;
            let rust_name = format!(
                "arg_{}",
                param_name
                    .replace("$", "_S")
                    .replace("%", "_I")
                    .replace("&", "_L")
                    .replace("!", "_F")
                    .replace("#", "_D")
            );
            if self.variable_is_string(param) {
                let idx = self.get_str_var_idx(param_name);
                if let Some(width) = param.fixed_length {
                    self.output.push_str(&format!(
                        "    *{} = qb_fit_fixed_string({}, &get_str(&str_vars, {}));\n",
                        rust_name, width, idx
                    ));
                } else {
                    self.output.push_str(&format!(
                        "    *{} = get_str(&str_vars, {});\n",
                        rust_name, idx
                    ));
                }
            } else {
                let idx = self.get_num_var_idx(param_name);
                self.output.push_str(&format!(
                    "    *{} = get_var(&num_vars, {});\n",
                    rust_name, idx
                ));
            }
        }

        // Return value
        if matches!(func.return_type, QType::String(_)) {
            let idx = self.get_str_var_idx(&ret_name);
            self.output
                .push_str(&format!("    get_str(&str_vars, {})\n", idx));
        } else {
            let idx = self.get_num_var_idx(&ret_name);
            self.output
                .push_str(&format!("    get_var(&num_vars, {})\n", idx));
        }

        self.output.push_str("}\n");

        self.num_vars = old_num;
        self.str_vars = old_str;
        self.arr_vars = old_arr;
        self.str_arr_vars = old_str_arr;
        self.field_widths = old_field_widths;
        self.udt_vars = old_udt_vars;
        self.udt_array_vars = old_udt_array_vars;
        self.params = old_params;
        self.is_in_sub = old_in_sub;
        self.current_proc_is_static = old_proc_is_static;
        self.current_function_name = old_func_name;
        Ok(())
    }
}
