use crate::opcodes::{ByRefTarget, OpCode};
use core_types::{DosMemory, QError, QResult, QType};
use std::collections::HashMap;
use std::io::IsTerminal;
use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ForLoopContext {
    pub var_name: String,
    pub var_index: Option<usize>,
    pub end_value: f64,
    pub step: f64,
    pub loop_start: usize,
}

/// Data pointer for READ/RESTORE operations
#[derive(Debug, Clone)]
pub struct DataPointer {
    pub section_index: usize, // Which DATA statement (0 = first, 1 = second, etc.)
    pub value_index: usize,   // Which value within that DATA statement
}

impl DataPointer {
    pub fn new() -> Self {
        Self {
            section_index: 0,
            value_index: 0,
        }
    }

    pub fn reset(&mut self) {
        self.section_index = 0;
        self.value_index = 0;
    }

    pub fn reset_to_label(&mut self, _label: &str) {
        // For labeled RESTORE, we'd need to map labels to section indices
        // For now, just reset to beginning
        self.reset();
    }
}

impl Default for DataPointer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RuntimeState {
    pub instruction_pointer: usize,
    pub value_stack: Vec<QType>,
    pub call_stack: Vec<usize>,
    pub variables: HashMap<String, QType>,
    pub arrays: HashMap<String, Vec<QType>>,
    pub array_dimensions: HashMap<String, Vec<(i32, i32)>>,
    pub random_fields: HashMap<i32, Vec<(usize, usize)>>,
    pub functions: HashMap<String, (Vec<usize>, usize, usize, usize)>,
    pub subs: HashMap<String, (Vec<usize>, usize, usize)>,
    pub def_fns: HashMap<String, (Vec<String>, Vec<OpCode>)>,
    pub error_handler_address: Option<usize>,
    pub error_resume_next: bool,
    pub last_error: Option<QError>,
    pub last_error_code: i16, // Store the actual error code for ERR function
    pub for_loop_stack: Vec<ForLoopContext>,
    pub data_pointer: DataPointer,
    pub data_section: Vec<Vec<QType>>, // All DATA values organized by statement
    pub globals: Vec<QType>,
    pub timer_handler_address: Option<usize>,
    pub timer_interval_secs: f64,
    pub timer_enabled: bool,
    pub next_timer_tick: Option<Instant>,
    pub timer_handler_depth: Option<usize>,
    pub procedure_frames: Vec<ProcedureFrame>,
    pub cursor_row: i16,
    pub cursor_col: i16,
    pub current_segment: u16,
    pub pseudo_var_offsets: HashMap<String, u16>,
    pub pseudo_var_sizes: HashMap<String, usize>,
    pub next_pseudo_offset: u16,
    pub view_print_top: Option<i16>,
    pub view_print_bottom: Option<i16>,
    pub pseudo_ports: HashMap<u16, u8>,
    pub string_widths: HashMap<usize, usize>,
    pub string_array_widths: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct ProcedureFrame {
    pub saved_slots: Vec<(usize, QType)>,
    pub copy_backs: Vec<(ByRefTarget, usize)>,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            instruction_pointer: 0,
            value_stack: Vec::with_capacity(256),
            call_stack: Vec::with_capacity(64),
            variables: HashMap::with_capacity(128),
            arrays: HashMap::with_capacity(32),
            array_dimensions: HashMap::with_capacity(32),
            random_fields: HashMap::with_capacity(8),
            functions: HashMap::new(),
            subs: HashMap::new(),
            def_fns: HashMap::new(),
            error_handler_address: None,
            error_resume_next: false,
            last_error: None,
            last_error_code: 0,
            for_loop_stack: Vec::with_capacity(16),
            data_pointer: DataPointer::new(),
            data_section: Vec::new(),
            globals: Vec::new(),
            timer_handler_address: None,
            timer_interval_secs: 0.0,
            timer_enabled: false,
            next_timer_tick: None,
            timer_handler_depth: None,
            procedure_frames: Vec::with_capacity(32),
            cursor_row: 1,
            cursor_col: 1,
            current_segment: 0,
            pseudo_var_offsets: HashMap::with_capacity(64),
            pseudo_var_sizes: HashMap::with_capacity(64),
            next_pseudo_offset: 1,
            view_print_top: None,
            view_print_bottom: None,
            pseudo_ports: HashMap::with_capacity(32),
            string_widths: HashMap::with_capacity(32),
            string_array_widths: HashMap::with_capacity(16),
        }
    }

    /// Read the next value from the DATA section
    pub fn read_data(&mut self) -> Option<QType> {
        // Find the next available value
        while self.data_pointer.section_index < self.data_section.len() {
            let section = &self.data_section[self.data_pointer.section_index];
            if self.data_pointer.value_index < section.len() {
                let value = section[self.data_pointer.value_index].clone();
                self.data_pointer.value_index += 1;
                return Some(value);
            }
            // Move to next section
            self.data_pointer.section_index += 1;
            self.data_pointer.value_index = 0;
        }
        // Out of DATA
        None
    }

    pub fn push(&mut self, value: QType) {
        self.value_stack.push(value);
    }

    pub fn pop(&mut self) -> QResult<QType> {
        self.value_stack
            .pop()
            .ok_or(QError::Internal("Stack underflow".to_string()))
    }

    pub fn peek(&self) -> QResult<QType> {
        self.value_stack
            .last()
            .cloned()
            .ok_or(QError::Internal("Stack underflow".to_string()))
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

fn split_input_fields(line: &str) -> Vec<String> {
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

#[cfg(windows)]
fn qb_inkey_nonblocking() -> String {
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
fn qb_inkey_nonblocking() -> String {
    String::new()
}

fn qb_wait_for_keypress() {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return;
    }

    #[cfg(windows)]
    loop {
        if !qb_inkey_nonblocking().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    #[cfg(not(windows))]
    {
        // Non-Windows builds currently keep SLEEP's key-wait path non-blocking to
        // avoid hanging automation until a proper console key polling layer exists.
    }
}

fn qb_fit_fixed_string(width: usize, text: &str) -> String {
    let mut text = text.to_string();
    if text.len() > width {
        text.truncate(width);
    } else if text.len() < width {
        text.push_str(&" ".repeat(width - text.len()));
    }
    text
}

fn qb_normalize_fixed_string_value(width: usize, value: QType) -> QType {
    match value {
        QType::String(s) => QType::String(qb_fit_fixed_string(width, &s)),
        other => QType::String(qb_fit_fixed_string(width, &other.to_string())),
    }
}

pub struct VM {
    pub runtime: RuntimeState,
    pub bytecode: Vec<OpCode>,
    pub memory: DosMemory,
    pub screen_mode: i32,
    pub running: bool,
    pub graphics: Option<hal_layer::VGAGraphics>,
    pub sound: hal_layer::SoundSynth,
    pub file_io: hal_layer::FileIO,
}

impl VM {
    const PSEUDO_VAR_SEGMENT: u16 = 0x6000;

    fn normalize_global_value_for_slot(&self, slot: usize, value: QType) -> QType {
        if let Some(width) = self.runtime.string_widths.get(&slot).copied() {
            qb_normalize_fixed_string_value(width, value)
        } else {
            value
        }
    }

    fn normalize_array_value_for_name(&self, name: &str, value: QType) -> QType {
        if let Some(width) = self.runtime.string_array_widths.get(name).copied() {
            qb_normalize_fixed_string_value(width, value)
        } else {
            value
        }
    }

    pub fn new(bytecode: Vec<OpCode>) -> Self {
        let memory = DosMemory::new();
        Self {
            runtime: RuntimeState::new(),
            bytecode,
            screen_mode: 0,
            running: true,
            graphics: Some(hal_layer::VGAGraphics::new(memory.clone())),
            sound: hal_layer::SoundSynth::new(),
            file_io: hal_layer::FileIO::new(),
            memory,
        }
    }

    pub fn run(&mut self) -> QResult<()> {
        while self.running && self.runtime.instruction_pointer < self.bytecode.len() {
            self.maybe_fire_timer();
            let prev_ip = self.runtime.instruction_pointer;
            if let Err(e) = self.execute_next() {
                self.runtime.last_error = Some(e.clone());
                // Only update last_error_code if it's not already set (from ERROR statement)
                // or if this is not a Runtime error (which preserves the user-specified code)
                if !matches!(e, QError::Runtime(_)) || self.runtime.last_error_code == 0 {
                    self.runtime.last_error_code = e.code();
                }
                if self.runtime.error_resume_next {
                    // IP is already advanced by execute_next
                    continue;
                } else if let Some(handler) = self.runtime.error_handler_address {
                    self.runtime.call_stack.push(prev_ip);
                    self.runtime.instruction_pointer = handler;
                } else {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn maybe_fire_timer(&mut self) {
        if !self.runtime.timer_enabled || self.runtime.timer_handler_depth.is_some() {
            return;
        }

        let Some(handler) = self.runtime.timer_handler_address else {
            return;
        };

        let interval = self.runtime.timer_interval_secs.max(0.001);
        let now = Instant::now();

        match self.runtime.next_timer_tick {
            Some(next_tick) if now >= next_tick => {
                let return_depth = self.runtime.call_stack.len();
                self.runtime
                    .call_stack
                    .push(self.runtime.instruction_pointer);
                self.runtime.timer_handler_depth = Some(return_depth);
                self.runtime.instruction_pointer = handler;
                self.runtime.next_timer_tick = Some(now + Duration::from_secs_f64(interval));
            }
            Some(_) => {}
            None => {
                self.runtime.next_timer_tick = Some(now + Duration::from_secs_f64(interval));
            }
        }
    }

    fn resolve_array_linear_index(&self, name: &str, index_slots: &[usize]) -> Option<usize> {
        let dims = self.runtime.array_dimensions.get(name)?;
        let indices = index_slots
            .iter()
            .map(|slot| {
                self.runtime
                    .globals
                    .get(*slot)
                    .cloned()
                    .unwrap_or(QType::Integer(0))
                    .to_f64() as i32
            })
            .collect::<Vec<_>>();

        let mut linear_index = 0usize;
        let mut multiplier = 1usize;
        for (i, (lower, upper)) in dims.iter().enumerate().rev() {
            let idx = indices.get(i).copied().unwrap_or(*lower);
            if idx < *lower || idx > *upper {
                return None;
            }
            linear_index += (idx - lower) as usize * multiplier;
            multiplier *= (upper - lower + 1) as usize;
        }
        Some(linear_index)
    }

    fn apply_by_ref_copy_back(&mut self, target: ByRefTarget, param_idx: usize) {
        while self.runtime.globals.len() <= param_idx {
            self.runtime.globals.push(QType::Empty);
        }
        let value = self.runtime.globals[param_idx].clone();

        match target {
            ByRefTarget::None => {}
            ByRefTarget::Global(caller_idx) => {
                while self.runtime.globals.len() <= caller_idx {
                    self.runtime.globals.push(QType::Empty);
                }
                self.runtime.globals[caller_idx] =
                    self.normalize_global_value_for_slot(caller_idx, value);
            }
            ByRefTarget::ArrayElement { name, index_slots } => {
                if let Some(linear_index) = self.resolve_array_linear_index(&name, &index_slots) {
                    let width = self.runtime.string_array_widths.get(&name).copied();
                    if let Some(array) = self.runtime.arrays.get_mut(&name) {
                        if linear_index < array.len() {
                            array[linear_index] = if let Some(width) = width {
                                qb_normalize_fixed_string_value(width, value)
                            } else {
                                value
                            };
                        }
                    }
                }
            }
        }
    }

    fn advance_cursor_text(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\n' => {
                    self.runtime.cursor_row = self.runtime.cursor_row.saturating_add(1);
                    self.runtime.cursor_col = 1;
                }
                '\r' => {
                    self.runtime.cursor_col = 1;
                }
                _ => {
                    self.runtime.cursor_col = self.runtime.cursor_col.saturating_add(1);
                }
            }
        }
    }

    fn resolve_var_ref<'a>(&'a self, var_ref: &'a str) -> (Option<&'a str>, Option<usize>) {
        if let Some(rest) = var_ref.strip_prefix('#') {
            if let Some((slot_text, name)) = rest.split_once(':') {
                if let Ok(slot) = slot_text.parse::<usize>() {
                    return (Some(name), Some(slot));
                }
            }
        }
        (Some(var_ref), None)
    }

    fn resolve_var_ref_value(&self, var_ref: &str) -> Option<QType> {
        let (name, slot) = self.resolve_var_ref(var_ref);
        if let Some(slot) = slot {
            if let Some(value) = self.runtime.globals.get(slot) {
                return Some(value.clone());
            }
        }
        name.and_then(|name| self.runtime.variables.get(name).cloned())
    }

    fn value_to_memory_bytes(value: &QType) -> Vec<u8> {
        match value {
            QType::Integer(v) => v.to_le_bytes().to_vec(),
            QType::Long(v) => v.to_le_bytes().to_vec(),
            QType::Single(v) => v.to_le_bytes().to_vec(),
            QType::Double(v) => v.to_le_bytes().to_vec(),
            QType::String(s) => {
                let mut bytes = s.as_bytes().to_vec();
                bytes.push(0);
                bytes
            }
            QType::UserDefined(bytes) => bytes.clone(),
            QType::Empty => vec![0],
        }
    }

    fn ensure_pseudo_var_storage(&mut self, var_ref: &str) -> (u16, u16, Vec<u8>) {
        let value = self
            .resolve_var_ref_value(var_ref)
            .unwrap_or(QType::Integer(0));
        let bytes = Self::value_to_memory_bytes(&value);
        let needed = bytes.len().max(1);
        let needs_realloc = self
            .runtime
            .pseudo_var_sizes
            .get(var_ref)
            .is_none_or(|size| *size < needed);

        if needs_realloc {
            let offset = self.runtime.next_pseudo_offset;
            let next = offset.saturating_add((needed + 1).min(u16::MAX as usize) as u16);
            self.runtime
                .pseudo_var_offsets
                .insert(var_ref.to_string(), offset);
            self.runtime
                .pseudo_var_sizes
                .insert(var_ref.to_string(), needed);
            self.runtime.next_pseudo_offset = next;
        }

        let offset = *self.runtime.pseudo_var_offsets.get(var_ref).unwrap_or(&0);
        let capacity = *self
            .runtime
            .pseudo_var_sizes
            .get(var_ref)
            .unwrap_or(&needed);
        for i in 0..capacity {
            let byte = bytes.get(i).copied().unwrap_or(0);
            self.memory.poke(
                Self::PSEUDO_VAR_SEGMENT,
                offset.saturating_add(i as u16),
                byte,
            );
        }
        (Self::PSEUDO_VAR_SEGMENT, offset, bytes)
    }

    fn execute_next(&mut self) -> QResult<()> {
        let ip = self.runtime.instruction_pointer;
        let opcode = self
            .bytecode
            .get(ip)
            .ok_or(QError::Internal("Invalid instruction pointer".to_string()))?
            .clone();

        self.runtime.instruction_pointer += 1;

        self.execute_opcode(opcode)
    }

    fn execute_opcode(&mut self, opcode: OpCode) -> QResult<()> {
        match opcode {
            OpCode::NoOp => {}

            OpCode::InitGlobals(count) => {
                // Resize globals vector and fill with default values
                self.runtime.globals.resize(count, QType::Integer(0));
            }

            OpCode::SetStringWidth { slot, width } => {
                self.runtime.string_widths.insert(slot, width);
                while self.runtime.globals.len() <= slot {
                    self.runtime.globals.push(QType::Empty);
                }
                let value = self.runtime.globals[slot].clone();
                self.runtime.globals[slot] = self.normalize_global_value_for_slot(slot, value);
            }

            OpCode::SetStringArrayWidth { name, width } => {
                self.runtime.string_array_widths.insert(name.clone(), width);
                if let Some(array) = self.runtime.arrays.get_mut(&name) {
                    for value in array.iter_mut() {
                        let current = std::mem::replace(value, QType::Empty);
                        *value = qb_normalize_fixed_string_value(width, current);
                    }
                }
            }

            OpCode::LoadFast(idx) => {
                if idx < self.runtime.globals.len() {
                    self.runtime.push(self.runtime.globals[idx].clone());
                } else {
                    return Err(QError::Internal(
                        "Global variable index out of bounds".to_string(),
                    ));
                }
            }

            OpCode::StoreFast(idx) => {
                let value = self.runtime.pop()?;
                let value = self.normalize_global_value_for_slot(idx, value);
                if idx < self.runtime.globals.len() {
                    self.runtime.globals[idx] = value;
                } else {
                    return Err(QError::Internal(
                        "Global variable index out of bounds".to_string(),
                    ));
                }
            }

            OpCode::LoadConstant(qtype) => {
                self.runtime.push(qtype);
            }

            OpCode::LoadVariable(name) => {
                let value = self
                    .runtime
                    .variables
                    .get(&name)
                    .cloned()
                    .unwrap_or(QType::Empty);
                self.runtime.push(value);
            }

            OpCode::StoreVariable(name) => {
                let value = self.runtime.pop()?;
                self.runtime.variables.insert(name, value);
            }

            OpCode::Add => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;

                // Check if either operand is a string
                match (&left, &right) {
                    (QType::String(s1), QType::String(s2)) => {
                        self.runtime.push(QType::String(format!("{}{}", s1, s2)));
                    }
                    (QType::String(s), _) => {
                        self.runtime.push(QType::String(format!("{}{}", s, right)));
                    }
                    (_, QType::String(s)) => {
                        self.runtime.push(QType::String(format!("{}{}", left, s)));
                    }
                    _ => {
                        let result = self.binary_op(&left, &right, |a, b| a + b)?;
                        self.runtime.push(result);
                    }
                }
            }

            OpCode::Subtract => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = self.binary_op(&left, &right, |a, b| a - b)?;
                self.runtime.push(result);
            }

            OpCode::Multiply => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = self.binary_op(&left, &right, |a, b| a * b)?;
                self.runtime.push(result);
            }

            OpCode::Divide => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                if right.to_f64() == 0.0 {
                    return Err(QError::DivisionByZero);
                }
                let result = self.binary_op(&left, &right, |a, b| a / b)?;
                self.runtime.push(result);
            }

            OpCode::IntegerDivide => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                if right.to_f64() == 0.0 {
                    return Err(QError::DivisionByZero);
                }
                let left_val = left.to_f64();
                let right_val = right.to_f64();
                let result_val = (left_val / right_val).floor();
                // Clamp to i16 range to prevent overflow
                let clamped = result_val.max(i16::MIN as f64).min(i16::MAX as f64);
                let result = QType::Integer(clamped as i16);
                self.runtime.push(result);
            }

            OpCode::Modulo => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                if right.to_f64() == 0.0 {
                    return Err(QError::DivisionByZero);
                }
                let left_val = left.to_f64();
                let right_val = right.to_f64();
                let result_val = left_val % right_val;
                // Clamp to i16 range to prevent overflow
                let clamped = result_val.max(i16::MIN as f64).min(i16::MAX as f64);
                let result = QType::Integer(clamped as i16);
                self.runtime.push(result);
            }

            OpCode::Power => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = QType::Double(left.to_f64().powf(right.to_f64()));
                self.runtime.push(result);
            }

            OpCode::Negate => {
                let value = self.runtime.pop()?;
                let result = match value {
                    QType::Integer(i) => QType::Integer(-i),
                    QType::Long(l) => QType::Long(-l),
                    QType::Single(s) => QType::Single(-s),
                    QType::Double(d) => QType::Double(-d),
                    _ => QType::Integer(0),
                };
                self.runtime.push(result);
            }

            OpCode::Not => {
                let value = self.runtime.pop()?;
                let result = QType::Integer(if value.to_f64() == 0.0 { -1 } else { 0 });
                self.runtime.push(result);
            }

            OpCode::Equal => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = QType::Integer(if left == right { -1 } else { 0 });
                self.runtime.push(result);
            }

            OpCode::NotEqual => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = QType::Integer(if left != right { -1 } else { 0 });
                self.runtime.push(result);
            }

            OpCode::LessThan => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = QType::Integer(if left.to_f64() < right.to_f64() {
                    -1
                } else {
                    0
                });
                self.runtime.push(result);
            }

            OpCode::GreaterThan => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = QType::Integer(if left.to_f64() > right.to_f64() {
                    -1
                } else {
                    0
                });
                self.runtime.push(result);
            }

            OpCode::LessOrEqual => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = QType::Integer(if left.to_f64() <= right.to_f64() {
                    -1
                } else {
                    0
                });
                self.runtime.push(result);
            }

            OpCode::GreaterOrEqual => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let result = QType::Integer(if left.to_f64() >= right.to_f64() {
                    -1
                } else {
                    0
                });
                self.runtime.push(result);
            }

            OpCode::And => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let left_val = left.to_f64() as i32;
                let right_val = right.to_f64() as i32;
                let result_val = left_val & right_val;
                // Clamp to i16 range
                let clamped = result_val.max(i16::MIN as i32).min(i16::MAX as i32);
                let result = QType::Integer(clamped as i16);
                self.runtime.push(result);
            }

            OpCode::Or => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let left_val = left.to_f64() as i32;
                let right_val = right.to_f64() as i32;
                let result_val = left_val | right_val;
                // Clamp to i16 range
                let clamped = result_val.max(i16::MIN as i32).min(i16::MAX as i32);
                let result = QType::Integer(clamped as i16);
                self.runtime.push(result);
            }

            OpCode::Xor => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let left_val = left.to_f64() as i32;
                let right_val = right.to_f64() as i32;
                let result_val = left_val ^ right_val;
                // Clamp to i16 range
                let clamped = result_val.max(i16::MIN as i32).min(i16::MAX as i32);
                let result = QType::Integer(clamped as i16);
                self.runtime.push(result);
            }

            OpCode::Eqv => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let left_val = left.to_f64() as i32;
                let right_val = right.to_f64() as i32;
                // Eqv is equivalence: NOT (a XOR b)
                let result_val = !(left_val ^ right_val);
                let clamped = result_val.max(i16::MIN as i32).min(i16::MAX as i32);
                let result = QType::Integer(clamped as i16);
                self.runtime.push(result);
            }

            OpCode::Imp => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let left_val = left.to_f64() as i32;
                let right_val = right.to_f64() as i32;
                // IMP is implication: (NOT a) OR b
                let result_val = (!left_val) | right_val;
                let clamped = result_val.max(i16::MIN as i32).min(i16::MAX as i32);
                let result = QType::Integer(clamped as i16);
                self.runtime.push(result);
            }

            OpCode::PrintTab => {
                print!("\t");
                io::stdout().flush().ok();
                let next_tab = (((self.runtime.cursor_col - 1) / 8) + 1) * 8 + 1;
                self.runtime.cursor_col = next_tab;
            }

            OpCode::PrintSpace => {
                print!(" ");
                io::stdout().flush().ok();
                self.runtime.cursor_col = self.runtime.cursor_col.saturating_add(1);
            }

            OpCode::Input => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| QError::Internal(e.to_string()))?;
                let input = input.trim_end_matches('\n').trim_end_matches('\r');
                self.runtime.push(QType::String(input.to_string()));
            }

            OpCode::LineInput(var_name) => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| QError::Internal(e.to_string()))?;
                let input = input.trim_end_matches('\n').trim_end_matches('\r');
                self.runtime
                    .variables
                    .insert(var_name.clone(), QType::String(input.to_string()));
            }

            OpCode::Jump(addr) => {
                self.runtime.instruction_pointer = addr;
            }

            OpCode::JumpIfFalse(addr) => {
                let value = self.runtime.pop()?;
                if value.to_f64() == 0.0 {
                    self.runtime.instruction_pointer = addr;
                }
            }

            OpCode::JumpIfTrue(addr) => {
                let value = self.runtime.pop()?;
                if value.to_f64() != 0.0 {
                    self.runtime.instruction_pointer = addr;
                }
            }

            OpCode::Gosub(addr) => {
                self.runtime
                    .call_stack
                    .push(self.runtime.instruction_pointer);
                self.runtime.instruction_pointer = addr;
            }

            OpCode::Return => {
                if let Some(addr) = self.runtime.call_stack.pop() {
                    self.runtime.instruction_pointer = addr;
                    if self.runtime.timer_handler_depth == Some(self.runtime.call_stack.len()) {
                        self.runtime.timer_handler_depth = None;
                    }
                } else {
                    return Err(QError::ReturnWithoutGosub);
                }
            }

            OpCode::ReadFast(idx) => {
                if let Some(val) = self.runtime.read_data() {
                    if idx < self.runtime.globals.len() {
                        self.runtime.globals[idx] = val;
                    } else {
                        return Err(QError::Internal(
                            "Global variable index out of bounds".to_string(),
                        ));
                    }
                } else {
                    return Err(QError::OutOfData);
                }
            }

            OpCode::SwapFast(idx1, idx2) => {
                if idx1 < self.runtime.globals.len() && idx2 < self.runtime.globals.len() {
                    self.runtime.globals.swap(idx1, idx2);
                } else {
                    return Err(QError::Internal(
                        "Global variable index out of bounds".to_string(),
                    ));
                }
            }

            OpCode::ForInit {
                var_name,
                end_label,
                step,
            } => {
                let end_val = self.runtime.pop()?.to_f64();
                let start_val = self
                    .runtime
                    .variables
                    .get(&var_name)
                    .cloned()
                    .unwrap_or(QType::Double(0.0))
                    .to_f64();

                let step_val = step;

                let ctx = ForLoopContext {
                    var_name: var_name.clone(),
                    var_index: None,
                    end_value: end_val,
                    step: step_val,
                    loop_start: self.runtime.instruction_pointer,
                };
                self.runtime.for_loop_stack.push(ctx);

                // Check if we should skip the loop entirely
                let should_skip = if step_val >= 0.0 {
                    start_val > end_val
                } else {
                    start_val < end_val
                };

                if should_skip {
                    self.runtime.instruction_pointer = end_label;
                    self.runtime.for_loop_stack.pop();
                }
            }

            OpCode::ForInitFast {
                var_index,
                end_label,
                step,
            } => {
                let end_val = self.runtime.pop()?.to_f64();
                let start_val = self
                    .runtime
                    .globals
                    .get(var_index)
                    .map(|v| v.to_f64())
                    .unwrap_or(0.0);

                let ctx = ForLoopContext {
                    var_name: String::new(),
                    var_index: Some(var_index),
                    end_value: end_val,
                    step,
                    loop_start: self.runtime.instruction_pointer,
                };
                self.runtime.for_loop_stack.push(ctx);

                let should_skip = if step >= 0.0 {
                    start_val > end_val
                } else {
                    start_val < end_val
                };

                if should_skip {
                    self.runtime.instruction_pointer = end_label;
                    self.runtime.for_loop_stack.pop();
                }
            }

            OpCode::ForStep { var_name, step } => {
                // Update step value in context
                if let Some(ctx) = self.runtime.for_loop_stack.last_mut() {
                    if ctx.var_name == var_name {
                        ctx.step = step;
                    }
                }

                // Increment the variable
                if let Some(var_val) = self.runtime.variables.get_mut(&var_name) {
                    *var_val = QType::Double(var_val.to_f64() + step);
                } else {
                    self.runtime
                        .variables
                        .insert(var_name.clone(), QType::Double(step));
                }
            }

            OpCode::ForStepFast { var_index, step } => {
                if let Some(ctx) = self.runtime.for_loop_stack.last_mut() {
                    if ctx.var_index == Some(var_index) {
                        ctx.step = step;
                    }
                }

                if let Some(val) = self.runtime.globals.get_mut(var_index) {
                    *val = QType::Double(val.to_f64() + step);
                }
            }

            OpCode::Next(name) => {
                if let Some(ctx) = self.runtime.for_loop_stack.last().cloned() {
                    let current_val = self
                        .runtime
                        .variables
                        .get(&name)
                        .cloned()
                        .unwrap_or(QType::Double(0.0))
                        .to_f64();

                    // Check if we should continue looping
                    let should_continue = if ctx.step >= 0.0 {
                        current_val <= ctx.end_value
                    } else {
                        current_val >= ctx.end_value
                    };

                    if should_continue {
                        // Jump back to loop body
                        self.runtime.instruction_pointer = ctx.loop_start;
                    } else {
                        // Exit loop
                        self.runtime.for_loop_stack.pop();
                    }
                } else {
                    return Err(QError::NextWithoutFor);
                }
            }

            OpCode::NextFast(idx) => {
                if let Some(ctx) = self.runtime.for_loop_stack.last().cloned() {
                    if ctx.var_index != Some(idx) {
                        return Err(QError::NextWithoutFor);
                    }

                    let current_val = self
                        .runtime
                        .globals
                        .get(idx)
                        .map(|v| v.to_f64())
                        .unwrap_or(0.0);

                    let should_continue = if ctx.step >= 0.0 {
                        current_val <= ctx.end_value
                    } else {
                        current_val >= ctx.end_value
                    };

                    if should_continue {
                        self.runtime.instruction_pointer = ctx.loop_start;
                    } else {
                        self.runtime.for_loop_stack.pop();
                    }
                } else {
                    return Err(QError::NextWithoutFor);
                }
            }

            OpCode::Print => {
                let value = self.runtime.pop()?;
                let rendered = format!("{}", value);
                print!("{}", rendered);
                io::stdout().flush().ok();
                self.advance_cursor_text(&rendered);
            }

            OpCode::PrintNewline => {
                println!();
                io::stdout().flush().ok();
                self.runtime.cursor_row = self.runtime.cursor_row.saturating_add(1);
                self.runtime.cursor_col = 1;
            }

            OpCode::PrintUsing(count) => {
                let mut values = Vec::new();
                for _ in 0..count {
                    values.push(self.runtime.pop()?);
                }
                values.reverse();
                let format_str = self.runtime.pop()?;

                let format = if let QType::String(str) = format_str {
                    str
                } else {
                    format!("{}", format_str)
                };

                // Format each value according to the format string
                let mut result = String::new();

                for val in &values {
                    let formatted = self.format_using_value(&format, val)?;
                    result.push_str(&formatted);
                }

                print!("{}", result);
                io::stdout().flush().ok();
            }

            OpCode::Beep => {
                print!("\x07");
                io::stdout().flush().ok();
            }

            OpCode::Screen(mode) => {
                self.screen_mode = mode;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.set_screen_mode(mode as u8);
                }
                println!("[SCREEN {}]", mode);
            }

            OpCode::Pset { x, y, color } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.pset(x, y, color as u8);
                } else {
                    println!("[PSET({},{}) Color:{}]", x, y, color);
                }
            }

            OpCode::Circle {
                x,
                y,
                radius,
                color,
            } => {
                // HAL layer doesn't have circle yet, use placeholder
                println!("[CIRCLE({},{}) Radius:{} Color:{}]", x, y, radius, color);
            }

            OpCode::Sound {
                frequency,
                duration,
            } => {
                self.sound.play_note(frequency as f32, duration as u32);
            }

            OpCode::Play(melody) => {
                self.sound.play_melody(&melody);
            }

            OpCode::End | OpCode::Stop => {
                self.running = false;
            }

            OpCode::Pop => {
                self.runtime.pop()?;
            }

            OpCode::Dup => {
                let value = self.runtime.peek()?;
                self.runtime.push(value);
            }

            OpCode::OnError(addr) => {
                if addr == 0 {
                    self.runtime.error_handler_address = None;
                    self.runtime.error_resume_next = false;
                } else {
                    self.runtime.error_handler_address = Some(addr);
                    self.runtime.error_resume_next = false;
                }
            }

            OpCode::OnErrorResumeNext => {
                self.runtime.error_resume_next = true;
                self.runtime.error_handler_address = None;
            }

            OpCode::Resume => {
                if let Some(addr) = self.runtime.call_stack.pop() {
                    self.runtime.instruction_pointer = addr;
                    self.runtime.last_error = None;
                    self.runtime.last_error_code = 0; // Reset error code
                } else {
                    return Err(QError::Internal("RESUME without error".to_string()));
                }
            }

            OpCode::ResumeNext => {
                if let Some(addr) = self.runtime.call_stack.pop() {
                    self.runtime.instruction_pointer = addr + 1;
                    self.runtime.last_error = None;
                    self.runtime.last_error_code = 0; // Reset error code
                } else {
                    return Err(QError::Internal("RESUME NEXT without error".to_string()));
                }
            }

            // String functions
            OpCode::Left => {
                let n = self.runtime.pop()?.to_f64() as usize;
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let result = str.chars().take(n).collect::<String>();
                    self.runtime.push(QType::String(result));
                }
            }

            OpCode::Right => {
                let n = self.runtime.pop()?.to_f64() as usize;
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let len = str.chars().count();
                    let start = len.saturating_sub(n);
                    let result = str.chars().skip(start).collect::<String>();
                    self.runtime.push(QType::String(result));
                }
            }

            OpCode::Mid => {
                let len = self.runtime.pop()?.to_f64() as usize;
                let start = self.runtime.pop()?.to_f64() as usize;
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let result = str
                        .chars()
                        .skip(start.saturating_sub(1))
                        .take(len)
                        .collect::<String>();
                    self.runtime.push(QType::String(result));
                }
            }

            OpCode::Len => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::Integer(str.len() as i16));
                }
            }

            OpCode::InStr => {
                let search = self.runtime.pop()?;
                let source = self.runtime.pop()?;
                if let (QType::String(src), QType::String(srch)) = (source, search) {
                    let pos = src.find(&srch).map(|p| (p + 1) as i16).unwrap_or(0);
                    self.runtime.push(QType::Integer(pos));
                }
            }

            OpCode::LCase => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::String(str.to_lowercase()));
                }
            }

            OpCode::UCase => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::String(str.to_uppercase()));
                }
            }

            OpCode::LTrim => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime
                        .push(QType::String(str.trim_start().to_string()));
                }
            }

            OpCode::RTrim => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::String(str.trim_end().to_string()));
                }
            }

            OpCode::Trim => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::String(str.trim().to_string()));
                }
            }

            OpCode::StrFunc => {
                let n = self.runtime.pop()?;
                let content = format!("{}", n);
                self.runtime.push(QType::String(content));
            }

            OpCode::ValFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let val = str.trim().parse::<f64>().unwrap_or(0.0);
                    self.runtime.push(QType::Double(val));
                }
            }

            OpCode::ChrFunc => {
                let n = self.runtime.pop()?;
                let ch = char::from_u32(n.to_f64() as u32).unwrap_or('\0');
                self.runtime.push(QType::String(ch.to_string()));
            }

            OpCode::AscFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let code = str.chars().next().map(|c| c as i16).unwrap_or(0);
                    self.runtime.push(QType::Integer(code));
                }
            }

            OpCode::SpaceFunc => {
                let n = self.runtime.pop()?.to_f64() as usize;
                self.runtime.push(QType::String(" ".repeat(n)));
            }

            OpCode::StringFunc => {
                let s = self.runtime.pop()?;
                let n = self.runtime.pop()?.to_f64() as usize;
                if let QType::String(str) = s {
                    let ch = str.chars().next().unwrap_or(' ');
                    self.runtime.push(QType::String(ch.to_string().repeat(n)));
                } else {
                    self.runtime.push(QType::String(" ".repeat(n)));
                }
            }

            // Math functions
            OpCode::Abs => {
                let n = self.runtime.pop()?;
                let result = n.to_f64().abs();
                self.runtime.push(QType::Double(result));
            }

            OpCode::Sgn => {
                let n = self.runtime.pop()?;
                let val = n.to_f64();
                let result = if val > 0.0 {
                    1
                } else if val < 0.0 {
                    -1
                } else {
                    0
                };
                self.runtime.push(QType::Integer(result));
            }

            OpCode::Sin => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64().sin()));
            }

            OpCode::Cos => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64().cos()));
            }

            OpCode::Tan => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64().tan()));
            }

            OpCode::Atn => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64().atan()));
            }

            OpCode::ExpFunc => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64().exp()));
            }

            OpCode::LogFunc => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64().ln()));
            }

            OpCode::Sqr => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64().sqrt()));
            }

            OpCode::IntFunc => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Integer(n.to_f64().floor() as i16));
            }

            OpCode::Fix => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Integer(n.to_f64().trunc() as i16));
            }

            OpCode::Rnd => {
                use std::cell::RefCell;
                thread_local! {
                    static RNG_STATE: RefCell<u32> = RefCell::new(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or(std::time::Duration::from_secs(0))
                            .as_secs() as u32
                    );
                }

                let random = RNG_STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    *s = s.wrapping_mul(1103515245).wrapping_add(12345);
                    (*s / 65536) % 32768
                });

                self.runtime.push(QType::Single(random as f32 / 32768.0));
            }

            // Type conversion
            OpCode::CInt => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Integer(n.to_f64().round() as i16));
            }

            OpCode::CLng => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Long(n.to_f64().round() as i32));
            }

            OpCode::CSng => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Single(n.to_f64() as f32));
            }

            OpCode::CDbl => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::Double(n.to_f64()));
            }

            OpCode::CStr => {
                let n = self.runtime.pop()?;
                self.runtime.push(QType::String(format!("{}", n)));
            }

            // Misc
            OpCode::Swap(a, b) => {
                let val_a = self
                    .runtime
                    .variables
                    .get(&a)
                    .cloned()
                    .unwrap_or(QType::Empty);
                let val_b = self
                    .runtime
                    .variables
                    .get(&b)
                    .cloned()
                    .unwrap_or(QType::Empty);
                self.runtime.variables.insert(a, val_b);
                self.runtime.variables.insert(b, val_a);
            }

            OpCode::Sleep => {
                let duration = self.runtime.pop()?;
                let seconds = duration.to_f64();
                if seconds < 0.0 {
                    qb_wait_for_keypress();
                } else {
                    std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
                }
            }

            // Arrays
            OpCode::ArrayDim { name, dimensions } => {
                // Calculate total size with strict limits
                let mut total_size: usize = 1;
                for (lower, upper) in &dimensions {
                    // Validate bounds
                    if *upper < *lower {
                        return Err(QError::Internal(format!(
                            "Invalid array bounds: {} to {}",
                            lower, upper
                        )));
                    }

                    // Calculate dimension size with overflow check
                    let dim_size = match (*upper - *lower + 1).try_into() {
                        Ok(size) if size > 0 && size <= 10000 => size, // Max 10k per dimension
                        _ => {
                            return Err(QError::Internal(format!(
                                "Array dimension too large: {} to {}",
                                lower, upper
                            )))
                        }
                    };

                    // Check for overflow and apply total limit
                    total_size = match total_size.checked_mul(dim_size) {
                        Some(size) if size <= 100000 => size, // Max 100k total elements
                        _ => {
                            return Err(QError::Internal(
                                "Array size exceeds limit (max 100,000 elements)".to_string(),
                            ))
                        }
                    };
                }

                // Pre-allocate with capacity hint
                let mut array = Vec::with_capacity(total_size);
                array.resize(total_size, QType::Integer(0));
                self.runtime.arrays.insert(name.clone(), array);
                self.runtime
                    .array_dimensions
                    .insert(name.clone(), dimensions.clone());
            }

            OpCode::ArrayRedim {
                name,
                dimensions,
                preserve,
            } => {
                // Calculate total size with strict limits
                let mut total_size: usize = 1;
                for (lower, upper) in &dimensions {
                    if *upper < *lower {
                        return Err(QError::Internal(format!(
                            "Invalid array bounds: {} to {}",
                            lower, upper
                        )));
                    }

                    let dim_size = match (*upper - *lower + 1).try_into() {
                        Ok(size) if size > 0 && size <= 10000 => size,
                        _ => {
                            return Err(QError::Internal(format!(
                                "Array dimension too large: {} to {}",
                                lower, upper
                            )))
                        }
                    };

                    total_size = match total_size.checked_mul(dim_size) {
                        Some(size) if size <= 100000 => size,
                        _ => {
                            return Err(QError::Internal(
                                "Array size exceeds limit (max 100,000 elements)".to_string(),
                            ))
                        }
                    };
                }

                if preserve {
                    if let Some(old_array) = self.runtime.arrays.get(&name.clone()) {
                        let mut new_array = Vec::with_capacity(total_size);
                        new_array.resize(total_size, QType::Integer(0));
                        let copy_size = old_array.len().min(total_size);
                        new_array[..copy_size].clone_from_slice(&old_array[..copy_size]);
                        self.runtime.arrays.insert(name.clone(), new_array);
                    }
                } else {
                    let mut array = Vec::with_capacity(total_size);
                    array.resize(total_size, QType::Integer(0));
                    self.runtime.arrays.insert(name.clone(), array);
                }
                self.runtime
                    .array_dimensions
                    .insert(name.clone(), dimensions.clone());
            }

            OpCode::ArrayLoad(name, num_indices) => {
                // Pop indices from stack
                let mut indices = Vec::with_capacity(num_indices);
                for _ in 0..num_indices {
                    let idx = self.runtime.pop()?;
                    indices.push(idx.to_f64() as i32);
                }
                indices.reverse();

                // create implicit if not exists - use smaller default size
                if let std::collections::hash_map::Entry::Vacant(e) =
                    self.runtime.arrays.entry(name.clone())
                {
                    let mut dimensions = Vec::with_capacity(num_indices);
                    let mut total_size: usize = 1;
                    for i in 0..num_indices {
                        // Use actual index + small buffer instead of fixed large size
                        let idx_val = indices.get(i).copied().unwrap_or(0);
                        let upper = (idx_val + 5).clamp(5, 100); // Reduced cap to 100
                        dimensions.push((0, upper));
                        total_size = match total_size.checked_mul((upper + 1) as usize) {
                            Some(size) if size <= 10000 => size,
                            _ => 10000, // Cap at 10k
                        };
                    }
                    let mut array = Vec::with_capacity(total_size);
                    array.resize(total_size, QType::Integer(0));
                    e.insert(array);
                    self.runtime
                        .array_dimensions
                        .insert(name.clone(), dimensions);
                }

                let dims = self
                    .runtime
                    .array_dimensions
                    .get(&name.clone())
                    .cloned()
                    .unwrap_or_default();

                // Calculate linear index
                let mut linear_index = 0;
                let mut multiplier = 1;
                for (i, (lower, upper)) in dims.iter().enumerate().rev() {
                    let idx = indices.get(i).copied().unwrap_or(0);
                    linear_index += (idx - lower) as usize * multiplier;
                    multiplier *= (upper - lower + 1) as usize;
                }

                if let Some(array) = self.runtime.arrays.get(&name.clone()) {
                    if linear_index < array.len() {
                        self.runtime.push(array[linear_index].clone());
                    } else {
                        return Err(QError::SubscriptOutOfRange);
                    }
                } else {
                    self.runtime.push(QType::Integer(0));
                }
            }

            OpCode::ArrayStore(name, num_indices) => {
                // Stack order: indices..., value (value on top)
                let value = self.runtime.pop()?;
                let value = self.normalize_array_value_for_name(&name, value);

                // Pop indices from stack
                let mut indices = Vec::with_capacity(num_indices);
                for _ in 0..num_indices {
                    let idx = self.runtime.pop()?;
                    indices.push(idx.to_f64() as i32);
                }
                indices.reverse();

                // create implicit if not exists - use smaller default size
                if let std::collections::hash_map::Entry::Vacant(e) =
                    self.runtime.arrays.entry(name.clone())
                {
                    let mut dimensions = Vec::with_capacity(num_indices);
                    let mut total_size: usize = 1;
                    for i in 0..num_indices {
                        // Use actual index + small buffer instead of fixed large size
                        let idx_val = indices.get(i).copied().unwrap_or(0);
                        let upper = (idx_val + 5).clamp(5, 100); // Reduced cap to 100
                        dimensions.push((0, upper));
                        total_size = match total_size.checked_mul((upper + 1) as usize) {
                            Some(size) if size <= 10000 => size,
                            _ => 10000, // Cap at 10k
                        };
                    }
                    let mut array = Vec::with_capacity(total_size);
                    array.resize(total_size, QType::Integer(0));
                    e.insert(array);
                    self.runtime
                        .array_dimensions
                        .insert(name.clone(), dimensions);
                }

                let dims = self
                    .runtime
                    .array_dimensions
                    .get(&name.clone())
                    .cloned()
                    .unwrap_or_default();

                // Calculate linear index
                let mut linear_index = 0;
                let mut multiplier = 1;
                for (i, (lower, upper)) in dims.iter().enumerate().rev() {
                    let idx = indices.get(i).copied().unwrap_or(0);
                    linear_index += (idx - lower) as usize * multiplier;
                    multiplier *= (upper - lower + 1) as usize;
                }

                if let Some(array) = self.runtime.arrays.get_mut(&name.clone()) {
                    if linear_index < array.len() {
                        array[linear_index] = value;
                    } else {
                        return Err(QError::SubscriptOutOfRange);
                    }
                }
            }

            // Functions and Subs
            OpCode::DefineFunction {
                name,
                params,
                result,
                body_start,
                body_end,
            } => {
                self.runtime
                    .functions
                    .insert(name.clone(), (params.clone(), result, body_start, body_end));
                // Skip to end of function definition
                self.runtime.instruction_pointer = body_end;
            }

            OpCode::DefineSub {
                name,
                params,
                body_start,
                body_end,
            } => {
                self.runtime
                    .subs
                    .insert(name.clone(), (params.clone(), body_start, body_end));
                // Skip to end of sub definition
                self.runtime.instruction_pointer = body_end;
            }

            OpCode::DefFn { name, params, body } => {
                self.runtime.def_fns.insert(name, (params, body));
            }

            OpCode::CallDefFn(name) => {
                if let Some((params, body)) = self.runtime.def_fns.get(&name).cloned() {
                    // Pop arguments from stack
                    let mut args = Vec::new();
                    for _ in 0..params.len() {
                        args.push(self.runtime.pop()?);
                    }
                    args.reverse();

                    // Backup globals that will be used for parameters
                    let mut backups = Vec::new();
                    for (i, arg) in args.iter().enumerate() {
                        if i < self.runtime.globals.len() {
                            backups.push(self.runtime.globals[i].clone());
                            self.runtime.globals[i] = arg.clone();
                        } else {
                            // Extend globals if needed
                            while self.runtime.globals.len() <= i {
                                self.runtime.globals.push(QType::Empty);
                            }
                            backups.push(QType::Empty);
                            self.runtime.globals[i] = arg.clone();
                        }
                    }

                    // Execute function body - the result will be pushed onto the stack
                    for opcode in body {
                        self.execute_opcode(opcode)?;
                    }

                    // Restore backed up globals
                    for (i, val) in backups.iter().enumerate() {
                        if i < self.runtime.globals.len() {
                            self.runtime.globals[i] = val.clone();
                        }
                    }
                } else {
                    return Err(QError::InvalidProcedure(name));
                }
            }

            OpCode::CallFunction { name, by_ref } => {
                if let Some((params, result_slot, body_start, _body_end)) =
                    self.runtime.functions.get(&name.clone()).cloned()
                {
                    let mut args = Vec::with_capacity(params.len());
                    for _ in 0..params.len() {
                        args.push(self.runtime.pop()?);
                    }
                    args.reverse();

                    let mut saved_slots = Vec::new();
                    let mut copy_backs = Vec::new();

                    for ((param_idx, arg), caller_target) in
                        params.iter().zip(args.into_iter()).zip(by_ref.iter())
                    {
                        while self.runtime.globals.len() <= *param_idx {
                            self.runtime.globals.push(QType::Empty);
                        }
                        match caller_target {
                            ByRefTarget::Global(caller_idx) if *caller_idx == *param_idx => {}
                            ByRefTarget::Global(_)
                            | ByRefTarget::ArrayElement { .. }
                            | ByRefTarget::None => {
                                saved_slots
                                    .push((*param_idx, self.runtime.globals[*param_idx].clone()));
                            }
                        }
                        if !matches!(caller_target, ByRefTarget::None) {
                            copy_backs.push((caller_target.clone(), *param_idx));
                        }
                        self.runtime.globals[*param_idx] =
                            self.normalize_global_value_for_slot(*param_idx, arg);
                    }

                    while self.runtime.globals.len() <= result_slot {
                        self.runtime.globals.push(QType::Empty);
                    }
                    saved_slots.push((result_slot, self.runtime.globals[result_slot].clone()));

                    self.runtime.procedure_frames.push(ProcedureFrame {
                        saved_slots,
                        copy_backs,
                    });
                    self.runtime
                        .call_stack
                        .push(self.runtime.instruction_pointer);

                    self.runtime.instruction_pointer = body_start;
                } else {
                    return Err(QError::InvalidProcedure(name.clone()));
                }
            }

            OpCode::CallSub { name, by_ref } => {
                if let Some((params, body_start, _body_end)) =
                    self.runtime.subs.get(&name.clone()).cloned()
                {
                    let mut args = Vec::with_capacity(params.len());
                    for _ in 0..params.len() {
                        args.push(self.runtime.pop()?);
                    }
                    args.reverse();

                    let mut saved_slots = Vec::new();
                    let mut copy_backs = Vec::new();

                    for ((param_idx, arg), caller_target) in
                        params.iter().zip(args.into_iter()).zip(by_ref.iter())
                    {
                        while self.runtime.globals.len() <= *param_idx {
                            self.runtime.globals.push(QType::Empty);
                        }
                        match caller_target {
                            ByRefTarget::Global(caller_idx) if *caller_idx == *param_idx => {}
                            ByRefTarget::Global(_)
                            | ByRefTarget::ArrayElement { .. }
                            | ByRefTarget::None => {
                                saved_slots
                                    .push((*param_idx, self.runtime.globals[*param_idx].clone()));
                            }
                        }
                        if !matches!(caller_target, ByRefTarget::None) {
                            copy_backs.push((caller_target.clone(), *param_idx));
                        }
                        self.runtime.globals[*param_idx] =
                            self.normalize_global_value_for_slot(*param_idx, arg);
                    }

                    self.runtime.procedure_frames.push(ProcedureFrame {
                        saved_slots,
                        copy_backs,
                    });
                    self.runtime
                        .call_stack
                        .push(self.runtime.instruction_pointer);

                    self.runtime.instruction_pointer = body_start;
                } else {
                    return Err(QError::InvalidProcedure(name.clone()));
                }
            }

            OpCode::CallNative(name) => {
                return Err(QError::InvalidProcedure(name));
            }

            OpCode::FunctionReturn | OpCode::SubReturn => {
                if let Some(frame) = self.runtime.procedure_frames.pop() {
                    for (target, param_idx) in frame.copy_backs {
                        self.apply_by_ref_copy_back(target, param_idx);
                    }
                    for (slot_idx, value) in frame.saved_slots {
                        while self.runtime.globals.len() <= slot_idx {
                            self.runtime.globals.push(QType::Empty);
                        }
                        self.runtime.globals[slot_idx] = value;
                    }
                }
                if let Some(addr) = self.runtime.call_stack.pop() {
                    self.runtime.instruction_pointer = addr;
                } else {
                    return Err(QError::ReturnWithoutGosub);
                }
            }

            // Error handling
            OpCode::Timer => {
                let elapsed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::from_secs(0))
                    .as_secs_f64();
                let seconds_since_midnight = elapsed % 86400.0;
                self.runtime
                    .push(QType::Single(seconds_since_midnight as f32));
            }

            OpCode::OnTimer {
                interval_secs,
                handler,
            } => {
                self.runtime.timer_handler_address = Some(handler);
                self.runtime.timer_interval_secs = interval_secs.max(0.001);
                self.runtime.next_timer_tick = None;
            }

            OpCode::TimerOn => {
                self.runtime.timer_enabled = true;
                self.runtime.next_timer_tick = None;
            }

            OpCode::TimerOff | OpCode::TimerStop => {
                self.runtime.timer_enabled = false;
                self.runtime.next_timer_tick = None;
            }

            OpCode::Date => {
                use chrono::Local;
                let now = Local::now();
                let date_str = now.format("%m-%d-%Y").to_string();
                self.runtime.push(QType::String(date_str));
            }

            OpCode::Time => {
                use chrono::Local;
                let now = Local::now();
                let time_str = now.format("%H:%M:%S").to_string();
                self.runtime.push(QType::String(time_str));
            }

            OpCode::Clear => {
                self.runtime.variables.clear();
                self.runtime.arrays.clear();
                self.runtime.array_dimensions.clear();
                for value in &mut self.runtime.globals {
                    *value = QType::Integer(0);
                }
            }

            OpCode::Cls => {
                // Clear screen (simplified)
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().ok();
                self.runtime.cursor_row = 1;
                self.runtime.cursor_col = 1;
            }

            OpCode::Locate(row, col) => {
                print!("\x1B[{};{}H", row, col);
                io::stdout().flush().ok();
                self.runtime.cursor_row = row as i16;
                self.runtime.cursor_col = col as i16;
            }

            OpCode::Color(fg, _bg) => {
                // Simplified color support
                print!("\x1B[{}m", 30 + fg);
                io::stdout().flush().ok();
            }

            OpCode::SelectCase => {
                // SelectCase just marks the start, the value is already on stack
                // We'll keep it there for comparison with each case
            }

            OpCode::EndSelect => {
                // Pop the select value from stack
                self.runtime.pop()?;
            }

            OpCode::Paint {
                x,
                y,
                paint_color,
                border_color,
            } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.paint(x, y, paint_color as u8, border_color as u8);
                } else {
                    println!(
                        "[PAINT({},{}) Color:{} Border:{}]",
                        x, y, paint_color, border_color
                    );
                }
            }

            OpCode::Draw { commands } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.draw(&commands);
                } else {
                    println!("[DRAW {}]", commands);
                }
            }

            OpCode::Palette { attribute, color } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.palette(attribute as u8, color as u8);
                } else {
                    println!("[PALETTE {} {}]", attribute, color);
                }
            }

            OpCode::View {
                x1,
                y1,
                x2,
                y2,
                fill_color,
                border_color,
            } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.view(x1, y1, x2, y2, fill_color as u8, border_color as u8);
                } else {
                    println!(
                        "[VIEW({},{})-({},{}) Fill:{} Border:{}]",
                        x1, y1, x2, y2, fill_color, border_color
                    );
                }
            }

            OpCode::ViewPrint { top, bottom } => {
                self.runtime.view_print_top = Some(top as i16);
                self.runtime.view_print_bottom = Some(bottom as i16);
            }

            OpCode::ViewReset => {
                self.runtime.view_print_top = None;
                self.runtime.view_print_bottom = None;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.view_reset();
                }
            }

            OpCode::Window { x1, y1, x2, y2 } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.window(x1, y1, x2, y2);
                } else {
                    println!("[WINDOW({},{})-({},{})]", x1, y1, x2, y2);
                }
            }

            OpCode::WindowReset => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.window_reset();
                } else {
                    println!("[WINDOW RESET]");
                }
            }

            OpCode::Data(values) => {
                // Store DATA values in the program's data section
                self.runtime.data_section.push(values);
            }

            OpCode::Read(var_name) => {
                // Read next value from DATA using the data pointer
                if let Some(value) = self.runtime.read_data() {
                    self.runtime.variables.insert(var_name.clone(), value);
                } else {
                    return Err(QError::Internal("Out of DATA".to_string()));
                }
            }

            OpCode::Restore(label) => {
                // Reset data pointer to beginning or to labeled position
                if let Some(lbl) = label {
                    self.runtime.data_pointer.reset_to_label(&lbl);
                } else {
                    self.runtime.data_pointer.reset();
                }
            }

            OpCode::Err => {
                // Return the stored error code
                let err_num = if self.runtime.last_error_code != 0 {
                    self.runtime.last_error_code
                } else {
                    self.runtime
                        .last_error
                        .as_ref()
                        .map(|e| e.code())
                        .unwrap_or(0)
                };
                self.runtime.push(QType::Integer(err_num));
            }

            OpCode::Erl => {
                let line_num = 0i16; // Would need to track line numbers
                self.runtime.push(QType::Integer(line_num));
            }

            OpCode::ErDev => {
                // Return device error code
                self.runtime.push(QType::Integer(0));
            }

            OpCode::ErDevStr => {
                // Return device error string
                self.runtime.push(QType::String(String::new()));
            }

            // Array bounds functions
            OpCode::LBound(name, dim) => {
                let dims = self.runtime.array_dimensions.get(&name);
                let lower = dims
                    .and_then(|d| d.get(dim as usize))
                    .map(|(l, _)| *l)
                    .unwrap_or(0);
                self.runtime.push(QType::Integer(lower as i16));
            }

            OpCode::UBound(name, dim) => {
                let dims = self.runtime.array_dimensions.get(&name);
                let upper = dims
                    .and_then(|d| d.get(dim as usize))
                    .map(|(_, u)| *u)
                    .unwrap_or(0);
                self.runtime.push(QType::Integer(upper as i16));
            }

            OpCode::Erase(name) => {
                self.runtime.arrays.remove(&name);
                self.runtime.array_dimensions.remove(&name);
            }

            // File I/O operations
            OpCode::Open { mode } => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let filename = self.runtime.pop()?;
                if let QType::String(fname) = filename {
                    self.file_io.open_by_num(file_num, &fname, &mode)?;
                }
            }

            OpCode::Close => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.file_io.close_by_num(file_num)?;
            }

            OpCode::PrintFile(file_num) => {
                let value = self.runtime.pop()?;
                let content = format!("{}", value);
                self.file_io
                    .write_line_by_num(file_num.parse().unwrap_or(0), &content)?;
            }

            OpCode::PrintFileDynamic => {
                let value = self.runtime.pop()?;
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let content = format!("{}", value);
                self.file_io.write_by_num(file_num, &content)?;
            }

            OpCode::LineInputDynamic => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let line = self.file_io.read_line_by_num(file_num)?;
                self.runtime.push(QType::String(line));
            }

            OpCode::InputFile(file_num) => {
                let line = self
                    .file_io
                    .read_line_by_num(file_num.parse().unwrap_or(0))?;
                self.runtime.push(QType::String(line));
            }

            OpCode::InputFileDynamic(value_count) => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let line = self.file_io.read_line_by_num(file_num)?;
                let mut fields = split_input_fields(&line);
                while fields.len() < value_count {
                    fields.push(String::new());
                }
                for value in fields.into_iter().take(value_count).rev() {
                    self.runtime.push(QType::String(value));
                }
            }

            OpCode::Eof(file_num) => {
                let eof = self.file_io.is_eof_by_num(file_num.parse().unwrap_or(0));
                self.runtime.push(QType::Integer(if eof { -1 } else { 0 }));
            }

            OpCode::Lof(file_num) => {
                let len = self.file_io.length_by_num(file_num.parse().unwrap_or(0));
                self.runtime.push(QType::Long(len as i32));
            }

            OpCode::FreeFile => {
                let file_num = self.file_io.get_free_file_num();
                self.runtime.push(QType::Integer(file_num));
            }

            OpCode::Seek(file_num, pos) => {
                self.file_io
                    .seek_by_num(file_num.parse().unwrap_or(0), pos as u64)?;
            }

            OpCode::SeekDynamic => {
                let pos = self.runtime.pop()?.to_f64() as u64;
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.file_io.seek_by_num(file_num, pos)?;
            }

            // Preset - same as Pset but with background color
            OpCode::Preset { x, y, color } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.preset(x, y, color as u8);
                }
            }

            // Line drawing
            OpCode::Line {
                x1,
                y1,
                x2,
                y2,
                color,
            } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.line(x1, y1, x2, y2, color as u8);
                }
            }

            OpCode::Get => {
                let record_num = self.runtime.pop()?.to_f64().max(1.0) as u64;
                let file_num = self.runtime.pop()?.to_f64() as i32;

                if let Some(fields) = self.runtime.random_fields.get(&file_num).cloned() {
                    let record_len: usize = fields.iter().map(|(width, _)| *width).sum();
                    let mut buffer = vec![0u8; record_len];
                    self.file_io.get(file_num as u8, record_num, &mut buffer)?;

                    let mut offset = 0usize;
                    for (width, var_index) in fields {
                        let end = offset + width;
                        let slice = &buffer[offset..end];
                        let text = String::from_utf8_lossy(slice).to_string();
                        if var_index < self.runtime.globals.len() {
                            self.runtime.globals[var_index] = QType::String(text);
                        }
                        offset = end;
                    }
                }
                self.runtime.push(QType::Empty);
            }
            OpCode::Put => {
                let value = self.runtime.pop()?;
                let record_num = self.runtime.pop()?.to_f64().max(1.0) as u64;
                let file_num = self.runtime.pop()?.to_f64() as i32;

                if matches!(value, QType::Empty) {
                    if let Some(fields) = self.runtime.random_fields.get(&file_num).cloned() {
                        let mut buffer = Vec::new();
                        for (width, var_index) in fields {
                            let field_value = self
                                .runtime
                                .globals
                                .get(var_index)
                                .cloned()
                                .unwrap_or(QType::String(String::new()));
                            let mut text = match field_value {
                                QType::String(s) => s,
                                other => format!("{}", other),
                            };
                            if text.len() > width {
                                text.truncate(width);
                            } else if text.len() < width {
                                text.push_str(&" ".repeat(width - text.len()));
                            }
                            buffer.extend_from_slice(text.as_bytes());
                        }
                        self.file_io.put(file_num as u8, record_num, &buffer)?;
                    }
                } else {
                    let bytes = match value {
                        QType::String(s) => s.into_bytes(),
                        other => format!("{}", other).into_bytes(),
                    };
                    self.file_io
                        .seek(file_num as u8, record_num.saturating_sub(1))?;
                    self.file_io.write_bytes(file_num as u8, &bytes)?;
                }
            }

            // WriteFile for formatted output
            OpCode::WriteFile(file_num) => {
                let value = self.runtime.pop()?;
                let content = format!("\"{}\"", value);
                self.file_io
                    .write_line_by_num(file_num.parse().unwrap_or(0), &content)?;
            }

            OpCode::WriteFileDynamic(value_count) => {
                let mut values = Vec::with_capacity(value_count);
                for _ in 0..value_count {
                    values.push(self.runtime.pop()?);
                }
                values.reverse();

                let file_num = self.runtime.pop()?.to_f64() as i32;
                let content = values
                    .into_iter()
                    .map(|value| match value {
                        QType::String(s) => format!("\"{}\"", s),
                        other => format!("{}", other),
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                self.file_io.write_line_by_num(file_num, &content)?;
            }

            // New advanced functions
            OpCode::HexFunc => {
                let n = self.runtime.pop()?;
                let val = n.to_f64() as i64;
                self.runtime.push(QType::String(format!("{:X}", val)));
            }

            OpCode::OctFunc => {
                let n = self.runtime.pop()?;
                let val = n.to_f64() as i64;
                self.runtime.push(QType::String(format!("{:o}", val)));
            }

            // Binary conversion functions
            OpCode::MkiFunc => {
                let n = self.runtime.pop()?;
                let val = n.to_f64() as i16;
                let bytes = val.to_le_bytes();
                // Store as raw bytes in string (unsafe but compatible with QBasic)
                let s: String = bytes.iter().map(|&b| b as char).collect();
                self.runtime.push(QType::String(s));
            }

            OpCode::MklFunc => {
                let n = self.runtime.pop()?;
                let val = n.to_f64() as i32;
                let bytes = val.to_le_bytes();
                let s: String = bytes.iter().map(|&b| b as char).collect();
                self.runtime.push(QType::String(s));
            }

            OpCode::MksFunc => {
                let n = self.runtime.pop()?;
                let val = n.to_f64() as f32;
                let bytes = val.to_le_bytes();
                let s: String = bytes.iter().map(|&b| b as char).collect();
                self.runtime.push(QType::String(s));
            }

            OpCode::MkdFunc => {
                let n = self.runtime.pop()?;
                let val = n.to_f64();
                let bytes = val.to_le_bytes();
                let s: String = bytes.iter().map(|&b| b as char).collect();
                self.runtime.push(QType::String(s));
            }

            OpCode::CviFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let bytes: Vec<u8> = str.chars().map(|c| c as u8).collect();
                    if bytes.len() >= 2 {
                        let val = i16::from_le_bytes([bytes[0], bytes[1]]);
                        self.runtime.push(QType::Integer(val));
                    } else {
                        self.runtime.push(QType::Integer(0));
                    }
                }
            }

            OpCode::CvlFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let bytes: Vec<u8> = str.chars().map(|c| c as u8).collect();
                    if bytes.len() >= 4 {
                        let val = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                        self.runtime.push(QType::Long(val));
                    } else {
                        self.runtime.push(QType::Long(0));
                    }
                }
            }

            OpCode::CvsFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let bytes: Vec<u8> = str.chars().map(|c| c as u8).collect();
                    if bytes.len() >= 4 {
                        let val = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                        self.runtime.push(QType::Single(val));
                    } else {
                        self.runtime.push(QType::Single(0.0));
                    }
                }
            }

            OpCode::CvdFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let bytes: Vec<u8> = str.chars().map(|c| c as u8).collect();
                    if bytes.len() >= 8 {
                        let val = f64::from_le_bytes([
                            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                            bytes[7],
                        ]);
                        self.runtime.push(QType::Double(val));
                    } else {
                        self.runtime.push(QType::Double(0.0));
                    }
                }
            }

            // System functions
            OpCode::FreFunc(arg_type) => {
                // Return free memory based on argument type
                // 0 = string memory, -1 = array memory, -2 = stack memory
                let free_mem = match arg_type {
                    0 => 524288,   // 512KB for strings
                    -1 => 1048576, // 1MB for arrays
                    -2 => 262144,  // 256KB for stack
                    _ => 1048576,  // Default 1MB
                };
                self.runtime.push(QType::Long(free_mem));
            }

            OpCode::CsrLinFunc => {
                self.runtime.push(QType::Integer(self.runtime.cursor_row));
            }

            OpCode::PosFunc(_arg) => {
                self.runtime.push(QType::Integer(self.runtime.cursor_col));
            }

            OpCode::EnvironFunc => {
                let var_name = self.runtime.pop()?;
                if let QType::String(name) = var_name {
                    // Try to get environment variable
                    let value = std::env::var(&name).unwrap_or_default();
                    self.runtime.push(QType::String(value));
                } else {
                    self.runtime.push(QType::String(String::new()));
                }
            }

            OpCode::CommandFunc => {
                let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
                self.runtime.push(QType::String(args));
            }

            OpCode::InKeyFunc => {
                self.runtime.push(QType::String(qb_inkey_nonblocking()));
            }

            // Memory/Hardware functions
            OpCode::PeekFunc(addr) => {
                let offset = addr.clamp(0, u16::MAX as i32) as u16;
                let value = self.memory.peek(self.runtime.current_segment, offset);
                self.runtime.push(QType::Integer(value as i16));
            }

            OpCode::PeekDynamic => {
                let addr = self.runtime.pop()?.to_f64().floor() as i32;
                let offset = addr.clamp(0, u16::MAX as i32) as u16;
                let value = self.memory.peek(self.runtime.current_segment, offset);
                self.runtime.push(QType::Integer(value as i16));
            }

            OpCode::PokeFunc(addr, value) => {
                let offset = addr.clamp(0, u16::MAX as i32) as u16;
                let byte = value.clamp(0, u8::MAX as i32) as u8;
                self.memory.poke(self.runtime.current_segment, offset, byte);
            }

            OpCode::PokeDynamic => {
                let value = self.runtime.pop()?.to_f64().floor() as i32;
                let addr = self.runtime.pop()?.to_f64().floor() as i32;
                let offset = addr.clamp(0, u16::MAX as i32) as u16;
                let byte = value.clamp(0, u8::MAX as i32) as u8;
                self.memory.poke(self.runtime.current_segment, offset, byte);
            }

            OpCode::WaitDynamic { has_xor } => {
                let xor_mask = if has_xor {
                    self.runtime.pop()?.to_f64().floor() as i32
                } else {
                    0
                };
                let and_mask = self.runtime.pop()?.to_f64().floor() as i32;
                let addr = self.runtime.pop()?.to_f64().floor() as i32;
                let offset = addr.clamp(0, u16::MAX as i32) as u16;
                let and_mask = and_mask.clamp(0, u8::MAX as i32) as u8;
                let xor_mask = xor_mask.clamp(0, u8::MAX as i32) as u8;

                loop {
                    let value = self.memory.peek(self.runtime.current_segment, offset);
                    if ((value ^ xor_mask) & and_mask) != 0 {
                        break;
                    }
                    std::thread::yield_now();
                }
            }

            OpCode::BLoadDynamic { has_offset } => {
                let offset = if has_offset {
                    self.runtime.pop()?.to_f64().floor() as i32
                } else {
                    0
                };
                let filename = self.runtime.pop()?;
                let path = match filename {
                    QType::String(s) => s,
                    other => format!("{}", other),
                };
                let data = std::fs::read(&path)
                    .map_err(|e| QError::FileNotFound(format!("BLOAD error: {}", e)))?;
                let base = offset.clamp(0, u16::MAX as i32) as u16;
                for (i, byte) in data.iter().enumerate() {
                    let addr = base.saturating_add(i.min(u16::MAX as usize) as u16);
                    self.memory.poke(self.runtime.current_segment, addr, *byte);
                }
            }

            OpCode::BSaveDynamic => {
                let length = self.runtime.pop()?.to_f64().floor() as i32;
                let offset = self.runtime.pop()?.to_f64().floor() as i32;
                let filename = self.runtime.pop()?;
                let path = match filename {
                    QType::String(s) => s,
                    other => format!("{}", other),
                };
                let offset = offset.clamp(0, u16::MAX as i32) as u16;
                let length = length.max(0) as usize;
                let mut data = Vec::with_capacity(length);
                for i in 0..length {
                    let addr = offset.saturating_add(i.min(u16::MAX as usize) as u16);
                    data.push(self.memory.peek(self.runtime.current_segment, addr));
                }
                std::fs::write(&path, data)
                    .map_err(|e| QError::Internal(format!("BSAVE error: {}", e)))?;
            }

            OpCode::InpDynamic => {
                let port = self.runtime.pop()?.to_f64().floor() as i32;
                let port = port.clamp(0, u16::MAX as i32) as u16;
                let value = self.runtime.pseudo_ports.get(&port).copied().unwrap_or(0);
                self.runtime.push(QType::Integer(value as i16));
            }

            OpCode::OutDynamic => {
                let value = self.runtime.pop()?.to_f64().floor() as i32;
                let port = self.runtime.pop()?.to_f64().floor() as i32;
                let port = port.clamp(0, u16::MAX as i32) as u16;
                let value = value.clamp(0, u8::MAX as i32) as u8;
                self.runtime.pseudo_ports.insert(port, value);
            }

            OpCode::DefSeg(seg) => {
                self.runtime.current_segment = seg.clamp(0, u16::MAX as i32) as u16;
            }

            OpCode::VarPtrFunc(var) => {
                let (_segment, offset, _bytes) = self.ensure_pseudo_var_storage(&var);
                self.runtime.push(QType::Long(offset as i32));
            }

            OpCode::VarSegFunc(var) => {
                let (segment, _offset, _bytes) = self.ensure_pseudo_var_storage(&var);
                self.runtime.push(QType::Integer(segment as i16));
            }

            OpCode::SaddFunc(var) => {
                let (segment, offset, _bytes) = self.ensure_pseudo_var_storage(&var);
                let address = DosMemory::absolute_address(segment, offset) as i32;
                self.runtime.push(QType::Long(address));
            }

            OpCode::VarPtrStrFunc(var) => {
                let (segment, offset, _bytes) = self.ensure_pseudo_var_storage(&var);
                let address = DosMemory::absolute_address(segment, offset) as u32;
                let bytes = address.to_le_bytes();
                let text = bytes.iter().map(|byte| *byte as char).collect::<String>();
                self.runtime.push(QType::String(text));
            }

            // Graphics functions
            OpCode::PointFunc(x, y) => {
                // Get pixel color at coordinates
                let color = if let Some(ref gfx) = self.graphics {
                    gfx.get_pixel(x, y) as i16
                } else {
                    0
                };
                self.runtime.push(QType::Integer(color));
            }

            OpCode::PMapFunc(coord, func) => {
                // Map coordinates between physical and logical
                let result = if let Some(ref gfx) = self.graphics {
                    gfx.pmap(coord, func) as f32
                } else {
                    coord as f32
                };
                self.runtime.push(QType::Single(result));
            }

            // Advanced file I/O
            OpCode::FieldStmt { file_num, fields } => {
                self.runtime.random_fields.insert(
                    file_num,
                    fields
                        .into_iter()
                        .map(|(width, var_index)| (width.max(0) as usize, var_index))
                        .collect(),
                );
            }

            OpCode::LSetField { var_index, width } => {
                let value = self.runtime.pop()?;
                let mut text = match value {
                    QType::String(s) => s,
                    other => format!("{}", other),
                };
                if text.len() > width {
                    text.truncate(width);
                } else if text.len() < width {
                    text.push_str(&" ".repeat(width - text.len()));
                }
                if var_index < self.runtime.globals.len() {
                    self.runtime.globals[var_index] = QType::String(text);
                }
            }

            OpCode::RSetField { var_index, width } => {
                let value = self.runtime.pop()?;
                let text = match value {
                    QType::String(s) => s,
                    other => format!("{}", other),
                };
                let padded = if text.len() >= width {
                    text.chars()
                        .rev()
                        .take(width)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>()
                } else {
                    format!("{}{}", " ".repeat(width - text.len()), text)
                };
                if var_index < self.runtime.globals.len() {
                    self.runtime.globals[var_index] = QType::String(padded);
                }
            }

            // Graphics GET/PUT
            OpCode::GetImage {
                x1,
                y1,
                x2,
                y2,
                array,
            } => {
                // Capture screen region to array
                if let Some(ref gfx) = self.graphics {
                    let image_data = gfx.get_image(x1, y1, x2, y2);

                    let arr = self.runtime.arrays.entry(array.clone()).or_default();
                    arr.clear();

                    for byte in image_data {
                        arr.push(QType::Integer(byte as i16));
                    }

                    let width = if x1.abs() > x2.abs() {
                        x1.abs() - x2.abs() + 1
                    } else {
                        x2.abs() - x1.abs() + 1
                    };
                    let height = if y1.abs() > y2.abs() {
                        y1.abs() - y2.abs() + 1
                    } else {
                        y2.abs() - y1.abs() + 1
                    };

                    self.runtime
                        .array_dimensions
                        .insert(array.clone(), vec![(0, width * height + 4)]);
                }
            }

            OpCode::PutImage {
                x,
                y,
                array,
                action,
            } => {
                // Display image from array
                if let Some(ref mut gfx) = self.graphics {
                    if let Some(arr) = self.runtime.arrays.get(&array) {
                        let data: Vec<u8> = arr
                            .iter()
                            .filter_map(|v| match v {
                                QType::Integer(i) => Some(*i as u8),
                                _ => None,
                            })
                            .collect();

                        gfx.put_image(x, y, &data, &action);
                    }
                }
            }

            // DEF FN - Handled earlier

            // MID$ statement
            OpCode::MidAssign {
                var_name: _,
                start: _,
                length: _,
            } => {
                // Stack: [original_string, start, length, replacement]
                let replacement = self.runtime.pop()?;
                let length_val = self.runtime.pop()?;
                let start_val = self.runtime.pop()?;
                let original = self.runtime.pop()?;

                let start_idx = (start_val.to_f64() as i32 - 1).max(0) as usize;
                let len = length_val.to_f64() as usize;

                if let QType::String(mut original_str) = original {
                    if let QType::String(replacement_str) = replacement {
                        // Ensure string is long enough
                        while original_str.len() < start_idx {
                            original_str.push(' ');
                        }

                        let mut chars: Vec<char> = original_str.chars().collect();
                        let replacement_chars: Vec<char> =
                            replacement_str.chars().take(len).collect();

                        // Replace characters
                        for (i, &ch) in replacement_chars.iter().enumerate() {
                            let pos = start_idx + i;
                            if pos < chars.len() {
                                chars[pos] = ch;
                            } else {
                                chars.push(ch);
                            }
                        }

                        original_str = chars.into_iter().collect();

                        // Push modified string back to stack for StoreFast
                        self.runtime.push(QType::String(original_str));
                    } else {
                        // If replacement is not a string, just push original back
                        self.runtime.push(QType::String(original_str));
                    }
                } else {
                    // If original is not a string, push empty string
                    self.runtime.push(QType::String(String::new()));
                }
            }

            OpCode::Shell => {
                let command = self.runtime.pop()?;
                let cmd_str = if let QType::String(s) = command {
                    s
                } else {
                    format!("{}", command)
                };

                if cmd_str.is_empty() {
                    // Empty SHELL is treated as a quiet no-op in VM mode to avoid
                    // launching an interactive shell or leaking compatibility markers.
                } else {
                    // Execute command
                    use std::process::Command;

                    #[cfg(target_os = "windows")]
                    let output = Command::new("cmd").args(["/C", &cmd_str]).output();

                    #[cfg(not(target_os = "windows"))]
                    let output = Command::new("sh").args(&["-c", &cmd_str]).output();

                    match output {
                        Ok(output) => {
                            print!("{}", String::from_utf8_lossy(&output.stdout));
                            eprint!("{}", String::from_utf8_lossy(&output.stderr));
                            io::stdout().flush().ok();
                            io::stderr().flush().ok();
                        }
                        Err(e) => {
                            eprintln!("SHELL error: {}", e);
                        }
                    }
                }
            }

            OpCode::Chain => {
                let filename = self.runtime.pop()?;
                let file_str = if let QType::String(s) = filename {
                    s
                } else {
                    format!("{}", filename)
                };

                // CHAIN loads and runs another BASIC program
                // For now, we'll use std::process::Command to run qb.exe with the new file
                // In a real implementation, we would reload the program in the same VM

                use std::env;
                use std::process::Command;

                // Get the current executable path
                let current_exe =
                    env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("qb"));

                // Execute the new program
                let status = Command::new(current_exe).arg("-x").arg(&file_str).status();

                match status {
                    Ok(status) => {
                        // Exit with the same code as the chained program
                        if !status.success() {
                            std::process::exit(status.code().unwrap_or(1));
                        }
                        // Stop current program
                        self.running = false;
                    }
                    Err(e) => {
                        return Err(QError::FileNotFound(format!("CHAIN error: {}", e)));
                    }
                }
            }

            OpCode::KillFile => {
                let filename = self.runtime.pop()?;
                if let QType::String(path) = filename {
                    std::fs::remove_file(path).map_err(|e| QError::FileIO(e.to_string()))?;
                }
            }

            OpCode::RenameFile => {
                let new_name = self.runtime.pop()?;
                let old_name = self.runtime.pop()?;
                if let (QType::String(old_name), QType::String(new_name)) = (old_name, new_name) {
                    std::fs::rename(old_name, new_name)
                        .map_err(|e| QError::FileIO(e.to_string()))?;
                }
            }

            OpCode::ChangeDir => {
                let path = self.runtime.pop()?;
                if let QType::String(path) = path {
                    std::env::set_current_dir(path).map_err(|e| QError::FileIO(e.to_string()))?;
                }
            }

            OpCode::MakeDir => {
                let path = self.runtime.pop()?;
                if let QType::String(path) = path {
                    std::fs::create_dir_all(path).map_err(|e| QError::FileIO(e.to_string()))?;
                }
            }

            OpCode::RemoveDir => {
                let path = self.runtime.pop()?;
                if let QType::String(path) = path {
                    std::fs::remove_dir(path).map_err(|e| QError::FileIO(e.to_string()))?;
                }
            }

            OpCode::Files => {
                let pattern = self.runtime.pop()?;
                let pattern = match pattern {
                    QType::String(s) if !s.is_empty() => Some(s),
                    _ => None,
                };
                let entries = std::fs::read_dir(".").map_err(|e| QError::FileIO(e.to_string()))?;
                for entry in entries {
                    let entry = entry.map_err(|e| QError::FileIO(e.to_string()))?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    let matched = match pattern.as_deref() {
                        None | Some("*") => true,
                        Some(pat) => wildcard_matches(pat, &name),
                    };
                    if matched {
                        println!("{}", name);
                    }
                }
            }

            OpCode::ErrorStmt => {
                let error_code = self.runtime.pop()?;
                let code = error_code.to_f64() as i16;
                // Store the error code for ERR function
                self.runtime.last_error_code = code;
                // Trigger an error with the specified code
                return Err(QError::Runtime(format!("ERROR {}", code)));
            }

            OpCode::ResumeLabel(addr) => {
                // Resume at a specific label
                self.runtime.instruction_pointer = addr;
                self.runtime.last_error = None;
            }

            _ => {}
        }

        Ok(())
    }

    fn binary_op<F>(&self, left: &QType, right: &QType, op: F) -> QResult<QType>
    where
        F: Fn(f64, f64) -> f64,
    {
        let result = op(left.to_f64(), right.to_f64());
        Ok(QType::Double(result))
    }

    /// Format a value using PRINT USING format string
    fn format_using_value(&self, format: &str, value: &QType) -> QResult<String> {
        // Parse format string and apply formatting
        let chars: Vec<char> = format.chars().collect();
        let mut i = 0;
        let mut digit_positions = Vec::new();
        let mut has_decimal = false;
        let mut decimal_pos = 0;
        let mut has_comma = false;
        let mut leading_dollar = false;
        let mut leading_asterisk = false;
        let mut trailing_minus = false;
        let mut trailing_plus = false;

        // Scan format string
        while i < chars.len() {
            match chars[i] {
                '#' => digit_positions.push(i),
                '.' => {
                    has_decimal = true;
                    decimal_pos = i;
                }
                ',' => has_comma = true,
                '$' if i == 0 || (i > 0 && chars[i - 1] == '$') => leading_dollar = true,
                '*' if i == 0 || (i > 0 && chars[i - 1] == '*') => leading_asterisk = true,
                '-' if i == chars.len() - 1 => trailing_minus = true,
                '+' if i == chars.len() - 1 => trailing_plus = true,
                '!' => {
                    // String format: first character only
                    if let QType::String(s) = value {
                        return Ok(s.chars().next().unwrap_or(' ').to_string());
                    }
                    return Ok(" ".to_string());
                }
                '&' => {
                    // String format: whole string
                    return Ok(format!("{}", value));
                }
                '\\' => {
                    // Fixed width string format: \  \ (spaces between)
                    let mut width = 0;
                    let start = i;
                    i += 1;
                    while i < chars.len() && chars[i] == ' ' {
                        width += 1;
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '\\' {
                        width += 2; // Include both backslashes
                        if let QType::String(s) = value {
                            let truncated = if s.len() > width { &s[..width] } else { s };
                            return Ok(format!("{:<width$}", truncated, width = width));
                        }
                    }
                    i = start; // Reset if not valid format
                }
                _ => {}
            }
            i += 1;
        }

        // If no # found, just return the value as string
        if digit_positions.is_empty() {
            return Ok(format!("{}", value));
        }

        // Convert value to number
        let num = value.to_f64();
        let is_negative = num < 0.0;
        let abs_num = num.abs();

        // Count digits before and after decimal
        let digits_before = if has_decimal {
            digit_positions.iter().filter(|&&p| p < decimal_pos).count()
        } else {
            digit_positions.len()
        };

        let digits_after = if has_decimal {
            digit_positions.iter().filter(|&&p| p > decimal_pos).count()
        } else {
            0
        };

        // Format the number
        let formatted_num = if has_decimal {
            format!("{:.*}", digits_after, abs_num)
        } else {
            format!("{:.0}", abs_num)
        };

        // Add thousand separators if needed
        let formatted_num = if has_comma {
            let parts: Vec<&str> = formatted_num.split('.').collect();
            let int_part = parts[0];
            let mut result = String::new();
            let chars: Vec<char> = int_part.chars().collect();
            for (i, ch) in chars.iter().enumerate() {
                if i > 0 && (chars.len() - i).is_multiple_of(3) {
                    result.push(',');
                }
                result.push(*ch);
            }
            if parts.len() > 1 {
                result.push('.');
                result.push_str(parts[1]);
            }
            result
        } else {
            formatted_num
        };

        // Pad with leading characters
        let total_width = digit_positions.len()
            + if has_decimal { 1 } else { 0 }
            + if has_comma {
                (digits_before - 1) / 3
            } else {
                0
            };
        let mut result = if leading_asterisk {
            format!("{:*>width$}", formatted_num, width = total_width)
        } else if leading_dollar {
            format!("${:>width$}", formatted_num, width = total_width - 1)
        } else {
            format!("{:>width$}", formatted_num, width = total_width)
        };

        // Add sign
        if is_negative {
            if trailing_minus {
                result.push('-');
            } else {
                result.insert(0, '-');
            }
        } else if trailing_plus {
            result.push('+');
        }

        Ok(result)
    }
}

fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_ascii_uppercase();
    let text = text.to_ascii_uppercase();

    if pattern == "*" {
        return true;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }

    let mut remaining = text.as_str();
    let mut anchored_start = !pattern.starts_with('*');

    for part in parts.iter().filter(|part| !part.is_empty()) {
        if anchored_start {
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
            anchored_start = false;
            continue;
        }

        if let Some(idx) = remaining.find(part) {
            remaining = &remaining[idx + part.len()..];
        } else {
            return false;
        }
    }

    pattern.ends_with('*')
        || parts
            .last()
            .is_none_or(|part| part.is_empty() || text.ends_with(part))
}

impl Default for VM {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
