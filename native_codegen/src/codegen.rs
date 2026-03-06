use core_types::{QResult, QType};
use std::collections::HashMap;
use syntax_tree::ast_nodes::*;

#[path = "codegen/graphics.rs"]
mod graphics;

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
    temp_var_counter: usize,
    pub use_graphics: bool,
    num_vars: HashMap<String, usize>,
    str_vars: HashMap<String, usize>,
    arr_vars: HashMap<String, usize>,
    params: HashMap<String, String>,
    is_in_sub: bool,
    current_function_name: Option<String>,
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

    fn get_num_var_idx(&mut self, name: &str) -> usize {
        let name_upper = name.to_uppercase();
        if let Some(&idx) = self.num_vars.get(&name_upper) {
            idx
        } else {
            let idx = self.num_vars.len();
            self.num_vars.insert(name_upper, idx);
            idx
        }
    }

    fn get_str_var_idx(&mut self, name: &str) -> usize {
        let name_upper = name.to_uppercase();
        if let Some(&idx) = self.str_vars.get(&name_upper) {
            idx
        } else {
            let idx = self.str_vars.len();
            self.str_vars.insert(name_upper, idx);
            idx
        }
    }

    fn get_arr_var_idx(&mut self, name: &str) -> usize {
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

    pub fn generate(&mut self, program: &Program) -> QResult<String> {
        self.output.clear();
        self.write_prelude();

        // Collect variables for main program
        self.collect_vars(&program.statements);

        // Generate DATA array if there are DATA statements
        // Scan program statements for DATA
        let mut data_values = Vec::new();
        for stmt in &program.statements {
            if let Statement::Data { values } = stmt {
                for val_str in values {
                    // Try to parse as number
                    if let Ok(i) = val_str.parse::<i64>() {
                        data_values.push(i as f64);
                    } else if let Ok(f) = val_str.parse::<f64>() {
                        data_values.push(f);
                    } else {
                        // String data not fully supported in this numeric array, use 0.0
                        data_values.push(0.0);
                    }
                }
            }
        }

        if !data_values.is_empty() {
            self.output.push_str("static DATA_VALUES: &[f64] = &[\n");
            for val in &data_values {
                self.output.push_str(&format!("    {},\n", val));
            }
            self.output.push_str("];\n\n");
        } else if !program.data_statements.is_empty() {
            // Fallback for pre-populated data_statements (if any)
            self.generate_data_array(&program.data_statements);
        }

        // Generate main function
        self.output.push_str("fn main() {\n");
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

        self.output.push_str("    let mut data_idx: usize = 0;\n\n");

        for stmt in &program.statements {
            self.generate_statement(stmt)?;
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

    fn collect_vars(&mut self, stmts: &[Statement]) {
        for stmt in stmts {
            match stmt {
                Statement::Assignment { target, .. } => self.collect_var_from_expr(target),
                Statement::Input { variables, .. } => {
                    for var in variables {
                        self.collect_var_from_expr(var);
                    }
                }
                Statement::Read { variables } => {
                    for var in variables {
                        // Read takes Variable struct, not Expression
                        let name = &var.name;
                        if var.type_suffix == Some('$') || name.ends_with('$') {
                            self.get_str_var_idx(name);
                        } else {
                            self.get_num_var_idx(name);
                        }
                    }
                }
                Statement::ForLoop { variable, body, .. } => {
                    self.get_num_var_idx(&variable.name);
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
                    for (var, _) in variables {
                        if var.indices.is_empty()
                            && var.type_suffix != Some('$')
                            && !var.name.ends_with('$')
                        {
                            // Scalar variable declared with DIM
                            self.get_num_var_idx(&var.name);
                        } else if var.indices.is_empty()
                            && (var.type_suffix == Some('$') || var.name.ends_with('$'))
                        {
                            // String variable
                            self.get_str_var_idx(&var.name);
                        } else {
                            // Array
                            self.get_arr_var_idx(&var.name);
                        }
                    }
                }
                Statement::Redim { variables, .. } => {
                    for (var, _) in variables {
                        self.get_arr_var_idx(&var.name);
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

    fn collect_var_from_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::Variable(var) => {
                if var.type_suffix == Some('$') || var.name.ends_with('$') {
                    self.get_str_var_idx(&var.name);
                } else {
                    self.get_num_var_idx(&var.name);
                }
            }
            Expression::ArrayAccess { name, .. } => {
                self.get_arr_var_idx(name);
            }
            _ => {}
        }
    }

    fn expr_to_array_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(var) => Some(var.name.clone()),
            Expression::ArrayAccess { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    fn generate_data_array(&mut self, data_statements: &[Vec<Expression>]) {
        let mut all_values: Vec<f64> = Vec::new();
        for stmt in data_statements {
            for expr in stmt {
                if let Expression::Literal(qtype) = expr {
                    match qtype {
                        QType::Integer(i) => all_values.push(*i as f64),
                        QType::Long(l) => all_values.push(*l as f64),
                        QType::Single(s) => all_values.push(*s as f64),
                        QType::Double(d) => all_values.push(*d),
                        _ => all_values.push(0.0),
                    }
                }
            }
        }

        if !all_values.is_empty() {
            self.output.push_str("static DATA_VALUES: &[f64] = &[\n");
            for val in &all_values {
                self.output.push_str(&format!("    {},\n", val));
            }
            self.output.push_str("];\n\n");
        }
    }

    fn write_prelude(&mut self) {
        self.output.push_str(
            r#"// Auto-generated by QBNex
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_parens)]
#![allow(static_mut_refs)]

use std::io::{self, Write};

"#,
        );
        if self.use_graphics {
            self.write_graphics_prelude();
        } else {
            self.output.push_str(
                r#"
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

fn arr_get(arrs: &[Vec<f64>], arr_idx: usize, idx: f64) -> f64 {
    if arr_idx < arrs.len() {
        let arr = &arrs[arr_idx];
        let i = (idx - 1.0) as usize;
        if i < arr.len() { arr[i] } else { 0.0 }
    } else {
        0.0
    }
}

fn arr_set(arrs: &mut [Vec<f64>], arr_idx: usize, idx: f64, val: f64) {
    if arr_idx < arrs.len() {
        let arr = &mut arrs[arr_idx];
        let i = (idx - 1.0) as usize;
        if i < arr.len() { arr[i] = val; }
    }
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
fn qb_rnd() -> f64 { 0.5 }

// String functions
fn qb_len(s: &str) -> f64 { s.len() as f64 }
fn qb_ltrim(s: &str) -> String { s.trim_start().to_string() }
fn qb_rtrim(s: &str) -> String { s.trim_end().to_string() }
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
fn qb_string(n: f64, c: &str) -> String { 
    let ch = c.chars().next().unwrap_or(' ');
    ch.to_string().repeat(n as usize) 
}
fn qb_space(n: f64) -> String { " ".repeat(n as usize) }
fn qb_asc(s: &str) -> f64 { s.chars().next().unwrap_or('\0') as u8 as f64 }
fn qb_chr(n: f64) -> String { ((n as u8) as char).to_string() }
fn qb_val(s: &str) -> f64 { s.parse().unwrap_or(0.0) }
fn qb_str(n: f64) -> String { format!("{}", n) }

// Array functions
fn qb_lbound(_arr: &[f64]) -> f64 { 1.0 }
fn qb_ubound(arr: &[f64]) -> f64 { arr.len() as f64 }

"#);
    }

    fn write_epilogue(&mut self) {
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
            self.output
                .push_str("    println!(\"\\nPress Enter to exit...\");\n");
            self.output.push_str("    qb_sleep(0.0);\n");
        }
        self.output.push_str("}\n");
    }

    fn generate_sub(&mut self, sub: &SubDef) -> QResult<()> {
        let old_num = self.num_vars.clone();
        let old_str = self.str_vars.clone();
        let old_arr = self.arr_vars.clone();
        let old_params = self.params.clone();
        let old_in_sub = self.is_in_sub;

        self.num_vars.clear();
        self.str_vars.clear();
        self.arr_vars.clear();
        self.params.clear();
        self.is_in_sub = true;

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

            if param.type_suffix == Some('$') || param_name.ends_with('$') {
                rust_params.push(format!("{}: &mut String", rust_name));
                let _ = self.get_str_var_idx(param_name);
            } else {
                rust_params.push(format!("{}: &mut f64", rust_name));
                let _ = self.get_num_var_idx(param_name);
            }
        }

        self.collect_vars(&sub.body);

        let fn_name = format!("sub_{}", sub.name.to_lowercase());
        self.output.push_str(&format!("\nfn {}(", fn_name));
        self.output.push_str("global_num_vars: &mut Vec<f64>, global_str_vars: &mut Vec<String>, global_arr_vars: &mut Vec<Vec<f64>>");
        if !rust_params.is_empty() {
            self.output.push_str(", ");
            self.output.push_str(&rust_params.join(", "));
        }
        self.output.push_str(") {\n");

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
            if param.type_suffix == Some('$') || param_name.ends_with('$') {
                let idx = self.get_str_var_idx(param_name);
                self.output.push_str(&format!(
                    "    set_str(&mut str_vars, {}, {});\n",
                    idx, rust_name
                ));
            } else {
                let idx = self.get_num_var_idx(param_name);
                self.output.push_str(&format!(
                    "    set_var(&mut num_vars, {}, *{});\n",
                    idx, rust_name
                ));
            }
        }

        for stmt in &sub.body {
            self.generate_statement(stmt)?;
        }

        // Copy back to args (pass by reference simulation)
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
            if param.type_suffix == Some('$') || param_name.ends_with('$') {
                let idx = self.get_str_var_idx(param_name);
                self.output.push_str(&format!(
                    "    *{} = get_str(&str_vars, {});\n",
                    rust_name, idx
                ));
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
        self.params = old_params;
        self.is_in_sub = old_in_sub;
        Ok(())
    }

    fn generate_function(&mut self, func: &FunctionDef) -> QResult<()> {
        let old_num = self.num_vars.clone();
        let old_str = self.str_vars.clone();
        let old_arr = self.arr_vars.clone();
        let old_params = self.params.clone();
        let old_in_sub = self.is_in_sub;
        let old_func_name = self.current_function_name.clone();

        self.num_vars.clear();
        self.str_vars.clear();
        self.arr_vars.clear();
        self.params.clear();
        self.is_in_sub = true;
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

            if param.type_suffix == Some('$') || param_name.ends_with('$') {
                rust_params.push(format!("{}: &mut String", rust_name));
                let _ = self.get_str_var_idx(param_name);
            } else {
                rust_params.push(format!("{}: &mut f64", rust_name));
                let _ = self.get_num_var_idx(param_name);
            }
        }

        // Reserve return value
        let ret_name = func.name.clone();
        if ret_name.ends_with('$') {
            self.get_str_var_idx(&ret_name);
        } else {
            self.get_num_var_idx(&ret_name);
        }

        self.collect_vars(&func.body);

        let fn_name = format!("func_{}", func.name.to_lowercase());
        let ret_type = if func.name.ends_with('$') {
            "String"
        } else {
            "f64"
        };

        self.output.push_str(&format!("\nfn {}(", fn_name));
        self.output.push_str("global_num_vars: &mut Vec<f64>, global_str_vars: &mut Vec<String>, global_arr_vars: &mut Vec<Vec<f64>>");
        if !rust_params.is_empty() {
            self.output.push_str(", ");
            self.output.push_str(&rust_params.join(", "));
        }
        self.output.push_str(&format!(") -> {} {{\n", ret_type));

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
            if param.type_suffix == Some('$') || param_name.ends_with('$') {
                let idx = self.get_str_var_idx(param_name);
                self.output.push_str(&format!(
                    "    set_str(&mut str_vars, {}, {});\n",
                    idx, rust_name
                ));
            } else {
                let idx = self.get_num_var_idx(param_name);
                self.output.push_str(&format!(
                    "    set_var(&mut num_vars, {}, *{});\n",
                    idx, rust_name
                ));
            }
        }

        for stmt in &func.body {
            self.generate_statement(stmt)?;
        }

        // Copy back to args
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
            if param.type_suffix == Some('$') || param_name.ends_with('$') {
                let idx = self.get_str_var_idx(param_name);
                self.output.push_str(&format!(
                    "    *{} = get_str(&str_vars, {});\n",
                    rust_name, idx
                ));
            } else {
                let idx = self.get_num_var_idx(param_name);
                self.output.push_str(&format!(
                    "    *{} = get_var(&num_vars, {});\n",
                    rust_name, idx
                ));
            }
        }

        // Return value
        if ret_name.ends_with('$') {
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
        self.params = old_params;
        self.is_in_sub = old_in_sub;
        self.current_function_name = old_func_name;
        Ok(())
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level + 1)
    }

    fn next_temp_var(&mut self) -> String {
        self.temp_var_counter += 1;
        format!("_tmp{}", self.temp_var_counter)
    }

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
                        if let Some(idx) = indices.first() {
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
                            .push_str(&format!("{}else if {} {{\n", indent, condition));
                    }
                    self.indent_level += 1;
                    for stmt in stmts {
                        self.generate_statement(stmt)?;
                    }
                    self.indent_level -= 1;
                }
                self.output.push_str(&format!("{}}}\n", indent));
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
                let sub_name = format!("sub_{}", name.to_lowercase());

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
                let param_list = params
                    .iter()
                    .map(|p| format!("{}: f64", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                let body_code = self.generate_expression(body)?;

                self.output.push_str(&format!(
                    "{}fn qb_{}(num_vars: &[f64], str_vars: &[String], {}) -> f64 {{\n",
                    indent, name, param_list
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
                    match var {
                        Expression::Variable(v) => {
                            let var_name = &v.name;
                            if v.type_suffix == Some('$') || var_name.ends_with('$') {
                                // String input
                                let idx = self.get_str_var_idx(var_name);
                                self.output.push_str(&format!(
                                    "{}io::stdout().flush().unwrap();\n",
                                    indent
                                ));
                                self.output.push_str(&format!(
                                    "{}let mut _input = String::new();\n",
                                    indent
                                ));
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
                                self.output.push_str(&format!(
                                    "{}io::stdout().flush().unwrap();\n",
                                    indent
                                ));
                                self.output.push_str(&format!(
                                    "{}let mut _input = String::new();\n",
                                    indent
                                ));
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
                        _ => {}
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

    fn generate_swap(&mut self, indent: &str, var1: &Expression, var2: &Expression) -> QResult<()> {
        let temp = self.next_temp_var();

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
                    "{}set_str(&mut str_vars, {}, &get_str(&str_vars, {}));\n",
                    indent, idx1, idx2
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
                    "{}set_var(&mut num_vars, {}, get_var(&num_vars, {}));\n",
                    indent, idx1, idx2
                ));
                self.output.push_str(&format!(
                    "{}set_var(&mut num_vars, {}, {});\n",
                    indent, idx2, temp
                ));
            }
        }
        Ok(())
    }

    fn generate_print(
        &mut self,
        indent: &str,
        expressions: &[Expression],
        newline: bool,
    ) -> QResult<()> {
        for expr in expressions {
            let code = self.generate_expression(expr)?;
            if code.starts_with('"') && code.ends_with('"') {
                let content = &code[1..code.len() - 1];
                self.output
                    .push_str(&format!("{}print!(\"{}\");\n", indent, content));
            } else {
                self.output
                    .push_str(&format!("{}print!(\"{{}}\", {});\n", indent, code));
            }
        }
        if newline {
            self.output.push_str(&format!("{}println!();\n", indent));
        }
        Ok(())
    }

    fn generate_condition(&mut self, expr: &Expression) -> String {
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

    fn op_to_str(&self, op: &BinaryOp) -> &'static str {
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

    fn evaluate_const_expr(&self, expr: &Expression) -> Option<f64> {
        match expr {
            Expression::Literal(QType::Integer(i)) => Some(*i as f64),
            Expression::Literal(QType::Long(l)) => Some(*l as f64),
            Expression::Literal(QType::Single(s)) => Some(*s as f64),
            Expression::Literal(QType::Double(d)) => Some(*d),
            _ => None,
        }
    }

    fn generate_expression(&mut self, expr: &Expression) -> QResult<String> {
        match expr {
            Expression::Literal(qtype) => Ok(match qtype {
                QType::Integer(i) => format!("{}.0", i),
                QType::Long(l) => format!("{}.0", l),
                QType::Single(s) => s.to_string(),
                QType::Double(d) => d.to_string(),
                QType::String(s) => format!("\"{}\".to_string()", s),
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
                if name_upper == "CHR$" && indices.len() == 1 {
                    let arg = self.generate_expression(&indices[0])?;
                    Ok(format!("qb_chr({})", arg))
                } else if name_upper == "STR$" && indices.len() == 1 {
                    let arg = self.generate_expression(&indices[0])?;
                    Ok(format!("qb_str({})", arg))
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
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Subtract => "-",
                    BinaryOp::Multiply => "*",
                    BinaryOp::Divide => "/",
                    BinaryOp::IntegerDivide => "/",
                    BinaryOp::Modulo => "%",
                    BinaryOp::Power => ".powf",
                    BinaryOp::Equal => "==",
                    BinaryOp::NotEqual => "!=",
                    BinaryOp::LessThan => "<",
                    BinaryOp::GreaterThan => ">",
                    BinaryOp::LessOrEqual => "<=",
                    BinaryOp::GreaterOrEqual => ">=",
                    BinaryOp::And => "&",
                    BinaryOp::Or => "|",
                    BinaryOp::Xor => "^",
                    _ => "+",
                };
                if *op == BinaryOp::Power {
                    Ok(format!("({}).powf({})", left_code, right_code))
                } else {
                    Ok(format!("({} {} {})", left_code, op_str, right_code))
                }
            }

            Expression::UnaryOp { op, operand } => {
                let operand_code = self.generate_expression(operand)?;
                match op {
                    UnaryOp::Negate => Ok(format!("(-{})", operand_code)),
                    UnaryOp::Not => Ok(format!("(!({} as i32) != 0)", operand_code)),
                }
            }

            Expression::CaseRange { .. } | Expression::CaseIs { .. } | Expression::CaseElse => Err(
                core_types::QError::Internal("Case expression used in value context".to_string()),
            ),

            Expression::FunctionCall(func) => self.generate_function_call(func),

            _ => Ok("0.0".to_string()),
        }
    }

    fn generate_function_call(&mut self, func: &FunctionCall) -> QResult<String> {
        let builtin = self.generate_builtin_call(&func.name, &func.args)?;
        if builtin != "0.0" {
            Ok(builtin)
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
                "func_{}({}, {})",
                func.name.to_lowercase(),
                global_args,
                args_str
            ))
        }
    }

    fn generate_builtin_call(&mut self, name: &str, args: &[Expression]) -> QResult<String> {
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

            "INKEY$" => Ok("inkey()".to_string()),
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
            "LBOUND" => Ok("1.0".to_string()),
            "UBOUND" => {
                if let Some(Expression::Variable(v)) = args.first() {
                    let idx = self.get_arr_var_idx(&v.name);
                    Ok(format!("qb_ubound(&arr_vars[{}])", idx))
                } else {
                    Ok("0.0".to_string())
                }
            }

            _ => Ok("0.0".to_string()),
        }
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}
