use core_types::{QError, QResult};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum FileMode {
    Append,
    Binary,
    Input,
    Output,
    Random,
}

pub struct FileHandle {
    pub file_number: u8,
    pub path: String,
    pub mode: FileMode,
    pub file: Option<File>,
    pub position: u64,
    pub is_open: bool,
}

pub struct FileIO {
    files: HashMap<u8, FileHandle>,
    next_file_number: u8,
}

impl FileIO {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_file_number: 1,
        }
    }

    pub fn open(&mut self, path: &str, mode: FileMode) -> QResult<u8> {
        let file_number = self.next_file_number;
        self.next_file_number = ((self.next_file_number as u16 + 1) % 256) as u8;

        if self.next_file_number == 0 {
            self.next_file_number = 1;
        }

        let file = match mode {
            FileMode::Input => OpenOptions::new()
                .read(true)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
            FileMode::Output => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
            FileMode::Append => OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
            FileMode::Random | FileMode::Binary => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
        };

        let handle = FileHandle {
            file_number,
            path: path.to_string(),
            mode,
            file: Some(file),
            position: 0,
            is_open: true,
        };

        self.files.insert(file_number, handle);

        Ok(file_number)
    }

    pub fn close(&mut self, file_number: u8) -> QResult<()> {
        if let Some(mut handle) = self.files.remove(&file_number) {
            if let Some(mut file) = handle.file.take() {
                file.flush().map_err(|e| QError::FileIO(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn close_all(&mut self) {
        let numbers: Vec<u8> = self.files.keys().cloned().collect();
        for num in numbers {
            let _ = self.close(num);
        }
    }

    pub fn read_byte(&mut self, file_number: u8) -> QResult<u8> {
        let handle = self
            .files
            .get_mut(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            let mut buf = [0u8; 1];
            file.read_exact(&mut buf)
                .map_err(|e| QError::FileIO(e.to_string()))?;
            handle.position += 1;
            Ok(buf[0])
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn read_bytes(&mut self, file_number: u8, count: usize) -> QResult<Vec<u8>> {
        let handle = self
            .files
            .get_mut(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            let mut buf = vec![0u8; count];
            let read_count = file
                .read(&mut buf)
                .map_err(|e| QError::FileIO(e.to_string()))?;
            handle.position += read_count as u64;
            buf.truncate(read_count);
            Ok(buf)
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn write_byte(&mut self, file_number: u8, byte: u8) -> QResult<()> {
        let handle = self
            .files
            .get_mut(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            file.write_all(&[byte])
                .map_err(|e| QError::FileIO(e.to_string()))?;
            handle.position += 1;
            Ok(())
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn write_bytes(&mut self, file_number: u8, bytes: &[u8]) -> QResult<()> {
        let handle = self
            .files
            .get_mut(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            file.write_all(bytes)
                .map_err(|e| QError::FileIO(e.to_string()))?;
            handle.position += bytes.len() as u64;
            Ok(())
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn seek(&mut self, file_number: u8, position: u64) -> QResult<()> {
        let handle = self
            .files
            .get_mut(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            file.seek(SeekFrom::Start(position))
                .map_err(|e| QError::FileIO(e.to_string()))?;
            handle.position = position;
            Ok(())
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn tell(&self, file_number: u8) -> QResult<u64> {
        let handle = self
            .files
            .get(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        Ok(handle.position)
    }

    pub fn get(&mut self, file_number: u8, record_number: u64, buffer: &mut [u8]) -> QResult<()> {
        let handle = self
            .files
            .get_mut(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            let pos = (record_number - 1) * buffer.len() as u64;
            file.seek(SeekFrom::Start(pos))
                .map_err(|e| QError::FileIO(e.to_string()))?;
            file.read_exact(buffer)
                .map_err(|e| QError::FileIO(e.to_string()))?;
            handle.position = pos + buffer.len() as u64;
            Ok(())
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn put(&mut self, file_number: u8, record_number: u64, data: &[u8]) -> QResult<()> {
        let handle = self
            .files
            .get_mut(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            let pos = (record_number - 1) * data.len() as u64;
            file.seek(SeekFrom::Start(pos))
                .map_err(|e| QError::FileIO(e.to_string()))?;
            file.write_all(data)
                .map_err(|e| QError::FileIO(e.to_string()))?;
            handle.position = pos + data.len() as u64;
            Ok(())
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn eof(&self, file_number: u8) -> QResult<bool> {
        let handle = self
            .files
            .get(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref file) = handle.file {
            Ok(handle.position
                >= file
                    .metadata()
                    .map_err(|e| QError::FileIO(e.to_string()))?
                    .len())
        } else {
            Ok(true)
        }
    }

    pub fn lof(&self, file_number: u8) -> QResult<u64> {
        let handle = self
            .files
            .get(&file_number)
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref file) = handle.file {
            Ok(file
                .metadata()
                .map_err(|e| QError::FileIO(e.to_string()))?
                .len())
        } else {
            Ok(0)
        }
    }

    pub fn free_file(&self) -> u8 {
        for i in 1..=255 {
            if !self.files.contains_key(&i) {
                return i;
            }
        }
        0
    }

    pub fn is_open(&self, file_number: u8) -> bool {
        self.files.contains_key(&file_number)
    }

    // Methods for VM runtime compatibility (using i32 file numbers)
    pub fn open_compat(&mut self, file_num: i32, path: &str, mode_str: &str) -> QResult<()> {
        let mode = match mode_str.to_uppercase().as_str() {
            "INPUT" => FileMode::Input,
            "OUTPUT" => FileMode::Output,
            "APPEND" => FileMode::Append,
            "BINARY" => FileMode::Binary,
            "RANDOM" => FileMode::Random,
            _ => FileMode::Input,
        };

        let file = match mode {
            FileMode::Input => OpenOptions::new()
                .read(true)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
            FileMode::Output => OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
            FileMode::Append => OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
            FileMode::Random | FileMode::Binary => OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)
                .map_err(|e| QError::FileIO(e.to_string()))?,
        };

        let handle = FileHandle {
            file_number: file_num as u8,
            path: path.to_string(),
            mode,
            file: Some(file),
            position: 0,
            is_open: true,
        };

        self.files.insert(file_num as u8, handle);
        Ok(())
    }

    pub fn close_compat(&mut self, file_num: i32) -> QResult<()> {
        self.close(file_num as u8)
    }

    pub fn read_line_compat(&mut self, file_num: i32) -> QResult<String> {
        let handle = self
            .files
            .get_mut(&(file_num as u8))
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            // Seek to current position first
            file.seek(SeekFrom::Start(handle.position))
                .map_err(|e| QError::FileIO(e.to_string()))?;

            let mut line = String::new();
            let mut buf = [0u8; 1];
            loop {
                match file.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        handle.position += 1;
                        let ch = buf[0] as char;
                        if ch == '\n' {
                            break;
                        } else if ch != '\r' {
                            line.push(ch);
                        }
                    }
                    Err(e) => return Err(QError::FileIO(e.to_string())),
                }
            }
            Ok(line)
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn write_line_compat(&mut self, file_num: i32, content: &str) -> QResult<()> {
        let handle = self
            .files
            .get_mut(&(file_num as u8))
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            writeln!(file, "{}", content).map_err(|e| QError::FileIO(e.to_string()))?;
            file.flush().map_err(|e| QError::FileIO(e.to_string()))?;
            // Update position to actual file position
            handle.position = file
                .stream_position()
                .map_err(|e| QError::FileIO(e.to_string()))?;
            Ok(())
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn write_compat(&mut self, file_num: i32, content: &str) -> QResult<()> {
        let handle = self
            .files
            .get_mut(&(file_num as u8))
            .ok_or(QError::BadFileNameOrNumber)?;

        if let Some(ref mut file) = handle.file {
            write!(file, "{}", content).map_err(|e| QError::FileIO(e.to_string()))?;
            file.flush().map_err(|e| QError::FileIO(e.to_string()))?;
            // Update position to actual file position
            handle.position = file
                .stream_position()
                .map_err(|e| QError::FileIO(e.to_string()))?;
            Ok(())
        } else {
            Err(QError::BadFileNameOrNumber)
        }
    }

    pub fn is_eof_compat(&self, file_num: i32) -> bool {
        self.eof(file_num as u8).unwrap_or(true)
    }

    pub fn get_length_compat(&self, file_num: i32) -> usize {
        self.lof(file_num as u8).unwrap_or(0) as usize
    }

    pub fn get_free_file_compat(&self) -> i16 {
        self.free_file() as i16
    }

    pub fn seek_compat(&mut self, file_num: i32, pos: u64) -> QResult<()> {
        self.seek(file_num as u8, pos.saturating_sub(1))
    }

    // VM runtime interface methods
    pub fn open_by_num(&mut self, file_num: i32, path: &str, mode: &str) -> QResult<()> {
        self.open_compat(file_num, path, mode)
    }

    pub fn close_by_num(&mut self, file_num: i32) -> QResult<()> {
        self.close_compat(file_num)
    }

    pub fn read_line_by_num(&mut self, file_num: i32) -> QResult<String> {
        self.read_line_compat(file_num)
    }

    pub fn write_line_by_num(&mut self, file_num: i32, content: &str) -> QResult<()> {
        self.write_line_compat(file_num, content)
    }

    pub fn write_by_num(&mut self, file_num: i32, content: &str) -> QResult<()> {
        self.write_compat(file_num, content)
    }

    pub fn is_eof_by_num(&self, file_num: i32) -> bool {
        self.is_eof_compat(file_num)
    }

    pub fn length_by_num(&self, file_num: i32) -> usize {
        self.get_length_compat(file_num)
    }

    pub fn get_free_file_num(&self) -> i16 {
        self.get_free_file_compat()
    }

    pub fn seek_by_num(&mut self, file_num: i32, pos: u64) -> QResult<()> {
        self.seek_compat(file_num, pos)
    }

    pub fn position_by_num(&self, file_num: i32) -> usize {
        self.tell(file_num as u8)
            .map(|pos| pos.saturating_add(1) as usize)
            .unwrap_or(0)
    }
}

impl Default for FileIO {
    fn default() -> Self {
        Self::new()
    }
}
