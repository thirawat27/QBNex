use core_types::QResult;
use core_types::QType;
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

#[derive(Clone)]
struct UdtFieldLayout {
    storage_name: String,
    field_type: QType,
    fixed_length: Option<usize>,
    offset: usize,
    array_indices: Option<Vec<Expression>>,
}

#[derive(Clone)]
struct ResolvedUdtObject {
    storage_base: String,
    type_name: String,
    array_indices: Option<Vec<Expression>>,
}

impl CodeGenerator {
    fn binary_scalar_kind(var: &Variable) -> &'static str {
        if let Some(declared_type) = &var.declared_type {
            match Self::declared_type_to_qtype(declared_type) {
                QType::Integer(_) => return "i16",
                QType::Long(_) => return "i32",
                QType::Single(_) => return "f32",
                QType::Double(_) => return "f64",
                QType::String(_) => return "str",
                _ => {}
            }
        }
        match var.type_suffix.or_else(|| {
            var.name
                .chars()
                .last()
                .filter(|suffix| matches!(suffix, '%' | '&' | '!' | '#' | '$'))
        }) {
            Some('%') => "i16",
            Some('&') => "i32",
            Some('!') => "f32",
            Some('#') => "f64",
            Some('$') => "str",
            _ => "f32",
        }
    }

    fn primitive_kind_for_type(field_type: &QType) -> Option<&'static str> {
        match field_type {
            QType::Integer(_) => Some("i16"),
            QType::Long(_) => Some("i32"),
            QType::Single(_) => Some("f32"),
            QType::Double(_) => Some("f64"),
            QType::String(_) => Some("str"),
            _ => None,
        }
    }

    fn normalize_udt_name(name: &str) -> String {
        name.to_uppercase()
    }

    fn normalize_declared_type_name(type_name: &str) -> String {
        type_name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_uppercase()
    }

    fn declared_type_to_qtype(type_name: &str) -> QType {
        match Self::normalize_declared_type_name(type_name).as_str() {
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

    fn qtype_is_string(qtype: &QType) -> bool {
        matches!(qtype, QType::String(_))
    }

    fn qtype_is_numeric(qtype: &QType) -> bool {
        matches!(
            qtype,
            QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_)
        )
    }

    fn variable_type_hint(&self, var: &Variable) -> Option<QType> {
        if let Some(declared_type) = &var.declared_type {
            return Some(Self::declared_type_to_qtype(declared_type));
        }

        if let Some(suffix) = var.type_suffix.or_else(|| {
            var.name
                .chars()
                .last()
                .filter(|suffix| matches!(suffix, '%' | '&' | '!' | '#' | '$'))
        }) {
            return Some(match suffix {
                '%' => QType::Integer(0),
                '&' => QType::Long(0),
                '!' => QType::Single(0.0),
                '#' => QType::Double(0.0),
                '$' => QType::String(String::new()),
                _ => QType::Empty,
            });
        }

        let name_upper = var.name.to_uppercase();
        if self.str_vars.contains_key(&name_upper) || self.shared_str_vars.contains_key(&name_upper)
        {
            Some(QType::String(String::new()))
        } else if self.num_vars.contains_key(&name_upper)
            || self.shared_num_vars.contains_key(&name_upper)
        {
            Some(QType::Double(0.0))
        } else {
            None
        }
    }

    fn variable_is_string(&self, var: &Variable) -> bool {
        self.variable_type_hint(var)
            .as_ref()
            .is_some_and(Self::qtype_is_string)
    }

    fn name_is_string(&self, name: &str) -> bool {
        let name_upper = name.to_uppercase();
        name.ends_with('$')
            || self.str_vars.contains_key(&name_upper)
            || self.shared_str_vars.contains_key(&name_upper)
    }

    fn array_is_string(&self, name: &str, type_suffix: Option<char>) -> bool {
        let name_upper = name.to_uppercase();
        type_suffix == Some('$')
            || name.ends_with('$')
            || self.str_arr_vars.contains_key(&name_upper)
            || self.shared_str_arr_vars.contains_key(&name_upper)
    }

    fn function_returns_string(&self, name: &str, type_suffix: Option<char>) -> bool {
        if type_suffix == Some('$') || name.ends_with('$') {
            return true;
        }

        self.function_return_types
            .get(&name.to_uppercase())
            .is_some_and(Self::qtype_is_string)
    }

    fn variable_udt_type(&self, var: &Variable) -> Option<String> {
        var.declared_type
            .as_ref()
            .and_then(|declared_type| {
                if matches!(
                    Self::declared_type_to_qtype(declared_type),
                    QType::UserDefined(_)
                ) {
                    Some(Self::normalize_udt_name(declared_type))
                } else {
                    None
                }
            })
            .or_else(|| {
                self.udt_vars
                    .get(&Self::normalize_udt_name(&var.name))
                    .cloned()
            })
            .or_else(|| {
                self.shared_udt_vars
                    .get(&Self::normalize_udt_name(&var.name))
                    .cloned()
            })
    }

    fn array_udt_type(&self, name: &str) -> Option<String> {
        self.udt_array_vars
            .get(&Self::normalize_udt_name(name))
            .cloned()
            .or_else(|| {
                self.shared_udt_array_vars
                    .get(&Self::normalize_udt_name(name))
                    .cloned()
            })
    }

    fn field_storage_name(path: &str) -> String {
        path.to_uppercase()
    }

    fn qualified_field_name(expr: &Expression) -> Option<String> {
        expr.flattened_qb64_name().filter(|name| name.contains('.'))
    }

    fn fixed_string_width_for_name(&self, name: &str) -> Option<usize> {
        self.field_widths.get(&name.to_uppercase()).copied()
    }

    fn shared_num_var_idx(&self, name: &str) -> Option<usize> {
        self.shared_num_vars.get(&name.to_uppercase()).copied()
    }

    fn shared_str_var_idx(&self, name: &str) -> Option<usize> {
        self.shared_str_vars.get(&name.to_uppercase()).copied()
    }

    fn shared_arr_var_idx(&self, name: &str) -> Option<usize> {
        self.shared_arr_vars.get(&name.to_uppercase()).copied()
    }

    fn shared_str_arr_var_idx(&self, name: &str) -> Option<usize> {
        self.shared_str_arr_vars.get(&name.to_uppercase()).copied()
    }

    fn shared_global_scalar_name(&self, name: &str) -> bool {
        let name_upper = name.to_uppercase();
        self.shared_num_vars.contains_key(&name_upper)
            || self.shared_str_vars.contains_key(&name_upper)
    }

    fn shared_global_array_name(&self, name: &str) -> bool {
        let name_upper = name.to_uppercase();
        self.shared_arr_vars.contains_key(&name_upper)
            || self.shared_str_arr_vars.contains_key(&name_upper)
            || self.shared_udt_array_vars.contains_key(&name_upper)
    }

    fn native_bound_value_expr(&mut self, expr: &Expression) -> QResult<String> {
        let value = self.generate_expression(expr)?;
        Ok(format!("qb_array_bound_value({})", value))
    }

    fn native_array_bounds_expr(&mut self, dimensions: &[ArrayDimension]) -> QResult<String> {
        let mut bounds = Vec::with_capacity(dimensions.len());
        for dimension in dimensions {
            let lower = if let Some(lower_bound) = &dimension.lower_bound {
                self.native_bound_value_expr(lower_bound)?
            } else {
                "qb_current_option_base()".to_string()
            };
            let upper = self.native_bound_value_expr(&dimension.upper_bound)?;
            bounds.push(format!("({}, {})", lower, upper));
        }
        Ok(format!("vec![{}]", bounds.join(", ")))
    }

    fn native_array_indices_expr(&mut self, indices: &[Expression]) -> QResult<String> {
        let mut values = Vec::with_capacity(indices.len());
        for index in indices {
            values.push(self.generate_expression(index)?);
        }
        Ok(format!("&[{}]", values.join(", ")))
    }

    fn native_cached_array_indices(
        &mut self,
        indices: &[Expression],
    ) -> QResult<(Vec<String>, String)> {
        let mut setup_lines = Vec::with_capacity(indices.len());
        let mut cached_values = Vec::with_capacity(indices.len());
        for index in indices {
            let tmp = self.next_temp_var();
            let value = self.generate_expression(index)?;
            setup_lines.push(format!("let {} = {};", tmp, value));
            cached_values.push(tmp);
        }
        Ok((setup_lines, format!("&[{}]", cached_values.join(", "))))
    }

    fn lookup_udt_field(&self, type_name: &str, field: &str) -> Option<TypeField> {
        self.user_types
            .get(&Self::normalize_udt_name(type_name))
            .and_then(|user_type| {
                user_type
                    .fields
                    .iter()
                    .find(|f| f.name.eq_ignore_ascii_case(field))
                    .cloned()
            })
    }

    fn type_size_bytes(&self, field_type: &QType, fixed_length: Option<usize>) -> usize {
        match field_type {
            QType::Integer(_) => 2,
            QType::Long(_) => 4,
            QType::Single(_) => 4,
            QType::Double(_) => 8,
            QType::String(_) => fixed_length.unwrap_or(0),
            QType::UserDefined(type_name_bytes) => {
                let type_name = String::from_utf8_lossy(type_name_bytes).to_string();
                self.collect_udt_layout("__SIZE__", &type_name)
                    .iter()
                    .map(|field| self.type_size_bytes(&field.field_type, field.fixed_length))
                    .sum()
            }
            QType::Empty => 0,
        }
    }

    fn collect_udt_layout(&self, base_name: &str, type_name: &str) -> Vec<UdtFieldLayout> {
        let mut fields = Vec::new();
        self.collect_udt_layout_into(base_name, type_name, 0, &mut fields);
        fields
    }

    fn collect_udt_layout_into(
        &self,
        base_name: &str,
        type_name: &str,
        base_offset: usize,
        out: &mut Vec<UdtFieldLayout>,
    ) -> usize {
        let mut offset = base_offset;
        if let Some(user_type) = self.user_types.get(&Self::normalize_udt_name(type_name)) {
            for field in &user_type.fields {
                let field_name = format!("{}.{}", base_name, field.name);
                match &field.field_type {
                    QType::UserDefined(type_name_bytes) => {
                        let nested_type = String::from_utf8_lossy(type_name_bytes).to_string();
                        offset =
                            self.collect_udt_layout_into(&field_name, &nested_type, offset, out);
                    }
                    _ => {
                        let size = self.type_size_bytes(&field.field_type, field.fixed_length);
                        out.push(UdtFieldLayout {
                            storage_name: Self::field_storage_name(&field_name),
                            field_type: field.field_type.clone(),
                            fixed_length: field.fixed_length,
                            offset,
                            array_indices: None,
                        });
                        offset += size;
                    }
                }
            }
        }
        offset
    }

    fn ensure_udt_storage(&mut self, var_name: &str, type_name: &str) {
        let normalized = Self::normalize_udt_name(var_name);
        self.udt_vars
            .insert(normalized.clone(), Self::normalize_udt_name(type_name));
        for field in self.collect_udt_layout(&normalized, type_name) {
            match field.field_type {
                QType::String(_) => {
                    self.get_str_var_idx(&field.storage_name);
                }
                QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_) => {
                    self.get_num_var_idx(&field.storage_name);
                }
                _ => {}
            }
        }
    }

    fn ensure_udt_array_storage(&mut self, var_name: &str, type_name: &str) {
        let normalized = Self::normalize_udt_name(var_name);
        self.udt_array_vars
            .insert(normalized.clone(), Self::normalize_udt_name(type_name));
        for field in self.collect_udt_layout(&normalized, type_name) {
            match field.field_type {
                QType::String(_) => {
                    self.get_str_arr_var_idx(&field.storage_name);
                }
                QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_) => {
                    self.get_arr_var_idx(&field.storage_name);
                }
                _ => {}
            }
        }
    }

    fn resolve_field_access_layout(&self, expr: &Expression) -> Option<UdtFieldLayout> {
        match expr {
            Expression::FieldAccess { object, field } => {
                let resolved = self.resolve_udt_object_access(object)?;
                let type_field = self.lookup_udt_field(&resolved.type_name, field)?;
                Some(UdtFieldLayout {
                    storage_name: Self::field_storage_name(&format!(
                        "{}.{}",
                        resolved.storage_base, field
                    )),
                    field_type: type_field.field_type,
                    fixed_length: type_field.fixed_length,
                    offset: 0,
                    array_indices: resolved.array_indices,
                })
            }
            _ => None,
        }
    }

    fn resolve_udt_object_access(&self, expr: &Expression) -> Option<ResolvedUdtObject> {
        match expr {
            Expression::Variable(var) => {
                self.variable_udt_type(var)
                    .map(|type_name| ResolvedUdtObject {
                        storage_base: Self::normalize_udt_name(&var.name),
                        type_name,
                        array_indices: None,
                    })
            }
            Expression::ArrayAccess { name, indices, .. } => {
                self.array_udt_type(name)
                    .map(|type_name| ResolvedUdtObject {
                        storage_base: Self::normalize_udt_name(name),
                        type_name,
                        array_indices: Some(indices.clone()),
                    })
            }
            Expression::FieldAccess { object, field } => {
                let resolved = self.resolve_udt_object_access(object)?;
                let type_field = self.lookup_udt_field(&resolved.type_name, field)?;
                match type_field.field_type {
                    QType::UserDefined(type_name_bytes) => Some(ResolvedUdtObject {
                        storage_base: Self::field_storage_name(&format!(
                            "{}.{}",
                            resolved.storage_base, field
                        )),
                        type_name: String::from_utf8_lossy(&type_name_bytes).to_string(),
                        array_indices: resolved.array_indices,
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn emit_udt_record_get(
        &mut self,
        indent: &str,
        base_name: &str,
        type_name: &str,
        file_number: &str,
        record: &str,
        array_indices: Option<&[Expression]>,
    ) {
        let base_pos = self.next_temp_var();
        self.output
            .push_str(&format!("{}let {} = {};\n", indent, base_pos, record));
        let cached_array_indices = if let Some(indices) = array_indices {
            let (setup_lines, index_slice) = self
                .native_cached_array_indices(indices)
                .unwrap_or_else(|_| (Vec::new(), "&[]".to_string()));
            for line in setup_lines {
                self.output.push_str(&format!("{}{}\n", indent, line));
            }
            Some(index_slice)
        } else {
            None
        };
        for field in self.collect_udt_layout(base_name, type_name) {
            let field_pos = self.next_temp_var();
            let pos_expr = format!("({} + {}.0)", base_pos, field.offset);
            self.output
                .push_str(&format!("{}let {} = {};\n", indent, field_pos, pos_expr));
            match field.field_type {
                QType::String(_) => {
                    let size_hint = field.fixed_length.unwrap_or(0);
                    let tmp = self.next_temp_var();
                    self.output.push_str(&format!(
                        "{}let {} = qb_get_string_from_file({}, {}, {}.0);\n",
                        indent, tmp, file_number, field_pos, size_hint
                    ));
                    if array_indices.is_some() {
                        let arr_idx = self.get_str_arr_var_idx(&field.storage_name);
                        let index_code = cached_array_indices
                            .as_deref()
                            .unwrap_or_else(|| unreachable!());
                        self.output.push_str(&format!(
                            "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{});\n",
                            indent, arr_idx, index_code, tmp
                        ));
                    } else {
                        let idx = self.get_str_var_idx(&field.storage_name);
                        self.output.push_str(&format!(
                            "{}set_str(&mut str_vars, {}, &{});\n",
                            indent, idx, tmp
                        ));
                    }
                }
                QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_) => {
                    let helper = match Self::primitive_kind_for_type(&field.field_type) {
                        Some("i16") => "qb_get_i16",
                        Some("i32") => "qb_get_i32",
                        Some("f64") => "qb_get_f64",
                        _ => "qb_get_f32",
                    };
                    let tmp = self.next_temp_var();
                    self.output.push_str(&format!(
                        "{}let {} = {}({}, {});\n",
                        indent, tmp, helper, file_number, field_pos
                    ));
                    if array_indices.is_some() {
                        let arr_idx = self.get_arr_var_idx(&field.storage_name);
                        let index_code = cached_array_indices
                            .as_deref()
                            .unwrap_or_else(|| unreachable!());
                        self.output.push_str(&format!(
                            "{}arr_set(&mut arr_vars, &mut arr_bounds, {}, {}, {});\n",
                            indent, arr_idx, index_code, tmp
                        ));
                    } else {
                        let idx = self.get_num_var_idx(&field.storage_name);
                        self.output.push_str(&format!(
                            "{}set_var(&mut num_vars, {}, {});\n",
                            indent, idx, tmp
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    fn emit_udt_record_put(
        &mut self,
        indent: &str,
        base_name: &str,
        type_name: &str,
        file_number: &str,
        record: &str,
        array_indices: Option<&[Expression]>,
    ) {
        let base_pos = self.next_temp_var();
        self.output
            .push_str(&format!("{}let {} = {};\n", indent, base_pos, record));
        let cached_array_indices = if let Some(indices) = array_indices {
            let (setup_lines, index_slice) = self
                .native_cached_array_indices(indices)
                .unwrap_or_else(|_| (Vec::new(), "&[]".to_string()));
            for line in setup_lines {
                self.output.push_str(&format!("{}{}\n", indent, line));
            }
            Some(index_slice)
        } else {
            None
        };
        for field in self.collect_udt_layout(base_name, type_name) {
            let field_pos = self.next_temp_var();
            let pos_expr = format!("({} + {}.0)", base_pos, field.offset);
            self.output
                .push_str(&format!("{}let {} = {};\n", indent, field_pos, pos_expr));
            match field.field_type {
                QType::String(_) => {
                    let tmp = self.next_temp_var();
                    if array_indices.is_some() {
                        let arr_idx = self.get_str_arr_var_idx(&field.storage_name);
                        let index_code = cached_array_indices
                            .as_deref()
                            .unwrap_or_else(|| unreachable!());
                        self.output.push_str(&format!(
                            "{}let {} = str_arr_get(&mut str_arr_vars, &mut str_arr_bounds, {}, {});\n",
                            indent, tmp, arr_idx, index_code
                        ));
                    } else {
                        let idx = self.get_str_var_idx(&field.storage_name);
                        self.output.push_str(&format!(
                            "{}let {} = get_str(&str_vars, {});\n",
                            indent, tmp, idx
                        ));
                    }
                    if let Some(width) = field.fixed_length {
                        self.output.push_str(&format!(
                            "{}qb_put_fixed_string({}, {}, {}, &{});\n",
                            indent, file_number, field_pos, width, tmp
                        ));
                    } else {
                        self.output.push_str(&format!(
                            "{}qb_put_string({}, {}, &{});\n",
                            indent, file_number, field_pos, tmp
                        ));
                    }
                }
                QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_) => {
                    let helper = match Self::primitive_kind_for_type(&field.field_type) {
                        Some("i16") => "qb_put_i16",
                        Some("i32") => "qb_put_i32",
                        Some("f64") => "qb_put_f64",
                        _ => "qb_put_f32",
                    };
                    let tmp = self.next_temp_var();
                    if array_indices.is_some() {
                        let arr_idx = self.get_arr_var_idx(&field.storage_name);
                        let index_code = cached_array_indices
                            .as_deref()
                            .unwrap_or_else(|| unreachable!());
                        self.output.push_str(&format!(
                            "{}let {} = arr_get(&mut arr_vars, &mut arr_bounds, {}, {});\n",
                            indent, tmp, arr_idx, index_code
                        ));
                    } else {
                        let idx = self.get_num_var_idx(&field.storage_name);
                        self.output.push_str(&format!(
                            "{}let {} = get_var(&num_vars, {});\n",
                            indent, tmp, idx
                        ));
                    }
                    self.output.push_str(&format!(
                        "{}{}({}, {}, {});\n",
                        indent, helper, file_number, field_pos, tmp
                    ));
                }
                _ => {}
            }
        }
    }

    fn emit_udt_field_get(
        &mut self,
        indent: &str,
        field: &UdtFieldLayout,
        file_number: &str,
        record: &str,
    ) -> QResult<()> {
        let cached_array_indices = if let Some(indices) = field.array_indices.as_deref() {
            let (setup_lines, index_slice) = self.native_cached_array_indices(indices)?;
            for line in setup_lines {
                self.output.push_str(&format!("{}{}\n", indent, line));
            }
            Some(index_slice)
        } else {
            None
        };
        match field.field_type {
            QType::String(_) => {
                let size_hint = field.fixed_length.unwrap_or(0);
                let tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {} = qb_get_string_from_file({}, {}, {}.0);\n",
                    indent, tmp, file_number, record, size_hint
                ));
                if field.array_indices.is_some() {
                    let arr_idx = self.get_str_arr_var_idx(&field.storage_name);
                    let index_code = cached_array_indices
                        .as_deref()
                        .unwrap_or_else(|| unreachable!());
                    self.output.push_str(&format!(
                        "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{});\n",
                        indent, arr_idx, index_code, tmp
                    ));
                } else {
                    let idx = self.get_str_var_idx(&field.storage_name);
                    self.output.push_str(&format!(
                        "{}set_str(&mut str_vars, {}, &{});\n",
                        indent, idx, tmp
                    ));
                }
                Ok(())
            }
            QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_) => {
                let helper = match Self::primitive_kind_for_type(&field.field_type) {
                    Some("i16") => "qb_get_i16",
                    Some("i32") => "qb_get_i32",
                    Some("f64") => "qb_get_f64",
                    _ => "qb_get_f32",
                };
                let tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {} = {}({}, {});\n",
                    indent, tmp, helper, file_number, record
                ));
                if field.array_indices.is_some() {
                    let arr_idx = self.get_arr_var_idx(&field.storage_name);
                    let index_code = cached_array_indices
                        .as_deref()
                        .unwrap_or_else(|| unreachable!());
                    self.output.push_str(&format!(
                        "{}arr_set(&mut arr_vars, &mut arr_bounds, {}, {}, {});\n",
                        indent, arr_idx, index_code, tmp
                    ));
                } else {
                    let idx = self.get_num_var_idx(&field.storage_name);
                    self.output.push_str(&format!(
                        "{}set_var(&mut num_vars, {}, {});\n",
                        indent, idx, tmp
                    ));
                }
                Ok(())
            }
            _ => Err(core_types::QError::UnsupportedFeature(
                "native GET field target must resolve to a primitive or fixed-length string leaf"
                    .to_string(),
            )),
        }
    }

    fn emit_udt_field_put(
        &mut self,
        indent: &str,
        field: &UdtFieldLayout,
        file_number: &str,
        record: &str,
    ) -> QResult<()> {
        let cached_array_indices = if let Some(indices) = field.array_indices.as_deref() {
            let (setup_lines, index_slice) = self.native_cached_array_indices(indices)?;
            for line in setup_lines {
                self.output.push_str(&format!("{}{}\n", indent, line));
            }
            Some(index_slice)
        } else {
            None
        };
        match field.field_type {
            QType::String(_) => {
                let tmp = self.next_temp_var();
                if field.array_indices.is_some() {
                    let arr_idx = self.get_str_arr_var_idx(&field.storage_name);
                    let index_code = cached_array_indices
                        .as_deref()
                        .unwrap_or_else(|| unreachable!());
                    self.output.push_str(&format!(
                        "{}let {} = str_arr_get(&mut str_arr_vars, &mut str_arr_bounds, {}, {});\n",
                        indent, tmp, arr_idx, index_code
                    ));
                } else {
                    let idx = self.get_str_var_idx(&field.storage_name);
                    self.output.push_str(&format!(
                        "{}let {} = get_str(&str_vars, {});\n",
                        indent, tmp, idx
                    ));
                }
                if let Some(width) = field.fixed_length {
                    self.output.push_str(&format!(
                        "{}qb_put_fixed_string({}, {}, {}, &{});\n",
                        indent, file_number, record, width, tmp
                    ));
                } else {
                    self.output.push_str(&format!(
                        "{}qb_put_string({}, {}, &{});\n",
                        indent, file_number, record, tmp
                    ));
                }
                Ok(())
            }
            QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_) => {
                let helper = match Self::primitive_kind_for_type(&field.field_type) {
                    Some("i16") => "qb_put_i16",
                    Some("i32") => "qb_put_i32",
                    Some("f64") => "qb_put_f64",
                    _ => "qb_put_f32",
                };
                let tmp = self.next_temp_var();
                if field.array_indices.is_some() {
                    let arr_idx = self.get_arr_var_idx(&field.storage_name);
                    let index_code = cached_array_indices
                        .as_deref()
                        .unwrap_or_else(|| unreachable!());
                    self.output.push_str(&format!(
                        "{}let {} = arr_get(&mut arr_vars, &mut arr_bounds, {}, {});\n",
                        indent, tmp, arr_idx, index_code
                    ));
                } else {
                    let idx = self.get_num_var_idx(&field.storage_name);
                    self.output.push_str(&format!(
                        "{}let {} = get_var(&num_vars, {});\n",
                        indent, tmp, idx
                    ));
                }
                self.output.push_str(&format!(
                    "{}{}({}, {}, {});\n",
                    indent, helper, file_number, record, tmp
                ));
                Ok(())
            }
            _ => Err(core_types::QError::UnsupportedFeature(
                "native PUT field target must resolve to a primitive or fixed-length string leaf"
                    .to_string(),
            )),
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) -> QResult<()> {
        let indent = self.indent();

        if self.emit_graphics_statement(&indent, stmt)? {
            return Ok(());
        }

        match stmt {
            Statement::Print {
                expressions,
                separators,
                newline,
            } => {
                self.generate_print(&indent, expressions, separators, *newline)?;
            }

            Statement::LPrint {
                expressions,
                separators,
                newline,
            } => {
                self.generate_lprint(&indent, expressions, separators, *newline)?;
            }

            Statement::PrintUsing {
                format,
                expressions,
                separators,
                newline,
            } => {
                self.generate_print_using(&indent, format, expressions, separators, *newline)?;
            }

            Statement::LPrintUsing {
                format,
                expressions,
                separators,
                newline,
            } => {
                self.generate_lprint_using(&indent, format, expressions, separators, *newline)?;
            }

            Statement::PrintFileUsing {
                file_number,
                format,
                expressions,
                separators,
                newline,
            } => {
                let file_number_code = self.generate_expression(file_number)?;
                let format_code = self.generate_expression(format)?;
                let file_number_tmp = self.next_temp_var();
                let pattern_tmp = self.next_temp_var();
                let values_tmp = self.next_temp_var();
                let comma_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {} = {};\n",
                    indent, file_number_tmp, file_number_code
                ));
                self.output.push_str(&format!(
                    "{}let {} = {};\n",
                    indent, pattern_tmp, format_code
                ));
                self.output.push_str(&format!(
                    "{}let mut {}: Vec<String> = Vec::new();\n",
                    indent, values_tmp
                ));
                self.output.push_str(&format!(
                    "{}let mut {}: Vec<bool> = Vec::new();\n",
                    indent, comma_tmp
                ));
                for expr in expressions.iter() {
                    let expr_code = self.generate_printable_expression(expr)?;
                    self.output
                        .push_str(&format!("{}{}.push({});\n", indent, values_tmp, expr_code));
                }
                for separator in separators.iter().take(expressions.len()) {
                    self.output.push_str(&format!(
                        "{}{}.push({});\n",
                        indent,
                        comma_tmp,
                        matches!(separator, Some(PrintSeparator::Comma))
                    ));
                }
                self.output.push_str(&format!(
                    "{}qb_file_print_using({}, &{}, &{}, &{}, {});\n",
                    indent, file_number_tmp, pattern_tmp, values_tmp, comma_tmp, newline
                ));
            }

            Statement::Write { expressions } => {
                let values_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let mut {}: Vec<String> = Vec::new();\n",
                    indent, values_tmp
                ));
                for expr in expressions {
                    let expr_code = self.generate_expression(expr)?;
                    if self.is_string_expression(expr) {
                        self.output.push_str(&format!(
                            "{}{}.push(qb_write_string_field(&{}));\n",
                            indent, values_tmp, expr_code
                        ));
                    } else {
                        self.output.push_str(&format!(
                            "{}{}.push(qb_write_numeric_field({}));\n",
                            indent, values_tmp, expr_code
                        ));
                    }
                }
                self.output.push_str(&format!(
                    "{}qb_print(&{}.join(\",\"));\n",
                    indent, values_tmp
                ));
                self.output
                    .push_str(&format!("{}qb_print_newline();\n", indent));
            }

            Statement::PrintFile {
                file_number,
                expressions,
                separators,
                newline,
            } => {
                let file_number = self.generate_expression(file_number)?;
                for (index, expr) in expressions.iter().enumerate() {
                    let expr_code = self.generate_printable_expression(expr)?;
                    self.output.push_str(&format!(
                        "{}qb_file_write({}, &{}, false);\n",
                        indent, file_number, expr_code
                    ));
                    if matches!(separators.get(index), Some(Some(PrintSeparator::Comma))) {
                        self.output.push_str(&format!(
                            "{}qb_file_print_comma({});\n",
                            indent, file_number
                        ));
                    }
                }
                if *newline {
                    self.output.push_str(&format!(
                        "{}qb_file_print_newline({});\n",
                        indent, file_number
                    ));
                }
            }

            Statement::Assignment { target, value } => {
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
                        let original_code = self.generate_expression(&args[0])?;
                        let position_code = self.generate_expression(&args[1])?;
                        let value_code = self.generate_expression(value)?;
                        let tmp = self.next_temp_var();
                        self.output.push_str(&format!(
                            "{}let {} = qb_set_asc(&{}, {}, {});\n",
                            indent, tmp, original_code, position_code, value_code
                        ));
                        self.generate_store_from_text_value(indent.as_str(), &args[0], &tmp, true)?;
                        return Ok(());
                    }
                }

                let value_code = self.generate_expression(value)?;
                match target {
                    Expression::Variable(var) => {
                        let name_upper = var.name.to_uppercase();
                        if self.variable_is_string(var) {
                            let shared_idx = if self.is_in_sub {
                                self.shared_str_var_idx(&name_upper)
                            } else {
                                None
                            };
                            let idx =
                                shared_idx.unwrap_or_else(|| self.get_str_var_idx(&name_upper));
                            let width = self.fixed_string_width_for_name(&name_upper);
                            // Avoid mutable borrow conflict
                            self.output
                                .push_str(&format!("{}let _tmp_str = {};\n", indent, value_code));
                            if let Some(width) = width {
                                self.output.push_str(&format!(
                                    "{}set_str({}, {}, &qb_fit_fixed_string({}, &_tmp_str));\n",
                                    indent,
                                    if shared_idx.is_some() {
                                        "global_str_vars"
                                    } else {
                                        "&mut str_vars"
                                    },
                                    idx,
                                    width
                                ));
                            } else {
                                self.output.push_str(&format!(
                                    "{}set_str({}, {}, &_tmp_str);\n",
                                    indent,
                                    if shared_idx.is_some() {
                                        "global_str_vars"
                                    } else {
                                        "&mut str_vars"
                                    },
                                    idx
                                ));
                            }
                        } else {
                            let shared_idx = if self.is_in_sub {
                                self.shared_num_var_idx(&name_upper)
                            } else {
                                None
                            };
                            let idx =
                                shared_idx.unwrap_or_else(|| self.get_num_var_idx(&name_upper));
                            self.output.push_str(&format!(
                                "{}let _val = {} as f64;\n",
                                indent, value_code
                            ));
                            self.output.push_str(&format!(
                                "{}set_var({}, {}, _val);\n",
                                indent,
                                if shared_idx.is_some() {
                                    "global_num_vars"
                                } else {
                                    "&mut num_vars"
                                },
                                idx
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
                        } else if self.array_is_string(name, None) {
                            if !indices.is_empty() {
                                let arr_idx = self.get_str_arr_var_idx(name);
                                let idx_code = self.native_array_indices_expr(indices)?;
                                let width = self.fixed_string_width_for_name(name);
                                self.output.push_str(&format!(
                                    "{}let _tmp_str = {};\n",
                                    indent, value_code
                                ));
                                if let Some(width) = width {
                                    self.output.push_str(&format!(
                                        "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &qb_fit_fixed_string({}, &_tmp_str));\n",
                                        indent, arr_idx, idx_code, width
                                    ));
                                } else {
                                    self.output.push_str(&format!(
                                        "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &_tmp_str);\n",
                                        indent, arr_idx, idx_code
                                    ));
                                }
                            }
                        } else if !indices.is_empty() {
                            let idx_code = self.native_array_indices_expr(indices)?;
                            if self.is_in_sub {
                                if let Some(arr_idx) = self.shared_arr_var_idx(name) {
                                    self.output.push_str(&format!(
                                        "{}let _val = {} as f64;\n",
                                        indent, value_code
                                    ));
                                    self.output.push_str(&format!(
                                        "{}arr_set(global_arr_vars, global_arr_bounds, {}, {}, _val);\n",
                                        indent, arr_idx, idx_code
                                    ));
                                    return Ok(());
                                }
                            }
                            let arr_idx = self.get_arr_var_idx(name);
                            self.output.push_str(&format!(
                                "{}let _val = {} as f64;\n",
                                indent, value_code
                            ));
                            self.output.push_str(&format!(
                                "{}arr_set(&mut arr_vars, &mut arr_bounds, {}, {}, _val);\n",
                                indent, arr_idx, idx_code
                            ));
                        }
                    }
                    Expression::FieldAccess { .. } => {
                        if let Some(field) = self.resolve_field_access_layout(target) {
                            match field.field_type {
                                QType::String(_) => {
                                    let width = field.fixed_length;
                                    self.output.push_str(&format!(
                                        "{}let _tmp_str = {};\n",
                                        indent, value_code
                                    ));
                                    if let Some(indices) = field.array_indices.as_ref() {
                                        let arr_idx = self.get_str_arr_var_idx(&field.storage_name);
                                        let index_code = self.native_array_indices_expr(indices)?;
                                        if let Some(width) = width {
                                            self.output.push_str(&format!(
                                                "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &qb_fit_fixed_string({}, &_tmp_str));\n",
                                                indent, arr_idx, index_code, width
                                            ));
                                        } else {
                                            self.output.push_str(&format!(
                                                "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &_tmp_str);\n",
                                                indent, arr_idx, index_code
                                            ));
                                        }
                                    } else {
                                        let idx = self.get_str_var_idx(&field.storage_name);
                                        if let Some(width) = width {
                                            self.output.push_str(&format!(
                                                "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &_tmp_str));\n",
                                                indent, idx, width
                                            ));
                                        } else {
                                            self.output.push_str(&format!(
                                                "{}set_str(&mut str_vars, {}, &_tmp_str);\n",
                                                indent, idx
                                            ));
                                        }
                                    }
                                }
                                QType::Integer(_)
                                | QType::Long(_)
                                | QType::Single(_)
                                | QType::Double(_) => {
                                    self.output.push_str(&format!(
                                        "{}let _val = {} as f64;\n",
                                        indent, value_code
                                    ));
                                    if let Some(indices) = field.array_indices.as_ref() {
                                        let arr_idx = self.get_arr_var_idx(&field.storage_name);
                                        let index_code = self.native_array_indices_expr(indices)?;
                                        self.output.push_str(&format!(
                                            "{}arr_set(&mut arr_vars, &mut arr_bounds, {}, {}, _val);\n",
                                            indent, arr_idx, index_code
                                        ));
                                    } else {
                                        let idx = self.get_num_var_idx(&field.storage_name);
                                        self.output.push_str(&format!(
                                            "{}set_var(&mut num_vars, {}, _val);\n",
                                            indent, idx
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        } else if let Some(name) = Self::qualified_field_name(target) {
                            if self.name_is_string(&name) {
                                let idx = self.get_str_var_idx(&name);
                                let width = self.fixed_string_width_for_name(&name);
                                self.output.push_str(&format!(
                                    "{}let _tmp_str = {};\n",
                                    indent, value_code
                                ));
                                if let Some(width) = width {
                                    self.output.push_str(&format!(
                                        "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &_tmp_str));\n",
                                        indent, idx, width
                                    ));
                                } else {
                                    self.output.push_str(&format!(
                                        "{}set_str(&mut str_vars, {}, &_tmp_str);\n",
                                        indent, idx
                                    ));
                                }
                            } else {
                                let idx = self.get_num_var_idx(&name);
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
                    }
                    _ => {}
                }
            }

            Statement::Dim { variables, .. } => {
                for (var, dimensions) in variables {
                    let var_name = &var.name;
                    if let Some(declared_type) = &var.declared_type {
                        match Self::declared_type_to_qtype(declared_type) {
                            QType::UserDefined(_) => {
                                if let Some(dimensions) = dimensions {
                                    let bounds_tmp = self.next_temp_var();
                                    let bounds = self.native_array_bounds_expr(dimensions)?;
                                    self.ensure_udt_array_storage(var_name, declared_type);
                                    self.output.push_str(&format!(
                                        "{}let {} = {};\n",
                                        indent, bounds_tmp, bounds
                                    ));
                                    let normalized = Self::normalize_udt_name(var_name);
                                    for field in self.collect_udt_layout(&normalized, declared_type)
                                    {
                                        match field.field_type {
                                            QType::String(_) => {
                                                let idx =
                                                    self.get_str_arr_var_idx(&field.storage_name);
                                                let init_guard = if self.current_proc_is_static {
                                                    format!(" && str_arr_vars[{}].is_empty()", idx)
                                                } else {
                                                    String::new()
                                                };
                                                self.output.push_str(&format!(
                                                    "{}if {} < str_arr_vars.len(){} {{ qb_dim_str_array(&mut str_arr_vars, &mut str_arr_bounds, {}, &{}, false); }}\n",
                                                    indent, idx, init_guard, idx, bounds_tmp
                                                ));
                                            }
                                            QType::Integer(_)
                                            | QType::Long(_)
                                            | QType::Single(_)
                                            | QType::Double(_) => {
                                                let idx = self.get_arr_var_idx(&field.storage_name);
                                                let init_guard = if self.current_proc_is_static {
                                                    format!(" && arr_vars[{}].is_empty()", idx)
                                                } else {
                                                    String::new()
                                                };
                                                self.output.push_str(&format!(
                                                    "{}if {} < arr_vars.len(){} {{ qb_dim_num_array(&mut arr_vars, &mut arr_bounds, {}, &{}, false); }}\n",
                                                    indent, idx, init_guard, idx, bounds_tmp
                                                ));
                                            }
                                            _ => {}
                                        }
                                    }
                                } else {
                                    self.ensure_udt_storage(var_name, declared_type);
                                }
                            }
                            QType::String(_) => {
                                if let Some(width) = var.fixed_length {
                                    self.field_widths.insert(var_name.to_uppercase(), width);
                                }
                                if let Some(dimensions) = dimensions {
                                    let bounds_tmp = self.next_temp_var();
                                    let bounds = self.native_array_bounds_expr(dimensions)?;
                                    let idx = self.get_str_arr_var_idx(var_name);
                                    let init_guard = if self.current_proc_is_static {
                                        format!(" && str_arr_vars[{}].is_empty()", idx)
                                    } else {
                                        String::new()
                                    };
                                    self.output.push_str(&format!(
                                        "{}let {} = {};\n",
                                        indent, bounds_tmp, bounds
                                    ));
                                    self.output.push_str(&format!(
                                        "{}if {} < str_arr_vars.len(){} {{ qb_dim_str_array(&mut str_arr_vars, &mut str_arr_bounds, {}, &{}, false); }}\n",
                                        indent, idx, init_guard, idx, bounds_tmp
                                    ));
                                }
                            }
                            qtype if Self::qtype_is_numeric(&qtype) => {
                                if let Some(dimensions) = dimensions {
                                    let bounds_tmp = self.next_temp_var();
                                    let bounds = self.native_array_bounds_expr(dimensions)?;
                                    let idx = self.get_arr_var_idx(var_name);
                                    let init_guard = if self.current_proc_is_static {
                                        format!(" && arr_vars[{}].is_empty()", idx)
                                    } else {
                                        String::new()
                                    };
                                    self.output.push_str(&format!(
                                        "{}let {} = {};\n",
                                        indent, bounds_tmp, bounds
                                    ));
                                    self.output.push_str(&format!(
                                        "{}if {} < arr_vars.len(){} {{ qb_dim_num_array(&mut arr_vars, &mut arr_bounds, {}, &{}, false); }}\n",
                                        indent, idx, init_guard, idx, bounds_tmp
                                    ));
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if self.variable_is_string(var) {
                        if let Some(width) = var.fixed_length {
                            self.field_widths.insert(var_name.to_uppercase(), width);
                        }
                        if let Some(dimensions) = dimensions {
                            let bounds_tmp = self.next_temp_var();
                            let bounds = self.native_array_bounds_expr(dimensions)?;
                            let idx = self.get_str_arr_var_idx(var_name);
                            let init_guard = if self.current_proc_is_static {
                                format!(" && str_arr_vars[{}].is_empty()", idx)
                            } else {
                                String::new()
                            };
                            self.output
                                .push_str(&format!("{}let {} = {};\n", indent, bounds_tmp, bounds));
                            self.output.push_str(&format!(
                                "{}if {} < str_arr_vars.len(){} {{ qb_dim_str_array(&mut str_arr_vars, &mut str_arr_bounds, {}, &{}, false); }}\n",
                                indent, idx, init_guard, idx, bounds_tmp
                            ));
                        }
                        continue;
                    }

                    if let Some(dimensions) = dimensions {
                        let bounds_tmp = self.next_temp_var();
                        let bounds = self.native_array_bounds_expr(dimensions)?;
                        let is_string_array = self.array_is_string(var_name, var.type_suffix);
                        if is_string_array {
                            if let Some(width) = var.fixed_length {
                                self.field_widths.insert(var.name.to_uppercase(), width);
                            }
                        }
                        let idx = if is_string_array {
                            self.get_str_arr_var_idx(var_name)
                        } else {
                            self.get_arr_var_idx(var_name)
                        };
                        let init_guard = if self.current_proc_is_static {
                            if is_string_array {
                                format!(" && str_arr_vars[{}].is_empty()", idx)
                            } else {
                                format!(" && arr_vars[{}].is_empty()", idx)
                            }
                        } else {
                            String::new()
                        };
                        self.output
                            .push_str(&format!("{}let {} = {};\n", indent, bounds_tmp, bounds));
                        if is_string_array {
                            self.output.push_str(&format!(
                                "{}if {} < str_arr_vars.len(){} {{ qb_dim_str_array(&mut str_arr_vars, &mut str_arr_bounds, {}, &{}, false); }}\n",
                                indent, idx, init_guard, idx, bounds_tmp
                            ));
                        } else {
                            self.output.push_str(&format!(
                                "{}if {} < arr_vars.len(){} {{ qb_dim_num_array(&mut arr_vars, &mut arr_bounds, {}, &{}, false); }}\n",
                                indent, idx, init_guard, idx, bounds_tmp
                            ));
                        }
                    }
                }
            }

            Statement::Redim {
                variables,
                preserve,
            } => {
                for (var, dimensions) in variables {
                    if let Some(declared_type) = &var.declared_type {
                        match Self::declared_type_to_qtype(declared_type) {
                            QType::UserDefined(_) => {
                                if let Some(dimensions) = dimensions {
                                    let bounds_tmp = self.next_temp_var();
                                    let bounds = self.native_array_bounds_expr(dimensions)?;
                                    self.ensure_udt_array_storage(&var.name, declared_type);
                                    self.output.push_str(&format!(
                                        "{}let {} = {};\n",
                                        indent, bounds_tmp, bounds
                                    ));
                                    let normalized = Self::normalize_udt_name(&var.name);
                                    for field in self.collect_udt_layout(&normalized, declared_type)
                                    {
                                        match field.field_type {
                                            QType::String(_) => {
                                                let idx =
                                                    self.get_str_arr_var_idx(&field.storage_name);
                                                self.output.push_str(&format!(
                                                    "{}if {} < str_arr_vars.len() {{ qb_dim_str_array(&mut str_arr_vars, &mut str_arr_bounds, {}, &{}, {}); }}\n",
                                                    indent, idx, idx, bounds_tmp, preserve
                                                ));
                                            }
                                            QType::Integer(_)
                                            | QType::Long(_)
                                            | QType::Single(_)
                                            | QType::Double(_) => {
                                                let idx = self.get_arr_var_idx(&field.storage_name);
                                                self.output.push_str(&format!(
                                                    "{}if {} < arr_vars.len() {{ qb_dim_num_array(&mut arr_vars, &mut arr_bounds, {}, &{}, {}); }}\n",
                                                    indent, idx, idx, bounds_tmp, preserve
                                                ));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            QType::String(_) => {
                                if let Some(width) = var.fixed_length {
                                    self.field_widths.insert(var.name.to_uppercase(), width);
                                }
                                if let Some(dimensions) = dimensions {
                                    let bounds_tmp = self.next_temp_var();
                                    let bounds = self.native_array_bounds_expr(dimensions)?;
                                    let idx = self.get_str_arr_var_idx(&var.name);
                                    self.output.push_str(&format!(
                                        "{}let {} = {};\n",
                                        indent, bounds_tmp, bounds
                                    ));
                                    self.output.push_str(&format!(
                                        "{}if {} < str_arr_vars.len() {{ qb_dim_str_array(&mut str_arr_vars, &mut str_arr_bounds, {}, &{}, {}); }}\n",
                                        indent, idx, idx, bounds_tmp, preserve
                                    ));
                                }
                            }
                            qtype if Self::qtype_is_numeric(&qtype) => {
                                if let Some(dimensions) = dimensions {
                                    let bounds_tmp = self.next_temp_var();
                                    let bounds = self.native_array_bounds_expr(dimensions)?;
                                    let idx = self.get_arr_var_idx(&var.name);
                                    self.output.push_str(&format!(
                                        "{}let {} = {};\n",
                                        indent, bounds_tmp, bounds
                                    ));
                                    self.output.push_str(&format!(
                                        "{}if {} < arr_vars.len() {{ qb_dim_num_array(&mut arr_vars, &mut arr_bounds, {}, &{}, {}); }}\n",
                                        indent, idx, idx, bounds_tmp, preserve
                                    ));
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if let Some(dimensions) = dimensions {
                        let bounds_tmp = self.next_temp_var();
                        let bounds = self.native_array_bounds_expr(dimensions)?;
                        self.output
                            .push_str(&format!("{}let {} = {};\n", indent, bounds_tmp, bounds));
                        if self.array_is_string(&var.name, var.type_suffix) {
                            if let Some(width) = var.fixed_length {
                                self.field_widths.insert(var.name.to_uppercase(), width);
                            }
                            let idx = self.get_str_arr_var_idx(&var.name);
                            self.output.push_str(&format!(
                                "{}if {} < str_arr_vars.len() {{ qb_dim_str_array(&mut str_arr_vars, &mut str_arr_bounds, {}, &{}, {}); }}\n",
                                indent, idx, idx, bounds_tmp, preserve
                            ));
                        } else {
                            let idx = self.get_arr_var_idx(&var.name);
                            self.output.push_str(&format!(
                                "{}if {} < arr_vars.len() {{ qb_dim_num_array(&mut arr_vars, &mut arr_bounds, {}, &{}, {}); }}\n",
                                indent, idx, idx, bounds_tmp, preserve
                            ));
                        }
                    }
                }
            }

            Statement::Erase { variables } => {
                for var in variables {
                    if let Some(declared_type) = &var.declared_type {
                        match Self::declared_type_to_qtype(declared_type) {
                            QType::UserDefined(_) => {
                                self.ensure_udt_array_storage(&var.name, declared_type);
                                let normalized = Self::normalize_udt_name(&var.name);
                                for field in self.collect_udt_layout(&normalized, declared_type) {
                                    match field.field_type {
                                        QType::String(_) => {
                                            let idx = self.get_str_arr_var_idx(&field.storage_name);
                                            self.output.push_str(&format!(
                                                "{}if {} < str_arr_vars.len() {{ str_arr_vars[{}] = Vec::new(); str_arr_bounds[{}] = Vec::new(); }}\n",
                                                indent, idx, idx, idx
                                            ));
                                        }
                                        QType::Integer(_)
                                        | QType::Long(_)
                                        | QType::Single(_)
                                        | QType::Double(_) => {
                                            let idx = self.get_arr_var_idx(&field.storage_name);
                                            self.output.push_str(&format!(
                                                "{}if {} < arr_vars.len() {{ arr_vars[{}] = Vec::new(); arr_bounds[{}] = Vec::new(); }}\n",
                                                indent, idx, idx, idx
                                            ));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            QType::String(_) => {
                                let idx = self.get_str_arr_var_idx(&var.name);
                                self.output.push_str(&format!(
                                    "{}if {} < str_arr_vars.len() {{ str_arr_vars[{}] = Vec::new(); str_arr_bounds[{}] = Vec::new(); }}\n",
                                    indent, idx, idx, idx
                                ));
                            }
                            qtype if Self::qtype_is_numeric(&qtype) => {
                                let idx = self.get_arr_var_idx(&var.name);
                                self.output.push_str(&format!(
                                    "{}if {} < arr_vars.len() {{ arr_vars[{}] = Vec::new(); arr_bounds[{}] = Vec::new(); }}\n",
                                    indent, idx, idx, idx
                                ));
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if self.array_is_string(&var.name, var.type_suffix) {
                        let idx = self.get_str_arr_var_idx(&var.name);
                        self.output.push_str(&format!(
                            "{}if {} < str_arr_vars.len() {{ str_arr_vars[{}] = Vec::new(); str_arr_bounds[{}] = Vec::new(); }}\n",
                            indent, idx, idx, idx
                        ));
                    } else {
                        let idx = self.get_arr_var_idx(&var.name);
                        self.output.push_str(&format!(
                            "{}if {} < arr_vars.len() {{ arr_vars[{}] = Vec::new(); arr_bounds[{}] = Vec::new(); }}\n",
                            indent, idx, idx, idx
                        ));
                    }
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

            Statement::IfElseBlock {
                condition,
                then_branch,
                else_ifs,
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

                for (else_if_condition, branch) in else_ifs {
                    let cond_code = self.generate_expression(else_if_condition)?;
                    self.output.push_str(&format!(
                        "{}}} else if ({} as i32) != 0 {{\n",
                        indent, cond_code
                    ));
                    self.indent_level += 1;
                    for stmt in branch {
                        self.generate_statement(stmt)?;
                    }
                    self.indent_level -= 1;
                }

                if let Some(branch) = else_branch {
                    self.output.push_str(&format!("{}}} else {{\n", indent));
                    self.indent_level += 1;
                    for stmt in branch {
                        self.generate_statement(stmt)?;
                    }
                    self.indent_level -= 1;
                }

                self.output.push_str(&format!("{}}}\n", indent));
            }

            Statement::Select { expression, cases } => {
                let expr_code = self.generate_expression(expression)?;
                let temp = self.next_temp_var();
                let select_is_string = self.is_string_expression(expression);
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
                let end_tmp = self.next_temp_var();
                let step_tmp = self.next_temp_var();

                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, {} as f64);\n",
                    indent, var_idx, start_code
                ));
                self.output.push_str(&format!(
                    "{}let {} = {} as f64;\n",
                    indent, end_tmp, end_code
                ));
                self.output.push_str(&format!(
                    "{}let {} = {} as f64;\n",
                    indent, step_tmp, step_code
                ));
                self.output.push_str(&format!(
                    "{}while if {} >= 0.0 {{ get_var(&num_vars, {}) <= {} }} else {{ get_var(&num_vars, {}) >= {} }} {{\n",
                    indent, step_tmp, var_idx, end_tmp, var_idx, end_tmp
                ));
                self.indent_level += 1;
                for stmt in body {
                    self.generate_statement(stmt)?;
                }
                self.output.push_str(&format!(
                    "{}let _next = get_var(&num_vars, {}) + {};\n",
                    self.indent(),
                    var_idx,
                    step_tmp
                ));
                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, _next);\n",
                    self.indent(),
                    var_idx
                ));
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }

            Statement::ForEach {
                variable,
                array,
                body,
            } => {
                if let Some(array_name) = self.expr_to_array_name(array) {
                    let arr_idx = self.get_arr_var_idx(&array_name);
                    let var_idx = self.get_num_var_idx(&variable.name);
                    let item_tmp = self.next_temp_var();
                    self.output.push_str(&format!(
                        "{}for {} in arr_vars[{}].clone() {{\n",
                        indent, item_tmp, arr_idx
                    ));
                    self.indent_level += 1;
                    self.output.push_str(&format!(
                        "{}set_var(&mut num_vars, {}, {});\n",
                        self.indent(),
                        var_idx,
                        item_tmp
                    ));
                    for stmt in body {
                        self.generate_statement(stmt)?;
                    }
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", indent));
                }
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
                self.output.push_str(&format!("{}loop {{\n", indent));
                self.indent_level += 1;
                if *pre_condition {
                    if let Some(cond) = condition {
                        let cond_code = self.generate_condition(cond);
                        self.output.push_str(&format!(
                            "{}if {} {{ break; }}\n",
                            self.indent(),
                            cond_code
                        ));
                    }
                }
                for stmt in body {
                    self.generate_statement(stmt)?;
                }
                if !*pre_condition {
                    if let Some(cond) = condition {
                        let cond_code = self.generate_condition(cond);
                        self.output.push_str(&format!(
                            "{}if {} {{ break; }}\n",
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
                let param_modes = self.param_modes_for_call(name, false, args.len());

                for (index, arg) in args.iter().enumerate() {
                    let mut setup_lines = Vec::new();
                    let arg_var = self.prepare_call_argument(
                        arg,
                        !param_modes.get(index).copied().unwrap_or(false),
                        &mut setup_lines,
                        &mut copy_back_ops,
                    )?;
                    for line in setup_lines {
                        self.output.push_str(&format!("{}{}\n", indent, line));
                    }
                    arg_vars.push(arg_var);
                }

                let global_args = if self.is_in_sub {
                    "global_num_vars, global_str_vars, global_arr_vars, global_arr_bounds, global_str_arr_vars, global_str_arr_bounds"
                } else {
                    "&mut num_vars, &mut str_vars, &mut arr_vars, &mut arr_bounds, &mut str_arr_vars, &mut str_arr_bounds"
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
                ExitType::Sub => {
                    if self.is_in_sub {
                        self.output
                            .push_str(&format!("{}break 'qb_proc;\n", indent));
                    } else {
                        self.output.push_str(&format!("{}return;\n", indent));
                    }
                }
                ExitType::Function => {
                    if self.is_in_sub {
                        self.output
                            .push_str(&format!("{}break 'qb_proc;\n", indent));
                    } else {
                        let func_name = self.current_function_name.clone();
                        if let Some(name) = func_name {
                            if self.function_returns_string(&name, None) {
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
                }
                ExitType::For => self.output.push_str(&format!("{}break;\n", indent)),
                ExitType::Do => self.output.push_str(&format!("{}break;\n", indent)),
                ExitType::While => self.output.push_str(&format!("{}break;\n", indent)),
            },

            Statement::Screen { mode } => {
                let mode_code = if let Some(m) = mode {
                    self.generate_expression(m).unwrap_or("0.0".to_string())
                } else {
                    "0.0".to_string()
                };
                self.output.push_str(&format!(
                    "{}qb_apply_screen_mode({} as i32);\n",
                    indent, mode_code
                ));
                if self.use_graphics {
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
                self.output
                    .push_str(&format!("{}qb_view_print_reset();\n", indent));
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
                    if self.variable_is_string(var) {
                        let var_idx = self.get_str_var_idx(var_name);
                        self.output
                            .push_str(&format!("{}if data_idx < DATA_VALUES.len() {{\n", indent));
                        self.output.push_str(&format!(
                            "{}    set_str(&mut str_vars, {}, DATA_VALUES[data_idx]);\n",
                            indent, var_idx
                        ));
                        self.output
                            .push_str(&format!("{}    data_idx += 1;\n", indent));
                        self.output.push_str(&format!("{}}}\n", indent));
                    } else {
                        let var_idx = self.get_num_var_idx(var_name);
                        self.output
                            .push_str(&format!("{}if data_idx < DATA_VALUES.len() {{\n", indent));
                        self.output.push_str(&format!(
                            "{}    set_var(&mut num_vars, {}, qb_data_to_num(DATA_VALUES[data_idx]));\n",
                            indent, var_idx
                        ));
                        self.output
                            .push_str(&format!("{}    data_idx += 1;\n", indent));
                        self.output.push_str(&format!("{}}}\n", indent));
                    }
                }
            }

            Statement::Restore { label } => {
                let data_index = label
                    .as_deref()
                    .map(Self::normalize_restore_target)
                    .and_then(|target| self.restore_targets.get(&target).copied())
                    .unwrap_or(0);
                self.output
                    .push_str(&format!("{}data_idx = {};\n", indent, data_index));
            }

            Statement::Open {
                filename,
                mode,
                file_number,
                ..
            } => {
                let filename = self.generate_expression(filename)?;
                let file_number = self.generate_expression(file_number)?;
                let mode_name = match mode {
                    OpenMode::Input => "INPUT",
                    OpenMode::Output => "OUTPUT",
                    OpenMode::Append => "APPEND",
                    OpenMode::Binary => "BINARY",
                    OpenMode::Random => "RANDOM",
                };
                self.output.push_str(&format!(
                    "{}qb_open(&{}, \"{}\", {});\n",
                    indent, filename, mode_name, file_number
                ));
            }

            Statement::Close { file_numbers } => {
                if file_numbers.is_empty() {
                    self.output
                        .push_str(&format!("{}qb_close_all();\n", indent));
                } else {
                    for file_number in file_numbers {
                        let file_number = self.generate_expression(file_number)?;
                        self.output
                            .push_str(&format!("{}qb_close({});\n", indent, file_number));
                    }
                }
            }

            Statement::InputFile {
                file_number,
                variables,
            } => {
                let file_number = self.generate_expression(file_number)?;
                let fields_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {} = qb_input_file_fields({}, {});\n",
                    indent,
                    fields_tmp,
                    file_number,
                    variables.len()
                ));
                for (index, variable) in variables.iter().enumerate() {
                    self.generate_store_from_text_value(
                        &indent,
                        variable,
                        &format!("{}[{}].clone()", fields_tmp, index),
                        false,
                    )?;
                }
            }

            Statement::Field {
                file_number,
                fields,
            } => {
                let file_number = self.generate_expression(file_number)?;
                let fields_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {}: Vec<(usize, usize)> = vec![\n",
                    indent, fields_tmp
                ));
                for (width_expr, field_expr) in fields {
                    let width =
                        self.evaluate_const_expr(width_expr).unwrap_or(0.0).max(0.0) as usize;
                    if let Expression::Variable(var) = field_expr {
                        let idx = self.get_str_var_idx(&var.name);
                        self.field_widths.insert(var.name.to_uppercase(), width);
                        self.output
                            .push_str(&format!("{}    ({}, {}),\n", indent, width, idx));
                    }
                }
                self.output.push_str(&format!("{}];\n", indent));
                self.output.push_str(&format!(
                    "{}qb_define_fields({}, {});\n",
                    indent, file_number, fields_tmp
                ));
            }

            Statement::Cls { mode } => {
                let mode = mode
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "-1.0".to_string());
                self.output
                    .push_str(&format!("{}qb_cls({});\n", indent, mode));
            }

            Statement::Locate {
                row,
                col,
                cursor,
                start,
                stop,
            } => {
                let row = row
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "0.0".to_string());
                let col = col
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "0.0".to_string());
                if cursor.is_some() || start.is_some() || stop.is_some() {
                    let cursor = cursor
                        .as_ref()
                        .map(|expr| self.generate_expression(expr))
                        .transpose()?
                        .unwrap_or_else(|| "-1.0".to_string());
                    let start = start
                        .as_ref()
                        .map(|expr| self.generate_expression(expr))
                        .transpose()?
                        .unwrap_or_else(|| "-1.0".to_string());
                    let stop = stop
                        .as_ref()
                        .map(|expr| self.generate_expression(expr))
                        .transpose()?
                        .unwrap_or_else(|| "-1.0".to_string());
                    self.output.push_str(&format!(
                        "{}locate_ex({}, {}, {}, {}, {});\n",
                        indent, row, col, cursor, start, stop
                    ));
                } else {
                    self.output
                        .push_str(&format!("{}locate({}, {});\n", indent, row, col));
                }
            }

            Statement::Width { columns, rows } => {
                let columns = self.generate_expression(columns)?;
                let rows = rows
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "25.0".to_string());
                self.output
                    .push_str(&format!("{}qb_width({}, {});\n", indent, columns, rows));
            }

            Statement::ViewPrint { top, bottom } => {
                let top = top
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "1.0".to_string());
                let bottom = bottom
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "25.0".to_string());
                self.output
                    .push_str(&format!("{}qb_view_print({}, {});\n", indent, top, bottom));
            }

            Statement::Beep => {
                self.output.push_str(&format!("{}qb_beep();\n", indent));
            }

            Statement::Sound {
                frequency,
                duration,
            } => {
                let frequency = self.generate_expression(frequency)?;
                let duration = self.generate_expression(duration)?;
                self.output.push_str(&format!(
                    "{}qb_sound({}, {});\n",
                    indent, frequency, duration
                ));
            }

            Statement::Play { melody } => {
                let melody = self.generate_expression(melody)?;
                self.output
                    .push_str(&format!("{}qb_play(&{});\n", indent, melody));
            }

            Statement::Randomize { seed } => {
                if let Some(seed) = seed {
                    let seed_code = self.generate_expression(seed)?;
                    self.output
                        .push_str(&format!("{}qb_randomize(Some({}));\n", indent, seed_code));
                } else {
                    self.output
                        .push_str(&format!("{}qb_randomize(None);\n", indent));
                }
            }

            Statement::Color {
                foreground,
                background,
            } => {
                let fg = foreground
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "7.0".to_string());
                let bg = background
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "0.0".to_string());
                self.output
                    .push_str(&format!("{}qb_color({}, {});\n", indent, fg, bg));
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

            Statement::Clear => {
                self.output.push_str(&format!(
                    "{}qb_clear(&mut num_vars, &mut str_vars, &mut arr_vars, &mut str_arr_vars);\n",
                    indent
                ));
                self.emit_restore_consts(&indent)?;
            }

            Statement::Error { code } => {
                let code = self.generate_expression(code)?;
                let code_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {} = (({}) as f64).round() as i16;\n",
                    indent, code_tmp, code
                ));
                self.output.push_str(&format!(
                    "{}qb_runtime_fail_code({}, format!(\"ERROR {{}}\", {}));\n",
                    indent, code_tmp, code_tmp
                ));
            }

            Statement::Kill { filename } => {
                let filename = self.generate_expression(filename)?;
                self.output
                    .push_str(&format!("{}qb_kill(&{});\n", indent, filename));
            }

            Statement::NameFile { old_name, new_name } => {
                let old_name = self.generate_expression(old_name)?;
                let new_name = self.generate_expression(new_name)?;
                self.output.push_str(&format!(
                    "{}qb_rename(&{}, &{});\n",
                    indent, old_name, new_name
                ));
            }

            Statement::Files { pattern } => {
                if let Some(pattern) = pattern {
                    let pattern_code = self.generate_expression(pattern)?;
                    let temp = self.next_temp_var();
                    self.output
                        .push_str(&format!("{}let {} = {};\n", indent, temp, pattern_code));
                    self.output
                        .push_str(&format!("{}qb_files(Some(&{}));\n", indent, temp));
                } else {
                    self.output
                        .push_str(&format!("{}qb_files(None);\n", indent));
                }
            }

            Statement::ChDir { path } => {
                let path = self.generate_expression(path)?;
                self.output
                    .push_str(&format!("{}qb_chdir(&{});\n", indent, path));
            }

            Statement::MkDir { path } => {
                let path = self.generate_expression(path)?;
                self.output
                    .push_str(&format!("{}qb_mkdir(&{});\n", indent, path));
            }

            Statement::RmDir { path } => {
                let path = self.generate_expression(path)?;
                self.output
                    .push_str(&format!("{}qb_rmdir(&{});\n", indent, path));
            }

            Statement::Shell { command } => {
                if let Some(command) = command {
                    let command = self.generate_expression(command)?;
                    let tmp = self.next_temp_var();
                    self.output
                        .push_str(&format!("{}let {} = {};\n", indent, tmp, command));
                    self.output
                        .push_str(&format!("{}qb_shell(Some(&{}));\n", indent, tmp));
                } else {
                    self.output
                        .push_str(&format!("{}qb_shell(None);\n", indent));
                }
            }

            Statement::Chain { filename, .. } => {
                let filename = self.generate_expression(filename)?;
                let tmp = self.next_temp_var();
                self.output
                    .push_str(&format!("{}let {} = {};\n", indent, tmp, filename));
                self.output
                    .push_str(&format!("{}qb_chain(&{});\n", indent, tmp));
            }

            Statement::Get {
                file_number,
                record,
                variable,
            } => {
                let file_number = self.generate_expression(file_number)?;
                let record = record
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "0.0".to_string());
                match variable {
                    None => {
                        self.output.push_str(&format!(
                            "{}qb_get_record_to_fields({}, {}, &mut str_vars);\n",
                            indent, file_number, record
                        ));
                    }
                    Some(expr)
                        if self.resolve_udt_object_access(expr).is_some()
                            && !matches!(
                                self.resolve_field_access_layout(expr)
                                    .as_ref()
                                    .map(|field| &field.field_type),
                                Some(
                                    QType::String(_)
                                        | QType::Integer(_)
                                        | QType::Long(_)
                                        | QType::Single(_)
                                        | QType::Double(_)
                                )
                            ) =>
                    {
                        let resolved = self.resolve_udt_object_access(expr).unwrap();
                        self.emit_udt_record_get(
                            &indent,
                            &resolved.storage_base,
                            &resolved.type_name,
                            &file_number,
                            &record,
                            resolved.array_indices.as_deref(),
                        );
                    }
                    Some(expr) if self.resolve_field_access_layout(expr).is_some() => {
                        let field = self.resolve_field_access_layout(expr).unwrap();
                        self.emit_udt_field_get(&indent, &field, &file_number, &record)?;
                    }
                    Some(Expression::Variable(var)) if self.variable_udt_type(var).is_some() => {
                        let type_name = self.variable_udt_type(var).unwrap();
                        self.emit_udt_record_get(
                            &indent,
                            &var.name,
                            &type_name,
                            &file_number,
                            &record,
                            None,
                        );
                    }
                    Some(Expression::Variable(var)) if self.variable_is_string(var) => {
                        let idx = self.get_str_var_idx(&var.name);
                        let width = self
                            .field_widths
                            .get(&var.name.to_uppercase())
                            .copied()
                            .unwrap_or(0);
                        let tmp = self.next_temp_var();
                        self.output.push_str(&format!(
                            "{}let {} = qb_get_string_from_file({}, {}, {});\n",
                            indent,
                            tmp,
                            file_number,
                            record,
                            if width > 0 {
                                format!("{}.0", width)
                            } else {
                                format!("get_str(&str_vars, {}).len() as f64", idx)
                            }
                        ));
                        self.output.push_str(&format!(
                            "{}set_str(&mut str_vars, {}, &{});\n",
                            indent, idx, tmp
                        ));
                    }
                    Some(Expression::Variable(var)) => {
                        let idx = self.get_num_var_idx(&var.name);
                        let helper = match Self::binary_scalar_kind(var) {
                            "i16" => "qb_get_i16",
                            "i32" => "qb_get_i32",
                            "f64" => "qb_get_f64",
                            _ => "qb_get_f32",
                        };
                        let tmp = self.next_temp_var();
                        self.output.push_str(&format!(
                            "{}let {} = {}({}, {});\n",
                            indent, tmp, helper, file_number, record
                        ));
                        self.output.push_str(&format!(
                            "{}set_var(&mut num_vars, {}, {});\n",
                            indent, idx, tmp
                        ));
                    }
                    Some(Expression::ArrayAccess {
                        name,
                        indices,
                        type_suffix,
                    }) if self.array_is_string(name, *type_suffix) => {
                        if !indices.is_empty() {
                            let arr_idx = self.get_str_arr_var_idx(name);
                            let width = self.fixed_string_width_for_name(name);
                            let (setup_lines, index_slice) =
                                self.native_cached_array_indices(indices)?;
                            let len_tmp = self.next_temp_var();
                            let value_tmp = self.next_temp_var();
                            for line in setup_lines {
                                self.output.push_str(&format!("{}{}\n", indent, line));
                            }
                            self.output.push_str(&format!(
                                "{}let {} = str_arr_get(&mut str_arr_vars, &mut str_arr_bounds, {}, {});\n",
                                indent, len_tmp, arr_idx, index_slice
                            ));
                            self.output.push_str(&format!(
                                "{}let {} = qb_get_string_from_file({}, {}, {});\n",
                                indent,
                                value_tmp,
                                file_number,
                                record,
                                width
                                    .map(|width| format!("{}.0", width))
                                    .unwrap_or_else(|| format!("{}.len() as f64", len_tmp))
                            ));
                            self.output.push_str(&format!(
                                "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{});\n",
                                indent, arr_idx, index_slice, value_tmp
                            ));
                        }
                    }
                    Some(Expression::ArrayAccess {
                        name,
                        indices,
                        type_suffix,
                    }) if !self.array_is_string(name, *type_suffix) => {
                        if !indices.is_empty() {
                            let arr_idx = self.get_arr_var_idx(name);
                            let (setup_lines, index_slice) =
                                self.native_cached_array_indices(indices)?;
                            let helper = match type_suffix.unwrap_or('!') {
                                '%' => "qb_get_i16",
                                '&' => "qb_get_i32",
                                '#' => "qb_get_f64",
                                _ => "qb_get_f32",
                            };
                            let tmp = self.next_temp_var();
                            for line in setup_lines {
                                self.output.push_str(&format!("{}{}\n", indent, line));
                            }
                            self.output.push_str(&format!(
                                "{}let {} = {}({}, {});\n",
                                indent, tmp, helper, file_number, record
                            ));
                            self.output.push_str(&format!(
                                "{}arr_set(&mut arr_vars, &mut arr_bounds, {}, {}, {});\n",
                                indent, arr_idx, index_slice, tmp
                            ));
                        }
                    }
                    Some(_) => {
                        return Err(core_types::QError::UnsupportedFeature(
                            "native GET target not supported for this expression".to_string(),
                        ));
                    }
                }
            }

            Statement::Put {
                file_number,
                record,
                variable,
            } => {
                let file_number = self.generate_expression(file_number)?;
                let record = record
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .unwrap_or_else(|| "0.0".to_string());
                match variable {
                    None => {
                        self.output.push_str(&format!(
                            "{}qb_put_record_from_fields({}, {}, &str_vars);\n",
                            indent, file_number, record
                        ));
                    }
                    Some(expr)
                        if self.resolve_udt_object_access(expr).is_some()
                            && !matches!(
                                self.resolve_field_access_layout(expr)
                                    .as_ref()
                                    .map(|field| &field.field_type),
                                Some(
                                    QType::String(_)
                                        | QType::Integer(_)
                                        | QType::Long(_)
                                        | QType::Single(_)
                                        | QType::Double(_)
                                )
                            ) =>
                    {
                        let resolved = self.resolve_udt_object_access(expr).unwrap();
                        self.emit_udt_record_put(
                            &indent,
                            &resolved.storage_base,
                            &resolved.type_name,
                            &file_number,
                            &record,
                            resolved.array_indices.as_deref(),
                        );
                    }
                    Some(expr) if self.resolve_field_access_layout(expr).is_some() => {
                        let field = self.resolve_field_access_layout(expr).unwrap();
                        self.emit_udt_field_put(&indent, &field, &file_number, &record)?;
                    }
                    Some(Expression::Variable(var)) if self.variable_udt_type(var).is_some() => {
                        let type_name = self.variable_udt_type(var).unwrap();
                        self.emit_udt_record_put(
                            &indent,
                            &var.name,
                            &type_name,
                            &file_number,
                            &record,
                            None,
                        );
                    }
                    Some(Expression::Variable(var)) if self.variable_is_string(var) => {
                        let idx = self.get_str_var_idx(&var.name);
                        let width = self
                            .field_widths
                            .get(&var.name.to_uppercase())
                            .copied()
                            .unwrap_or(0);
                        let tmp = self.next_temp_var();
                        self.output.push_str(&format!(
                            "{}let {} = get_str(&str_vars, {});\n",
                            indent, tmp, idx
                        ));
                        if width > 0 {
                            self.output.push_str(&format!(
                                "{}qb_put_fixed_string({}, {}, {}, &{});\n",
                                indent, file_number, record, width, tmp
                            ));
                        } else {
                            self.output.push_str(&format!(
                                "{}qb_put_string({}, {}, &{});\n",
                                indent, file_number, record, tmp
                            ));
                        }
                    }
                    Some(Expression::Variable(var)) => {
                        let idx = self.get_num_var_idx(&var.name);
                        let helper = match Self::binary_scalar_kind(var) {
                            "i16" => "qb_put_i16",
                            "i32" => "qb_put_i32",
                            "f64" => "qb_put_f64",
                            _ => "qb_put_f32",
                        };
                        let tmp = self.next_temp_var();
                        self.output.push_str(&format!(
                            "{}let {} = get_var(&num_vars, {});\n",
                            indent, tmp, idx
                        ));
                        self.output.push_str(&format!(
                            "{}{}({}, {}, {});\n",
                            indent, helper, file_number, record, tmp
                        ));
                    }
                    Some(Expression::ArrayAccess {
                        name,
                        indices,
                        type_suffix,
                    }) if self.array_is_string(name, *type_suffix) => {
                        if !indices.is_empty() {
                            let arr_idx = self.get_str_arr_var_idx(name);
                            let width = self.fixed_string_width_for_name(name);
                            let (setup_lines, index_slice) =
                                self.native_cached_array_indices(indices)?;
                            let tmp = self.next_temp_var();
                            for line in setup_lines {
                                self.output.push_str(&format!("{}{}\n", indent, line));
                            }
                            self.output.push_str(&format!(
                                "{}let {} = str_arr_get(&mut str_arr_vars, &mut str_arr_bounds, {}, {});\n",
                                indent, tmp, arr_idx, index_slice
                            ));
                            if let Some(width) = width {
                                self.output.push_str(&format!(
                                    "{}qb_put_fixed_string({}, {}, {}, &{});\n",
                                    indent, file_number, record, width, tmp
                                ));
                            } else {
                                self.output.push_str(&format!(
                                    "{}qb_put_string({}, {}, &{});\n",
                                    indent, file_number, record, tmp
                                ));
                            }
                        }
                    }
                    Some(Expression::ArrayAccess {
                        name,
                        indices,
                        type_suffix,
                    }) if !self.array_is_string(name, *type_suffix) => {
                        if !indices.is_empty() {
                            let arr_idx = self.get_arr_var_idx(name);
                            let (setup_lines, index_slice) =
                                self.native_cached_array_indices(indices)?;
                            let helper = match type_suffix.unwrap_or('!') {
                                '%' => "qb_put_i16",
                                '&' => "qb_put_i32",
                                '#' => "qb_put_f64",
                                _ => "qb_put_f32",
                            };
                            let tmp = self.next_temp_var();
                            for line in setup_lines {
                                self.output.push_str(&format!("{}{}\n", indent, line));
                            }
                            self.output.push_str(&format!(
                                "{}let {} = arr_get(&mut arr_vars, &mut arr_bounds, {}, {});\n",
                                indent, tmp, arr_idx, index_slice
                            ));
                            self.output.push_str(&format!(
                                "{}{}({}, {}, {});\n",
                                indent, helper, file_number, record, tmp
                            ));
                        }
                    }
                    Some(_) => {
                        return Err(core_types::QError::UnsupportedFeature(
                            "native PUT target not supported for this expression".to_string(),
                        ));
                    }
                }
            }

            Statement::LineInputFile {
                file_number,
                variable,
            } => {
                let file_number = self.generate_expression(file_number)?;
                let line_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let {} = qb_read_line_from_file({});\n",
                    indent, line_tmp, file_number
                ));
                self.generate_store_from_text_value(&indent, variable, &line_tmp, true)?;
            }

            Statement::WriteFile {
                file_number,
                expressions,
            } => {
                let file_number = self.generate_expression(file_number)?;
                let values_tmp = self.next_temp_var();
                self.output.push_str(&format!(
                    "{}let mut {}: Vec<String> = Vec::new();\n",
                    indent, values_tmp
                ));
                for expr in expressions {
                    let expr_code = self.generate_expression(expr)?;
                    if self.is_string_expression(expr) {
                        self.output.push_str(&format!(
                            "{}{}.push(qb_write_string_field(&{}));\n",
                            indent, values_tmp, expr_code
                        ));
                    } else {
                        self.output.push_str(&format!(
                            "{}{}.push(qb_write_numeric_field({}));\n",
                            indent, values_tmp, expr_code
                        ));
                    }
                }
                self.output.push_str(&format!(
                    "{}qb_file_write_csv({}, &{});\n",
                    indent, file_number, values_tmp
                ));
            }

            Statement::Seek {
                file_number,
                position,
            } => {
                let file_number = self.generate_expression(file_number)?;
                let position = self.generate_expression(position)?;
                self.output.push_str(&format!(
                    "{}qb_seek({}, {});\n",
                    indent, file_number, position
                ));
            }

            Statement::LSet { target, value } => {
                if let Some(field) = self.resolve_field_access_layout(target) {
                    if matches!(field.field_type, QType::String(_)) {
                        let width = field.fixed_length.unwrap_or(0);
                        let value = self.generate_expression(value)?;
                        if let Some(indices) = field.array_indices.as_ref() {
                            let idx = self.get_str_arr_var_idx(&field.storage_name);
                            let index_code = self.native_array_indices_expr(indices)?;
                            if width == 0 {
                                self.output.push_str(&format!(
                                    "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &format!(\"{{}}\", {}));\n",
                                    indent, idx, index_code, value
                                ));
                            } else {
                                let tmp = self.next_temp_var();
                                self.output.push_str(&format!(
                                    "{}let mut {} = str_arr_get(&mut str_arr_vars, &mut str_arr_bounds, {}, {});\n",
                                    indent, tmp, idx, index_code
                                ));
                                self.output.push_str(&format!(
                                    "{}qb_lset_field(std::slice::from_mut(&mut {}), 0, {}, &format!(\"{{}}\", {}));\n",
                                    indent, tmp, width, value
                                ));
                                self.output.push_str(&format!(
                                    "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{});\n",
                                    indent, idx, index_code, tmp
                                ));
                            }
                        } else {
                            let idx = self.get_str_var_idx(&field.storage_name);
                            if width == 0 {
                                self.output.push_str(&format!(
                                    "{}set_str(&mut str_vars, {}, &format!(\"{{}}\", {}));\n",
                                    indent, idx, value
                                ));
                            } else {
                                self.output.push_str(&format!(
                                    "{}qb_lset_field(&mut str_vars, {}, {}, &format!(\"{{}}\", {}));\n",
                                    indent, idx, width, value
                                ));
                            }
                        }
                    }
                } else if let Expression::Variable(var) = target {
                    let idx = self.get_str_var_idx(&var.name);
                    let width = self
                        .field_widths
                        .get(&var.name.to_uppercase())
                        .copied()
                        .unwrap_or(0);
                    let value = self.generate_expression(value)?;
                    if width == 0 {
                        self.output.push_str(&format!(
                            "{}set_str(&mut str_vars, {}, &format!(\"{{}}\", {}));\n",
                            indent, idx, value
                        ));
                    } else {
                        self.output.push_str(&format!(
                            "{}qb_lset_field(&mut str_vars, {}, {}, &format!(\"{{}}\", {}));\n",
                            indent, idx, width, value
                        ));
                    }
                }
            }

            Statement::RSet { target, value } => {
                if let Some(field) = self.resolve_field_access_layout(target) {
                    if matches!(field.field_type, QType::String(_)) {
                        let width = field.fixed_length.unwrap_or(0);
                        let value = self.generate_expression(value)?;
                        if let Some(indices) = field.array_indices.as_ref() {
                            let idx = self.get_str_arr_var_idx(&field.storage_name);
                            let index_code = self.native_array_indices_expr(indices)?;
                            if width == 0 {
                                self.output.push_str(&format!(
                                    "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &format!(\"{{}}\", {}));\n",
                                    indent, idx, index_code, value
                                ));
                            } else {
                                let tmp = self.next_temp_var();
                                self.output.push_str(&format!(
                                    "{}let mut {} = str_arr_get(&mut str_arr_vars, &mut str_arr_bounds, {}, {});\n",
                                    indent, tmp, idx, index_code
                                ));
                                self.output.push_str(&format!(
                                    "{}qb_rset_field(std::slice::from_mut(&mut {}), 0, {}, &format!(\"{{}}\", {}));\n",
                                    indent, tmp, width, value
                                ));
                                self.output.push_str(&format!(
                                    "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{});\n",
                                    indent, idx, index_code, tmp
                                ));
                            }
                        } else {
                            let idx = self.get_str_var_idx(&field.storage_name);
                            if width == 0 {
                                self.output.push_str(&format!(
                                    "{}set_str(&mut str_vars, {}, &format!(\"{{}}\", {}));\n",
                                    indent, idx, value
                                ));
                            } else {
                                self.output.push_str(&format!(
                                    "{}qb_rset_field(&mut str_vars, {}, {}, &format!(\"{{}}\", {}));\n",
                                    indent, idx, width, value
                                ));
                            }
                        }
                    }
                } else if let Expression::Variable(var) = target {
                    let idx = self.get_str_var_idx(&var.name);
                    let width = self
                        .field_widths
                        .get(&var.name.to_uppercase())
                        .copied()
                        .unwrap_or(0);
                    let value = self.generate_expression(value)?;
                    if width == 0 {
                        self.output.push_str(&format!(
                            "{}set_str(&mut str_vars, {}, &format!(\"{{}}\", {}));\n",
                            indent, idx, value
                        ));
                    } else {
                        self.output.push_str(&format!(
                            "{}qb_rset_field(&mut str_vars, {}, {}, &format!(\"{{}}\", {}));\n",
                            indent, idx, width, value
                        ));
                    }
                }
            }

            Statement::End | Statement::Stop | Statement::System => {
                if self.is_in_sub {
                    self.output
                        .push_str(&format!("{}break 'qb_proc;\n", indent));
                } else {
                    self.output.push_str(&format!("{}return;\n", indent));
                }
            }

            Statement::Label { .. } => {}

            Statement::LineNumber { number } => {
                self.output
                    .push_str(&format!("{}qb_set_current_line({});\n", indent, number));
            }

            Statement::LineInput { prompt, variable } => {
                if let Some(p) = prompt {
                    let prompt_code = self.generate_printable_expression(p)?;
                    self.output
                        .push_str(&format!("{}qb_print(&{});\n", indent, prompt_code));
                }
                if let Expression::Variable(v) = variable {
                    let idx = self.get_str_var_idx(&v.name);
                    self.output
                        .push_str(&format!("{}io::stdout().flush().unwrap();\n", indent));
                    self.output
                        .push_str(&format!("{}let mut _line_input = String::new();\n", indent));
                    self.output.push_str(&format!(
                        "{}io::stdin().read_line(&mut _line_input).unwrap();\n",
                        indent
                    ));
                    self.output.push_str(&format!(
                        "{}set_str(&mut str_vars, {}, _line_input.trim_end_matches(['\\r', '\\n']));\n",
                        indent, idx
                    ));
                }
            }

            Statement::DefSeg { segment } => {
                let segment_code = if let Some(segment) = segment {
                    self.generate_expression(segment)?
                } else {
                    "0.0".to_string()
                };
                self.output
                    .push_str(&format!("{}qb_set_def_seg({});\n", indent, segment_code));
            }

            Statement::Poke { address, value } => {
                let address_code = self.generate_expression(address)?;
                let value_code = self.generate_expression(value)?;
                self.output.push_str(&format!(
                    "{}qb_poke({}, {});\n",
                    indent, address_code, value_code
                ));
            }

            Statement::Wait {
                address,
                and_mask,
                xor_mask,
            } => {
                let address_code = self.generate_expression(address)?;
                let and_mask_code = self.generate_expression(and_mask)?;
                let xor_mask_code = xor_mask
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .map(|expr| format!("Some({})", expr))
                    .unwrap_or_else(|| "None".to_string());
                self.output.push_str(&format!(
                    "{}qb_wait({}, {}, {});\n",
                    indent, address_code, and_mask_code, xor_mask_code
                ));
            }

            Statement::BLoad { filename, offset } => {
                let filename_code = self.generate_expression(filename)?;
                let offset_code = offset
                    .as_ref()
                    .map(|expr| self.generate_expression(expr))
                    .transpose()?
                    .map(|expr| format!("Some({})", expr))
                    .unwrap_or_else(|| "None".to_string());
                self.output.push_str(&format!(
                    "{}qb_bload(&{}, {});\n",
                    indent, filename_code, offset_code
                ));
            }

            Statement::BSave {
                filename,
                offset,
                length,
            } => {
                let filename_code = self.generate_expression(filename)?;
                let offset_code = self.generate_expression(offset)?;
                let length_code = self.generate_expression(length)?;
                self.output.push_str(&format!(
                    "{}qb_bsave(&{}, {}, {});\n",
                    indent, filename_code, offset_code, length_code
                ));
            }

            Statement::Out { port, value } => {
                let port_code = self.generate_expression(port)?;
                let value_code = self.generate_expression(value)?;
                self.output.push_str(&format!(
                    "{}qb_out({}, {});\n",
                    indent, port_code, value_code
                ));
            }

            Statement::TrOn => {
                self.output.push_str(&format!("{}qb_tron();\n", indent));
            }

            Statement::TrOff => {
                self.output.push_str(&format!("{}qb_troff();\n", indent));
            }

            Statement::Key {
                key_num,
                key_string,
            } => {
                let key_num = self.generate_expression(key_num)?;
                let key_string = self.generate_expression(key_string)?;
                self.output.push_str(&format!(
                    "{}qb_key_set({}, &{});\n",
                    indent, key_num, key_string
                ));
            }

            Statement::KeyOn => {
                self.output.push_str(&format!("{}qb_key_on();\n", indent));
            }

            Statement::KeyOff => {
                self.output.push_str(&format!("{}qb_key_off();\n", indent));
            }

            Statement::KeyList => {
                self.output.push_str(&format!("{}qb_key_list();\n", indent));
            }

            Statement::OptionBase { base } => {
                self.output
                    .push_str(&format!("{}qb_set_option_base({});\n", indent, base));
            }

            Statement::Declare { .. } => {}

            Statement::Const { name, value } => {
                self.record_const(name, value);
                self.emit_const_assignment(&indent, name, value)?;
            }

            Statement::DefFn { name, params, body } => {
                let fn_name = self.rust_symbol("qbfn", name);
                let saved_params = self.params.clone();
                self.params.clear();
                let param_list = params
                    .iter()
                    .map(|p| format!("{}: f64", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                for param in params {
                    self.params.insert(param.clone(), param.clone());
                }
                let body_code = self.generate_expression(body)?;
                self.params = saved_params;

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
                    let prompt_code = self.generate_printable_expression(p)?;
                    self.output
                        .push_str(&format!("{}qb_print(&{});\n", indent, prompt_code));
                }

                // Read input for each variable
                for var in variables {
                    if let Expression::Variable(v) = var {
                        let var_name = &v.name;
                        if self.variable_is_string(v) {
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

    fn record_const(&mut self, name: &str, value: &Expression) {
        self.const_defs
            .retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
        self.const_defs.push((name.to_string(), value.clone()));
    }

    fn emit_const_assignment(
        &mut self,
        indent: &str,
        name: &str,
        value: &Expression,
    ) -> QResult<()> {
        let value_code = self.generate_expression(value)?;
        let name_upper = name.to_uppercase();
        if name.ends_with('$') {
            let idx = self.get_str_var_idx(&name_upper);
            self.output
                .push_str(&format!("{}let _tmp_const = {};\n", indent, value_code));
            self.output.push_str(&format!(
                "{}set_str(&mut str_vars, {}, &_tmp_const);\n",
                indent, idx
            ));
        } else {
            let idx = self.get_num_var_idx(&name_upper);
            self.output.push_str(&format!(
                "{}set_var(&mut num_vars, {}, ({} as f64));\n",
                indent, idx, value_code
            ));
        }
        Ok(())
    }

    fn emit_restore_consts(&mut self, indent: &str) -> QResult<()> {
        let const_defs = self.const_defs.clone();
        for (name, value) in const_defs {
            self.emit_const_assignment(indent, &name, &value)?;
        }
        Ok(())
    }

    fn generate_store_from_text_value(
        &mut self,
        indent: &str,
        target: &Expression,
        value_code: &str,
        preserve_string_whitespace: bool,
    ) -> QResult<()> {
        match target {
            Expression::Variable(var) => {
                let name = &var.name;
                if self.variable_is_string(var) {
                    let idx = self.get_str_var_idx(name);
                    let width = self.fixed_string_width_for_name(name);
                    if preserve_string_whitespace {
                        if let Some(width) = width {
                            self.output.push_str(&format!(
                                "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}));\n",
                                indent, idx, width, value_code
                            ));
                        } else {
                            self.output.push_str(&format!(
                                "{}set_str(&mut str_vars, {}, &{});\n",
                                indent, idx, value_code
                            ));
                        }
                    } else if let Some(width) = width {
                        self.output.push_str(&format!(
                            "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}.trim()));\n",
                            indent, idx, width, value_code
                        ));
                    } else {
                        self.output.push_str(&format!(
                            "{}set_str(&mut str_vars, {}, &{}.trim());\n",
                            indent, idx, value_code
                        ));
                    }
                } else {
                    let idx = self.get_num_var_idx(name);
                    self.output.push_str(&format!(
                        "{}set_var(&mut num_vars, {}, {}.trim().parse::<f64>().unwrap_or(0.0));\n",
                        indent, idx, value_code
                    ));
                }
            }
            Expression::ArrayAccess { name, indices, .. } => {
                if !indices.is_empty() {
                    let idx_code = self.native_array_indices_expr(indices)?;
                    if self.array_is_string(name, None) {
                        let arr_idx = self.get_str_arr_var_idx(name);
                        let width = self.fixed_string_width_for_name(name);
                        if preserve_string_whitespace {
                            if let Some(width) = width {
                                self.output.push_str(&format!(
                                    "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &qb_fit_fixed_string({}, &{}));\n",
                                    indent, arr_idx, idx_code, width, value_code
                                ));
                            } else {
                                self.output.push_str(&format!(
                                    "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{});\n",
                                    indent, arr_idx, idx_code, value_code
                                ));
                            }
                        } else if let Some(width) = width {
                            self.output.push_str(&format!(
                                "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &qb_fit_fixed_string({}, &{}.trim()));\n",
                                indent, arr_idx, idx_code, width, value_code
                            ));
                        } else {
                            self.output.push_str(&format!(
                                "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{}.trim());\n",
                                indent, arr_idx, idx_code, value_code
                            ));
                        }
                    } else {
                        let arr_idx = self.get_arr_var_idx(name);
                        self.output.push_str(&format!(
                            "{}arr_set(&mut arr_vars, &mut arr_bounds, {}, {}, {}.trim().parse::<f64>().unwrap_or(0.0));\n",
                            indent, arr_idx, idx_code, value_code
                        ));
                    }
                }
            }
            Expression::FieldAccess { .. } => {
                if let Some(field) = self.resolve_field_access_layout(target) {
                    match field.field_type {
                        QType::String(_) => {
                            let width = field.fixed_length;
                            if let Some(indices) = field.array_indices.as_ref() {
                                let idx = self.get_str_arr_var_idx(&field.storage_name);
                                let index_code = self.native_array_indices_expr(indices)?;
                                if preserve_string_whitespace {
                                    if let Some(width) = width {
                                        self.output.push_str(&format!(
                                            "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &qb_fit_fixed_string({}, &{}));\n",
                                            indent, idx, index_code, width, value_code
                                        ));
                                    } else {
                                        self.output.push_str(&format!(
                                            "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{});\n",
                                            indent, idx, index_code, value_code
                                        ));
                                    }
                                } else if let Some(width) = width {
                                    self.output.push_str(&format!(
                                        "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &qb_fit_fixed_string({}, &{}.trim()));\n",
                                        indent, idx, index_code, width, value_code
                                    ));
                                } else {
                                    self.output.push_str(&format!(
                                        "{}str_arr_set(&mut str_arr_vars, &mut str_arr_bounds, {}, {}, &{}.trim());\n",
                                        indent, idx, index_code, value_code
                                    ));
                                }
                            } else {
                                let idx = self.get_str_var_idx(&field.storage_name);
                                if preserve_string_whitespace {
                                    if let Some(width) = width {
                                        self.output.push_str(&format!(
                                            "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}));\n",
                                            indent, idx, width, value_code
                                        ));
                                    } else {
                                        self.output.push_str(&format!(
                                            "{}set_str(&mut str_vars, {}, &{});\n",
                                            indent, idx, value_code
                                        ));
                                    }
                                } else if let Some(width) = width {
                                    self.output.push_str(&format!(
                                        "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}.trim()));\n",
                                        indent, idx, width, value_code
                                    ));
                                } else {
                                    self.output.push_str(&format!(
                                        "{}set_str(&mut str_vars, {}, &{}.trim());\n",
                                        indent, idx, value_code
                                    ));
                                }
                            }
                        }
                        QType::Integer(_)
                        | QType::Long(_)
                        | QType::Single(_)
                        | QType::Double(_) => {
                            if let Some(indices) = field.array_indices.as_ref() {
                                let idx = self.get_arr_var_idx(&field.storage_name);
                                let index_code = self.native_array_indices_expr(indices)?;
                                self.output.push_str(&format!(
                                    "{}arr_set(&mut arr_vars, &mut arr_bounds, {}, {}, {}.trim().parse::<f64>().unwrap_or(0.0));\n",
                                    indent, idx, index_code, value_code
                                ));
                            } else {
                                let idx = self.get_num_var_idx(&field.storage_name);
                                self.output.push_str(&format!(
                                    "{}set_var(&mut num_vars, {}, {}.trim().parse::<f64>().unwrap_or(0.0));\n",
                                    indent, idx, value_code
                                ));
                            }
                        }
                        _ => {}
                    }
                } else if let Some(name) = Self::qualified_field_name(target) {
                    if self.name_is_string(&name) {
                        let idx = self.get_str_var_idx(&name);
                        let width = self.fixed_string_width_for_name(&name);
                        if preserve_string_whitespace {
                            if let Some(width) = width {
                                self.output.push_str(&format!(
                                    "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}));\n",
                                    indent, idx, width, value_code
                                ));
                            } else {
                                self.output.push_str(&format!(
                                    "{}set_str(&mut str_vars, {}, &{});\n",
                                    indent, idx, value_code
                                ));
                            }
                        } else if let Some(width) = width {
                            self.output.push_str(&format!(
                                "{}set_str(&mut str_vars, {}, &qb_fit_fixed_string({}, &{}.trim()));\n",
                                indent, idx, width, value_code
                            ));
                        } else {
                            self.output.push_str(&format!(
                                "{}set_str(&mut str_vars, {}, &{}.trim());\n",
                                indent, idx, value_code
                            ));
                        }
                    } else {
                        let idx = self.get_num_var_idx(&name);
                        self.output.push_str(&format!(
                            "{}set_var(&mut num_vars, {}, {}.trim().parse::<f64>().unwrap_or(0.0));\n",
                            indent, idx, value_code
                        ));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
