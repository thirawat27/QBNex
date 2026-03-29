use crate::builtin_functions::is_builtin_function;
use crate::opcodes::{BinaryFileKind, ByRefTarget, OpCode};
use core_types::{QResult, QType};
use std::collections::HashMap;
use syntax_tree::ast_nodes::{
    ArrayDimension, BinaryOp, DefTypeMap, ExitType, Expression, GotoTarget, PrintSeparator,
    Program, Statement, UnaryOp,
};
use syntax_tree::{validate_program, Backend};

struct LoopContext {
    exit_type: ExitType,
    exit_jumps: Vec<usize>,
}

pub struct BytecodeCompiler {
    program: Program,
    bytecode: Vec<OpCode>,
    labels: HashMap<String, usize>,
    line_numbers: HashMap<String, usize>,
    restore_targets: HashMap<String, usize>,
    pending_jumps: Vec<(usize, String)>, // (bytecode_index, label_name)
    pending_gosubs: Vec<(usize, String)>, // (bytecode_index, label_name)
    pending_on_errors: Vec<(usize, GotoTarget)>, // (bytecode_index, target) for ON ERROR GOTO
    pending_on_timers: Vec<(usize, String)>, // (bytecode_index, label_name) for ON TIMER GOSUB
    pending_on_plays: Vec<(usize, String)>, // (bytecode_index, label_name) for ON PLAY GOSUB
    variable_map: HashMap<String, usize>,
    field_widths: HashMap<usize, usize>,
    function_param_modes: HashMap<String, Vec<bool>>,
    sub_param_modes: HashMap<String, Vec<bool>>,
    def_types: DefTypeMap,
    next_var_index: usize,
    loop_contexts: Vec<LoopContext>,
    current_function: Option<String>,
    option_base: i32,
}

