use crate::opcodes::{BinaryFileKind, ByRefTarget, OpCode};
use core_types::{DosMemory, QError, QResult, QType};
use std::collections::{HashMap, VecDeque};
use std::io::IsTerminal;
use std::io::{self, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn runtime_binary_bytes(s: &str) -> Vec<u8> {
    s.chars().map(|ch| ch as u32 as u8).collect()
}

fn runtime_binary_prefix<const N: usize>(s: &str) -> Option<[u8; N]> {
    let bytes = runtime_binary_bytes(s);
    if bytes.len() < N {
        return None;
    }
    let mut prefix = [0u8; N];
    prefix.copy_from_slice(&bytes[..N]);
    Some(prefix)
}

fn runtime_parse_number(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(0.0)
}

fn runtime_cv_i8(s: &str) -> i8 {
    runtime_binary_prefix::<1>(s)
        .map(|bytes| bytes[0] as i8)
        .unwrap_or_else(|| runtime_parse_number(s).round() as i8)
}

fn runtime_cv_u8(s: &str) -> u8 {
    runtime_binary_prefix::<1>(s)
        .map(|bytes| bytes[0])
        .unwrap_or_else(|| runtime_parse_number(s).round() as u8)
}

fn runtime_cv_i16(s: &str) -> i16 {
    runtime_binary_prefix::<2>(s)
        .map(i16::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s).round() as i16)
}

fn runtime_cv_u16(s: &str) -> u16 {
    runtime_binary_prefix::<2>(s)
        .map(u16::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s).round() as u16)
}

fn runtime_cv_i32(s: &str) -> i32 {
    runtime_binary_prefix::<4>(s)
        .map(i32::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s).round() as i32)
}

fn runtime_cv_u32(s: &str) -> u32 {
    runtime_binary_prefix::<4>(s)
        .map(u32::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s).round() as u32)
}

fn runtime_cv_f32(s: &str) -> f32 {
    runtime_binary_prefix::<4>(s)
        .map(f32::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s) as f32)
}

fn runtime_cv_f64(s: &str) -> f64 {
    runtime_binary_prefix::<8>(s)
        .map(f64::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s))
}

fn runtime_cv_i64(s: &str) -> i64 {
    runtime_binary_prefix::<8>(s)
        .map(i64::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s).round() as i64)
}

fn runtime_cv_u64(s: &str) -> u64 {
    runtime_binary_prefix::<8>(s)
        .map(u64::from_le_bytes)
        .unwrap_or_else(|| runtime_parse_number(s).round() as u64)
}

fn runtime_normalize_qb64_type_name(type_name: &str) -> String {
    type_name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

fn runtime_cv_to_qtype(type_name: &str, s: &str) -> QType {
    match runtime_normalize_qb64_type_name(type_name).as_str() {
        "_BYTE" | "BYTE" | "_BIT" | "BIT" => QType::Integer(runtime_cv_i8(s) as i16),
        "_UNSIGNED _BYTE" | "UNSIGNED _BYTE" | "_UNSIGNED BYTE" | "UNSIGNED BYTE"
        | "_UNSIGNED _BIT" | "UNSIGNED _BIT" | "_UNSIGNED BIT" | "UNSIGNED BIT" => {
            QType::Integer(runtime_cv_u8(s) as i16)
        }
        "INTEGER" => QType::Integer(runtime_cv_i16(s)),
        "_UNSIGNED INTEGER" | "UNSIGNED INTEGER" => QType::Long(runtime_cv_u16(s) as i32),
        "LONG" => QType::Long(runtime_cv_i32(s)),
        "_UNSIGNED LONG" | "UNSIGNED LONG" => QType::Double(runtime_cv_u32(s) as f64),
        "SINGLE" => QType::Single(runtime_cv_f32(s)),
        "DOUBLE" | "_FLOAT" | "FLOAT" => QType::Double(runtime_cv_f64(s)),
        "_INTEGER64" | "INTEGER64" | "_OFFSET" | "OFFSET" => {
            QType::Double(runtime_cv_i64(s) as f64)
        }
        "_UNSIGNED _INTEGER64"
        | "UNSIGNED _INTEGER64"
        | "_UNSIGNED INTEGER64"
        | "UNSIGNED INTEGER64"
        | "_UNSIGNED _OFFSET"
        | "UNSIGNED _OFFSET"
        | "_UNSIGNED OFFSET"
        | "UNSIGNED OFFSET" => QType::Double(runtime_cv_u64(s) as f64),
        _ => QType::Double(runtime_parse_number(s)),
    }
}

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

    pub fn reset_to_section(&mut self, section_index: usize) {
        self.section_index = section_index;
        self.value_index = 0;
    }
}

impl Default for DataPointer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayTrapState {
    Off,
    On,
    Stop,
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
    pub def_fns: HashMap<String, (Vec<usize>, Vec<OpCode>)>,
    pub error_handler_address: Option<usize>,
    pub error_resume_next: bool,
    pub last_error: Option<QError>,
    pub last_error_code: i16, // Store the actual error code for ERR function
    pub last_error_line: i16,
    pub for_loop_stack: Vec<ForLoopContext>,
    pub data_pointer: DataPointer,
    pub data_section: Vec<Vec<QType>>, // All DATA values organized by statement
    pub globals: Vec<QType>,
    pub timer_handler_address: Option<usize>,
    pub timer_interval_secs: f64,
    pub timer_enabled: bool,
    pub next_timer_tick: Option<Instant>,
    pub timer_handler_depth: Option<usize>,
    pub play_handler_address: Option<usize>,
    pub play_queue_limit: usize,
    pub play_trap_state: PlayTrapState,
    pub play_pending_event: bool,
    pub play_handler_depth: Option<usize>,
    pub play_note_deadlines: VecDeque<Instant>,
    pub procedure_frames: Vec<ProcedureFrame>,
    pub cursor_row: i16,
    pub cursor_col: i16,
    pub text_columns: i16,
    pub text_rows: i16,
    pub text_chars: Vec<u8>,
    pub text_attrs: Vec<u8>,
    pub text_foreground: u8,
    pub text_background: u8,
    pub current_line: i16,
    pub trace_enabled: bool,
    pub current_segment: u16,
    pub printer_col: i16,
    pub cursor_visible: bool,
    pub cursor_start: Option<i16>,
    pub cursor_stop: Option<i16>,
    pub key_enabled: bool,
    pub key_assignments: std::collections::BTreeMap<i16, String>,
    pub pseudo_var_offsets: HashMap<String, u16>,
    pub pseudo_var_sizes: HashMap<String, usize>,
    pub next_pseudo_offset: u16,
    pub view_print_top: Option<i16>,
    pub view_print_bottom: Option<i16>,
    pub pseudo_ports: HashMap<u16, u8>,
    pub string_widths: HashMap<usize, usize>,
    pub string_array_widths: HashMap<String, usize>,
    pub file_print_columns: HashMap<i32, i16>,
    pub pending_view_print_scroll: bool,
    pub noninteractive_inkey_emitted: bool,
    pub const_globals: HashMap<usize, QType>,
    pub rng_state: u32,
    pub last_random: Option<f32>,
    pub default_array_base: i32,
    pub captured_stdout: Option<String>,
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
            last_error_line: 0,
            for_loop_stack: Vec::with_capacity(16),
            data_pointer: DataPointer::new(),
            data_section: Vec::new(),
            globals: Vec::new(),
            timer_handler_address: None,
            timer_interval_secs: 0.0,
            timer_enabled: false,
            next_timer_tick: None,
            timer_handler_depth: None,
            play_handler_address: None,
            play_queue_limit: 1,
            play_trap_state: PlayTrapState::Off,
            play_pending_event: false,
            play_handler_depth: None,
            play_note_deadlines: VecDeque::with_capacity(32),
            procedure_frames: Vec::with_capacity(32),
            cursor_row: 1,
            cursor_col: 1,
            text_columns: 80,
            text_rows: 25,
            text_chars: vec![b' '; 80 * 25],
            text_attrs: vec![7; 80 * 25],
            text_foreground: 7,
            text_background: 0,
            current_line: 0,
            trace_enabled: false,
            current_segment: 0,
            printer_col: 1,
            cursor_visible: true,
            cursor_start: None,
            cursor_stop: None,
            key_enabled: false,
            key_assignments: std::collections::BTreeMap::new(),
            pseudo_var_offsets: HashMap::with_capacity(64),
            pseudo_var_sizes: HashMap::with_capacity(64),
            next_pseudo_offset: 1,
            view_print_top: None,
            view_print_bottom: None,
            pseudo_ports: HashMap::with_capacity(32),
            string_widths: HashMap::with_capacity(32),
            string_array_widths: HashMap::with_capacity(16),
            file_print_columns: HashMap::with_capacity(8),
            pending_view_print_scroll: false,
            noninteractive_inkey_emitted: false,
            const_globals: HashMap::with_capacity(16),
            rng_state: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs() as u32,
            last_random: None,
            default_array_base: 0,
            captured_stdout: None,
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

    pub fn begin_stdout_capture(&mut self) {
        self.captured_stdout = Some(String::new());
    }

