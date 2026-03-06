use core_types::{QError, QResult};
use std::process::Command;

pub struct Linker {
    output_path: String,
    object_files: Vec<String>,
    libraries: Vec<String>,
    link_options: Vec<String>,
}

impl Linker {
    pub fn new(output_path: String) -> Self {
        Self {
            output_path,
            object_files: Vec::new(),
            libraries: Vec::new(),
            link_options: Vec::new(),
        }
    }

    pub fn add_object_file(&mut self, path: impl Into<String>) {
        self.object_files.push(path.into());
    }

    pub fn add_library(&mut self, name: impl Into<String>) {
        self.libraries.push(name.into());
    }

    pub fn add_link_option(&mut self, option: impl Into<String>) {
        self.link_options.push(option.into());
    }

    /// Link object files into an executable using the system linker
    pub fn link(&self) -> QResult<()> {
        if self.output_path.is_empty() {
            return Err(QError::Internal("No output path specified".to_string()));
        }

        // Determine the linker based on platform
        #[cfg(target_os = "windows")]
        {
            self.link_with_link_exe()
        }

        #[cfg(target_os = "linux")]
        {
            self.link_with_ld()
        }

        #[cfg(target_os = "macos")]
        {
            self.link_with_ld()
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            self.link_with_generic()
        }
    }

    #[cfg(target_os = "windows")]
    fn link_with_link_exe(&self) -> QResult<()> {
        use std::env;

        // Try to find link.exe (MSVC) or use lld-link
        let linker = env::var("LINK").unwrap_or_else(|_| "link.exe".to_string());

        let mut cmd = Command::new(&linker);

        // Output file
        cmd.arg(format!("/OUT:{}", self.output_path));

        // Subsystem
        cmd.arg("/SUBSYSTEM:CONSOLE");

        // Entry point
        cmd.arg("/ENTRY:main");

        // Add default libraries
        cmd.arg("kernel32.lib");
        cmd.arg("libcmt.lib");
        cmd.arg("ucrt.lib");
        cmd.arg("vcruntime.lib");

        // Add custom options
        for opt in &self.link_options {
            cmd.arg(opt);
        }

        // Add libraries
        for lib in &self.libraries {
            cmd.arg(format!("{}.lib", lib));
        }

        // Add object files
        for obj in &self.object_files {
            cmd.arg(obj);
        }

        // Execute
        let output = cmd
            .output()
            .map_err(|e| QError::Internal(format!("Failed to execute linker: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(QError::Internal(format!("Linking failed: {}", stderr)));
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn link_with_ld(&self) -> QResult<()> {
        let mut cmd = Command::new("cc");

        // Output file
        cmd.arg("-o").arg(&self.output_path);

        // Add object files
        for obj in &self.object_files {
            cmd.arg(obj);
        }

        // Add libraries
        for lib in &self.libraries {
            cmd.arg(format!("-l{}", lib));
        }

        // Add custom options
        for opt in &self.link_options {
            cmd.arg(opt);
        }

        // Execute
        let output = cmd
            .output()
            .map_err(|e| QError::Internal(format!("Failed to execute linker: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(QError::Internal(format!("Linking failed: {}", stderr)));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn link_with_ld(&self) -> QResult<()> {
        let mut cmd = Command::new("cc");

        // Output file
        cmd.arg("-o").arg(&self.output_path);

        // macOS specific
        cmd.arg("-arch").arg("x86_64");
        cmd.arg("-macosx_version_min").arg("10.15");
        cmd.arg("-lSystem");

        // Add object files
        for obj in &self.object_files {
            cmd.arg(obj);
        }

        // Add libraries
        for lib in &self.libraries {
            cmd.arg(format!("-l{}", lib));
        }

        // Add custom options
        for opt in &self.link_options {
            cmd.arg(opt);
        }

        // Execute
        let output = cmd
            .output()
            .map_err(|e| QError::Internal(format!("Failed to execute linker: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(QError::Internal(format!("Linking failed: {}", stderr)));
        }

        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    #[allow(dead_code)]
    fn link_with_generic(&self) -> QResult<()> {
        // Fallback to cc
        let mut cmd = Command::new("cc");

        cmd.arg("-o").arg(&self.output_path);

        for obj in &self.object_files {
            cmd.arg(obj);
        }

        for lib in &self.libraries {
            cmd.arg(format!("-l{}", lib));
        }

        let output = cmd
            .output()
            .map_err(|e| QError::Internal(format!("Failed to execute linker: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(QError::Internal(format!("Linking failed: {}", stderr)));
        }

        Ok(())
    }

    /// Link LLVM IR directly using llc and system linker
    pub fn link_llvm_ir(&self, llvm_ir: &str) -> QResult<()> {
        use std::env;
        use std::fs;

        // Create temporary directory
        let temp_dir = env::temp_dir();
        let temp_id = format!("qb_{}", std::process::id());

        // Write LLVM IR to file
        let ir_file = temp_dir.join(format!("{}.ll", temp_id));
        fs::write(&ir_file, llvm_ir)
            .map_err(|e| QError::Internal(format!("Failed to write IR file: {}", e)))?;

        // Compile IR to object file using llc
        let obj_file = temp_dir.join(format!("{}.obj", temp_id));

        let llc_status = Command::new("llc")
            .args([
                "-filetype=obj",
                "-o",
                obj_file.to_str().unwrap(),
                ir_file.to_str().unwrap(),
            ])
            .status()
            .map_err(|e| {
                QError::Internal(format!("Failed to run llc: {}. Is LLVM installed?", e))
            })?;

        if !llc_status.success() {
            return Err(QError::Internal("llc compilation failed".to_string()));
        }

        // Add object file to list
        let mut linker = Self::new(self.output_path.clone());
        linker.add_object_file(obj_file.to_str().unwrap());

        // Copy other files and libraries
        for obj in &self.object_files {
            linker.add_object_file(obj.clone());
        }
        for lib in &self.libraries {
            linker.add_library(lib.clone());
        }
        for opt in &self.link_options {
            linker.add_link_option(opt.clone());
        }

        // Link
        linker.link()?;

        // Cleanup temp files
        let _ = fs::remove_file(&ir_file);
        let _ = fs::remove_file(&obj_file);

        Ok(())
    }
}

impl Default for Linker {
    fn default() -> Self {
        Self::new("output.exe".to_string())
    }
}