impl BytecodeCompiler {
    pub fn new(program: Program) -> Self {
        let def_types = Self::collect_def_types(&program);
        let function_param_modes = Self::collect_param_modes(&program, true);
        let sub_param_modes = Self::collect_param_modes(&program, false);
        Self {
            program,
            bytecode: Vec::with_capacity(1024),
            labels: HashMap::with_capacity(64),
            line_numbers: HashMap::with_capacity(128),
            restore_targets: HashMap::with_capacity(64),
            pending_jumps: Vec::with_capacity(32),
            pending_gosubs: Vec::with_capacity(32),
            pending_on_errors: Vec::with_capacity(16),
            pending_on_timers: Vec::with_capacity(8),
            pending_on_plays: Vec::with_capacity(8),
            variable_map: HashMap::with_capacity(64),
            field_widths: HashMap::with_capacity(16),
            function_param_modes,
            sub_param_modes,
            def_types,
            next_var_index: 0,
            loop_contexts: Vec::with_capacity(16),
            current_function: None,
            option_base: 0,
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

    fn encode_var_ref(&mut self, name: &str) -> String {
        let slot = self.get_var_index(name);
        format!("#{}:{}", slot, name)
    }

    fn qualified_field_name(expr: &Expression) -> Option<String> {
        expr.flattened_qb64_name().filter(|name| name.contains('.'))
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

    fn normalize_label(name: &str) -> String {
        name.to_ascii_uppercase()
    }

    fn collect_def_types(program: &Program) -> DefTypeMap {
        let mut def_types = DefTypeMap::new();
        for statement in &program.statements {
            if let Statement::DefType {
                letter_ranges,
                type_name,
            } = statement
            {
                let qtype = Self::declared_type_to_qtype(type_name);
                for (start, end) in letter_ranges {
                    def_types.set_range(
                        start.to_ascii_lowercase(),
                        end.to_ascii_lowercase(),
                        qtype.clone(),
                    );
                }
            }
        }
        def_types
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

    fn name_type_hint(&self, name: &str, type_suffix: Option<char>) -> QType {
        let mut variable = syntax_tree::ast_nodes::Variable::new(name.to_string());
        if let Some(type_suffix) = type_suffix {
            variable.type_suffix = Some(type_suffix);
        }
        variable.get_default_type(&self.def_types)
    }

    fn variable_type_hint(&self, var: &syntax_tree::ast_nodes::Variable) -> QType {
        if let Some(declared_type) = &var.declared_type {
            return Self::declared_type_to_qtype(declared_type);
        }
        var.get_default_type(&self.def_types)
    }

    fn variable_is_string(&self, var: &syntax_tree::ast_nodes::Variable) -> bool {
        matches!(self.variable_type_hint(var), QType::String(_))
    }

    fn qtype_to_binary_kind(qtype: &QType) -> Option<BinaryFileKind> {
        match qtype {
            QType::Integer(_) => Some(BinaryFileKind::Integer),
            QType::Long(_) => Some(BinaryFileKind::Long),
            QType::Single(_) => Some(BinaryFileKind::Single),
            QType::Double(_) => Some(BinaryFileKind::Double),
            QType::String(_) => Some(BinaryFileKind::String),
            _ => None,
        }
    }

    fn binary_file_kind_for_name(&self, name: &str, type_suffix: Option<char>) -> BinaryFileKind {
        Self::qtype_to_binary_kind(&self.name_type_hint(name, type_suffix))
            .unwrap_or(BinaryFileKind::Single)
    }

    fn zero_value_for_qtype(qtype: &QType) -> QType {
        match qtype {
            QType::Integer(_) => QType::Integer(0),
            QType::Long(_) => QType::Long(0),
            QType::Single(_) => QType::Single(0.0),
            QType::Double(_) => QType::Double(0.0),
            QType::String(_) => QType::String(String::new()),
            QType::UserDefined(bytes) => QType::UserDefined(bytes.clone()),
            QType::Empty => QType::Empty,
        }
    }

    fn register_numeric_slot_type(&mut self, slot: usize, qtype: &QType) {
        if let Some(kind) =
            Self::qtype_to_binary_kind(qtype).filter(|kind| *kind != BinaryFileKind::String)
        {
            self.bytecode.push(OpCode::SetNumericType { slot, kind });
        }
    }

    fn register_numeric_array_type(&mut self, name: &str, qtype: &QType) {
        if let Some(kind) =
            Self::qtype_to_binary_kind(qtype).filter(|kind| *kind != BinaryFileKind::String)
        {
            self.bytecode.push(OpCode::SetNumericArrayType {
                name: name.to_string(),
                kind,
            });
        }
    }

    fn register_scalar_storage_metadata(
        &mut self,
        var: &syntax_tree::ast_nodes::Variable,
        slot: usize,
    ) {
        if self.variable_is_string(var) {
            if let Some(width) = var.fixed_length {
                self.bytecode.push(OpCode::SetStringWidth { slot, width });
            }
        } else {
            let qtype = self.variable_type_hint(var);
            self.register_numeric_slot_type(slot, &qtype);
        }
    }

    fn register_array_storage_metadata(&mut self, var: &syntax_tree::ast_nodes::Variable) {
        if self.variable_is_string(var) {
            self.bytecode.push(OpCode::SetStringArrayWidth {
                name: var.name.clone(),
                width: var.fixed_length.unwrap_or(0),
            });
        } else {
            let qtype = self.variable_type_hint(var);
            self.register_numeric_array_type(&var.name, &qtype);
        }
    }

    fn binary_file_target_metadata(&self, expr: &Expression) -> Option<(BinaryFileKind, usize)> {
        match expr {
            Expression::Variable(var) => {
                if self.variable_is_string(var) {
                    Some((BinaryFileKind::String, var.fixed_length.unwrap_or(0)))
                } else {
                    Some((
                        self.binary_file_kind_for_name(&var.name, var.type_suffix),
                        0,
                    ))
                }
            }
            Expression::ArrayAccess {
                name, type_suffix, ..
            } => {
                let kind = self.binary_file_kind_for_name(name, *type_suffix);
                Some((kind, 0))
            }
            Expression::FieldAccess { .. } => Self::qualified_field_name(expr).map(|name| {
                let fixed_length = self.fixed_string_width_for_expr(expr).unwrap_or(0);
                (self.binary_file_kind_for_name(&name, None), fixed_length)
            }),
            _ => None,
        }
    }

    fn lookup_user_type(&self, type_name: &str) -> Option<syntax_tree::ast_nodes::UserType> {
        self.program
            .user_types
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(type_name))
            .map(|(_, user_type)| user_type.clone())
    }

    fn find_declared_type_for_variable(&self, name: &str) -> Option<String> {
        for stmt in &self.program.statements {
            if let Statement::Dim { variables, .. } | Statement::Redim { variables, .. } = stmt {
                for (var, _) in variables {
                    if var.name.eq_ignore_ascii_case(name) {
                        return var.declared_type.clone();
                    }
                }
            }
        }

        for func in self.program.functions.values() {
            for param in &func.params {
                if param.name.eq_ignore_ascii_case(name) {
                    return param.declared_type.clone();
                }
            }
        }

        for sub in self.program.subs.values() {
            for param in &sub.params {
                if param.name.eq_ignore_ascii_case(name) {
                    return param.declared_type.clone();
                }
            }
        }

        None
    }

    fn resolve_udt_object_type(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(var) => var
                .declared_type
                .clone()
                .or_else(|| self.find_declared_type_for_variable(&var.name))
                .filter(|type_name| self.lookup_user_type(type_name).is_some()),
            Expression::FieldAccess { object, field } => {
                let parent_type = self.resolve_udt_object_type(object)?;
                let user_type = self.lookup_user_type(&parent_type)?;
                let type_field = user_type
                    .fields
                    .iter()
                    .find(|candidate| candidate.name.eq_ignore_ascii_case(field))?;
                match &type_field.field_type {
                    QType::UserDefined(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn fixed_string_width_for_expr(&self, expr: &Expression) -> Option<usize> {
        match expr {
            Expression::Variable(var) => var.fixed_length,
            Expression::FieldAccess { object, field } => {
                let parent_type = self.resolve_udt_object_type(object)?;
                let user_type = self.lookup_user_type(&parent_type)?;
                let type_field = user_type
                    .fields
                    .iter()
                    .find(|candidate| candidate.name.eq_ignore_ascii_case(field))?;
                matches!(type_field.field_type, QType::String(_))
                    .then_some(type_field.fixed_length)
                    .flatten()
            }
            _ => None,
        }
    }

    fn register_udt_field_slots(&mut self, base_name: &str, type_name: &str) {
        let Some(user_type) = self.lookup_user_type(type_name) else {
            return;
        };

        for field in user_type.fields {
            let field_name = format!("{base_name}.{}", field.name);
            match field.field_type {
                QType::UserDefined(bytes) => {
                    let nested_type = String::from_utf8_lossy(&bytes).into_owned();
                    self.register_udt_field_slots(&field_name, &nested_type);
                }
                QType::String(_) => {
                    let slot = self.get_var_index(&field_name);
                    if let Some(width) = field.fixed_length {
                        self.field_widths.insert(slot, width);
                        self.bytecode.push(OpCode::SetStringWidth { slot, width });
                    }
                    self.bytecode
                        .push(OpCode::LoadConstant(QType::String(String::new())));
                    self.bytecode.push(OpCode::StoreFast(slot));
                }
                _ => {
                    let slot = self.get_var_index(&field_name);
                    self.register_numeric_slot_type(slot, &field.field_type);
                }
            }
        }
    }

    fn normalize_restore_target(name: &str) -> String {
        if name.parse::<u16>().is_ok() {
            name.to_string()
        } else {
            Self::normalize_label(name)
        }
    }

    fn normalize_proc_name(name: &str) -> String {
        name.to_ascii_uppercase()
    }

    fn print_control_argument<'a>(expr: &'a Expression, name: &str) -> Option<&'a Expression> {
        match expr {
            Expression::ArrayAccess {
                name: expr_name,
                indices,
                ..
            } if expr_name.eq_ignore_ascii_case(name) && indices.len() == 1 => indices.first(),
            Expression::FunctionCall(func)
                if func.name.eq_ignore_ascii_case(name) && func.args.len() == 1 =>
            {
                func.args.first()
            }
            _ => None,
        }
    }

    fn comma_after(separators: &[Option<PrintSeparator>]) -> Vec<bool> {
        separators
            .iter()
            .map(|separator| matches!(separator, Some(PrintSeparator::Comma)))
            .collect()
    }

    fn has_function(&self, name: &str) -> bool {
        self.program
            .functions
            .keys()
            .any(|func_name| func_name.eq_ignore_ascii_case(name))
    }

    fn has_sub(&self, name: &str) -> bool {
        self.program
            .subs
            .keys()
            .any(|sub_name| sub_name.eq_ignore_ascii_case(name))
    }

    fn collect_param_modes(program: &Program, is_function: bool) -> HashMap<String, Vec<bool>> {
        let mut modes = HashMap::new();

        for stmt in &program.statements {
            if let Statement::Declare {
                name,
                is_function: declared_is_function,
                params,
                ..
            } = stmt
            {
                if *declared_is_function == is_function {
                    modes.insert(
                        Self::normalize_proc_name(name),
                        params.iter().map(|param| param.by_val).collect(),
                    );
                }
            }
        }

        if is_function {
            for (name, func_def) in &program.functions {
                modes.insert(
                    Self::normalize_proc_name(name),
                    func_def.params.iter().map(|param| param.by_val).collect(),
                );
            }
        } else {
            for (name, sub_def) in &program.subs {
                modes.insert(
                    Self::normalize_proc_name(name),
                    sub_def.params.iter().map(|param| param.by_val).collect(),
                );
            }
        }

        modes
    }

    fn validate_declared_signature_matches_definition(&self) -> QResult<()> {
        for stmt in &self.program.statements {
            let Statement::Declare {
                name,
                is_function,
                params,
                ..
            } = stmt
            else {
                continue;
            };

            let definition_params = if *is_function {
                self.program
                    .functions
                    .iter()
                    .find(|(func_name, _)| func_name.eq_ignore_ascii_case(name))
                    .map(|(_, func_def)| &func_def.params)
            } else {
                self.program
                    .subs
                    .iter()
                    .find(|(sub_name, _)| sub_name.eq_ignore_ascii_case(name))
                    .map(|(_, sub_def)| &sub_def.params)
            };

            let Some(definition_params) = definition_params else {
                continue;
            };

            if params.len() != definition_params.len() {
                let proc_kind = if *is_function { "FUNCTION" } else { "SUB" };
                return Err(core_types::QError::InvalidProcedure(format!(
                    "{} {} declaration expects {} argument(s), definition has {}",
                    proc_kind,
                    name,
                    params.len(),
                    definition_params.len()
                )));
            }

            for (position, (declared, defined)) in
                params.iter().zip(definition_params.iter()).enumerate()
            {
                if declared.by_val != defined.by_val {
                    let proc_kind = if *is_function { "FUNCTION" } else { "SUB" };
                    let declared_mode = if declared.by_val { "BYVAL" } else { "BYREF" };
                    let defined_mode = if defined.by_val { "BYVAL" } else { "BYREF" };
                    return Err(core_types::QError::InvalidProcedure(format!(
                        "{} {} argument {} declared as {}, defined as {}",
                        proc_kind,
                        name,
                        position + 1,
                        declared_mode,
                        defined_mode
                    )));
                }
            }
        }

        Ok(())
    }

    fn require_param_modes(
        &self,
        name: &str,
        args_len: usize,
        is_function: bool,
    ) -> QResult<Vec<bool>> {
        let normalized_name = Self::normalize_proc_name(name);
        let param_modes = if is_function {
            self.function_param_modes.get(&normalized_name)
        } else {
            self.sub_param_modes.get(&normalized_name)
        };

        let Some(param_modes) = param_modes else {
            let proc_kind = if is_function { "FUNCTION" } else { "SUB" };
            return Err(core_types::QError::InvalidProcedure(format!(
                "{} {} is not defined or declared",
                proc_kind, name
            )));
        };

        if args_len != param_modes.len() {
            let proc_kind = if is_function { "FUNCTION" } else { "SUB" };
            return Err(core_types::QError::InvalidProcedure(format!(
                "{} {} expects {} argument(s), got {}",
                proc_kind,
                name,
                param_modes.len(),
                args_len
            )));
        }

        if is_function && !self.has_function(name) {
            return Err(core_types::QError::InvalidProcedure(format!(
                "FUNCTION {} is declared but has no definition",
                name
            )));
        }

        if !is_function && !self.has_sub(name) {
            return Err(core_types::QError::InvalidProcedure(format!(
                "SUB {} is declared but has no definition",
                name
            )));
        }

        Ok(param_modes.clone())
    }

    fn compile_call_arguments(
        &mut self,
        args: &[Expression],
        param_modes: Option<&[bool]>,
    ) -> QResult<Vec<ByRefTarget>> {
        let mut by_ref = Vec::with_capacity(args.len());
        for (arg_index, arg) in args.iter().enumerate() {
            if param_modes
                .and_then(|modes| modes.get(arg_index))
                .copied()
                .unwrap_or(false)
            {
                by_ref.push(ByRefTarget::None);
                self.compile_expression(arg)?;
                continue;
            }

            match arg {
                Expression::Variable(var) => {
                    if is_builtin_function(&var.name)
                        || (self.has_function(&var.name)
                            && self
                                .current_function
                                .as_ref()
                                .is_none_or(|current| !current.eq_ignore_ascii_case(&var.name)))
                    {
                        by_ref.push(ByRefTarget::None);
                        self.compile_expression(arg)?;
                    } else {
                        let var_idx = self.get_var_index(&var.name);
                        by_ref.push(ByRefTarget::Global(var_idx));
                        self.compile_expression(arg)?;
                    }
                }
                Expression::ArrayAccess { name, indices, .. } => {
                    if is_builtin_function(name)
                        || self.has_function(name)
                        || name.to_uppercase().starts_with("FN")
                    {
                        by_ref.push(ByRefTarget::None);
                        self.compile_expression(arg)?;
                    } else {
                        let mut index_slots = Vec::with_capacity(indices.len());
                        for (index_pos, index_expr) in indices.iter().enumerate() {
                            self.compile_expression(index_expr)?;
                            let temp_name = format!(
                                "__byref_{}_{}_{}_{}",
                                name,
                                self.bytecode.len(),
                                arg_index,
                                index_pos
                            );
                            let slot = self.get_var_index(&temp_name);
                            self.bytecode.push(OpCode::StoreFast(slot));
                            index_slots.push(slot);
                        }
                        for slot in &index_slots {
                            self.bytecode.push(OpCode::LoadFast(*slot));
                        }
                        self.bytecode
                            .push(OpCode::ArrayLoad(name.clone(), indices.len()));
                        by_ref.push(ByRefTarget::ArrayElement {
                            name: name.clone(),
                            index_slots,
                        });
                    }
                }
                _ => {
                    by_ref.push(ByRefTarget::None);
                    self.compile_expression(arg)?;
                }
            }
        }
        Ok(by_ref)
    }

    pub fn compile(&mut self) -> QResult<Vec<OpCode>> {
        validate_program(&self.program, Backend::Vm)?;
        self.validate_declared_signature_matches_definition()?;

        // Reserve space for InitGlobals instruction
        self.bytecode.push(OpCode::NoOp);

        // First, collect all DATA statements from the program
        let mut all_data_stmts = Vec::new();
        let mut pending_restore_targets = Vec::new();
        Self::collect_data_from_stmts(
            &self.program.statements,
            &mut all_data_stmts,
            &mut pending_restore_targets,
            &mut self.restore_targets,
        );
        let mut sub_names: Vec<_> = self.program.subs.keys().cloned().collect();
        sub_names.sort();
        for name in sub_names {
            let Some(sub_def) = self.program.subs.get(&name) else {
                continue;
            };
            Self::collect_data_from_stmts(
                &sub_def.body,
                &mut all_data_stmts,
                &mut pending_restore_targets,
                &mut self.restore_targets,
            );
        }
        let mut function_names: Vec<_> = self.program.functions.keys().cloned().collect();
        function_names.sort();
        for name in function_names {
            let Some(func_def) = self.program.functions.get(&name) else {
                continue;
            };
            Self::collect_data_from_stmts(
                &func_def.body,
                &mut all_data_stmts,
                &mut pending_restore_targets,
                &mut self.restore_targets,
            );
        }
        let end_of_data = all_data_stmts.len();
        for target in pending_restore_targets.drain(..) {
            self.restore_targets.entry(target).or_insert(end_of_data);
        }

        // Emit Data opcodes at the beginning
        for data_values in all_data_stmts {
            self.bytecode.push(OpCode::Data(data_values));
        }

        self.compile_procedure_definitions()?;

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

    fn compile_procedure_definitions(&mut self) -> QResult<()> {
        let mut function_names: Vec<_> = self.program.functions.keys().cloned().collect();
        function_names.sort();
        for name in function_names {
            let Some(func_def) = self.program.functions.get(&name).cloned() else {
                continue;
            };
            self.compile_function_definition(&func_def)?;
        }

        let mut sub_names: Vec<_> = self.program.subs.keys().cloned().collect();
        sub_names.sort();
        for name in sub_names {
            let Some(sub_def) = self.program.subs.get(&name).cloned() else {
                continue;
            };
            self.compile_sub_definition(&sub_def)?;
        }

        Ok(())
    }

    fn compile_function_definition(
        &mut self,
        func_def: &syntax_tree::ast_nodes::FunctionDef,
    ) -> QResult<()> {
        let params = func_def
            .params
            .iter()
            .map(|param| self.get_var_index(&param.name))
            .collect::<Vec<_>>();
        let function_index = self.get_var_index(&func_def.name);
        let define_idx = self.bytecode.len();
        self.bytecode.push(OpCode::NoOp);
        let body_start = self.bytecode.len();
        let previous_function = self.current_function.replace(func_def.name.clone());
        for param in &func_def.params {
            let slot = self.get_var_index(&param.name);
            if let Some(declared_type) = &param.declared_type {
                self.register_udt_field_slots(&param.name, declared_type);
            }
            self.register_scalar_storage_metadata(param, slot);
        }
        if matches!(func_def.return_type, QType::String(_)) {
            if let Some(width) = func_def.return_fixed_length {
                self.bytecode.push(OpCode::SetStringWidth {
                    slot: function_index,
                    width,
                });
            }
        } else {
            self.register_numeric_slot_type(function_index, &func_def.return_type);
        }
        for stmt in &func_def.body {
            self.compile_statement(stmt)?;
        }
        self.bytecode.push(OpCode::LoadFast(function_index));
        self.bytecode.push(OpCode::FunctionReturn);
        self.current_function = previous_function;
        let body_end = self.bytecode.len();
        self.bytecode[define_idx] = OpCode::DefineFunction {
            name: Self::normalize_proc_name(&func_def.name),
            params,
            result: function_index,
            body_start,
            body_end,
        };
        Ok(())
    }

    fn compile_sub_definition(&mut self, sub_def: &syntax_tree::ast_nodes::SubDef) -> QResult<()> {
        let params = sub_def
            .params
            .iter()
            .map(|param| self.get_var_index(&param.name))
            .collect::<Vec<_>>();
        let define_idx = self.bytecode.len();
        self.bytecode.push(OpCode::NoOp);
        let body_start = self.bytecode.len();
        let previous_function = self.current_function.take();
        for param in &sub_def.params {
            let slot = self.get_var_index(&param.name);
            if let Some(declared_type) = &param.declared_type {
                self.register_udt_field_slots(&param.name, declared_type);
            }
            self.register_scalar_storage_metadata(param, slot);
        }
        for stmt in &sub_def.body {
            self.compile_statement(stmt)?;
        }
        self.bytecode.push(OpCode::SubReturn);
        self.current_function = previous_function;
        let body_end = self.bytecode.len();
        self.bytecode[define_idx] = OpCode::DefineSub {
            name: Self::normalize_proc_name(&sub_def.name),
            params,
            body_start,
            body_end,
        };
        Ok(())
    }

    fn collect_data_from_stmts(
        stmts: &[Statement],
        data: &mut Vec<Vec<QType>>,
        pending_restore_targets: &mut Vec<String>,
        restore_targets: &mut HashMap<String, usize>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::Label { name } => {
                    pending_restore_targets.push(Self::normalize_label(name));
                }
                Statement::LineNumber { number } => {
                    pending_restore_targets.push(number.to_string());
                }
                Statement::Data { values } => {
                    let section_index = data.len();
                    for target in pending_restore_targets.drain(..) {
                        restore_targets.entry(target).or_insert(section_index);
                    }
                    data.push(values.iter().map(|v| QType::String(v.clone())).collect());
                }
                Statement::IfBlock {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    Self::collect_data_from_stmts(
                        then_branch,
                        data,
                        pending_restore_targets,
                        restore_targets,
                    );
                    if let Some(else_br) = else_branch {
                        Self::collect_data_from_stmts(
                            else_br,
                            data,
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
                    Self::collect_data_from_stmts(
                        then_branch,
                        data,
                        pending_restore_targets,
                        restore_targets,
                    );
                    for (_, branch) in else_ifs {
                        Self::collect_data_from_stmts(
                            branch,
                            data,
                            pending_restore_targets,
                            restore_targets,
                        );
                    }
                    if let Some(branch) = else_branch {
                        Self::collect_data_from_stmts(
                            branch,
                            data,
                            pending_restore_targets,
                            restore_targets,
                        );
                    }
                }
                Statement::ForLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::DoLoop { body, .. }
                | Statement::ForEach { body, .. } => Self::collect_data_from_stmts(
                    body,
                    data,
                    pending_restore_targets,
                    restore_targets,
                ),
                Statement::Select { cases, .. } => {
                    for (_, body) in cases {
                        Self::collect_data_from_stmts(
                            body,
                            data,
                            pending_restore_targets,
                            restore_targets,
                        );
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

        for (idx, target) in &self.pending_on_errors {
            let addr = match target {
                GotoTarget::Label(label) => self.labels.get(label).copied(),
                GotoTarget::LineNumber(line) => self.line_numbers.get(&line.to_string()).copied(),
            };
            if let Some(addr) = addr {
                if let Some(OpCode::OnError(_)) = self.bytecode.get_mut(*idx) {
                    self.bytecode[*idx] = OpCode::OnError(addr);
                }
            }
        }

        for (idx, label) in &self.pending_on_timers {
            if let Some(&addr) = self.labels.get(label) {
                if let Some(OpCode::OnTimer { handler, .. }) = self.bytecode.get_mut(*idx) {
                    *handler = addr;
                }
            }
        }

        for (idx, label) in &self.pending_on_plays {
            if let Some(&addr) = self.labels.get(label) {
                match self.bytecode.get_mut(*idx) {
                    Some(OpCode::OnPlay { handler, .. }) => *handler = addr,
                    Some(OpCode::OnPlayDynamic { handler }) => *handler = addr,
                    _ => {}
                }
            }
        }
    }

    fn push_loop_context(&mut self, exit_type: ExitType) {
        self.loop_contexts.push(LoopContext {
            exit_type,
            exit_jumps: Vec::new(),
        });
    }

    fn register_loop_exit(&mut self, exit_type: ExitType) -> QResult<()> {
        let jump_idx = self.bytecode.len();
        self.bytecode.push(OpCode::Jump(0));

        if let Some(context) = self
            .loop_contexts
            .iter_mut()
            .rev()
            .find(|context| context.exit_type == exit_type)
        {
            context.exit_jumps.push(jump_idx);
            Ok(())
        } else {
            Err(core_types::QError::Internal(format!(
                "EXIT {:?} used outside matching block",
                exit_type
            )))
        }
    }

    fn patch_loop_exits(&mut self, exit_type: ExitType, end_addr: usize) {
        if let Some(context) = self.loop_contexts.pop() {
            debug_assert!(context.exit_type == exit_type);
            for jump_idx in context.exit_jumps {
                if let Some(OpCode::Jump(addr)) = self.bytecode.get_mut(jump_idx) {
                    *addr = end_addr;
                }
            }
        }
    }

    fn compile_statement(&mut self, stmt: &Statement) -> QResult<()> {
        match stmt {
            Statement::Print {
                expressions,
                separators,
                newline,
            } => {
                for (index, expr) in expressions.iter().enumerate() {
                    if let Some(arg) = Self::print_control_argument(expr, "TAB") {
                        self.compile_expression(arg)?;
                        self.bytecode.push(OpCode::PrintTab);
                    } else if let Some(arg) = Self::print_control_argument(expr, "SPC") {
                        self.compile_expression(arg)?;
                        self.bytecode.push(OpCode::PrintSpace);
                    } else {
                        self.compile_expression(expr)?;
                        self.bytecode.push(OpCode::Print);
                    }

                    if matches!(
                        separators.get(index),
                        Some(Some(syntax_tree::ast_nodes::PrintSeparator::Comma))
                    ) {
                        self.bytecode.push(OpCode::PrintComma);
                    }
                }
                if *newline {
                    self.bytecode.push(OpCode::PrintNewline);
                }
            }

            Statement::LPrint {
                expressions,
                separators,
                newline,
            } => {
                for (index, expr) in expressions.iter().enumerate() {
                    if let Some(arg) = Self::print_control_argument(expr, "TAB") {
                        self.compile_expression(arg)?;
                        self.bytecode.push(OpCode::LPrintTab);
                    } else if let Some(arg) = Self::print_control_argument(expr, "SPC") {
                        self.compile_expression(arg)?;
                        self.bytecode.push(OpCode::LPrintSpace);
                    } else {
                        self.compile_expression(expr)?;
                        self.bytecode.push(OpCode::LPrint);
                    }

                    if matches!(
                        separators.get(index),
                        Some(Some(syntax_tree::ast_nodes::PrintSeparator::Comma))
                    ) {
                        self.bytecode.push(OpCode::LPrintComma);
                    }
                }
                if *newline {
                    self.bytecode.push(OpCode::LPrintNewline);
                }
            }

            Statement::PrintFile {
                file_number,
                expressions,
                separators,
                newline,
            } => {
                for (index, expr) in expressions.iter().enumerate() {
                    self.compile_expression(file_number)?; // Push file number
                    self.compile_expression(expr)?; // Push value
                    self.bytecode.push(OpCode::PrintFileDynamic);

                    if matches!(
                        separators.get(index),
                        Some(Some(syntax_tree::ast_nodes::PrintSeparator::Comma))
                    ) {
                        self.compile_expression(file_number)?;
                        self.bytecode.push(OpCode::PrintFileCommaDynamic);
                    }
                }

                if *newline {
                    self.compile_expression(file_number)?;
                    self.bytecode.push(OpCode::PrintFileNewlineDynamic);
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

                let asc_assign_args = match target {
                    Expression::FunctionCall(func) if func.name.eq_ignore_ascii_case("ASC") => {
                        Some(func.args.clone())
                    }
                    Expression::ArrayAccess { name, indices, .. }
                        if name.eq_ignore_ascii_case("ASC") =>
                    {
                        Some(indices.clone())
                    }
                    _ => None,
                };

                if let Some(args) = asc_assign_args {
                    if args.len() >= 2 {
                        self.compile_expression(&args[0])?;
                        self.compile_expression(&args[1])?;
                        self.compile_expression(value)?;
                        self.bytecode.push(OpCode::AscAssign);
                        self.compile_store_target(&args[0])?;
                        return Ok(());
                    }
                }

                // Regular assignment
                self.compile_expression(value)?;
                self.compile_store_target(target)?;
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
            Statement::IfElseBlock {
                condition,
                then_branch,
                else_ifs,
                else_branch,
            } => {
                let mut end_jumps = Vec::new();

                self.compile_expression(condition)?;
                let mut next_branch_idx = self.bytecode.len();
                self.bytecode.push(OpCode::JumpIfFalse(0));

                for s in then_branch {
                    self.compile_statement(s)?;
                }

                end_jumps.push(self.bytecode.len());
                self.bytecode.push(OpCode::Jump(0));

                let mut branch_start = self.bytecode.len();
                if let Some(OpCode::JumpIfFalse(addr)) = self.bytecode.get_mut(next_branch_idx) {
                    *addr = branch_start;
                }

                for (else_if_cond, else_if_body) in else_ifs {
                    self.compile_expression(else_if_cond)?;
                    next_branch_idx = self.bytecode.len();
                    self.bytecode.push(OpCode::JumpIfFalse(0));

                    for s in else_if_body {
                        self.compile_statement(s)?;
                    }

                    end_jumps.push(self.bytecode.len());
                    self.bytecode.push(OpCode::Jump(0));

                    branch_start = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(addr)) = self.bytecode.get_mut(next_branch_idx)
                    {
                        *addr = branch_start;
                    }
                }

                if let Some(branch) = else_branch {
                    for s in branch {
                        self.compile_statement(s)?;
                    }
                }

                let end_addr = self.bytecode.len();
                for jump_idx in end_jumps {
                    if let Some(OpCode::Jump(addr)) = self.bytecode.get_mut(jump_idx) {
                        *addr = end_addr;
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
                self.push_loop_context(ExitType::For);

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
                self.register_scalar_storage_metadata(variable, var_idx);
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
                self.patch_loop_exits(ExitType::For, end_addr);

                // Patch ForInit end_label
                if let Some(OpCode::ForInitFast { end_label, .. }) =
                    self.bytecode.get_mut(for_init_idx)
                {
                    *end_label = end_addr;
                }
            }

            Statement::ForEach {
                variable,
                array,
                body,
            } => {
                let Some(array_name) = self.expr_to_var_name(array) else {
                    return Ok(());
                };

                let item_var_idx = self.get_var_index(&variable.name);
                self.register_scalar_storage_metadata(variable, item_var_idx);
                let index_var_idx = self.get_var_index(&format!("__FOREACH_IDX_{}", item_var_idx));
                let end_var_idx = self.get_var_index(&format!("__FOREACH_END_{}", item_var_idx));

                self.bytecode.push(OpCode::LoadConstant(QType::Integer(0)));
                self.bytecode.push(OpCode::StoreFast(index_var_idx));
                self.bytecode.push(OpCode::UBound(array_name.clone(), 0));
                self.bytecode.push(OpCode::StoreFast(end_var_idx));

                let loop_start = self.bytecode.len();
                self.bytecode.push(OpCode::LoadFast(index_var_idx));
                self.bytecode.push(OpCode::LoadFast(end_var_idx));
                self.bytecode.push(OpCode::GreaterThan);
                let exit_jump = self.bytecode.len();
                self.bytecode.push(OpCode::JumpIfTrue(0));

                self.bytecode.push(OpCode::LoadFast(index_var_idx));
                self.bytecode.push(OpCode::ArrayLoad(array_name, 1));
                self.bytecode.push(OpCode::StoreFast(item_var_idx));

                for stmt in body {
                    self.compile_statement(stmt)?;
                }

                self.bytecode.push(OpCode::LoadFast(index_var_idx));
                self.bytecode.push(OpCode::LoadConstant(QType::Integer(1)));
                self.bytecode.push(OpCode::Add);
                self.bytecode.push(OpCode::StoreFast(index_var_idx));
                self.bytecode.push(OpCode::Jump(loop_start));

                let loop_end = self.bytecode.len();
                if let Some(OpCode::JumpIfTrue(target)) = self.bytecode.get_mut(exit_jump) {
                    *target = loop_end;
                }
            }

            Statement::WhileLoop { condition, body } => {
                self.push_loop_context(ExitType::While);
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
                self.patch_loop_exits(ExitType::While, end_idx);
            }

            Statement::DoLoop {
                condition,
                body,
                pre_condition,
            } => {
                self.push_loop_context(ExitType::Do);

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
                self.patch_loop_exits(ExitType::Do, end_idx);

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
                        let normalized = Self::normalize_label(name);
                        if let Some(&addr) = self.labels.get(&normalized) {
                            addr
                        } else {
                            // Label not yet defined, add to pending
                            let idx = self.bytecode.len();
                            self.pending_jumps.push((idx, normalized));
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
                        let normalized = Self::normalize_label(name);
                        if let Some(&addr) = self.labels.get(&normalized) {
                            addr
                        } else {
                            // Label not yet defined, add to pending
                            let idx = self.bytecode.len();
                            self.pending_gosubs.push((idx, normalized));
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
                self.labels
                    .insert(Self::normalize_label(name), self.bytecode.len());
            }

            Statement::LineNumber { number } => {
                self.line_numbers
                    .insert(number.to_string(), self.bytecode.len());
                self.bytecode.push(OpCode::SetCurrentLine(*number));
            }

            Statement::Screen { mode } => {
                if let Some(expr) = mode {
                    if let Some(mode) = self.try_expr_to_i32(expr) {
                        self.bytecode.push(OpCode::Screen(mode));
                    } else {
                        self.compile_expression(expr)?;
                        self.bytecode.push(OpCode::ScreenDynamic);
                    }
                } else {
                    self.bytecode.push(OpCode::Screen(0));
                }
            }

            Statement::Pset { coords, color } => {
                if let (Some(x), Some(y), Some(c)) = (
                    self.try_expr_to_i32(&coords.0),
                    self.try_expr_to_i32(&coords.1),
                    color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(0)),
                ) {
                    self.bytecode.push(OpCode::Pset { x, y, color: c });
                } else {
                    self.compile_expression(&coords.0)?;
                    self.compile_expression(&coords.1)?;
                    self.compile_optional_expression(color.as_ref(), QType::Integer(0))?;
                    self.bytecode.push(OpCode::PsetDynamic);
                }
            }

            Statement::Preset { coords, color } => {
                if let (Some(x), Some(y), Some(c)) = (
                    self.try_expr_to_i32(&coords.0),
                    self.try_expr_to_i32(&coords.1),
                    color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(0)),
                ) {
                    self.bytecode.push(OpCode::Preset { x, y, color: c });
                } else {
                    self.compile_expression(&coords.0)?;
                    self.compile_expression(&coords.1)?;
                    self.compile_optional_expression(color.as_ref(), QType::Integer(0))?;
                    self.bytecode.push(OpCode::PresetDynamic);
                }
            }

            Statement::Line { coords, color, .. } => {
                if let (Some(x1), Some(y1), Some(x2), Some(y2), Some(c)) = (
                    self.try_expr_to_i32(&coords.0 .0),
                    self.try_expr_to_i32(&coords.0 .1),
                    self.try_expr_to_i32(&coords.1 .0),
                    self.try_expr_to_i32(&coords.1 .1),
                    color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(0)),
                ) {
                    self.bytecode.push(OpCode::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        color: c,
                    });
                } else {
                    self.compile_expression(&coords.0 .0)?;
                    self.compile_expression(&coords.0 .1)?;
                    self.compile_expression(&coords.1 .0)?;
                    self.compile_expression(&coords.1 .1)?;
                    self.compile_optional_expression(color.as_ref(), QType::Integer(0))?;
                    self.bytecode.push(OpCode::LineDynamic);
                }
            }

            Statement::Circle {
                center,
                radius,
                color,
                ..
            } => {
                if let (Some(x), Some(y), Some(r), Some(c)) = (
                    self.try_expr_to_i32(&center.0),
                    self.try_expr_to_i32(&center.1),
                    self.try_expr_to_i32(radius),
                    color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(0)),
                ) {
                    self.bytecode.push(OpCode::Circle {
                        x,
                        y,
                        radius: r,
                        color: c,
                    });
                } else {
                    self.compile_expression(&center.0)?;
                    self.compile_expression(&center.1)?;
                    self.compile_expression(radius)?;
                    self.compile_optional_expression(color.as_ref(), QType::Integer(0))?;
                    self.bytecode.push(OpCode::CircleDynamic);
                }
            }

            Statement::Sound {
                frequency,
                duration,
            } => {
                if let (Some(frequency), Some(duration)) = (
                    self.try_expr_to_i32(frequency),
                    self.try_expr_to_i32(duration),
                ) {
                    self.bytecode.push(OpCode::Sound {
                        frequency,
                        duration,
                    });
                } else {
                    self.compile_expression(frequency)?;
                    self.compile_expression(duration)?;
                    self.bytecode.push(OpCode::SoundDynamic);
                }
            }

            Statement::Play {
                melody: Expression::Literal(QType::String(s)),
            } => {
                self.bytecode.push(OpCode::Play(s.clone()));
            }
            Statement::Play { melody } => {
                self.compile_expression(melody)?;
                self.bytecode.push(OpCode::PlayDynamic);
            }

            Statement::Beep => {
                self.bytecode.push(OpCode::Beep);
            }

            Statement::Cls { mode } => {
                if let Some(mode_expr) = mode {
                    if let Some(mode) = self.try_expr_to_i32(mode_expr) {
                        self.bytecode.push(OpCode::Cls(mode));
                    } else {
                        self.compile_expression(mode_expr)?;
                        self.bytecode.push(OpCode::ClsDynamic);
                    }
                } else {
                    self.bytecode.push(OpCode::Cls(-1));
                }
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
                    self.bytecode.push(OpCode::RandomizeDynamic);
                } else {
                    self.bytecode.push(OpCode::Randomize);
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
                for (var, dimensions) in variables {
                    if let Some(dimensions) = dimensions {
                        self.register_array_storage_metadata(var);
                        self.compile_array_dimensions(&var.name, dimensions, false)?;
                    } else {
                        if let Some(declared_type) = &var.declared_type {
                            self.register_udt_field_slots(&var.name, declared_type);
                        }
                        // Simple variable
                        let var_idx = self.get_var_index(&var.name);
                        self.register_scalar_storage_metadata(var, var_idx);
                        self.bytecode
                            .push(OpCode::LoadConstant(Self::zero_value_for_qtype(
                                &self.variable_type_hint(var),
                            )));
                        self.bytecode.push(OpCode::StoreFast(var_idx));
                    }
                }
            }

            Statement::Redim {
                variables,
                preserve,
            } => {
                for (var, dimensions) in variables {
                    self.register_array_storage_metadata(var);
                    let dimensions = dimensions.as_ref().ok_or_else(|| {
                        core_types::QError::Syntax(
                            "REDIM requires at least one array dimension".to_string(),
                        )
                    })?;
                    self.compile_array_dimensions(&var.name, dimensions, *preserve)?;
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
                separators,
                newline,
            } => {
                self.compile_expression(format)?;
                for expr in expressions {
                    self.compile_expression(expr)?;
                }
                self.bytecode.push(OpCode::PrintUsing {
                    count: expressions.len(),
                    comma_after: Self::comma_after(separators),
                });
                if *newline {
                    self.bytecode.push(OpCode::PrintNewline);
                }
            }

            Statement::LPrintUsing {
                format,
                expressions,
                separators,
                newline,
            } => {
                self.compile_expression(format)?;
                for expr in expressions {
                    self.compile_expression(expr)?;
                }
                self.bytecode.push(OpCode::LPrintUsing {
                    count: expressions.len(),
                    comma_after: Self::comma_after(separators),
                });
                if *newline {
                    self.bytecode.push(OpCode::LPrintNewline);
                }
            }

            Statement::PrintFileUsing {
                file_number,
                format,
                expressions,
                separators,
                newline,
            } => {
                self.compile_expression(file_number)?;
                self.compile_expression(format)?;
                for expr in expressions {
                    self.compile_expression(expr)?;
                }
                self.bytecode.push(OpCode::PrintFileUsingDynamic {
                    count: expressions.len(),
                    comma_after: Self::comma_after(separators),
                });
                if *newline {
                    self.compile_expression(file_number)?;
                    self.bytecode.push(OpCode::PrintFileNewlineDynamic);
                }
            }

            Statement::OnError { target } => {
                let addr = match target {
                    Some(GotoTarget::Label(name)) => {
                        let normalized = Self::normalize_label(name);
                        if let Some(&addr) = self.labels.get(&normalized) {
                            addr
                        } else {
                            let idx = self.bytecode.len();
                            self.pending_on_errors
                                .push((idx, GotoTarget::Label(normalized)));
                            0
                        }
                    }
                    Some(GotoTarget::LineNumber(line)) => {
                        if let Some(&addr) = self.line_numbers.get(&line.to_string()) {
                            addr
                        } else {
                            let idx = self.bytecode.len();
                            self.pending_on_errors
                                .push((idx, GotoTarget::LineNumber(*line)));
                            0
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
                let addr = self
                    .labels
                    .get(&Self::normalize_label(label))
                    .copied()
                    .unwrap_or(0);
                self.bytecode.push(OpCode::ResumeLabel(addr));
            }

            Statement::Paint {
                coords,
                paint_color,
                border_color,
            } => {
                if let (Some(x), Some(y), Some(pc), Some(bc)) = (
                    self.try_expr_to_i32(&coords.0),
                    self.try_expr_to_i32(&coords.1),
                    paint_color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(-1)),
                    border_color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(-1)),
                ) {
                    self.bytecode.push(OpCode::Paint {
                        x,
                        y,
                        paint_color: pc,
                        border_color: bc,
                    });
                } else {
                    self.compile_expression(&coords.0)?;
                    self.compile_expression(&coords.1)?;
                    self.compile_optional_expression(paint_color.as_ref(), QType::Integer(-1))?;
                    self.compile_optional_expression(border_color.as_ref(), QType::Integer(-1))?;
                    self.bytecode.push(OpCode::PaintDynamic);
                }
            }

            Statement::Draw {
                commands: Expression::Literal(QType::String(s)),
            } => {
                self.bytecode.push(OpCode::Draw {
                    commands: s.clone(),
                });
            }
            Statement::Draw { commands } => {
                self.compile_expression(commands)?;
                self.bytecode.push(OpCode::DrawDynamic);
            }

            Statement::Palette { attribute, color } => {
                if let (Some(attribute), Some(color)) = (
                    self.try_expr_to_i32(attribute),
                    color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(-1)),
                ) {
                    self.bytecode.push(OpCode::Palette { attribute, color });
                } else {
                    self.compile_expression(attribute)?;
                    self.compile_optional_expression(color.as_ref(), QType::Integer(-1))?;
                    self.bytecode.push(OpCode::PaletteDynamic);
                }
            }

            Statement::View {
                coords,
                fill_color,
                border_color,
            } => {
                if let (Some(x1), Some(y1), Some(x2), Some(y2), Some(fc), Some(bc)) = (
                    self.try_expr_to_i32(&coords.0 .0),
                    self.try_expr_to_i32(&coords.0 .1),
                    self.try_expr_to_i32(&coords.1 .0),
                    self.try_expr_to_i32(&coords.1 .1),
                    fill_color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(-1)),
                    border_color
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(-1)),
                ) {
                    self.bytecode.push(OpCode::View {
                        x1,
                        y1,
                        x2,
                        y2,
                        fill_color: fc,
                        border_color: bc,
                    });
                } else {
                    self.compile_expression(&coords.0 .0)?;
                    self.compile_expression(&coords.0 .1)?;
                    self.compile_expression(&coords.1 .0)?;
                    self.compile_expression(&coords.1 .1)?;
                    self.compile_optional_expression(fill_color.as_ref(), QType::Integer(-1))?;
                    self.compile_optional_expression(border_color.as_ref(), QType::Integer(-1))?;
                    self.bytecode.push(OpCode::ViewDynamic);
                }
            }

            Statement::ViewPrint { top, bottom } => {
                if let (Some(top), Some(bottom)) = (
                    top.as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(1)),
                    bottom
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(25)),
                ) {
                    self.bytecode.push(OpCode::ViewPrint { top, bottom });
                } else {
                    self.compile_optional_expression(top.as_ref(), QType::Integer(1))?;
                    self.compile_optional_expression(bottom.as_ref(), QType::Integer(25))?;
                    self.bytecode.push(OpCode::ViewPrintDynamic);
                }
            }

            Statement::ViewReset => {
                self.bytecode.push(OpCode::ViewReset);
            }

            Statement::Window { coords } => {
                if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                    self.try_expr_to_f64(&coords.0 .0),
                    self.try_expr_to_f64(&coords.0 .1),
                    self.try_expr_to_f64(&coords.1 .0),
                    self.try_expr_to_f64(&coords.1 .1),
                ) {
                    self.bytecode.push(OpCode::Window { x1, y1, x2, y2 });
                } else {
                    self.compile_expression(&coords.0 .0)?;
                    self.compile_expression(&coords.0 .1)?;
                    self.compile_expression(&coords.1 .0)?;
                    self.compile_expression(&coords.1 .1)?;
                    self.bytecode.push(OpCode::WindowDynamic);
                }
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
                    self.register_scalar_storage_metadata(var, var_idx);
                    self.bytecode.push(OpCode::ReadFast(var_idx));
                }
            }

            Statement::Restore { label } => {
                let section = if let Some(label_name) = label.as_deref() {
                    let target = Self::normalize_restore_target(label_name);
                    Some(*self.restore_targets.get(&target).ok_or_else(|| {
                        core_types::QError::Syntax(format!(
                            "RESTORE target not found: {}",
                            label_name
                        ))
                    })?)
                } else {
                    None
                };
                self.bytecode.push(OpCode::Restore(section));
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

            Statement::Clear => {
                self.bytecode.push(OpCode::Clear);
            }

            Statement::Locate {
                row,
                col,
                cursor,
                start,
                stop,
            } => {
                if let (Some(row), Some(col)) = (
                    row.as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(0)),
                    col.as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(0)),
                ) {
                    self.bytecode.push(OpCode::Locate(row, col));
                } else {
                    self.compile_optional_expression(row.as_ref(), QType::Integer(0))?;
                    self.compile_optional_expression(col.as_ref(), QType::Integer(0))?;
                    self.bytecode.push(OpCode::LocateDynamic);
                }

                if cursor.is_some() || start.is_some() || stop.is_some() {
                    if let (Some(visible), Some(start), Some(stop)) = (
                        cursor
                            .as_ref()
                            .map(|expr| self.try_expr_to_i32(expr))
                            .unwrap_or(Some(-1)),
                        start
                            .as_ref()
                            .map(|expr| self.try_expr_to_i32(expr))
                            .unwrap_or(Some(-1)),
                        stop.as_ref()
                            .map(|expr| self.try_expr_to_i32(expr))
                            .unwrap_or(Some(-1)),
                    ) {
                        self.bytecode.push(OpCode::SetCursorState {
                            visible,
                            start,
                            stop,
                        });
                    } else {
                        self.compile_optional_expression(cursor.as_ref(), QType::Integer(-1))?;
                        self.compile_optional_expression(start.as_ref(), QType::Integer(-1))?;
                        self.compile_optional_expression(stop.as_ref(), QType::Integer(-1))?;
                        self.bytecode.push(OpCode::SetCursorStateDynamic);
                    }
                }
            }

            Statement::Color {
                foreground,
                background,
            } => {
                if let (Some(fg), Some(bg)) = (
                    foreground
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(7)),
                    background
                        .as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(0)),
                ) {
                    self.bytecode.push(OpCode::Color(fg, bg));
                } else {
                    self.compile_optional_expression(foreground.as_ref(), QType::Integer(7))?;
                    self.compile_optional_expression(background.as_ref(), QType::Integer(0))?;
                    self.bytecode.push(OpCode::ColorDynamic);
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
                self.bytecode.push(OpCode::Input);
                self.compile_store_target(variable)?;
            }

            Statement::InputFile {
                file_number,
                variables,
            } => {
                self.compile_expression(file_number)?;
                self.bytecode
                    .push(OpCode::InputFileDynamic(variables.len()));
                for variable in variables {
                    self.compile_store_target(variable)?;
                }
            }

            Statement::Write { expressions } => {
                for expr in expressions {
                    self.compile_expression(expr)?;
                }
                self.bytecode.push(OpCode::WriteConsole(expressions.len()));
            }

            Statement::WriteFile {
                file_number,
                expressions,
            } => {
                self.compile_expression(file_number)?;
                for expr in expressions {
                    self.compile_expression(expr)?;
                }
                self.bytecode
                    .push(OpCode::WriteFileDynamic(expressions.len()));
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
                if let Some(var) = variable {
                    if let Some((kind, fixed_length)) = self.binary_file_target_metadata(var) {
                        self.bytecode.push(OpCode::GetBinary { kind, fixed_length });
                    } else {
                        self.bytecode.push(OpCode::Get);
                    }
                    self.compile_store_target(var)?;
                } else {
                    self.bytecode.push(OpCode::Get);
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
                    if let Some((kind, fixed_length)) = self.binary_file_target_metadata(var) {
                        self.bytecode.push(OpCode::PutBinary { kind, fixed_length });
                    } else {
                        self.bytecode.push(OpCode::Put);
                    }
                } else {
                    self.bytecode.push(OpCode::LoadConstant(QType::Empty));
                    self.bytecode.push(OpCode::Put);
                }
            }

            Statement::GetImage { coords, variable } => {
                let array = self.expr_to_var_name(variable).unwrap_or_default();
                if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                    self.try_expr_to_i32(&coords.0 .0),
                    self.try_expr_to_i32(&coords.0 .1),
                    self.try_expr_to_i32(&coords.1 .0),
                    self.try_expr_to_i32(&coords.1 .1),
                ) {
                    self.bytecode.push(OpCode::GetImage {
                        x1,
                        y1,
                        x2,
                        y2,
                        array,
                    });
                } else {
                    self.compile_expression(&coords.0 .0)?;
                    self.compile_expression(&coords.0 .1)?;
                    self.compile_expression(&coords.1 .0)?;
                    self.compile_expression(&coords.1 .1)?;
                    self.bytecode.push(OpCode::GetImageDynamic { array });
                }
            }

            Statement::PutImage {
                coords,
                variable,
                action,
            } => {
                let array = self.expr_to_var_name(variable).unwrap_or_default();
                if let (Some(x), Some(y), Some(action_text)) = (
                    self.try_expr_to_i32(&coords.0),
                    self.try_expr_to_i32(&coords.1),
                    action
                        .as_ref()
                        .map(|expr| self.try_expr_to_string(expr))
                        .unwrap_or_else(|| Some("PSET".to_string())),
                ) {
                    self.bytecode.push(OpCode::PutImage {
                        x,
                        y,
                        array,
                        action: action_text,
                    });
                } else {
                    self.compile_expression(&coords.0)?;
                    self.compile_expression(&coords.1)?;
                    self.compile_optional_expression(
                        action.as_ref(),
                        QType::String("PSET".to_string()),
                    )?;
                    self.bytecode.push(OpCode::PutImageDynamic { array });
                }
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
                let normalized_name = Self::normalize_proc_name(name);
                let param_modes = self.require_param_modes(name, args.len(), false)?;
                let by_ref = self.compile_call_arguments(args, Some(param_modes.as_slice()))?;
                self.bytecode.push(OpCode::CallSub {
                    name: normalized_name,
                    by_ref,
                });
            }

            Statement::OnGotoGosub {
                expression,
                targets,
                is_gosub,
            } => {
                self.compile_expression(expression)?;
                let mut end_jumps = Vec::new();

                for (idx, target) in targets.iter().enumerate() {
                    self.bytecode.push(OpCode::Dup);
                    self.bytecode
                        .push(OpCode::LoadConstant(QType::Integer((idx + 1) as i16)));
                    self.bytecode.push(OpCode::Equal);

                    let skip_idx = self.bytecode.len();
                    self.bytecode.push(OpCode::JumpIfFalse(0));
                    self.bytecode.push(OpCode::Pop);

                    match target {
                        GotoTarget::LineNumber(line) => {
                            let addr = self
                                .line_numbers
                                .get(&line.to_string())
                                .copied()
                                .unwrap_or(0);
                            self.bytecode.push(if *is_gosub {
                                OpCode::Gosub(addr)
                            } else {
                                OpCode::Jump(addr)
                            });
                        }
                        GotoTarget::Label(name) => {
                            let normalized = Self::normalize_label(name);
                            let op_idx = self.bytecode.len();
                            if *is_gosub {
                                self.bytecode.push(OpCode::Gosub(0));
                                self.pending_gosubs.push((op_idx, normalized));
                            } else {
                                self.bytecode.push(OpCode::Jump(0));
                                self.pending_jumps.push((op_idx, normalized));
                            }
                        }
                    }

                    let end_jump = self.bytecode.len();
                    self.bytecode.push(OpCode::Jump(0));
                    end_jumps.push(end_jump);

                    let next_check = self.bytecode.len();
                    if let Some(OpCode::JumpIfFalse(addr)) = self.bytecode.get_mut(skip_idx) {
                        *addr = next_check;
                    }
                }

                self.bytecode.push(OpCode::Pop);
                let end_addr = self.bytecode.len();
                for jump_idx in end_jumps {
                    if let Some(OpCode::Jump(addr)) = self.bytecode.get_mut(jump_idx) {
                        *addr = end_addr;
                    }
                }
            }

            Statement::Declare { .. } => {}
            Statement::OptionBase { base } => match *base {
                0 | 1 => {
                    self.option_base = *base as i32;
                    self.bytecode.push(OpCode::SetOptionBase(self.option_base));
                }
                _ => {
                    return Err(core_types::QError::Syntax(
                        "OPTION BASE must be 0 or 1".to_string(),
                    ))
                }
            },
            Statement::Erase { variables } => {
                for var in variables {
                    self.bytecode.push(OpCode::Erase(var.name.clone()));
                }
            }
            Statement::Field {
                file_number,
                fields,
            } => {
                let file_num = self.expr_to_i32(file_number);
                let mut compiled_fields = Vec::with_capacity(fields.len());
                for (width_expr, field_expr) in fields {
                    let width = self.expr_to_i32(width_expr).max(0) as usize;
                    let Some(field_name) = self.expr_to_var_name(field_expr) else {
                        continue;
                    };
                    let var_index = self.get_var_index(&field_name);
                    self.field_widths.insert(var_index, width);
                    compiled_fields.push((width as i32, var_index));
                }
                self.bytecode.push(OpCode::FieldStmt {
                    file_num,
                    fields: compiled_fields,
                });
            }
            Statement::LSet { target, value } => {
                if let Some(field_name) = self.expr_to_var_name(target) {
                    let var_index = self.get_var_index(&field_name);
                    let width = self
                        .field_widths
                        .get(&var_index)
                        .copied()
                        .or_else(|| self.fixed_string_width_for_expr(target))
                        .unwrap_or(0);
                    self.compile_expression(value)?;
                    self.bytecode.push(OpCode::LSetField { var_index, width });
                }
            }
            Statement::RSet { target, value } => {
                if let Some(field_name) = self.expr_to_var_name(target) {
                    let var_index = self.get_var_index(&field_name);
                    let width = self
                        .field_widths
                        .get(&var_index)
                        .copied()
                        .or_else(|| self.fixed_string_width_for_expr(target))
                        .unwrap_or(0);
                    self.compile_expression(value)?;
                    self.bytecode.push(OpCode::RSetField { var_index, width });
                }
            }
            Statement::TrOn => {
                self.bytecode.push(OpCode::TraceOn);
            }

            Statement::TrOff => {
                self.bytecode.push(OpCode::TraceOff);
            }

            Statement::Key {
                key_num,
                key_string,
            } => {
                self.compile_expression(key_num)?;
                self.compile_expression(key_string)?;
                self.bytecode.push(OpCode::KeySetDynamic);
            }

            Statement::KeyOn => {
                self.bytecode.push(OpCode::KeyOn);
            }

            Statement::KeyOff => {
                self.bytecode.push(OpCode::KeyOff);
            }

            Statement::KeyList => {
                self.bytecode.push(OpCode::KeyList);
            }

            Statement::Width { columns, rows } => {
                if let (Some(columns), Some(rows)) = (
                    self.try_expr_to_i32(columns),
                    rows.as_ref()
                        .map(|expr| self.try_expr_to_i32(expr))
                        .unwrap_or(Some(25)),
                ) {
                    self.bytecode.push(OpCode::Width { columns, rows });
                } else {
                    self.compile_expression(columns)?;
                    self.compile_optional_expression(rows.as_ref(), QType::Integer(25))?;
                    self.bytecode.push(OpCode::WidthDynamic);
                }
            }

            Statement::Const { name, value } => {
                let qtype = self.name_type_hint(name, None);
                self.compile_expression(value)?;
                let slot = self.get_var_index(name);
                if !matches!(qtype, QType::String(_)) {
                    self.register_numeric_slot_type(slot, &qtype);
                }
                self.bytecode.push(OpCode::StoreFast(slot));
                self.bytecode.push(OpCode::MarkConst(slot));
            }

            Statement::DefType { .. } => {
                // DefType is a compile-time directive, no runtime code needed
                // The type information should be handled by the parser/analyzer
            }

            Statement::DefSeg { segment } => {
                if let Some(expr) = segment.as_deref() {
                    if let Some(seg) = self.try_expr_to_i32(expr) {
                        self.bytecode.push(OpCode::DefSeg(seg));
                    } else {
                        self.compile_expression(expr)?;
                        self.bytecode.push(OpCode::DefSegDynamic);
                    }
                } else {
                    self.bytecode.push(OpCode::DefSeg(0));
                }
            }

            Statement::Poke { address, value } => {
                self.compile_expression(address)?;
                self.compile_expression(value)?;
                self.bytecode.push(OpCode::PokeDynamic);
            }

            Statement::Wait {
                address,
                and_mask,
                xor_mask,
            } => {
                self.compile_expression(address)?;
                self.compile_expression(and_mask)?;
                if let Some(xor_mask) = xor_mask {
                    self.compile_expression(xor_mask)?;
                }
                self.bytecode.push(OpCode::WaitDynamic {
                    has_xor: xor_mask.is_some(),
                });
            }

            Statement::BLoad { filename, offset } => {
                self.compile_expression(filename)?;
                if let Some(offset) = offset {
                    self.compile_expression(offset)?;
                }
                self.bytecode.push(OpCode::BLoadDynamic {
                    has_offset: offset.is_some(),
                });
            }

            Statement::BSave {
                filename,
                offset,
                length,
            } => {
                self.compile_expression(filename)?;
                self.compile_expression(offset)?;
                self.compile_expression(length)?;
                self.bytecode.push(OpCode::BSaveDynamic);
            }

            Statement::Out { port, value } => {
                self.compile_expression(port)?;
                self.compile_expression(value)?;
                self.bytecode.push(OpCode::OutDynamic);
            }

            Statement::System => {
                self.bytecode.push(OpCode::End);
            }

            Statement::Kill { filename } => {
                self.compile_expression(filename)?;
                self.bytecode.push(OpCode::KillFile);
            }

            Statement::NameFile { old_name, new_name } => {
                self.compile_expression(old_name)?;
                self.compile_expression(new_name)?;
                self.bytecode.push(OpCode::RenameFile);
            }

            Statement::Files { pattern } => {
                if let Some(pattern) = pattern {
                    self.compile_expression(pattern)?;
                } else {
                    self.bytecode.push(OpCode::LoadConstant(QType::Empty));
                }
                self.bytecode.push(OpCode::Files);
            }

            Statement::ChDir { path } => {
                self.compile_expression(path)?;
                self.bytecode.push(OpCode::ChangeDir);
            }

            Statement::MkDir { path } => {
                self.compile_expression(path)?;
                self.bytecode.push(OpCode::MakeDir);
            }

            Statement::RmDir { path } => {
                self.compile_expression(path)?;
                self.bytecode.push(OpCode::RemoveDir);
            }

            Statement::OnTimer { interval, label } => {
                let normalized = Self::normalize_label(label);
                let handler = self.labels.get(&normalized).copied().unwrap_or(0);
                let op_idx = self.bytecode.len();
                self.bytecode.push(OpCode::OnTimer {
                    interval_secs: self.expr_to_f64(interval),
                    handler,
                });
                if handler == 0 {
                    self.pending_on_timers.push((op_idx, normalized));
                }
            }

            Statement::OnPlay { queue_limit, label } => {
                let normalized = Self::normalize_label(label);
                let handler = self.labels.get(&normalized).copied().unwrap_or(0);
                let op_idx = self.bytecode.len();
                if let Some(limit) = self.try_expr_to_i32(queue_limit) {
                    self.bytecode.push(OpCode::OnPlay {
                        queue_limit: limit.clamp(1, 32) as usize,
                        handler,
                    });
                } else {
                    self.compile_expression(queue_limit)?;
                    self.bytecode.push(OpCode::OnPlayDynamic { handler });
                }
                if handler == 0 {
                    self.pending_on_plays.push((op_idx, normalized));
                }
            }

            Statement::TimerOn => {
                self.bytecode.push(OpCode::TimerOn);
            }

            Statement::TimerOff => {
                self.bytecode.push(OpCode::TimerOff);
            }

            Statement::TimerStop => {
                self.bytecode.push(OpCode::TimerStop);
            }

            Statement::PlayOn => {
                self.bytecode.push(OpCode::PlayOn);
            }

            Statement::PlayOff => {
                self.bytecode.push(OpCode::PlayOff);
            }

            Statement::PlayStop => {
                self.bytecode.push(OpCode::PlayStop);
            }

            Statement::Exit { exit_type } => match exit_type {
                ExitType::For => self.register_loop_exit(ExitType::For)?,
                ExitType::Do => self.register_loop_exit(ExitType::Do)?,
                ExitType::While => self.register_loop_exit(ExitType::While)?,
                ExitType::Function => {
                    if let Some(current_function) = self.current_function.clone() {
                        let return_idx = self.get_var_index(&current_function);
                        self.bytecode.push(OpCode::LoadFast(return_idx));
                    } else {
                        self.bytecode.push(OpCode::LoadConstant(QType::Empty));
                    }
                    self.bytecode.push(OpCode::FunctionReturn);
                }
                ExitType::Sub => self.bytecode.push(OpCode::SubReturn),
            },

            Statement::DefFn { name, params, body } => {
                // Save current variable mapping
                let saved_var_map = self.variable_map.clone();
                let saved_var_counter = self.next_var_index;

                // Keep outer scope slots visible inside DEF FN, but bind parameters to fresh slots
                // so calls can temporarily override only those parameter locations.
                let mut param_slots = Vec::with_capacity(params.len());
                for param in params {
                    let slot = self.next_var_index;
                    self.variable_map.insert(param.clone(), slot);
                    param_slots.push(slot);
                    let qtype = self.name_type_hint(param, None);
                    if !matches!(qtype, QType::String(_)) {
                        self.register_numeric_slot_type(slot, &qtype);
                    }
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
                    param_slots,
                    body: body_ops,
                });
            }
        }

        Ok(())
    }

    fn try_expr_to_i32(&self, expr: &Expression) -> Option<i32> {
        self.try_expr_to_f64(expr).map(|value| value as i32)
    }

    fn expr_to_i32(&self, expr: &Expression) -> i32 {
        self.try_expr_to_i32(expr).unwrap_or(0)
    }

    fn compile_array_dimension_lower(&mut self, lower_bound: Option<&Expression>) -> QResult<()> {
        if let Some(lower_bound) = lower_bound {
            self.compile_expression(lower_bound)?;
        } else {
            self.bytecode.push(OpCode::LoadConstant(QType::Integer(
                self.option_base as i16,
            )));
        }
        Ok(())
    }

    fn compile_array_dimensions(
        &mut self,
        name: &str,
        dimensions: &[ArrayDimension],
        preserve: bool,
    ) -> QResult<()> {
        for dimension in dimensions {
            self.compile_array_dimension_lower(dimension.lower_bound.as_ref())?;
            self.compile_expression(&dimension.upper_bound)?;
        }

        if preserve {
            self.bytecode.push(OpCode::ArrayRedimDynamic {
                name: name.to_string(),
                dimensions: dimensions.len(),
                preserve,
            });
        } else {
            self.bytecode.push(OpCode::ArrayDimDynamic {
                name: name.to_string(),
                dimensions: dimensions.len(),
            });
        }

        Ok(())
    }

    fn try_expr_to_f64(&self, expr: &Expression) -> Option<f64> {
        match expr {
            Expression::Literal(QType::Integer(i)) => Some(*i as f64),
            Expression::Literal(QType::Long(l)) => Some(*l as f64),
            Expression::Literal(QType::Single(s)) => Some(*s as f64),
            Expression::Literal(QType::Double(d)) => Some(*d),
            Expression::UnaryOp { op, operand } => {
                let value = self.try_expr_to_f64(operand)?;
                Some(match op {
                    UnaryOp::Negate => -value,
                    UnaryOp::Not => value,
                })
            }
            _ => None,
        }
    }

    fn expr_to_f64(&self, expr: &Expression) -> f64 {
        self.try_expr_to_f64(expr).unwrap_or(0.0)
    }

    fn try_expr_to_string(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Literal(QType::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn compile_optional_expression(
        &mut self,
        expr: Option<&Expression>,
        default: QType,
    ) -> QResult<()> {
        if let Some(expr) = expr {
            self.compile_expression(expr)?;
        } else {
            self.bytecode.push(OpCode::LoadConstant(default));
        }
        Ok(())
    }

    fn expr_to_var_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(var) => Some(var.name.clone()),
            Expression::FieldAccess { .. } => Self::qualified_field_name(expr),
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
                } else if self.has_function(&var.name)
                    && self
                        .current_function
                        .as_ref()
                        .is_none_or(|current| !current.eq_ignore_ascii_case(&var.name))
                {
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
                } else if self.has_function(name) {
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
                    let upper_name = func.name.to_uppercase();
                    let custom_compiled_builtin = matches!(
                        upper_name.as_str(),
                        "LBOUND"
                            | "UBOUND"
                            | "VARPTR"
                            | "VARSEG"
                            | "SADD"
                            | "VARPTR$"
                            | "FRE"
                            | "POS"
                            | "LPOS"
                            | "POINT"
                            | "PMAP"
                            | "_CV"
                    );

                    if !custom_compiled_builtin {
                        for arg in &func.args {
                            self.compile_expression(arg)?;
                        }
                    }

                    // Generate appropriate opcode for built-in function
                    match upper_name.as_str() {
                        "LEFT$" => self.bytecode.push(OpCode::Left),
                        "RIGHT$" => self.bytecode.push(OpCode::Right),
                        "MID$" => {
                            if func.args.len() > 2 {
                                self.bytecode.push(OpCode::Mid);
                            } else {
                                self.bytecode.push(OpCode::MidNoLen);
                            }
                        }
                        "LEN" => self.bytecode.push(OpCode::Len),
                        "INSTR" => {
                            if func.args.len() > 2 {
                                self.bytecode.push(OpCode::InStrFrom);
                            } else {
                                self.bytecode.push(OpCode::InStr);
                            }
                        }
                        "LCASE$" => self.bytecode.push(OpCode::LCase),
                        "UCASE$" => self.bytecode.push(OpCode::UCase),
                        "LTRIM$" => self.bytecode.push(OpCode::LTrim),
                        "RTRIM$" => self.bytecode.push(OpCode::RTrim),
                        "TRIM$" | "_TRIM$" => self.bytecode.push(OpCode::Trim),
                        "STR$" => self.bytecode.push(OpCode::StrFunc),
                        "VAL" => self.bytecode.push(OpCode::ValFunc),
                        "CHR$" => self.bytecode.push(OpCode::ChrFunc),
                        "ASC" => {
                            if func.args.len() > 1 {
                                self.bytecode.push(OpCode::LoadConstant(QType::Integer(1)));
                                self.bytecode.push(OpCode::Mid);
                            }
                            self.bytecode.push(OpCode::AscFunc);
                        }
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
                        "RND" => {
                            if func.args.is_empty() {
                                self.bytecode.push(OpCode::Rnd);
                            } else {
                                self.bytecode.push(OpCode::RndWithArg);
                            }
                        }
                        "CINT" => self.bytecode.push(OpCode::CInt),
                        "CLNG" => self.bytecode.push(OpCode::CLng),
                        "CSNG" => self.bytecode.push(OpCode::CSng),
                        "CDBL" => self.bytecode.push(OpCode::CDbl),
                        "CSTR" => self.bytecode.push(OpCode::CStr),
                        "TIMER" => self.bytecode.push(OpCode::Timer),
                        "DATE$" => self.bytecode.push(OpCode::Date),
                        "TIME$" => self.bytecode.push(OpCode::Time),
                        "SCREEN" => self.bytecode.push(OpCode::ScreenFn(func.args.len())),
                        "PLAY" => self.bytecode.push(OpCode::PlayFunc),
                        // Binary conversion functions
                        "MKI$" => self.bytecode.push(OpCode::MkiFunc),
                        "MKL$" => self.bytecode.push(OpCode::MklFunc),
                        "MKS$" => self.bytecode.push(OpCode::MksFunc),
                        "MKD$" => self.bytecode.push(OpCode::MkdFunc),
                        "CVI" => self.bytecode.push(OpCode::CviFunc),
                        "CVL" => self.bytecode.push(OpCode::CvlFunc),
                        "CVS" => self.bytecode.push(OpCode::CvsFunc),
                        "CVD" => self.bytecode.push(OpCode::CvdFunc),
                        "_CV" => {
                            let type_name = Self::cv_type_name_from_expr(
                                func.args.first().ok_or_else(|| {
                                    core_types::QError::Syntax(
                                        "_CV requires a QB64 numeric type argument".to_string(),
                                    )
                                })?,
                            )
                            .ok_or_else(|| {
                                core_types::QError::Syntax(
                                    "_CV requires a QB64 numeric type argument".to_string(),
                                )
                            })?;
                            let value_expr = func.args.get(1).ok_or_else(|| {
                                core_types::QError::Syntax(
                                    "_CV requires a binary string argument".to_string(),
                                )
                            })?;
                            self.compile_expression(value_expr)?;
                            self.bytecode.push(OpCode::CvFunc(type_name));
                        }
                        "_FILEEXISTS" => self.bytecode.push(OpCode::FileExistsFunc),
                        "_DIREXISTS" => self.bytecode.push(OpCode::DirExistsFunc),
                        // System functions
                        "FRE" => match func.args.first() {
                            Some(arg) if self.try_expr_to_string(arg).is_some() => {
                                self.bytecode.push(OpCode::FreFunc(0));
                            }
                            Some(arg) if self.try_expr_to_i32(arg).is_some() => {
                                self.bytecode
                                    .push(OpCode::FreFunc(self.try_expr_to_i32(arg).unwrap()));
                            }
                            Some(arg) => {
                                self.compile_expression(arg)?;
                                self.bytecode.push(OpCode::FreDynamic);
                            }
                            None => self.bytecode.push(OpCode::FreFunc(0)),
                        },
                        "CSRLIN" => self.bytecode.push(OpCode::CsrLinFunc),
                        "POS" => match func.args.first() {
                            Some(arg) if self.try_expr_to_i32(arg).is_some() => {
                                self.bytecode
                                    .push(OpCode::PosFunc(self.try_expr_to_i32(arg).unwrap()));
                            }
                            Some(arg) => {
                                self.compile_expression(arg)?;
                                self.bytecode.push(OpCode::PosDynamic);
                            }
                            None => self.bytecode.push(OpCode::PosFunc(0)),
                        },
                        "LPOS" => match func.args.first() {
                            Some(arg) if self.try_expr_to_i32(arg).is_some() => {
                                self.bytecode
                                    .push(OpCode::LPosFunc(self.try_expr_to_i32(arg).unwrap()));
                            }
                            Some(arg) => {
                                self.compile_expression(arg)?;
                                self.bytecode.push(OpCode::LPosDynamic);
                            }
                            None => self.bytecode.push(OpCode::LPosFunc(0)),
                        },
                        "ENVIRON$" => self.bytecode.push(OpCode::EnvironFunc),
                        "INPUT$" => self.bytecode.push(OpCode::InputChars {
                            has_file_number: func.args.len() > 1,
                        }),
                        "COMMAND$" => self.bytecode.push(OpCode::CommandFunc),
                        "INKEY$" => self.bytecode.push(OpCode::InKeyFunc),
                        "FREEFILE" => self.bytecode.push(OpCode::FreeFile),
                        "EOF" => match func.args.first() {
                            Some(arg) if self.try_expr_to_i32(arg).is_some() => {
                                self.bytecode.push(OpCode::Eof(
                                    self.try_expr_to_i32(arg).unwrap().to_string(),
                                ));
                            }
                            Some(arg) => {
                                self.compile_expression(arg)?;
                                self.bytecode.push(OpCode::EofDynamic);
                            }
                            None => {}
                        },
                        "LOF" => match func.args.first() {
                            Some(arg) if self.try_expr_to_i32(arg).is_some() => {
                                self.bytecode.push(OpCode::Lof(
                                    self.try_expr_to_i32(arg).unwrap().to_string(),
                                ));
                            }
                            Some(arg) => {
                                self.compile_expression(arg)?;
                                self.bytecode.push(OpCode::LofDynamic);
                            }
                            None => {}
                        },
                        "LOC" => match func.args.first() {
                            Some(arg) if self.try_expr_to_i32(arg).is_some() => {
                                self.bytecode.push(OpCode::Loc(
                                    self.try_expr_to_i32(arg).unwrap().to_string(),
                                ));
                            }
                            Some(arg) => {
                                self.compile_expression(arg)?;
                                self.bytecode.push(OpCode::LocDynamic);
                            }
                            None => {}
                        },
                        "PEEK" => self.bytecode.push(OpCode::PeekDynamic),
                        "INP" => self.bytecode.push(OpCode::InpDynamic),
                        "POINT" => {
                            if let [x, y] = func.args.as_slice() {
                                if let (Some(x), Some(y)) =
                                    (self.try_expr_to_i32(x), self.try_expr_to_i32(y))
                                {
                                    self.bytecode.push(OpCode::PointFunc(x, y));
                                } else {
                                    self.compile_expression(x)?;
                                    self.compile_expression(y)?;
                                    self.bytecode.push(OpCode::PointDynamic);
                                }
                            }
                        }
                        "PMAP" => {
                            if let [coord, func_num] = func.args.as_slice() {
                                if let (Some(coord), Some(func_num)) =
                                    (self.try_expr_to_f64(coord), self.try_expr_to_i32(func_num))
                                {
                                    self.bytecode.push(OpCode::PMapFunc(coord, func_num));
                                } else {
                                    self.compile_expression(coord)?;
                                    self.compile_expression(func_num)?;
                                    self.bytecode.push(OpCode::PMapDynamic);
                                }
                            }
                        }
                        "VARPTR" => {
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                let var_ref = self.encode_var_ref(&var.name);
                                self.bytecode.push(OpCode::VarPtrFunc(var_ref));
                            }
                        }
                        "VARSEG" => {
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                let var_ref = self.encode_var_ref(&var.name);
                                self.bytecode.push(OpCode::VarSegFunc(var_ref));
                            }
                        }
                        "SADD" => {
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                let var_ref = self.encode_var_ref(&var.name);
                                self.bytecode.push(OpCode::SaddFunc(var_ref));
                            }
                        }
                        "VARPTR$" => {
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                let var_ref = self.encode_var_ref(&var.name);
                                self.bytecode.push(OpCode::VarPtrStrFunc(var_ref));
                            }
                        }
                        // Error functions
                        "ERR" => self.bytecode.push(OpCode::Err),
                        "ERL" => self.bytecode.push(OpCode::Erl),
                        "ERDEV" => self.bytecode.push(OpCode::ErDev),
                        "ERDEV$" => self.bytecode.push(OpCode::ErDevStr),
                        // Array functions
                        "LBOUND" => {
                            // Get array name from first argument
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                if let Some(dim_expr) = func.args.get(1) {
                                    if let Some(dim) = self.try_expr_to_i32(dim_expr) {
                                        let dim = (dim - 1).max(0);
                                        self.bytecode.push(OpCode::LBound(var.name.clone(), dim));
                                    } else {
                                        self.compile_expression(dim_expr)?;
                                        self.bytecode.push(OpCode::LBoundDynamic(var.name.clone()));
                                    }
                                } else {
                                    self.bytecode.push(OpCode::LBound(var.name.clone(), 0));
                                }
                            }
                        }
                        "UBOUND" => {
                            // Get array name from first argument
                            if let Some(Expression::Variable(var)) = func.args.first() {
                                if let Some(dim_expr) = func.args.get(1) {
                                    if let Some(dim) = self.try_expr_to_i32(dim_expr) {
                                        let dim = (dim - 1).max(0);
                                        self.bytecode.push(OpCode::UBound(var.name.clone(), dim));
                                    } else {
                                        self.compile_expression(dim_expr)?;
                                        self.bytecode.push(OpCode::UBoundDynamic(var.name.clone()));
                                    }
                                } else {
                                    self.bytecode.push(OpCode::UBound(var.name.clone(), 0));
                                }
                            }
                        }
                        _ => {}
                    }
                } else {
                    // User-defined function or unknown
                    if func.name.to_uppercase().starts_with("FN") {
                        for arg in &func.args {
                            self.compile_expression(arg)?;
                        }
                        self.bytecode.push(OpCode::CallDefFn(func.name.clone()));
                        return Ok(());
                    }

                    let normalized_name = Self::normalize_proc_name(&func.name);
                    let param_modes =
                        self.require_param_modes(&func.name, func.args.len(), true)?;
                    let by_ref =
                        self.compile_call_arguments(&func.args, Some(param_modes.as_slice()))?;

                    if self.has_function(&func.name) {
                        self.bytecode.push(OpCode::CallFunction {
                            name: normalized_name,
                            by_ref,
                        });
                    } else {
                        return Err(core_types::QError::InvalidProcedure(format!(
                            "FUNCTION {} is declared but has no definition",
                            func.name
                        )));
                    }
                }
            }

            Expression::FieldAccess { .. } => {
                if let Some(name) = Self::qualified_field_name(expr) {
                    let var_idx = self.get_var_index(&name);
                    self.bytecode.push(OpCode::LoadFast(var_idx));
                } else if let Expression::FieldAccess { object, .. } = expr {
                    self.compile_expression(object.as_ref())?;
                }
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
                self.register_scalar_storage_metadata(var, var_idx);
                self.bytecode.push(OpCode::StoreFast(var_idx));
            }
            Expression::ArrayAccess {
                name,
                indices,
                type_suffix,
            } => {
                let qtype = self.name_type_hint(name, *type_suffix);
                if !matches!(qtype, QType::String(_)) {
                    self.register_numeric_array_type(name, &qtype);
                }
                // ArrayStore expects the stack as indices..., value with the value on top.
                // Preserve that convention across assignment, GET/INPUT stores, and file reads.
                let temp_var = format!("__ARR_STORE_TMP_{}", self.next_var_index);
                let temp_idx = self.get_var_index(&temp_var);

                self.bytecode.push(OpCode::StoreFast(temp_idx));
                for idx in indices {
                    self.compile_expression(idx)?;
                }
                self.bytecode.push(OpCode::LoadFast(temp_idx));
                self.bytecode
                    .push(OpCode::ArrayStore(name.clone(), indices.len()));
            }
            Expression::FieldAccess { .. } => {
                if let Some(name) = Self::qualified_field_name(target) {
                    let var_idx = self.get_var_index(&name);
                    let qtype = self.name_type_hint(
                        &name,
                        syntax_tree::ast_nodes::Variable::suffix_from_name(&name),
                    );
                    if !matches!(qtype, QType::String(_)) {
                        self.register_numeric_slot_type(var_idx, &qtype);
                    }
                    self.bytecode.push(OpCode::StoreFast(var_idx));
                } else if let Expression::FieldAccess { object, .. } = target {
                    self.compile_expression(object.as_ref())?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