    pub fn take_captured_stdout(&mut self) -> String {
        self.captured_stdout.take().unwrap_or_default()
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

    pub fn enable_stdout_capture(&mut self) {
        self.runtime.begin_stdout_capture();
    }

    pub fn take_captured_stdout(&mut self) -> String {
        self.runtime.take_captured_stdout()
    }

    fn write_stdout(&mut self, text: &str) {
        if let Some(captured) = &mut self.runtime.captured_stdout {
            captured.push_str(text);
        } else {
            print!("{}", text);
            io::stdout().flush().ok();
        }
    }

    fn poll_inkey_nonblocking(&mut self) -> String {
        if io::stdin().is_terminal() && io::stdout().is_terminal() {
            return qb_inkey_nonblocking();
        }

        if self.runtime.noninteractive_inkey_emitted {
            String::new()
        } else {
            self.runtime.noninteractive_inkey_emitted = true;
            "\r".to_string()
        }
    }

    fn wait_for_keypress(&mut self) {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return;
        }

        #[cfg(windows)]
        loop {
            if !self.poll_inkey_nonblocking().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        #[cfg(not(windows))]
        {
            // Non-Windows builds keep the key-wait path non-blocking until a
            // portable console polling implementation exists.
        }
    }

    fn normalize_global_value_for_slot(&self, slot: usize, value: QType) -> QType {
        if let Some(width) = self.runtime.string_widths.get(&slot).copied() {
            qb_normalize_fixed_string_value(width, value)
        } else {
            value
        }
    }

    fn normalize_array_value_for_name(&self, name: &str, value: QType) -> QType {
        if let Some(width) = self.runtime.string_array_widths.get(name).copied() {
            if width == 0 {
                match value {
                    QType::String(text) => QType::String(text),
                    other => QType::String(format!("{}", other)),
                }
            } else {
                qb_normalize_fixed_string_value(width, value)
            }
        } else if name.ends_with('$') {
            match value {
                QType::String(text) => QType::String(text),
                other => QType::String(format!("{}", other)),
            }
        } else {
            value
        }
    }

    fn default_array_value_for_name(&self, name: &str) -> QType {
        if let Some(width) = self.runtime.string_array_widths.get(name).copied() {
            if width == 0 {
                QType::String(String::new())
            } else {
                qb_normalize_fixed_string_value(width, QType::String(String::new()))
            }
        } else if name.ends_with('$') {
            QType::String(String::new())
        } else {
            QType::Integer(0)
        }
    }

    fn pop_i32_rounded(&mut self) -> QResult<i32> {
        Ok(self.runtime.pop()?.to_f64().round() as i32)
    }

    fn pop_f64(&mut self) -> QResult<f64> {
        Ok(self.runtime.pop()?.to_f64())
    }

    fn pop_string_value(&mut self) -> QResult<String> {
        Ok(match self.runtime.pop()? {
            QType::String(value) => value,
            other => format!("{}", other),
        })
    }

    fn binary_file_offset(&self, file_num: u8, record_num: u64) -> QResult<u64> {
        if record_num == 0 {
            self.file_io.tell(file_num)
        } else {
            Ok(record_num.saturating_sub(1))
        }
    }

    fn read_binary_bytes(
        &mut self,
        file_num: u8,
        record_num: u64,
        size: usize,
    ) -> QResult<Vec<u8>> {
        let offset = self.binary_file_offset(file_num, record_num)?;
        self.file_io.seek(file_num, offset)?;
        let mut bytes = self.file_io.read_bytes(file_num, size)?;
        if bytes.len() < size {
            bytes.resize(size, 0);
        }
        Ok(bytes)
    }

    fn write_binary_bytes(&mut self, file_num: u8, record_num: u64, bytes: &[u8]) -> QResult<()> {
        let offset = self.binary_file_offset(file_num, record_num)?;
        self.file_io.seek(file_num, offset)?;
        self.file_io.write_bytes(file_num, bytes)
    }

    fn read_binary_value(
        &mut self,
        file_num: u8,
        record_num: u64,
        kind: BinaryFileKind,
        fixed_length: usize,
    ) -> QResult<QType> {
        Ok(match kind {
            BinaryFileKind::Integer => {
                let bytes = self.read_binary_bytes(file_num, record_num, 2)?;
                QType::Integer(i16::from_le_bytes([bytes[0], bytes[1]]))
            }
            BinaryFileKind::Long => {
                let bytes = self.read_binary_bytes(file_num, record_num, 4)?;
                QType::Long(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            }
            BinaryFileKind::Single => {
                let bytes = self.read_binary_bytes(file_num, record_num, 4)?;
                QType::Single(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            }
            BinaryFileKind::Double => {
                let bytes = self.read_binary_bytes(file_num, record_num, 8)?;
                QType::Double(f64::from_le_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]))
            }
            BinaryFileKind::String => {
                let bytes = if fixed_length > 0 {
                    self.read_binary_bytes(file_num, record_num, fixed_length)?
                } else {
                    let offset = self.binary_file_offset(file_num, record_num)?;
                    let length = self.file_io.length_by_num(file_num as i32) as u64;
                    let remaining = length.saturating_sub(offset) as usize;
                    self.read_binary_bytes(file_num, record_num, remaining)?
                };
                QType::String(String::from_utf8_lossy(&bytes).to_string())
            }
        })
    }

    fn write_binary_value(
        &mut self,
        file_num: u8,
        record_num: u64,
        kind: BinaryFileKind,
        fixed_length: usize,
        value: QType,
    ) -> QResult<()> {
        match kind {
            BinaryFileKind::Integer => {
                let value = value.to_f64().round() as i16;
                self.write_binary_bytes(file_num, record_num, &value.to_le_bytes())
            }
            BinaryFileKind::Long => {
                let value = value.to_f64().round() as i32;
                self.write_binary_bytes(file_num, record_num, &value.to_le_bytes())
            }
            BinaryFileKind::Single => {
                let value = value.to_f64() as f32;
                self.write_binary_bytes(file_num, record_num, &value.to_le_bytes())
            }
            BinaryFileKind::Double => {
                let value = value.to_f64();
                self.write_binary_bytes(file_num, record_num, &value.to_le_bytes())
            }
            BinaryFileKind::String => {
                let mut text = match value {
                    QType::String(text) => text,
                    other => other.to_string(),
                };
                if fixed_length > 0 {
                    if text.len() > fixed_length {
                        text.truncate(fixed_length);
                    } else if text.len() < fixed_length {
                        text.push_str(&" ".repeat(fixed_length - text.len()));
                    }
                }
                self.write_binary_bytes(file_num, record_num, text.as_bytes())
            }
        }
    }

    fn pop_array_dimensions(&mut self, count: usize) -> QResult<Vec<(i32, i32)>> {
        let mut dimensions = Vec::with_capacity(count);
        for _ in 0..count {
            let upper = self.pop_i32_rounded()?;
            let lower = self.pop_i32_rounded()?;
            dimensions.push((lower, upper));
        }
        dimensions.reverse();
        Ok(dimensions)
    }

    fn validate_array_dimensions(dimensions: &[(i32, i32)]) -> QResult<usize> {
        let mut total_size: usize = 1;
        for (lower, upper) in dimensions {
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

        Ok(total_size)
    }

    fn store_array_dimensions(
        &mut self,
        name: String,
        dimensions: Vec<(i32, i32)>,
        preserve: bool,
    ) -> QResult<()> {
        let total_size = Self::validate_array_dimensions(&dimensions)?;
        let default_value = self.default_array_value_for_name(&name);

        if preserve {
            if let Some(old_array) = self.runtime.arrays.get(&name) {
                let mut new_array = Vec::with_capacity(total_size);
                new_array.resize(total_size, default_value.clone());
                let copy_size = old_array.len().min(total_size);
                new_array[..copy_size].clone_from_slice(&old_array[..copy_size]);
                self.runtime.arrays.insert(name.clone(), new_array);
            } else {
                let mut array = Vec::with_capacity(total_size);
                array.resize(total_size, default_value.clone());
                self.runtime.arrays.insert(name.clone(), array);
            }
        } else {
            let mut array = Vec::with_capacity(total_size);
            array.resize(total_size, default_value);
            self.runtime.arrays.insert(name.clone(), array);
        }

        self.runtime.array_dimensions.insert(name, dimensions);
        Ok(())
    }

    fn create_implicit_array_dimensions(&self, indices: &[i32]) -> QResult<Vec<(i32, i32)>> {
        let lower = self.runtime.default_array_base;
        let mut dimensions = Vec::with_capacity(indices.len());
        for idx in indices {
            let upper = (*idx + 5).max(lower + 5).min(lower + 100);
            dimensions.push((lower, upper));
        }
        Self::validate_array_dimensions(&dimensions)?;
        Ok(dimensions)
    }

    fn linear_array_index(&self, dimensions: &[(i32, i32)], indices: &[i32]) -> Option<usize> {
        if dimensions.len() != indices.len() {
            return None;
        }

        let mut linear_index = 0usize;
        let mut multiplier = 1usize;

        for ((lower, upper), idx) in dimensions.iter().zip(indices.iter()).rev() {
            if idx < lower || idx > upper {
                return None;
            }

            linear_index += (*idx - *lower) as usize * multiplier;
            multiplier = multiplier.checked_mul((*upper - *lower + 1) as usize)?;
        }

        Some(linear_index)
    }

    fn read_input_chars(&mut self, count: usize, file_number: Option<i32>) -> QResult<String> {
        if let Some(file_number) = file_number {
            let bytes = self.file_io.read_bytes(file_number as u8, count)?;
            return Ok(String::from_utf8_lossy(&bytes).to_string());
        }

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| QError::Internal(e.to_string()))?;
        Ok(input.chars().take(count).collect())
    }

    fn emit_terminal_bell(&mut self) {
        self.write_stdout("\x07");
    }

    fn sleep_sound_ticks(&self, duration_ticks: i32) {
        if duration_ticks > 0 {
            std::thread::sleep(Duration::from_secs_f64(duration_ticks as f64 / 18.2));
        }
    }

    fn render_sound_notes(&mut self, notes: &[hal_layer::sound_synth::Note]) {
        if notes.iter().any(|note| note.frequency > 0.0) {
            self.emit_terminal_bell();
        }

        let total_ms: u64 = notes.iter().map(|note| note.duration_ms as u64).sum();
        if total_ms > 0 {
            std::thread::sleep(Duration::from_millis(total_ms));
        }
    }

    fn queue_background_sound_notes(&mut self, notes: &[hal_layer::sound_synth::Note]) {
        self.update_play_queue();
        if notes.iter().any(|note| note.frequency > 0.0) {
            self.emit_terminal_bell();
        }

        let now = Instant::now();
        let mut next_deadline = self
            .runtime
            .play_note_deadlines
            .back()
            .copied()
            .unwrap_or(now)
            .max(now);
        for note in notes {
            next_deadline += Duration::from_millis(note.duration_ms as u64);
            self.runtime.play_note_deadlines.push_back(next_deadline);
        }
    }

    fn current_play_queue_count(&mut self) -> i16 {
        if self.runtime.play_handler_depth.is_none() {
            self.update_play_queue();
        }
        self.runtime
            .play_note_deadlines
            .len()
            .min(i16::MAX as usize) as i16
    }

    fn qbasic_str(value: &QType) -> String {
        let mut text = format!("{}", value);
        if value.to_f64() >= 0.0 {
            text.insert(0, ' ');
        }
        text
    }

    fn qbasic_val(text: &str) -> f64 {
        let trimmed = text.trim_start();
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
            .parse::<f64>()
            .unwrap_or(0.0)
    }

    fn qbasic_string(count: usize, value: QType) -> String {
        let ch = match value {
            QType::String(text) => text.chars().next().unwrap_or(' '),
            other => char::from_u32(other.to_f64() as u32).unwrap_or('\0'),
        };
        ch.to_string().repeat(count)
    }

    fn seed_rng_with_value(&mut self, seed: f64) {
        self.runtime.rng_state = (seed as u32).wrapping_add(1);
        self.runtime.last_random = None;
    }

    fn randomize_rng(&mut self) {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs_f64();
        self.seed_rng_with_value(seed);
    }

    fn next_random_value(&mut self) -> f32 {
        self.runtime.rng_state = self
            .runtime
            .rng_state
            .wrapping_mul(1103515245)
            .wrapping_add(12345);
        let random = ((self.runtime.rng_state / 65536) % 32768) as f32 / 32768.0;
        self.runtime.last_random = Some(random);
        random
    }

    fn rnd_value(&mut self, arg: Option<f64>) -> f32 {
        match arg {
            Some(value) if value < 0.0 => {
                self.seed_rng_with_value(value);
                self.next_random_value()
            }
            Some(value) if value == 0.0 => self
                .runtime
                .last_random
                .unwrap_or_else(|| self.next_random_value()),
            _ => self.next_random_value(),
        }
    }

    fn normalize_color(value: i32, default: u8) -> u8 {
        if value < 0 {
            default
        } else {
            value.clamp(0, u8::MAX as i32) as u8
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
            self.update_play_queue();
            self.maybe_fire_play_event();
            self.maybe_fire_timer();
            let prev_ip = self.runtime.instruction_pointer;
            if let Err(e) = self.execute_next() {
                self.runtime.last_error = Some(e.clone());
                self.runtime.last_error_line = self.runtime.current_line;
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

    fn update_play_queue(&mut self) {
        if self.runtime.play_pending_event || self.runtime.play_handler_depth.is_some() {
            return;
        }
        let now = Instant::now();
        while matches!(self.runtime.play_note_deadlines.front(), Some(deadline) if *deadline <= now)
        {
            let previous = self.runtime.play_note_deadlines.len();
            self.runtime.play_note_deadlines.pop_front();
            let current = self.runtime.play_note_deadlines.len();
            if previous == self.runtime.play_queue_limit && current + 1 == previous {
                match self.runtime.play_trap_state {
                    PlayTrapState::Off => self.runtime.play_pending_event = false,
                    PlayTrapState::On | PlayTrapState::Stop => {
                        self.runtime.play_pending_event = true;
                        break;
                    }
                }
            }
        }
    }

    fn maybe_fire_play_event(&mut self) {
        if self.runtime.play_handler_depth.is_some()
            || !self.runtime.play_pending_event
            || self.runtime.play_trap_state != PlayTrapState::On
        {
            return;
        }

        let Some(handler) = self.runtime.play_handler_address else {
            self.runtime.play_pending_event = false;
            return;
        };

        let return_depth = self.runtime.call_stack.len();
        self.runtime
            .call_stack
            .push(self.runtime.instruction_pointer);
        self.runtime.play_handler_depth = Some(return_depth);
        self.runtime.play_pending_event = false;
        self.runtime.play_trap_state = PlayTrapState::Stop;
        self.runtime.instruction_pointer = handler;
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
                                if width == 0 {
                                    match value {
                                        QType::String(text) => QType::String(text),
                                        other => QType::String(format!("{}", other)),
                                    }
                                } else {
                                    qb_normalize_fixed_string_value(width, value)
                                }
                            } else {
                                value
                            };
                        }
                    }
                }
            }
        }
    }

    fn active_text_rows(&self) -> i16 {
        self.runtime.text_rows.max(1)
    }

    fn active_text_columns(&self) -> i16 {
        self.runtime.text_columns.max(1)
    }

    fn active_view_print_top(&self) -> i16 {
        self.runtime
            .view_print_top
            .unwrap_or(1)
            .clamp(1, self.active_text_rows())
    }

    fn active_view_print_bottom(&self) -> i16 {
        let top = self.active_view_print_top();
        self.runtime
            .view_print_bottom
            .unwrap_or(self.active_text_rows())
            .clamp(top, self.active_text_rows())
    }

    fn ensure_cursor_in_text_window(&mut self) {
        let text_rows = self.active_text_rows();
        let text_columns = self.active_text_columns();
        let top = self.active_view_print_top();
        let bottom = self.active_view_print_bottom();

        if self.runtime.view_print_top.is_some()
            && (self.runtime.cursor_row < top || self.runtime.cursor_row > bottom)
        {
            self.runtime.cursor_row = top;
            self.runtime.cursor_col = 1;
        } else {
            self.runtime.cursor_row = self.runtime.cursor_row.clamp(1, text_rows);
            self.runtime.cursor_col = self.runtime.cursor_col.clamp(1, text_columns);
        }
    }

    fn current_text_attr(&self) -> u8 {
        self.runtime
            .text_foreground
            .saturating_add(self.runtime.text_background.saturating_mul(16))
    }

    fn resize_text_buffer(&mut self, columns: i16, rows: i16, preserve_contents: bool) {
        let columns = columns.max(1) as usize;
        let rows = rows.max(1) as usize;
        let mut chars = vec![b' '; columns * rows];
        let mut attrs = vec![self.current_text_attr(); columns * rows];

        if preserve_contents {
            let old_columns = self.runtime.text_columns.max(1) as usize;
            let old_rows = self.runtime.text_rows.max(1) as usize;
            let copy_rows = old_rows.min(rows);
            let copy_cols = old_columns.min(columns);
            for row in 0..copy_rows {
                let old_start = row * old_columns;
                let new_start = row * columns;
                chars[new_start..new_start + copy_cols]
                    .copy_from_slice(&self.runtime.text_chars[old_start..old_start + copy_cols]);
                attrs[new_start..new_start + copy_cols]
                    .copy_from_slice(&self.runtime.text_attrs[old_start..old_start + copy_cols]);
            }
        }

        self.runtime.text_chars = chars;
        self.runtime.text_attrs = attrs;
    }

    fn text_buffer_index(&self, row: i16, col: i16) -> Option<usize> {
        let row = row.max(1).min(self.active_text_rows()) as usize - 1;
        let col = col.max(1).min(self.active_text_columns()) as usize - 1;
        let width = self.active_text_columns() as usize;
        let idx = row * width + col;
        (idx < self.runtime.text_chars.len()).then_some(idx)
    }

    fn put_text_cell(&mut self, row: i16, col: i16, ch: u8) {
        if let Some(idx) = self.text_buffer_index(row, col) {
            self.runtime.text_chars[idx] = ch;
            self.runtime.text_attrs[idx] = self.current_text_attr();
        }
    }

    fn clear_text_rows(&mut self, top: i16, bottom: i16) {
        let width = self.active_text_columns() as usize;
        let top = top.clamp(1, self.active_text_rows()) as usize;
        let bottom = bottom.clamp(top as i16, self.active_text_rows()) as usize;
        let attr = self.current_text_attr();
        for row in top..=bottom {
            let start = (row - 1) * width;
            let end = start + width;
            self.runtime.text_chars[start..end].fill(b' ');
            self.runtime.text_attrs[start..end].fill(attr);
        }
    }

    fn scroll_text_rows_up(&mut self, top: i16, bottom: i16) {
        let width = self.active_text_columns() as usize;
        let top = top.clamp(1, self.active_text_rows()) as usize;
        let bottom = bottom.clamp(top as i16, self.active_text_rows()) as usize;
        if top >= bottom {
            self.clear_text_rows(top as i16, bottom as i16);
            return;
        }

        for row in top..bottom {
            let dst_start = (row - 1) * width;
            let src_start = row * width;
            let src_end = src_start + width;
            self.runtime
                .text_chars
                .copy_within(src_start..src_end, dst_start);
            self.runtime
                .text_attrs
                .copy_within(src_start..src_end, dst_start);
        }

        self.clear_text_rows(bottom as i16, bottom as i16);
    }

    fn screen_fn_value(&self, row: i32, col: i32, color_flag: i32) -> i16 {
        let row = row.clamp(1, self.active_text_rows() as i32) as i16;
        let col = col.clamp(1, self.active_text_columns() as i32) as i16;
        if let Some(idx) = self.text_buffer_index(row, col) {
            if color_flag != 0 {
                self.runtime.text_attrs[idx] as i16
            } else {
                self.runtime.text_chars[idx] as i16
            }
        } else {
            0
        }
    }

    fn consume_pending_view_print_scroll(&mut self) {
        if self.runtime.pending_view_print_scroll {
            let (top, bottom) = if self.runtime.view_print_top.is_some() {
                (
                    self.active_view_print_top(),
                    self.active_view_print_bottom(),
                )
            } else {
                (1, self.active_text_rows())
            };
            self.scroll_text_rows_up(top, bottom);
            self.runtime.cursor_row = bottom;
            self.runtime.cursor_col = 1;
            self.runtime.pending_view_print_scroll = false;
        }
    }

    fn advance_console_line(&mut self) {
        let next_row = if self.runtime.view_print_top.is_some() {
            let top = self.active_view_print_top();
            let bottom = self.active_view_print_bottom();
            let current = self.runtime.cursor_row.clamp(top, bottom);
            if current < bottom {
                self.runtime.pending_view_print_scroll = false;
                current + 1
            } else {
                self.runtime.pending_view_print_scroll = true;
                bottom
            }
        } else if self.runtime.cursor_row < self.active_text_rows() {
            self.runtime.pending_view_print_scroll = false;
            self.runtime.cursor_row + 1
        } else {
            self.runtime.pending_view_print_scroll = true;
            self.active_text_rows()
        };
        self.runtime.cursor_row = next_row;
        self.runtime.cursor_col = 1;
    }

    fn emit_console_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        self.ensure_cursor_in_text_window();
        let columns = self.active_text_columns();
        let mut rendered = String::with_capacity(text.len());
        for ch in text.chars() {
            self.consume_pending_view_print_scroll();
            match ch {
                '\n' => {
                    rendered.push('\n');
                    self.advance_console_line();
                }
                '\r' => {
                    rendered.push('\r');
                    self.runtime.cursor_col = 1;
                }
                _ => {
                    rendered.push(ch);
                    self.put_text_cell(self.runtime.cursor_row, self.runtime.cursor_col, ch as u8);
                    self.runtime.cursor_col = self.runtime.cursor_col.saturating_add(1);
                    if self.runtime.cursor_col > columns {
                        rendered.push('\n');
                        self.advance_console_line();
                    }
                }
            }
        }

        self.write_stdout(&rendered);
    }

    fn emit_spaces(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.emit_console_text(&" ".repeat(count));
    }

    fn print_tab_to(&mut self, target_col: i16) {
        self.ensure_cursor_in_text_window();
        let target_col = target_col.max(1).min(self.active_text_columns());
        if target_col <= self.runtime.cursor_col {
            self.emit_console_text("\n");
        }

        let spaces = target_col.saturating_sub(self.runtime.cursor_col) as usize;
        self.emit_spaces(spaces);
    }

    fn next_print_zone(current_col: i16, line_width: i16) -> Option<i16> {
        const PRINT_ZONE_WIDTH: i16 = 14;

        let normalized = current_col.max(1);
        let target = ((normalized - 1) / PRINT_ZONE_WIDTH + 1) * PRINT_ZONE_WIDTH + 1;
        if target > line_width.max(1) {
            None
        } else {
            Some(target)
        }
    }

    fn print_comma(&mut self) {
        self.ensure_cursor_in_text_window();
        if let Some(target) =
            Self::next_print_zone(self.runtime.cursor_col, self.active_text_columns())
        {
            self.print_tab_to(target);
        } else {
            self.emit_console_text("\n");
        }
    }

    fn advance_file_print_column(&mut self, file_num: i32, text: &str) {
        let column = self.runtime.file_print_columns.entry(file_num).or_insert(1);
        for ch in text.chars() {
            match ch {
                '\n' | '\r' => *column = 1,
                _ => *column = column.saturating_add(1),
            }
        }
    }

    fn file_print_write(&mut self, file_num: i32, text: &str) -> QResult<()> {
        self.file_io.write_by_num(file_num, text)?;
        self.advance_file_print_column(file_num, text);
        Ok(())
    }

    fn file_print_newline(&mut self, file_num: i32) -> QResult<()> {
        self.file_print_write(file_num, "\n")?;
        self.runtime.file_print_columns.insert(file_num, 1);
        Ok(())
    }

    fn file_print_spaces(&mut self, file_num: i32, count: usize) -> QResult<()> {
        if count == 0 {
            return Ok(());
        }
        self.file_print_write(file_num, &" ".repeat(count))
    }

    fn file_print_comma(&mut self, file_num: i32) -> QResult<()> {
        let current_col = *self.runtime.file_print_columns.get(&file_num).unwrap_or(&1);
        if let Some(target) = Self::next_print_zone(current_col, 80) {
            let spaces = target.saturating_sub(current_col) as usize;
            self.file_print_spaces(file_num, spaces)
        } else {
            self.file_print_newline(file_num)
        }
    }

    fn pop_using_operands(&mut self, count: usize) -> QResult<(String, Vec<QType>)> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.runtime.pop()?);
        }
        values.reverse();
        let format = Self::coerce_using_format(self.runtime.pop()?);
        Ok((format, values))
    }

    fn coerce_using_format(format: QType) -> String {
        if let QType::String(text) = format {
            text
        } else {
            format!("{}", format)
        }
    }

    fn emit_print_using_values(
        &mut self,
        format: &str,
        values: &[QType],
        comma_after: &[bool],
    ) -> QResult<()> {
        let chunks = qb_format_using_chunks(format, values)?;
        for (index, formatted) in chunks.iter().enumerate() {
            self.emit_console_text(&formatted);
            if comma_after.get(index).copied().unwrap_or(false) {
                self.print_comma();
            }
        }
        Ok(())
    }

    fn emit_lprint_using_values(
        &mut self,
        format: &str,
        values: &[QType],
        comma_after: &[bool],
    ) -> QResult<()> {
        let chunks = qb_format_using_chunks(format, values)?;
        for (index, formatted) in chunks.iter().enumerate() {
            self.emit_lprint_text(&formatted);
            if comma_after.get(index).copied().unwrap_or(false) {
                self.lprint_comma();
            }
        }
        Ok(())
    }

    fn emit_file_print_using_values(
        &mut self,
        file_num: i32,
        format: &str,
        values: &[QType],
        comma_after: &[bool],
    ) -> QResult<()> {
        let chunks = qb_format_using_chunks(format, values)?;
        for (index, formatted) in chunks.iter().enumerate() {
            self.file_print_write(file_num, &formatted)?;
            if comma_after.get(index).copied().unwrap_or(false) {
                self.file_print_comma(file_num)?;
            }
        }
        Ok(())
    }

    fn format_write_value(value: QType) -> String {
        match value {
            QType::String(text) => format!("\"{}\"", text),
            QType::Integer(value) => value.to_string(),
            QType::Long(value) => value.to_string(),
            QType::Single(value) => {
                if value.fract().abs() < f32::EPSILON {
                    (value as i64).to_string()
                } else {
                    value.to_string()
                }
            }
            QType::Double(value) => {
                if value.fract().abs() < f64::EPSILON {
                    (value as i64).to_string()
                } else {
                    value.to_string()
                }
            }
            QType::UserDefined(_) => "[UserDefined]".to_string(),
            QType::Empty => String::new(),
        }
    }

    fn write_record(values: Vec<QType>) -> String {
        values
            .into_iter()
            .map(Self::format_write_value)
            .collect::<Vec<_>>()
            .join(",")
    }

    fn sorted_env_entries() -> Vec<String> {
        let mut entries = std::env::vars()
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    fn set_text_width(&mut self, columns: i32, rows: i32) {
        let columns = columns.max(1).min(i16::MAX as i32) as i16;
        let rows = rows.max(1).min(i16::MAX as i32) as i16;
        self.resize_text_buffer(columns, rows, true);
        self.runtime.text_columns = columns;
        self.runtime.text_rows = rows;
        if let Some(top) = self.runtime.view_print_top {
            self.runtime.view_print_top = Some(top.clamp(1, self.runtime.text_rows));
        }
        if let Some(bottom) = self.runtime.view_print_bottom {
            let top = self.active_view_print_top();
            self.runtime.view_print_bottom = Some(bottom.clamp(top, self.runtime.text_rows));
        }

        self.runtime.pending_view_print_scroll = false;
        self.ensure_cursor_in_text_window();
    }

    fn default_text_geometry_for_screen_mode(mode: i32) -> (i32, i32) {
        match mode {
            1 | 7 | 13 => (40, 25),
            11 | 12 => (80, 30),
            _ => (80, 25),
        }
    }

    fn apply_screen_mode_text_state(&mut self, mode: i32) {
        let (columns, rows) = Self::default_text_geometry_for_screen_mode(mode);
        self.resize_text_buffer(columns as i16, rows as i16, false);
        self.runtime.text_columns = columns as i16;
        self.runtime.text_rows = rows as i16;
        self.runtime.view_print_top = None;
        self.runtime.view_print_bottom = None;
        self.runtime.pending_view_print_scroll = false;
        self.runtime.cursor_row = 1;
        self.runtime.cursor_col = 1;
    }

    fn set_view_print_region(&mut self, top: i32, bottom: i32) {
        let rows = self.active_text_rows() as i32;
        let top = top.clamp(1, rows) as i16;
        let bottom = bottom.clamp(top as i32, rows) as i16;
        self.runtime.view_print_top = Some(top);
        self.runtime.view_print_bottom = Some(bottom);
        self.runtime.pending_view_print_scroll = false;
        self.runtime.cursor_row = top;
        self.runtime.cursor_col = 1;
    }

    fn reset_view_print_region(&mut self) {
        self.runtime.view_print_top = None;
        self.runtime.view_print_bottom = None;
        self.runtime.pending_view_print_scroll = false;
        self.runtime.cursor_row = self.runtime.cursor_row.clamp(1, self.active_text_rows());
        self.runtime.cursor_col = self.runtime.cursor_col.clamp(1, self.active_text_columns());
    }

    fn locate_cursor(&mut self, row: i32, col: i32) -> bool {
        self.ensure_cursor_in_text_window();
        let previous = (self.runtime.cursor_row, self.runtime.cursor_col);
        let row = if row == 0 {
            self.runtime.cursor_row as i32
        } else {
            row.max(1).min(self.active_text_rows() as i32)
        } as i16;
        let col = if col == 0 {
            self.runtime.cursor_col as i32
        } else {
            col.max(1).min(self.active_text_columns() as i32)
        } as i16;
        if self.runtime.view_print_top.is_some() {
            self.runtime.cursor_row = row.clamp(
                self.active_view_print_top(),
                self.active_view_print_bottom(),
            );
        } else {
            self.runtime.cursor_row = row;
        }
        self.runtime.pending_view_print_scroll = false;
        self.runtime.cursor_col = col;
        previous != (self.runtime.cursor_row, self.runtime.cursor_col)
    }

    fn set_cursor_state(&mut self, visible: i32, start: i32, stop: i32) {
        if visible >= 0 {
            let visible = visible != 0;
            if self.runtime.cursor_visible != visible {
                self.write_stdout(if visible { "\x1B[?25h" } else { "\x1B[?25l" });
            }
            self.runtime.cursor_visible = visible;
        }

        if start >= 0 {
            self.runtime.cursor_start = Some(start.clamp(0, i16::MAX as i32) as i16);
        }

        if stop >= 0 {
            self.runtime.cursor_stop = Some(stop.clamp(0, i16::MAX as i32) as i16);
        }
    }

    fn has_active_graphics_viewport(&self) -> bool {
        self.graphics
            .as_ref()
            .map(|gfx| gfx.viewport.active)
            .unwrap_or(false)
    }

    fn clear_text_for_cls(&mut self, full_screen: bool) {
        let (top, bottom) = if full_screen {
            (1, self.active_text_rows())
        } else {
            (
                self.active_view_print_top(),
                self.active_view_print_bottom(),
            )
        };
        self.clear_text_rows(top, bottom);
        self.runtime.pending_view_print_scroll = false;
        self.runtime.cursor_row = top;
        self.runtime.cursor_col = 1;
    }

    fn clear_graphics_for_cls(&mut self, full_screen: bool) {
        if let Some(ref mut gfx) = self.graphics {
            if full_screen {
                gfx.clear_graphics_screen(0);
            } else {
                gfx.clear_graphics_viewport(0);
            }
        }
    }

    fn execute_cls_mode(&mut self, mode: i32) {
        match mode {
            0 => {
                self.clear_graphics_for_cls(true);
                self.clear_text_for_cls(true);
            }
            1 => {
                if self.has_active_graphics_viewport() {
                    self.clear_graphics_for_cls(false);
                } else {
                    self.clear_graphics_for_cls(true);
                    self.clear_text_for_cls(true);
                }
            }
            2 => {
                self.clear_text_for_cls(false);
            }
            _ => {
                if self.has_active_graphics_viewport() {
                    self.clear_graphics_for_cls(false);
                } else {
                    self.clear_text_for_cls(false);
                }
            }
        }
    }

    fn set_current_line(&mut self, line: u16) {
        self.runtime.current_line = line as i16;
        if self.runtime.trace_enabled {
            self.emit_console_text(&format!("[{}]\n", line));
        }
    }

    fn format_key_assignment(text: &str) -> String {
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

    fn list_keys(&mut self) {
        if !self.runtime.key_enabled {
            return;
        }
        let lines = self
            .runtime
            .key_assignments
            .iter()
            .map(|(key, value)| format!("F{} {}\n", key, Self::format_key_assignment(value)))
            .collect::<Vec<_>>();
        for line in lines {
            self.write_stdout(&line);
            self.runtime.cursor_row = self.runtime.cursor_row.saturating_add(1);
            self.runtime.cursor_col = 1;
        }
    }

    fn emit_lprint_text(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\n' | '\r' => self.runtime.printer_col = 1,
                _ => self.runtime.printer_col = self.runtime.printer_col.saturating_add(1),
            }
        }
    }

    fn lprint_newline(&mut self) {
        self.runtime.printer_col = 1;
    }

    fn lprint_tab_to(&mut self, target: i16) {
        self.runtime.printer_col = target.max(1);
    }

    fn lprint_space(&mut self, count: usize) {
        self.runtime.printer_col = self
            .runtime
            .printer_col
            .saturating_add(count.min(i16::MAX as usize) as i16);
    }

    fn lprint_comma(&mut self) {
        if let Some(target) = Self::next_print_zone(self.runtime.printer_col, 80) {
            self.lprint_tab_to(target);
        } else {
            self.lprint_newline();
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
            .map_or(true, |size| *size < needed);

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

            OpCode::SetOptionBase(base) => {
                self.runtime.default_array_base = base;
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
                        *value = if width == 0 {
                            match current {
                                QType::String(text) => QType::String(text),
                                other => QType::String(format!("{}", other)),
                            }
                        } else {
                            qb_normalize_fixed_string_value(width, current)
                        };
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
                let equal = match (&left, &right) {
                    (QType::String(a), QType::String(b)) => a == b,
                    (QType::UserDefined(a), QType::UserDefined(b)) => a == b,
                    _ => (left.to_f64() - right.to_f64()).abs() < f64::EPSILON,
                };
                let result = QType::Integer(if equal { -1 } else { 0 });
                self.runtime.push(result);
            }

            OpCode::NotEqual => {
                let right = self.runtime.pop()?;
                let left = self.runtime.pop()?;
                let equal = match (&left, &right) {
                    (QType::String(a), QType::String(b)) => a == b,
                    (QType::UserDefined(a), QType::UserDefined(b)) => a == b,
                    _ => (left.to_f64() - right.to_f64()).abs() < f64::EPSILON,
                };
                let result = QType::Integer(if !equal { -1 } else { 0 });
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
                let target = self.runtime.pop()?.to_f64().round() as i16;
                self.print_tab_to(target);
            }

            OpCode::PrintSpace => {
                let count = self.runtime.pop()?.to_f64().round().max(0.0) as usize;
                self.emit_spaces(count);
            }

            OpCode::Input => {
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| QError::Internal(e.to_string()))?;
                let input = input.trim_end_matches('\n').trim_end_matches('\r');
                self.runtime.push(QType::String(input.to_string()));
            }

            OpCode::InputChars { has_file_number } => {
                let file_number = if has_file_number {
                    Some(self.runtime.pop()?.to_f64().round() as i32)
                } else {
                    None
                };
                let count = self.runtime.pop()?.to_f64().max(0.0) as usize;
                let text = self.read_input_chars(count, file_number)?;
                self.runtime.push(QType::String(text));
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
                    if self.runtime.play_handler_depth == Some(self.runtime.call_stack.len()) {
                        self.runtime.play_handler_depth = None;
                        if self.runtime.play_trap_state != PlayTrapState::Off {
                            self.runtime.play_trap_state = PlayTrapState::On;
                        }
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
                self.emit_console_text(&rendered);
            }

            OpCode::LPrint => {
                let value = self.runtime.pop()?;
                let rendered = format!("{}", value);
                self.emit_lprint_text(&rendered);
            }

            OpCode::PrintNewline => {
                self.emit_console_text("\n");
            }

            OpCode::LPrintNewline => {
                self.lprint_newline();
            }

            OpCode::PrintComma => {
                self.print_comma();
            }

            OpCode::LPrintComma => {
                self.lprint_comma();
            }

            OpCode::LPrintTab => {
                let target = self.runtime.pop()?.to_f64().round() as i16;
                self.lprint_tab_to(target);
            }

            OpCode::LPrintSpace => {
                let count = self.runtime.pop()?.to_f64().round().max(0.0) as usize;
                self.lprint_space(count);
            }

            OpCode::PrintUsing { count, comma_after } => {
                let (format, values) = self.pop_using_operands(count)?;
                self.emit_print_using_values(&format, &values, &comma_after)?;
            }

            OpCode::LPrintUsing { count, comma_after } => {
                let (format, values) = self.pop_using_operands(count)?;
                self.emit_lprint_using_values(&format, &values, &comma_after)?;
            }

            OpCode::Beep => {
                self.sound.stop();
                self.sound.beep();
                let notes = self.sound.drain_notes();
                self.render_sound_notes(&notes);
            }

            OpCode::Screen(mode) => {
                self.screen_mode = mode;
                self.apply_screen_mode_text_state(mode);
                if let Some(ref mut gfx) = self.graphics {
                    gfx.set_screen_mode(mode as u8);
                }
            }

            OpCode::ScreenDynamic => {
                let mode = self.pop_i32_rounded()?;
                self.screen_mode = mode;
                self.apply_screen_mode_text_state(mode);
                if let Some(ref mut gfx) = self.graphics {
                    gfx.set_screen_mode(mode as u8);
                }
            }

            OpCode::ScreenFn(arg_count) => {
                let color_flag = if arg_count >= 3 {
                    self.pop_i32_rounded()?
                } else {
                    0
                };
                let col = self.pop_i32_rounded()?;
                let row = self.pop_i32_rounded()?;
                self.runtime
                    .push(QType::Integer(self.screen_fn_value(row, col, color_flag)));
            }

            OpCode::Pset { x, y, color } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.pset(x, y, Self::normalize_color(color, 0));
                }
            }

            OpCode::PsetDynamic => {
                let color = self.pop_i32_rounded()?;
                let y = self.pop_i32_rounded()?;
                let x = self.pop_i32_rounded()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.pset(x, y, Self::normalize_color(color, 0));
                }
            }

            OpCode::Circle {
                x,
                y,
                radius,
                color,
            } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.circle(x, y, radius, Self::normalize_color(color, 0));
                }
            }

            OpCode::CircleDynamic => {
                let color = self.pop_i32_rounded()?;
                let radius = self.pop_i32_rounded()?;
                let y = self.pop_i32_rounded()?;
                let x = self.pop_i32_rounded()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.circle(x, y, radius, Self::normalize_color(color, 0));
                }
            }

            OpCode::Sound {
                frequency,
                duration,
            } => {
                if frequency > 0 {
                    self.emit_terminal_bell();
                }
                self.sleep_sound_ticks(duration.max(0));
            }

            OpCode::SoundDynamic => {
                let duration = self.pop_i32_rounded()?.max(0);
                let frequency = self.pop_i32_rounded()?.max(0);
                if frequency > 0 {
                    self.emit_terminal_bell();
                }
                self.sleep_sound_ticks(duration);
            }

            OpCode::Play(melody) => {
                self.sound.stop();
                let parsed = self.sound.parse_melody(&melody);
                if parsed.background {
                    self.queue_background_sound_notes(&parsed.notes);
                } else {
                    self.sound.play_melody(&melody);
                    let notes = self.sound.drain_notes();
                    self.render_sound_notes(&notes);
                }
            }

            OpCode::PlayDynamic => {
                let melody = self.pop_string_value()?;
                self.sound.stop();
                let parsed = self.sound.parse_melody(&melody);
                if parsed.background {
                    self.queue_background_sound_notes(&parsed.notes);
                } else {
                    self.sound.play_melody(&melody);
                    let notes = self.sound.drain_notes();
                    self.render_sound_notes(&notes);
                }
            }

            OpCode::PlayFunc => {
                let _ = self.runtime.pop()?;
                let queue_count = self.current_play_queue_count();
                self.runtime.push(QType::Integer(queue_count));
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

            OpCode::MidNoLen => {
                let start = self.runtime.pop()?.to_f64() as usize;
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let result = str
                        .chars()
                        .skip(start.saturating_sub(1))
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

            OpCode::InStrFrom => {
                let search = self.runtime.pop()?;
                let source = self.runtime.pop()?;
                let start = self.runtime.pop()?.to_f64().round().max(1.0) as usize;
                if let (QType::String(src), QType::String(srch)) = (source, search) {
                    let chars = src.chars().collect::<Vec<_>>();
                    let offset = start.saturating_sub(1);
                    let haystack = chars.iter().skip(offset).collect::<String>();
                    let pos = haystack
                        .find(&srch)
                        .map(|p| (offset + p + 1) as i16)
                        .unwrap_or(0);
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
                let content = Self::qbasic_str(&n);
                self.runtime.push(QType::String(content));
            }

            OpCode::ValFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    let val = Self::qbasic_val(&str);
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
                let value = self.runtime.pop()?;
                let n = self.runtime.pop()?.to_f64() as usize;
                self.runtime
                    .push(QType::String(Self::qbasic_string(n, value)));
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
                let value = self.rnd_value(None);
                self.runtime.push(QType::Single(value));
            }

            OpCode::RndWithArg => {
                let arg = self.runtime.pop()?.to_f64();
                let value = self.rnd_value(Some(arg));
                self.runtime.push(QType::Single(value));
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
                    self.wait_for_keypress();
                } else {
                    std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
                }
            }

            OpCode::Randomize => self.randomize_rng(),

            OpCode::RandomizeDynamic => {
                let seed = self.runtime.pop()?.to_f64();
                self.seed_rng_with_value(seed);
            }

            // Arrays
            OpCode::ArrayDim { name, dimensions } => {
                self.store_array_dimensions(name, dimensions, false)?;
            }

            OpCode::ArrayDimDynamic { name, dimensions } => {
                let dimensions = self.pop_array_dimensions(dimensions)?;
                self.store_array_dimensions(name, dimensions, false)?;
            }

            OpCode::ArrayRedim {
                name,
                dimensions,
                preserve,
            } => {
                self.store_array_dimensions(name, dimensions, preserve)?;
            }

            OpCode::ArrayRedimDynamic {
                name,
                dimensions,
                preserve,
            } => {
                let dimensions = self.pop_array_dimensions(dimensions)?;
                self.store_array_dimensions(name, dimensions, preserve)?;
            }

            OpCode::ArrayLoad(name, num_indices) => {
                // Pop indices from stack
                let mut indices = Vec::with_capacity(num_indices);
                for _ in 0..num_indices {
                    let idx = self.runtime.pop()?;
                    indices.push(idx.to_f64().round() as i32);
                }
                indices.reverse();

                // create implicit if not exists - use smaller default size
                if !self.runtime.arrays.contains_key(&name) {
                    let dimensions = self.create_implicit_array_dimensions(&indices)?;
                    let total_size = Self::validate_array_dimensions(&dimensions)?;
                    let default_value = self.default_array_value_for_name(&name);
                    let mut array = Vec::with_capacity(total_size);
                    array.resize(total_size, default_value);
                    self.runtime.arrays.insert(name.clone(), array);
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

                let Some(linear_index) = self.linear_array_index(&dims, &indices) else {
                    return Err(QError::SubscriptOutOfRange);
                };

                if let Some(array) = self.runtime.arrays.get(&name.clone()) {
                    if linear_index < array.len() {
                        self.runtime.push(array[linear_index].clone());
                    } else {
                        return Err(QError::SubscriptOutOfRange);
                    }
                } else {
                    self.runtime.push(self.default_array_value_for_name(&name));
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
                    indices.push(idx.to_f64().round() as i32);
                }
                indices.reverse();

                // create implicit if not exists - use smaller default size
                if !self.runtime.arrays.contains_key(&name) {
                    let dimensions = self.create_implicit_array_dimensions(&indices)?;
                    let total_size = Self::validate_array_dimensions(&dimensions)?;
                    let default_value = self.default_array_value_for_name(&name);
                    let mut array = Vec::with_capacity(total_size);
                    array.resize(total_size, default_value);
                    self.runtime.arrays.insert(name.clone(), array);
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

                let Some(linear_index) = self.linear_array_index(&dims, &indices) else {
                    return Err(QError::SubscriptOutOfRange);
                };

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

            OpCode::DefFn {
                name,
                param_slots,
                body,
            } => {
                self.runtime.def_fns.insert(name, (param_slots, body));
            }

            OpCode::MarkConst(slot) => {
                if let Some(value) = self.runtime.globals.get(slot).cloned() {
                    self.runtime.const_globals.insert(slot, value);
                }
            }

            OpCode::CallDefFn(name) => {
                if let Some((param_slots, body)) = self.runtime.def_fns.get(&name).cloned() {
                    // Pop arguments from stack
                    let mut args = Vec::new();
                    for _ in 0..param_slots.len() {
                        args.push(self.runtime.pop()?);
                    }
                    args.reverse();

                    // Backup globals that will be used for parameters.
                    let mut backups = Vec::new();
                    for (slot, arg) in param_slots.iter().copied().zip(args.iter()) {
                        if slot < self.runtime.globals.len() {
                            backups.push((slot, self.runtime.globals[slot].clone()));
                            self.runtime.globals[slot] = arg.clone();
                        } else {
                            while self.runtime.globals.len() <= slot {
                                self.runtime.globals.push(QType::Empty);
                            }
                            backups.push((slot, QType::Empty));
                            self.runtime.globals[slot] = arg.clone();
                        }
                    }

                    // Execute function body - the result will be pushed onto the stack
                    for opcode in body {
                        self.execute_opcode(opcode)?;
                    }

                    // Restore backed up globals
                    for (slot, val) in backups {
                        if slot < self.runtime.globals.len() {
                            self.runtime.globals[slot] = val;
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

            OpCode::OnPlay {
                queue_limit,
                handler,
            } => {
                self.runtime.play_handler_address = Some(handler);
                self.runtime.play_queue_limit = queue_limit.clamp(1, 32);
                self.runtime.play_pending_event = false;
            }

            OpCode::OnPlayDynamic { handler } => {
                let queue_limit = self.pop_i32_rounded()?.clamp(1, 32) as usize;
                self.runtime.play_handler_address = Some(handler);
                self.runtime.play_queue_limit = queue_limit;
                self.runtime.play_pending_event = false;
            }

            OpCode::PlayOn => {
                self.runtime.play_trap_state = PlayTrapState::On;
            }

            OpCode::PlayOff => {
                self.runtime.play_trap_state = PlayTrapState::Off;
                self.runtime.play_pending_event = false;
                self.runtime.play_handler_depth = None;
            }

            OpCode::PlayStop => {
                self.runtime.play_trap_state = PlayTrapState::Stop;
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
                self.runtime.random_fields.clear();
                self.runtime.file_print_columns.clear();
                self.runtime.current_segment = 0;
                self.runtime.play_note_deadlines.clear();
                self.runtime.play_pending_event = false;
                self.runtime.play_handler_depth = None;
                self.runtime.play_trap_state = PlayTrapState::Off;
                self.file_io.close_all();
                for value in &mut self.runtime.globals {
                    *value = match value {
                        QType::String(_) => QType::String(String::new()),
                        _ => QType::Integer(0),
                    };
                }
                for (&slot, value) in &self.runtime.const_globals {
                    if let Some(target) = self.runtime.globals.get_mut(slot) {
                        *target = value.clone();
                    }
                }
            }

            OpCode::Cls(mode) => {
                self.execute_cls_mode(mode);
            }

            OpCode::ClsDynamic => {
                let mode = self.pop_i32_rounded()?;
                self.execute_cls_mode(mode);
            }

            OpCode::Locate(row, col) => {
                if self.locate_cursor(row, col) {
                    self.write_stdout(&format!(
                        "\x1B[{};{}H",
                        self.runtime.cursor_row, self.runtime.cursor_col
                    ));
                }
            }

            OpCode::LocateDynamic => {
                let col = self.pop_i32_rounded()?;
                let row = self.pop_i32_rounded()?;
                if self.locate_cursor(row, col) {
                    self.write_stdout(&format!(
                        "\x1B[{};{}H",
                        self.runtime.cursor_row, self.runtime.cursor_col
                    ));
                }
            }

            OpCode::SetCursorState {
                visible,
                start,
                stop,
            } => {
                self.set_cursor_state(visible, start, stop);
            }

            OpCode::SetCursorStateDynamic => {
                let stop = self.pop_i32_rounded()?;
                let start = self.pop_i32_rounded()?;
                let visible = self.pop_i32_rounded()?;
                self.set_cursor_state(visible, start, stop);
            }

            OpCode::Width { columns, rows } => {
                self.set_text_width(columns, rows);
            }

            OpCode::WidthDynamic => {
                let rows = self.pop_i32_rounded()?;
                let columns = self.pop_i32_rounded()?;
                self.set_text_width(columns, rows);
            }

            OpCode::Color(fg, bg) => {
                // Simplified color support
                self.runtime.text_foreground = fg.clamp(0, 255) as u8;
                self.runtime.text_background = bg.clamp(0, 255) as u8;
                self.write_stdout(&format!("\x1B[{}m", 30 + fg));
            }

            OpCode::ColorDynamic => {
                let bg = self.pop_i32_rounded()?;
                let fg = self.pop_i32_rounded()?;
                self.runtime.text_foreground = fg.clamp(0, 255) as u8;
                self.runtime.text_background = bg.clamp(0, 255) as u8;
                self.write_stdout(&format!("\x1B[{}m", 30 + fg));
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
                    gfx.paint(
                        x,
                        y,
                        Self::normalize_color(paint_color, 0),
                        Self::normalize_color(border_color, 0),
                    );
                }
            }

            OpCode::PaintDynamic => {
                let border_color = self.pop_i32_rounded()?;
                let paint_color = self.pop_i32_rounded()?;
                let y = self.pop_i32_rounded()?;
                let x = self.pop_i32_rounded()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.paint(
                        x,
                        y,
                        Self::normalize_color(paint_color, 0),
                        Self::normalize_color(border_color, 0),
                    );
                }
            }

            OpCode::Draw { commands } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.draw(&commands);
                }
            }

            OpCode::DrawDynamic => {
                let commands = self.pop_string_value()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.draw(&commands);
                }
            }

            OpCode::Palette { attribute, color } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.palette(
                        Self::normalize_color(attribute, 0),
                        Self::normalize_color(color, 0),
                    );
                }
            }

            OpCode::PaletteDynamic => {
                let color = self.pop_i32_rounded()?;
                let attribute = self.pop_i32_rounded()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.palette(
                        Self::normalize_color(attribute, 0),
                        Self::normalize_color(color, 0),
                    );
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
                    gfx.view(
                        x1,
                        y1,
                        x2,
                        y2,
                        Self::normalize_color(fill_color, 0),
                        Self::normalize_color(border_color, 0),
                    );
                }
            }

            OpCode::ViewDynamic => {
                let border_color = self.pop_i32_rounded()?;
                let fill_color = self.pop_i32_rounded()?;
                let y2 = self.pop_i32_rounded()?;
                let x2 = self.pop_i32_rounded()?;
                let y1 = self.pop_i32_rounded()?;
                let x1 = self.pop_i32_rounded()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.view(
                        x1,
                        y1,
                        x2,
                        y2,
                        Self::normalize_color(fill_color, 0),
                        Self::normalize_color(border_color, 0),
                    );
                }
            }

            OpCode::ViewPrint { top, bottom } => {
                self.set_view_print_region(top, bottom);
            }

            OpCode::ViewPrintDynamic => {
                let bottom = self.pop_i32_rounded()?;
                let top = self.pop_i32_rounded()?;
                self.set_view_print_region(top, bottom);
            }

            OpCode::ViewReset => {
                self.reset_view_print_region();
                if let Some(ref mut gfx) = self.graphics {
                    gfx.view_reset();
                }
            }

            OpCode::Window { x1, y1, x2, y2 } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.window(x1, y1, x2, y2);
                }
            }

            OpCode::WindowDynamic => {
                let y2 = self.pop_f64()?;
                let x2 = self.pop_f64()?;
                let y1 = self.pop_f64()?;
                let x1 = self.pop_f64()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.window(x1, y1, x2, y2);
                }
            }

            OpCode::WindowReset => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.window_reset();
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

            OpCode::Restore(section) => {
                if let Some(section) = section {
                    self.runtime
                        .data_pointer
                        .reset_to_section(section.min(self.runtime.data_section.len()));
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
                self.runtime
                    .push(QType::Integer(self.runtime.last_error_line));
            }

            OpCode::ErDev => {
                // Return device error code
                self.runtime.push(QType::Integer(0));
            }

            OpCode::ErDevStr => {
                // Return device error string
                self.runtime.push(QType::String(String::new()));
            }

            OpCode::SetCurrentLine(line) => {
                self.set_current_line(line);
            }

            OpCode::TraceOn => {
                self.runtime.trace_enabled = true;
            }

            OpCode::TraceOff => {
                self.runtime.trace_enabled = false;
            }

            // Array bounds functions
            OpCode::LBound(name, dim) => {
                let dims = self.runtime.array_dimensions.get(&name);
                let lower = dims
                    .and_then(|d| d.get(dim as usize))
                    .map(|(l, _)| *l)
                    .unwrap_or(self.runtime.default_array_base);
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

            OpCode::LBoundDynamic(name) => {
                let dim = self.pop_i32_rounded()?.max(1) as usize - 1;
                let dims = self.runtime.array_dimensions.get(&name);
                let lower = dims
                    .and_then(|d| d.get(dim))
                    .map(|(l, _)| *l)
                    .unwrap_or(self.runtime.default_array_base);
                self.runtime.push(QType::Integer(lower as i16));
            }

            OpCode::UBoundDynamic(name) => {
                let dim = self.pop_i32_rounded()?.max(1) as usize - 1;
                let dims = self.runtime.array_dimensions.get(&name);
                let upper = dims.and_then(|d| d.get(dim)).map(|(_, u)| *u).unwrap_or(0);
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
                    self.runtime.file_print_columns.insert(file_num, 1);
                }
            }

            OpCode::Close => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.file_io.close_by_num(file_num)?;
                self.runtime.file_print_columns.remove(&file_num);
            }

            OpCode::PrintFile(file_num) => {
                let value = self.runtime.pop()?;
                let content = format!("{}", value);
                self.file_print_write(file_num.parse().unwrap_or(0), &content)?;
            }

            OpCode::PrintFileDynamic => {
                let value = self.runtime.pop()?;
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let content = format!("{}", value);
                self.file_print_write(file_num, &content)?;
            }

            OpCode::PrintFileCommaDynamic => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.file_print_comma(file_num)?;
            }

            OpCode::PrintFileNewlineDynamic => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.file_print_newline(file_num)?;
            }

            OpCode::PrintFileUsingDynamic { count, comma_after } => {
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.runtime.pop()?);
                }
                values.reverse();
                let format = Self::coerce_using_format(self.runtime.pop()?);
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.emit_file_print_using_values(file_num, &format, &values, &comma_after)?;
            }

            OpCode::LineInputDynamic => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let line = self.file_io.read_line_by_num(file_num)?;
                self.runtime.push(QType::String(line));
            }

            OpCode::WriteConsole(value_count) => {
                let mut values = Vec::with_capacity(value_count);
                for _ in 0..value_count {
                    values.push(self.runtime.pop()?);
                }
                values.reverse();
                let content = Self::write_record(values);
                self.emit_console_text(&content);
                self.emit_console_text("\n");
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

            OpCode::EofDynamic => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let eof = self.file_io.is_eof_by_num(file_num);
                self.runtime.push(QType::Integer(if eof { -1 } else { 0 }));
            }

            OpCode::Lof(file_num) => {
                let len = self.file_io.length_by_num(file_num.parse().unwrap_or(0));
                self.runtime.push(QType::Long(len as i32));
            }

            OpCode::LofDynamic => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let len = self.file_io.length_by_num(file_num);
                self.runtime.push(QType::Long(len as i32));
            }

            OpCode::Loc(file_num) => {
                let pos = self.file_io.position_by_num(file_num.parse().unwrap_or(0));
                self.runtime.push(QType::Long(pos as i32));
            }

            OpCode::LocDynamic => {
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let pos = self.file_io.position_by_num(file_num);
                self.runtime.push(QType::Long(pos as i32));
            }

            OpCode::FreeFile => {
                let file_num = self.file_io.get_free_file_num();
                self.runtime.push(QType::Integer(file_num));
            }

            OpCode::Seek(file_num, pos) => {
                let file_num = file_num.parse().unwrap_or(0);
                self.file_io.seek_by_num(file_num, pos as u64)?;
                self.runtime.file_print_columns.remove(&file_num);
            }

            OpCode::SeekDynamic => {
                let pos = self.runtime.pop()?.to_f64() as u64;
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.file_io.seek_by_num(file_num, pos)?;
                self.runtime.file_print_columns.remove(&file_num);
            }

            // Preset - same as Pset but with background color
            OpCode::Preset { x, y, color } => {
                if let Some(ref mut gfx) = self.graphics {
                    gfx.preset(x, y, Self::normalize_color(color, 0));
                }
            }

            OpCode::PresetDynamic => {
                let color = self.pop_i32_rounded()?;
                let y = self.pop_i32_rounded()?;
                let x = self.pop_i32_rounded()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.preset(x, y, Self::normalize_color(color, 0));
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
                    gfx.line(x1, y1, x2, y2, Self::normalize_color(color, 0));
                }
            }

            OpCode::LineDynamic => {
                let color = self.pop_i32_rounded()?;
                let y2 = self.pop_i32_rounded()?;
                let x2 = self.pop_i32_rounded()?;
                let y1 = self.pop_i32_rounded()?;
                let x1 = self.pop_i32_rounded()?;
                if let Some(ref mut gfx) = self.graphics {
                    gfx.line(x1, y1, x2, y2, Self::normalize_color(color, 0));
                }
            }

            OpCode::GetBinary { kind, fixed_length } => {
                let record_num = self.runtime.pop()?.to_f64().round().max(0.0) as u64;
                let file_num = self.runtime.pop()?.to_f64() as i32;
                let value =
                    self.read_binary_value(file_num as u8, record_num, kind, fixed_length)?;
                self.runtime.push(value);
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
            OpCode::PutBinary { kind, fixed_length } => {
                let value = self.runtime.pop()?;
                let record_num = self.runtime.pop()?.to_f64().round().max(0.0) as u64;
                let file_num = self.runtime.pop()?.to_f64() as i32;
                self.write_binary_value(file_num as u8, record_num, kind, fixed_length, value)?;
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
                let content = Self::format_write_value(value);
                self.file_io
                    .write_line_by_num(file_num.parse().unwrap_or(0), &content)?;
                self.runtime
                    .file_print_columns
                    .insert(file_num.parse().unwrap_or(0), 1);
            }

            OpCode::WriteFileDynamic(value_count) => {
                let mut values = Vec::with_capacity(value_count);
                for _ in 0..value_count {
                    values.push(self.runtime.pop()?);
                }
                values.reverse();

                let file_num = self.runtime.pop()?.to_f64() as i32;
                let content = Self::write_record(values);
                self.file_io.write_line_by_num(file_num, &content)?;
                self.runtime.file_print_columns.insert(file_num, 1);
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
                    self.runtime.push(QType::Integer(runtime_cv_i16(&str)));
                } else {
                    self.runtime.push(QType::Integer(0));
                }
            }

            OpCode::CvlFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::Long(runtime_cv_i32(&str)));
                } else {
                    self.runtime.push(QType::Long(0));
                }
            }

            OpCode::CvsFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::Single(runtime_cv_f32(&str)));
                } else {
                    self.runtime.push(QType::Single(0.0));
                }
            }

            OpCode::CvdFunc => {
                let s = self.runtime.pop()?;
                if let QType::String(str) = s {
                    self.runtime.push(QType::Double(runtime_cv_f64(&str)));
                } else {
                    self.runtime.push(QType::Double(0.0));
                }
            }

            OpCode::CvFunc(type_name) => {
                let value = self.runtime.pop()?;
                if let QType::String(str) = value {
                    self.runtime.push(runtime_cv_to_qtype(&type_name, &str));
                } else {
                    self.runtime.push(QType::Double(0.0));
                }
            }

            OpCode::FileExistsFunc => {
                let value = self.runtime.pop()?;
                if let QType::String(path) = value {
                    let exists = std::path::Path::new(&path).is_file();
                    self.runtime
                        .push(QType::Integer(if exists { -1 } else { 0 }));
                } else {
                    self.runtime.push(QType::Integer(0));
                }
            }

            OpCode::DirExistsFunc => {
                let value = self.runtime.pop()?;
                if let QType::String(path) = value {
                    let exists = std::path::Path::new(&path).is_dir();
                    self.runtime
                        .push(QType::Integer(if exists { -1 } else { 0 }));
                } else {
                    self.runtime.push(QType::Integer(0));
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

            OpCode::FreDynamic => {
                let arg = self.runtime.pop()?;
                let arg_type = match arg {
                    QType::String(_) => 0,
                    QType::Integer(value) => value as i32,
                    QType::Long(value) => value,
                    QType::Single(value) => value.round() as i32,
                    QType::Double(value) => value.round() as i32,
                    _ => 0,
                };
                let free_mem = match arg_type {
                    0 => 524288,
                    -1 => 1048576,
                    -2 => 262144,
                    _ => 1048576,
                };
                self.runtime.push(QType::Long(free_mem));
            }

            OpCode::CsrLinFunc => {
                self.runtime.push(QType::Integer(self.runtime.cursor_row));
            }

            OpCode::PosFunc(_arg) => {
                self.runtime.push(QType::Integer(self.runtime.cursor_col));
            }

            OpCode::PosDynamic => {
                let _ = self.runtime.pop()?;
                self.runtime.push(QType::Integer(self.runtime.cursor_col));
            }

            OpCode::LPosFunc(_arg) => {
                self.runtime.push(QType::Integer(self.runtime.printer_col));
            }

            OpCode::LPosDynamic => {
                let _ = self.runtime.pop()?;
                self.runtime.push(QType::Integer(self.runtime.printer_col));
            }

            OpCode::EnvironFunc => {
                let arg = self.runtime.pop()?;
                let indexed_env = Self::sorted_env_entries();
                let value = match arg {
                    QType::String(name) => std::env::var(&name).unwrap_or_default(),
                    QType::Integer(index) if index >= 1 => {
                        let index = index as usize;
                        indexed_env
                            .get(index.saturating_sub(1))
                            .cloned()
                            .unwrap_or_default()
                    }
                    QType::Long(index) if index >= 1 => {
                        let index = index as usize;
                        indexed_env
                            .get(index.saturating_sub(1))
                            .cloned()
                            .unwrap_or_default()
                    }
                    QType::Single(index) if index >= 1.0 => {
                        let index = index.round() as usize;
                        indexed_env
                            .get(index.saturating_sub(1))
                            .cloned()
                            .unwrap_or_default()
                    }
                    QType::Double(index) if index >= 1.0 => {
                        let index = index.round() as usize;
                        indexed_env
                            .get(index.saturating_sub(1))
                            .cloned()
                            .unwrap_or_default()
                    }
                    _ => String::new(),
                };
                self.runtime.push(QType::String(value));
            }

            OpCode::CommandFunc => {
                let args = std::env::var("QBNEX_COMMAND_LINE")
                    .unwrap_or_else(|_| std::env::args().skip(1).collect::<Vec<_>>().join(" "));
                self.runtime.push(QType::String(args));
            }

            OpCode::InKeyFunc => {
                let key = self.poll_inkey_nonblocking();
                self.runtime.push(QType::String(key));
            }

            OpCode::KeySetDynamic => {
                let key_string = self.pop_string_value()?;
                let key_num = self.pop_i32_rounded()? as i16;
                self.runtime.key_assignments.insert(key_num, key_string);
            }

            OpCode::KeyOn => {
                self.runtime.key_enabled = true;
            }

            OpCode::KeyOff => {
                self.runtime.key_enabled = false;
            }

            OpCode::KeyList => {
                self.list_keys();
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

            OpCode::DefSegDynamic => {
                let seg = self.pop_i32_rounded()?;
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

            OpCode::PointDynamic => {
                let y = self.pop_i32_rounded()?;
                let x = self.pop_i32_rounded()?;
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

            OpCode::PMapDynamic => {
                let func = self.pop_i32_rounded()?;
                let coord = self.pop_f64()?;
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

            OpCode::GetImageDynamic { array } => {
                let y2 = self.pop_i32_rounded()?;
                let x2 = self.pop_i32_rounded()?;
                let y1 = self.pop_i32_rounded()?;
                let x1 = self.pop_i32_rounded()?;
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

            OpCode::PutImageDynamic { array } => {
                let action = self.pop_string_value()?;
                let y = self.pop_i32_rounded()?;
                let x = self.pop_i32_rounded()?;
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

            OpCode::AscAssign => {
                let replacement = self.runtime.pop()?;
                let position = self.runtime.pop()?;
                let original = self.runtime.pop()?;

                let mut original_text = match original {
                    QType::String(text) => text,
                    other => format!("{}", other),
                };
                let index = (position.to_f64().round() as i32 - 1).max(0) as usize;
                let ascii = replacement.to_f64().round().clamp(0.0, 255.0) as u8;
                let mut chars: Vec<char> = original_text.chars().collect();

                while chars.len() < index {
                    chars.push(' ');
                }

                let new_char = char::from(ascii);
                if index < chars.len() {
                    chars[index] = new_char;
                } else {
                    chars.push(new_char);
                }

                original_text = chars.into_iter().collect();
                self.runtime.push(QType::String(original_text));
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
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            self.write_stdout(stdout.as_ref());
                            eprint!("{}", String::from_utf8_lossy(&output.stderr));
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
                        self.write_stdout(&format!("{name}\n"));
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

fn qb_format_using_pattern(format: &str, value: &QType) -> QResult<String> {
    let decoded = qb_decode_using_pattern(format);
    if let Some((start, end, mode)) = qb_find_using_string_field(&decoded) {
        let prefix = qb_collect_using_chars(&decoded[..start]);
        let suffix = qb_collect_using_chars(&decoded[end + 1..]);
        return Ok(format!(
            "{}{}{}",
            prefix,
            qb_format_using_string_value(mode, value),
            suffix
        ));
    }

    if let Some((start, end)) = qb_find_using_numeric_span(&decoded) {
        let prefix = qb_collect_using_chars(&decoded[..start]);
        let suffix = qb_collect_using_chars(&decoded[end + 1..]);
        let core = qb_collect_using_chars(&decoded[start..=end]);
        return Ok(format!(
            "{}{}{}",
            prefix,
            qb_format_using_numeric_value(&core, value.to_f64())?,
            suffix
        ));
    }

    Ok(format!("{}", value))
}

fn qb_decode_using_pattern(format: &str) -> Vec<(char, bool)> {
    let mut decoded = Vec::new();
    let mut chars = format.chars();
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

fn qb_find_using_string_field(
    decoded: &[(char, bool)],
) -> Option<(usize, usize, QbUsingStringMode)> {
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
    let Some((start_char, literal)) = decoded.get(start).copied() else {
        return None;
    };
    if literal {
        return None;
    }

    match start_char {
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
                && matches!(
                    decoded[end + 1].0,
                    '#' | '.' | '+' | '-' | '*' | '$' | ',' | '^'
                )
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

fn qb_format_using_chunks(pattern: &str, values: &[QType]) -> QResult<Vec<String>> {
    let (leading, fields) = qb_parse_using_fields(pattern);
    if fields.is_empty() {
        if values.is_empty() {
            return Ok(if leading.is_empty() {
                Vec::new()
            } else {
                vec![leading]
            });
        }
        return Ok(values.iter().map(|_| leading.clone()).collect());
    }

    let mut chunks = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let field_index = index % fields.len();
        let field = &fields[field_index];
        let mut chunk = String::new();
        if field_index == 0 {
            chunk.push_str(&leading);
        }
        chunk.push_str(&qb_format_using_pattern(&field.core, value)?);
        chunk.push_str(&field.suffix);
        chunks.push(chunk);
    }

    Ok(chunks)
}

fn qb_format_using_string_value(mode: QbUsingStringMode, value: &QType) -> String {
    let text = match value {
        QType::String(text) => text.clone(),
        _ => format!("{}", value),
    };
    match mode {
        QbUsingStringMode::FirstChar => text.chars().next().unwrap_or(' ').to_string(),
        QbUsingStringMode::Whole => text,
        QbUsingStringMode::FixedWidth(width) => {
            let truncated = text.chars().take(width).collect::<String>();
            format!("{:<width$}", truncated, width = width)
        }
    }
}

fn qb_format_using_numeric_value(core: &str, value: f64) -> QResult<String> {
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

fn qb_using_extra_integer_positions(prefix_kind: QbUsingPrefixKind) -> usize {
    match prefix_kind {
        QbUsingPrefixKind::None => 0,
        QbUsingPrefixKind::Stars => 2,
        QbUsingPrefixKind::Dollars => 1,
        QbUsingPrefixKind::StarDollars => 2,
    }
}

fn qb_insert_grouping(int_part: &str) -> String {
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

fn qb_format_using_fixed(
    core: &str,
    mantissa: &str,
    value: f64,
    leading_plus: bool,
    trailing_plus: bool,
    trailing_minus: bool,
    prefix_kind: QbUsingPrefixKind,
    target_width: usize,
) -> QResult<String> {
    let parts: Vec<&str> = mantissa.splitn(2, '.').collect();
    let int_pattern = parts[0];
    let frac_pattern = parts.get(1).copied().unwrap_or("");
    let comma_slots = int_pattern.chars().filter(|ch| *ch == ',').count();
    let int_hashes = int_pattern.chars().filter(|ch| *ch == '#').count();
    let frac_hashes = frac_pattern.chars().filter(|ch| *ch == '#').count();
    let extra_integer_positions = qb_using_extra_integer_positions(prefix_kind);

    if int_hashes + frac_hashes + extra_integer_positions > 24 {
        return Err(QError::IllegalFunctionCall(
            "PRINT USING digit count exceeds 24".to_string(),
        ));
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
        return Ok(format!("%{}", rounded));
    }

    let show_zero_before_decimal = int_hashes + extra_integer_positions > 0;
    let int_digits = if rounded_int == "0" && !show_zero_before_decimal {
        String::new()
    } else {
        rounded_int.to_string()
    };

    let grouped_int = if comma_slots > 0 {
        qb_insert_grouping(&int_digits)
    } else {
        int_digits
    };

    if grouped_int.chars().count() > integer_capacity {
        return Ok(format!("%{}", rounded));
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
        return Ok(overflow);
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
    debug_assert_eq!(output.chars().count(), target_width);
    let _ = core;
    Ok(output)
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

    (
        format!("{:0width$}", digits, width = significant_digits),
        exponent,
    )
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

fn qb_format_using_exponential(
    core: &str,
    mantissa: &str,
    value: f64,
    leading_plus: bool,
    trailing_plus: bool,
    trailing_minus: bool,
    exponent_digits: usize,
) -> QResult<String> {
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
        return Err(QError::IllegalFunctionCall(
            "PRINT USING significant digit count exceeds 24".to_string(),
        ));
    }

    let (digits, exponent) = qb_scientific_digits(value, significant_digits);
    let adjusted_exponent = exponent + 1 - digits_before_decimal as i32;
    let mantissa_text = qb_compose_scientific_mantissa(&digits, digits_before_decimal, frac_hashes);
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
        return Ok(overflow);
    }

    let mut output = String::new();
    output.push_str(&" ".repeat(core.chars().count() - total_len));
    output.push_str(&body);
    if let Some(sign) = sign_suffix {
        output.push(sign);
    }
    Ok(output)
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
            .map_or(true, |part| part.is_empty() || text.ends_with(part))
}

impl Default for VM {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
