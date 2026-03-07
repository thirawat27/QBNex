use super::CodeGenerator;
use core_types::{QResult, QType};
use syntax_tree::ast_nodes::*;

impl CodeGenerator {
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
                self.output.push_str(&format!("    {:.1},\n", val));
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

        if self.has_data_ops(&program.statements) {
            self.output.push_str("    let mut data_idx: usize = 0;\n\n");
        } else {
            self.output.push('\n');
        }

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

    pub(super) fn collect_vars(&mut self, stmts: &[Statement]) {
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

    pub(super) fn collect_var_from_expr(&mut self, expr: &Expression) {
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

    pub(super) fn expr_to_array_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Variable(var) => Some(var.name.clone()),
            Expression::ArrayAccess { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    pub(super) fn generate_data_array(&mut self, data_statements: &[Vec<Expression>]) {
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
                self.output.push_str(&format!("    {:.1},\n", val));
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

use std::io::{self, Write};

"#,
        );
        if self.use_graphics {
            self.write_graphics_prelude();
        } else {
            self.output.push_str(
                r#"
fn cls() {
     print!("\x1B[2J\x1B[1;1H");
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
fn qb_cint(x: f64) -> f64 { x.round() }
fn qb_clng(x: f64) -> f64 { x.round() }
fn qb_csng(x: f64) -> f64 { x }
fn qb_cdbl(x: f64) -> f64 { x }
fn qb_mki(x: f64) -> String { format!("__BIN:I16:{:04X}", (x as i16) as u16) }
fn qb_mkl(x: f64) -> String { format!("__BIN:I32:{:08X}", (x as i32) as u32) }
fn qb_mks(x: f64) -> String { format!("__BIN:F32:{:08X}", (x as f32).to_bits()) }
fn qb_mkd(x: f64) -> String { format!("__BIN:F64:{:016X}", x.to_bits()) }
fn qb_cvi(s: &str) -> f64 {
    s.strip_prefix("__BIN:I16:")
        .and_then(|hex| u16::from_str_radix(hex, 16).ok())
        .map(|bits| (bits as i16) as f64)
        .unwrap_or_else(|| s.parse().unwrap_or(0.0))
}
fn qb_cvl(s: &str) -> f64 {
    s.strip_prefix("__BIN:I32:")
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .map(|bits| (bits as i32) as f64)
        .unwrap_or_else(|| s.parse().unwrap_or(0.0))
}
fn qb_cvs(s: &str) -> f64 {
    s.strip_prefix("__BIN:F32:")
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .map(|bits| f32::from_bits(bits) as f64)
        .unwrap_or_else(|| s.parse().unwrap_or(0.0))
}
fn qb_cvd(s: &str) -> f64 {
    s.strip_prefix("__BIN:F64:")
        .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        .map(f64::from_bits)
        .unwrap_or_else(|| s.parse().unwrap_or(0.0))
}
fn qb_fre(_arg: &str) -> f64 { 524288.0 }
fn qb_fre_num(_arg: f64) -> f64 { 1048576.0 }
fn qb_peek(_addr: f64) -> f64 { 0.0 }
fn qb_varptr(_s: &str) -> f64 { 0.0 }
fn qb_varseg(_s: &str) -> f64 { 0.0 }
fn qb_sadd(_s: &str) -> f64 { 0.0 }
fn qb_varptr_str(_s: &str) -> String { "\0\0\0\0".to_string() }
fn qb_timer() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() % 86_400) as f64 + d.subsec_nanos() as f64 / 1_000_000_000.0)
        .unwrap_or(0.0)
}
fn qb_date() -> String { "01-01-1980".to_string() }
fn qb_time() -> String { "00:00:00".to_string() }
fn qb_command() -> String { std::env::args().skip(1).collect::<Vec<_>>().join(" ") }
fn qb_environ(key: &str) -> String { std::env::var(key).unwrap_or_default() }
fn qb_csrlin() -> f64 { 1.0 }
fn qb_pos(_dummy: f64) -> f64 { 1.0 }
fn qb_err() -> f64 { 0.0 }
fn qb_erl() -> f64 { 0.0 }
fn qb_erdev() -> f64 { 0.0 }
fn qb_erdev_str() -> String { String::new() }
fn qb_input_str(count: f64) -> String { " ".repeat(count.max(0.0) as usize) }

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
fn qb_hex(n: f64) -> String { format!("{:X}", n as i64) }
fn qb_oct(n: f64) -> String { format!("{:o}", n as i64) }

// Array functions
fn qb_lbound(_arr: &[f64]) -> f64 { 1.0 }
fn qb_ubound(arr: &[f64]) -> f64 { arr.len() as f64 }

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
            self.output
                .push_str("    println!(\"\\nPress Enter to exit...\");\n");
            self.output.push_str("    qb_sleep(0.0);\n");
        }
        self.output.push_str("}\n");
    }

    pub(super) fn generate_sub(&mut self, sub: &SubDef) -> QResult<()> {
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

        let fn_name = self.rust_symbol("sub", &sub.name);
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

    pub(super) fn generate_function(&mut self, func: &FunctionDef) -> QResult<()> {
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

        let fn_name = self.rust_symbol("func", &func.name);
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
}
