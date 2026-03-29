use core_types::QType;
use std::collections::{HashMap, HashSet};
use syntax_tree::ast_nodes::{Expression, UserType, Variable};

pub struct CodeGenerator {
    pub(super) output: String,
    pub(super) indent_level: usize,
    pub(super) temp_var_counter: usize,
    pub use_graphics: bool,
    pub(super) num_vars: HashMap<String, usize>,
    pub(super) str_vars: HashMap<String, usize>,
    pub(super) arr_vars: HashMap<String, usize>,
    pub(super) str_arr_vars: HashMap<String, usize>,
    pub(super) shared_num_vars: HashMap<String, usize>,
    pub(super) shared_str_vars: HashMap<String, usize>,
    pub(super) shared_arr_vars: HashMap<String, usize>,
    pub(super) shared_str_arr_vars: HashMap<String, usize>,
    pub(super) field_widths: HashMap<String, usize>,
    pub(super) user_types: HashMap<String, UserType>,
    pub(super) udt_vars: HashMap<String, String>,
    pub(super) udt_array_vars: HashMap<String, String>,
    pub(super) shared_udt_vars: HashMap<String, String>,
    pub(super) shared_udt_array_vars: HashMap<String, String>,
    pub(super) functions: HashSet<String>,
    pub(super) function_return_types: HashMap<String, QType>,
    pub(super) sub_param_modes: HashMap<String, Vec<bool>>,
    pub(super) function_param_modes: HashMap<String, Vec<bool>>,
    pub(super) params: HashMap<String, String>,
    pub(super) const_defs: Vec<(String, Expression)>,
    pub(super) restore_targets: HashMap<String, usize>,
    pub(super) is_in_sub: bool,
    pub(super) current_proc_is_static: bool,
    pub(super) current_function_name: Option<String>,
    pub(super) loop_state_counter: usize,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            output: String::with_capacity(1024 * 1024),
            indent_level: 0,
            temp_var_counter: 0,
            use_graphics: false,
            num_vars: HashMap::new(),
            str_vars: HashMap::new(),
            arr_vars: HashMap::new(),
            str_arr_vars: HashMap::new(),
            shared_num_vars: HashMap::new(),
            shared_str_vars: HashMap::new(),
            shared_arr_vars: HashMap::new(),
            shared_str_arr_vars: HashMap::new(),
            field_widths: HashMap::new(),
            user_types: HashMap::new(),
            udt_vars: HashMap::new(),
            udt_array_vars: HashMap::new(),
            shared_udt_vars: HashMap::new(),
            shared_udt_array_vars: HashMap::new(),
            functions: HashSet::new(),
            function_return_types: HashMap::new(),
            sub_param_modes: HashMap::new(),
            function_param_modes: HashMap::new(),
            params: HashMap::new(),
            const_defs: Vec::new(),
            restore_targets: HashMap::new(),
            is_in_sub: false,
            current_proc_is_static: false,
            current_function_name: None,
            loop_state_counter: 0,
        }
    }

    pub(super) fn get_num_var_idx(&mut self, name: &str) -> usize {
        let name_upper = name.to_uppercase();
        if let Some(&idx) = self.num_vars.get(&name_upper) {
            idx
        } else {
            let idx = self.num_vars.len();
            self.num_vars.insert(name_upper, idx);
            idx
        }
    }

    pub(super) fn get_str_var_idx(&mut self, name: &str) -> usize {
        let name_upper = name.to_uppercase();
        if let Some(&idx) = self.str_vars.get(&name_upper) {
            idx
        } else {
            let idx = self.str_vars.len();
            self.str_vars.insert(name_upper, idx);
            idx
        }
    }

    pub(super) fn get_arr_var_idx(&mut self, name: &str) -> usize {
        let name_upper = name.to_uppercase();
        if let Some(&idx) = self.arr_vars.get(&name_upper) {
            idx
        } else {
            let idx = self.arr_vars.len();
            self.arr_vars.insert(name_upper, idx);
            idx
        }
    }

    pub(super) fn get_str_arr_var_idx(&mut self, name: &str) -> usize {
        let name_upper = name.to_uppercase();
        if let Some(&idx) = self.str_arr_vars.get(&name_upper) {
            idx
        } else {
            let idx = self.str_arr_vars.len();
            self.str_arr_vars.insert(name_upper, idx);
            idx
        }
    }

    pub fn enable_graphics(&mut self) {
        self.use_graphics = true;
    }

    pub(super) fn indent(&self) -> String {
        "    ".repeat(self.indent_level + 1)
    }

    pub(super) fn next_temp_var(&mut self) -> String {
        self.temp_var_counter += 1;
        format!("_tmp{}", self.temp_var_counter)
    }

    pub(super) fn next_loop_state_id(&mut self) -> usize {
        let id = self.loop_state_counter;
        self.loop_state_counter += 1;
        id
    }

    pub(super) fn rust_symbol(&self, prefix: &str, name: &str) -> String {
        let mut symbol = format!("{}_", prefix);
        for ch in name.chars() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' => symbol.push(ch.to_ascii_lowercase()),
                '$' => symbol.push_str("_str"),
                '%' => symbol.push_str("_int"),
                '&' => symbol.push_str("_lng"),
                '!' => symbol.push_str("_sng"),
                '#' => symbol.push_str("_dbl"),
                _ => symbol.push('_'),
            }
        }
        symbol
    }

    pub(super) fn register_sub_param_modes(&mut self, name: &str, params: &[Variable]) {
        self.sub_param_modes.insert(
            name.to_uppercase(),
            params.iter().map(|param| param.by_val).collect(),
        );
    }

    pub(super) fn register_function_param_modes(&mut self, name: &str, params: &[Variable]) {
        self.function_param_modes.insert(
            name.to_uppercase(),
            params.iter().map(|param| param.by_val).collect(),
        );
    }

    pub(super) fn param_modes_for_call(
        &self,
        name: &str,
        is_function: bool,
        arg_count: usize,
    ) -> Vec<bool> {
        let modes = if is_function {
            self.function_param_modes.get(&name.to_uppercase())
        } else {
            self.sub_param_modes.get(&name.to_uppercase())
        };

        let mut resolved = vec![false; arg_count];
        if let Some(modes) = modes {
            for (index, by_val) in modes.iter().take(arg_count).enumerate() {
                resolved[index] = *by_val;
            }
        }
        resolved
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}
