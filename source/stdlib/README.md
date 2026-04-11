# QBNex Standard Library (stdlib)

This directory contains the **Core Library Infrastructure** for QBNex — a set of standard
modules that extend QBNex BASIC into a full high-level language capable of OOP, data
manipulation, system integration, and complex module composition.

## Structure

```
stdlib/
├── oop/                  # Object-Oriented Programming foundation
│   ├── class.bas         # CLASS/END CLASS syntax + method dispatch
│   ├── interface.bas     # INTERFACE / IMPLEMENTS support
│   └── generics.bas      # Generic (parameterised) type helpers
│
├── collections/          # Data structure library
│   ├── list.bas          # Dynamic ordered list (ArrayList semantics)
│   ├── dictionary.bas    # Key-value map (hash-table backed)
│   ├── stack.bas         # LIFO stack
│   ├── queue.bas         # FIFO queue
│   └── set.bas           # Unique-value set
│
├── strings/              # Extended string operations
│   ├── strbuilder.bas    # Mutable string builder (avoids concat overhead)
│   ├── regex.bas         # Simple pattern-matching (glob + basic regex)
│   └── encoding.bas      # Base64, URL-encode/decode, UTF-8 helpers
│
├── math/                 # Extended mathematics
│   ├── vector.bas        # 2D/3D vector maths
│   ├── matrix.bas        # 4x4 matrix (for graphics transforms)
│   └── stats.bas         # Descriptive statistics helpers
│
├── io/                   # I/O helpers
│   ├── path.bas          # Cross-platform path manipulation
│   ├── csv.bas           # CSV reader/writer
│   └── json.bas          # Lightweight JSON serialiser/deserialiser
│
├── datetime/             # Date & time utilities
│   └── datetime.bas      # DateTime TYPE + arithmetic + formatting
│
├── error/                # Structured error handling
│   └── error.bas         # Error TYPE, try/catch macro helpers
│
└── sys/                  # System integration
    ├── env.bas           # Environment variable access wrappers
    ├── process.bas       # Shell/process launch helpers
    └── args.bas          # Command-line argument parsing helpers
```

## Usage

Include any module with `$INCLUDE`:

```basic
'$INCLUDE:'stdlib/collections/list.bas'
'$INCLUDE:'stdlib/oop/class.bas'
```

Or use the convenience umbrella:

```basic
'$INCLUDE:'stdlib/qbnex_stdlib.bas'
```
