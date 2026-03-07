use std::collections::HashMap;

pub struct CodeGenerator {
    pub(super) output: String,
    pub(super) indent_level: usize,
    pub(super) temp_var_counter: usize,
    pub use_graphics: bool,
    pub(super) num_vars: HashMap<String, usize>,
    pub(super) str_vars: HashMap<String, usize>,
    pub(super) arr_vars: HashMap<String, usize>,
    pub(super) params: HashMap<String, String>,
    pub(super) is_in_sub: bool,
    pub(super) current_function_name: Option<String>,
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
            params: HashMap::new(),
            is_in_sub: false,
            current_function_name: None,
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
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}
