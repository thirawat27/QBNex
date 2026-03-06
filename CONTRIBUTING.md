# Contributing to QBNex

Thank you for your interest in contributing to QBNex!

## Development Setup

1. Install Rust (1.75 or newer):
   ```bash
   rustup update stable
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/thirawat27/QBNex.git
   cd QBNex
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run tests:
   ```bash
   cargo test --all
   ```

## Code Quality Standards

### Before Submitting

1. **Format your code**:
   ```bash
   cargo fmt --all
   ```

2. **Run Clippy**:
   ```bash
   cargo clippy --all-targets -- -W clippy::all
   ```

3. **Run all tests**:
   ```bash
   cargo test --all
   ```

4. **Build documentation**:
   ```bash
   cargo doc --no-deps
   ```

### Code Style

- Follow Rust naming conventions
- Add doc comments for public APIs
- Include unit tests for new features
- Keep functions focused and small
- Use meaningful variable names

### Testing

- Add unit tests in the same file as the code
- Add integration tests in `tests/` directory
- Test edge cases and error conditions
- Aim for high code coverage

### Documentation

- Document all public functions and types
- Include examples in doc comments
- Update README.md for new features
- Add entries to CHANGELOG.md

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with clear commit messages
3. Ensure all tests pass
4. Update documentation as needed
5. Submit a pull request with a clear description

## Reporting Issues

When reporting bugs, please include:
- QBNex version
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Sample QBasic code if applicable

## Feature Requests

We welcome feature requests! Please:
- Check if the feature already exists
- Describe the use case clearly
- Provide examples if possible

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
