# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.0   | :white_check_mark: |
| < 1.0.0 | :x:                |

## Reporting a Vulnerability

We take the security of QBNex seriously. If you discover a security vulnerability, please follow these guidelines:

### **DO NOT** create a public GitHub issue for security vulnerabilities

### How to Report

1. **Email**: Send security reports to the project maintainer
2. **Include**:
   - Description of the vulnerability
   - Steps to reproduce the issue
   - Potential impact assessment
   - Suggested fix (if applicable)
   - Your contact information for follow-up

### What to Expect

- **Acknowledgment**: Within 48 hours of report submission
- **Initial Assessment**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: Within 7 days
  - High: Within 14 days
  - Medium: Within 30 days
  - Low: Best effort in next release

### Disclosure Process

1. Reporter submits vulnerability details
2. Maintainer acknowledges receipt and begins assessment
3. Maintainer validates the vulnerability and determines impact
4. Fix is developed and tested privately
5. Security patch is released
6. Public disclosure occurs after users have had reasonable time to update

## Security Considerations

### Compiler Output

QBNex compiles BASIC source code into native binaries. Consider the following:

- **Source Code Validation**: The compiler performs syntax validation but does not scan for malicious payloads in source code
- **Generated Binaries**: Compiled binaries execute with the permissions of the user running them
- **System Access**: QBNex programs can access files, network sockets, and system memory via PEEK/POKE

### File I/O Security

QBNex provides full file system access. Be aware:

- Programs can read, write, and delete files accessible to the user
- Always validate file paths before operations
- Be cautious with user-supplied file paths
- Directory traversal vulnerabilities are possible if not properly handled

### Network Security

TCP/IP networking support enables:

- Server sockets (`_OPENHOST`)
- Client connections (`_OPENCLIENT`)
- Data transmission over networks

**Security recommendations**:
- Validate all network input
- Use appropriate port ranges
- Be aware that network data is unencrypted by default
- Implement application-level security for sensitive data

### Memory Operations

Low-level memory access features:

- `PEEK` / `POKE`: Direct memory access
- `DEF SEG`: Memory segment control
- `VARPTR`, `VARSEG`: Variable address exposure
- `CALL ABSOLUTE`: Execute machine code at specified addresses

**Warning**: These features can cause system instability or security breaches if misused. Use with extreme caution.

### Command Execution

- `SHELL`: Execute operating system commands
- `CHAIN`: Load and execute other BASIC programs

These features provide OS-level access and should be used carefully with untrusted input.

### External Dependencies

QBNex relies on several third-party libraries:

- **OpenGL/FreeGLUT**: Graphics rendering
- **GLEW**: OpenGL extension loading
- **miniaudio**: Cross-platform audio output
- **FreeType**: TrueType font rendering
- **STB Image**: Image format loading
- **Platform-specific libraries**: ALSA (Linux), CoreAudio (macOS), WinMM (Windows)

We monitor these dependencies for security updates and incorporate patches as needed.

## Best Practices for QBNex Developers

### Writing Secure QBNex Code

1. **Validate all input**: Never trust user input or external data
2. **File path sanitization**: Check for directory traversal attempts
3. **Network input validation**: Verify all received data
4. **Error handling**: Use proper error handling to prevent unexpected behavior
5. **Resource cleanup**: Close files and network connections properly
6. **Avoid hardcoded paths**: Use relative paths or configuration files
7. **Limit permissions**: Run compiled programs with minimal necessary permissions

### Distribution Security

When distributing QBNex programs:

1. **Scan binaries**: Use antivirus software before distribution
2. **Document permissions**: Inform users of required system access
3. **Sign binaries**: Consider code signing for Windows distributions
4. **Provide source**: Open distribution allows verification

### Compiler Security

The QBNex compiler itself:

- Runs with user permissions only
- Does not require elevated privileges
- Generates self-contained binaries
- Can include source in output for transparency (`SaveExeWithSource` setting)

## Known Limitations

- No built-in encryption for network communications
- No sandboxing or sandboxing support for compiled programs
- Direct memory access can bypass safety mechanisms
- File operations use OS-level permissions only

## Security Updates

Security updates will be released as:

- **Patch releases** for critical vulnerabilities (e.g., `1.0.1`)
- **Minor releases** for security feature additions (e.g., `1.1.0`)

All security-related changes will be documented in [CHANGELOG.md](CHANGELOG.md).

## Acknowledgments

We appreciate responsible disclosure from the security community and will credit reporters in security advisories (with permission).

---

**Last Updated**: April 11, 2024

**Contact**: For security concerns, contact the project maintainer through GitHub Issues (for non-sensitive matters) or email for sensitive vulnerabilities.
