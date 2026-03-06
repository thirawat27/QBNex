# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in QBNex, please report it by:

1. **DO NOT** open a public issue
2. Email the maintainer at: [security contact]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will respond within 48 hours and work with you to address the issue.

## Security Considerations

### Input Validation

QBNex validates all user input including:
- Source code syntax
- File paths
- Command-line arguments
- Runtime values

### Memory Safety

QBNex is written in Rust, providing:
- Memory safety without garbage collection
- Thread safety
- Protection against buffer overflows
- No null pointer dereferences

### File System Access

- File I/O operations use safe Rust APIs
- Path traversal is prevented
- File permissions are respected

### Code Execution

- Compiled executables run in user space
- No elevated privileges required
- Sandboxing recommended for untrusted code

## Best Practices

When using QBNex:

1. **Validate Input**: Always validate QBasic source code from untrusted sources
2. **Limit Resources**: Set appropriate limits for memory and execution time
3. **Sandbox Execution**: Run untrusted programs in isolated environments
4. **Keep Updated**: Use the latest version of QBNex
5. **Review Code**: Audit QBasic programs before compilation

## Known Limitations

- No built-in sandboxing (use OS-level containers)
- File I/O has full user permissions
- No resource limits by default

## Acknowledgments

We thank security researchers who responsibly disclose vulnerabilities.
