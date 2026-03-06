use crate::opcodes::OpCode;
use core_types::{QResult, QType};

/// Check if a function name is a built-in QBASIC function
pub fn is_builtin_function(name: &str) -> bool {
    let upper = name.to_uppercase();
    matches!(
        upper.as_str(),
        // String functions
        "LEFT$" | "RIGHT$" | "MID$" | "LEN" | "INSTR" | "LCASE$" | "UCASE$"
        | "LTRIM$" | "RTRIM$" | "TRIM$" | "STR$" | "VAL" | "CHR$" | "ASC"
        | "SPACE$" | "STRING$" | "HEX$" | "OCT$"
        // Math functions
        | "ABS" | "SGN" | "SIN" | "COS" | "TAN" | "ATN" | "EXP" | "LOG"
        | "SQR" | "INT" | "FIX" | "RND" | "RANDOMIZE"
        // Type conversion
        | "CINT" | "CLNG" | "CSNG" | "CDBL" | "CSTR"
        | "MKI$" | "MKL$" | "MKS$" | "MKD$"
        | "CVI" | "CVL" | "CVS" | "CVD"
        // Date/Time
        | "TIMER" | "DATE$" | "TIME$"
        // Array functions
        | "LBOUND" | "UBOUND"
        // File I/O
        | "EOF" | "LOF" | "FREEFILE" | "LOC" | "INPUT$"
        // System
        | "FRE" | "CSRLIN" | "POS" | "ENVIRON$" | "COMMAND$" | "INKEY$"
        // Memory/Hardware
        | "PEEK" | "VARPTR" | "VARSEG" | "SADD" | "VARPTR$"
        // Graphics
        | "POINT" | "PMAP"
        // Error handling
        | "ERR" | "ERL" | "ERDEV" | "ERDEV$"
    )
}

/// Get the number of arguments expected for a built-in function
pub fn get_builtin_arity(name: &str) -> Option<(usize, usize)> {
    match name.to_uppercase().as_str() {
        // Functions with 1 required argument
        "LEN" | "STR$" | "VAL" | "CHR$" | "ASC" | "LCASE$" | "UCASE$" | "LTRIM$" | "RTRIM$"
        | "TRIM$" | "HEX$" | "OCT$" | "ABS" | "SGN" | "SIN" | "COS" | "TAN" | "ATN" | "EXP"
        | "LOG" | "SQR" | "INT" | "FIX" | "CINT" | "CLNG" | "CSNG" | "CDBL" | "CSTR" | "MKI$"
        | "MKL$" | "MKS$" | "MKD$" | "CVI" | "CVL" | "CVS" | "CVD" | "TIMER" | "DATE$"
        | "TIME$" | "RND" | "FRE" | "CSRLIN" | "POS" | "COMMAND$" | "ENVIRON$" | "PEEK"
        | "VARPTR" | "VARSEG" | "SADD" | "VARPTR$" => Some((1, 1)),

        // Functions with 0 arguments
        "INKEY$" => Some((0, 0)),

        // Functions with 2 arguments
        "POINT" | "PMAP" => Some((2, 2)),

        // Functions with 0 arguments (error handling)
        "ERR" | "ERL" | "ERDEV" | "ERDEV$" => Some((0, 0)),

        // Functions with 2 required arguments
        "LEFT$" | "RIGHT$" | "SPACE$" | "STRING$" | "INSTR" => Some((2, 2)),

        // Functions with 2-3 arguments
        "MID$" => Some((2, 3)),

        // Functions with optional arguments
        "LBOUND" | "UBOUND" => Some((1, 2)), // array name, optional dimension
        "EOF" | "LOF" | "LOC" => Some((1, 1)), // file number
        "FREEFILE" => Some((0, 0)),
        "INPUT$" => Some((1, 2)),    // n, optional file number
        "RANDOMIZE" => Some((0, 1)), // optional seed

        _ => None,
    }
}

/// Compile a built-in function call to bytecode
pub fn compile_builtin_function(name: &str, args: &[OpCode]) -> QResult<Vec<OpCode>> {
    let mut bytecode = Vec::new();

    // Push arguments onto stack (already compiled)
    bytecode.extend_from_slice(args);

    match name.to_uppercase().as_str() {
        // String functions
        "LEFT$" => {
            if args.len() >= 2 {
                bytecode.push(OpCode::Left);
            }
        }
        "RIGHT$" => {
            if args.len() >= 2 {
                bytecode.push(OpCode::Right);
            }
        }
        "MID$" => {
            if args.len() >= 2 {
                bytecode.push(OpCode::Mid);
            }
        }
        "LEN" => bytecode.push(OpCode::Len),
        "INSTR" => bytecode.push(OpCode::InStr),
        "LCASE$" => bytecode.push(OpCode::LCase),
        "UCASE$" => bytecode.push(OpCode::UCase),
        "LTRIM$" => bytecode.push(OpCode::LTrim),
        "RTRIM$" => bytecode.push(OpCode::RTrim),
        "TRIM$" => bytecode.push(OpCode::Trim),
        "STR$" => bytecode.push(OpCode::StrFunc),
        "VAL" => bytecode.push(OpCode::ValFunc),
        "CHR$" => bytecode.push(OpCode::ChrFunc),
        "ASC" => bytecode.push(OpCode::AscFunc),
        "SPACE$" => {
            if !args.is_empty() {
                bytecode.push(OpCode::SpaceFunc);
            }
        }
        "STRING$" => {
            if args.len() >= 2 {
                bytecode.push(OpCode::StringFunc);
            }
        }
        "HEX$" => {
            // Convert to hex string
            bytecode.push(OpCode::HexFunc);
        }
        "OCT$" => {
            // Convert to octal string
            bytecode.push(OpCode::OctFunc);
        }

        // Math functions
        "ABS" => bytecode.push(OpCode::Abs),
        "SGN" => bytecode.push(OpCode::Sgn),
        "SIN" => bytecode.push(OpCode::Sin),
        "COS" => bytecode.push(OpCode::Cos),
        "TAN" => bytecode.push(OpCode::Tan),
        "ATN" => bytecode.push(OpCode::Atn),
        "EXP" => bytecode.push(OpCode::ExpFunc),
        "LOG" => bytecode.push(OpCode::LogFunc),
        "SQR" => bytecode.push(OpCode::Sqr),
        "INT" => bytecode.push(OpCode::IntFunc),
        "FIX" => bytecode.push(OpCode::Fix),
        "RND" => bytecode.push(OpCode::Rnd),

        // Type conversion
        "CINT" => bytecode.push(OpCode::CInt),
        "CLNG" => bytecode.push(OpCode::CLng),
        "CSNG" => bytecode.push(OpCode::CSng),
        "CDBL" => bytecode.push(OpCode::CDbl),
        "CSTR" => bytecode.push(OpCode::CStr),

        // MKx$ functions - convert numeric to binary string representation
        "MKI$" => {
            // Make integer string (2 bytes)
            bytecode.push(OpCode::MkiFunc);
        }
        "MKL$" => {
            // Make long string (4 bytes)
            bytecode.push(OpCode::MklFunc);
        }
        "MKS$" => {
            // Make single string (4 bytes)
            bytecode.push(OpCode::MksFunc);
        }
        "MKD$" => {
            // Make double string (8 bytes)
            bytecode.push(OpCode::MkdFunc);
        }

        // CVx functions - convert binary string to numeric
        "CVI" => {
            // Convert string to integer
            bytecode.push(OpCode::CviFunc);
        }
        "CVL" => {
            // Convert string to long
            bytecode.push(OpCode::CvlFunc);
        }
        "CVS" => {
            // Convert string to single
            bytecode.push(OpCode::CvsFunc);
        }
        "CVD" => {
            // Convert string to double
            bytecode.push(OpCode::CvdFunc);
        }

        // Date/Time
        "TIMER" => bytecode.push(OpCode::Timer),
        "DATE$" => bytecode.push(OpCode::Date),
        "TIME$" => bytecode.push(OpCode::Time),

        // Array functions
        "LBOUND" => {
            // Pop array name from args and get lower bound
            bytecode.push(OpCode::LBound("".to_string(), 1));
        }
        "UBOUND" => {
            // Pop array name from args and get upper bound
            bytecode.push(OpCode::UBound("".to_string(), 1));
        }

        // File I/O
        "EOF" => {
            if let Some(OpCode::LoadConstant(QType::Integer(n))) = args.first() {
                bytecode.push(OpCode::Eof(n.to_string()));
            }
        }
        "LOF" => {
            if let Some(OpCode::LoadConstant(QType::Integer(n))) = args.first() {
                bytecode.push(OpCode::Lof(n.to_string()));
            }
        }
        "FREEFILE" => bytecode.push(OpCode::FreeFile),
        "LOC" => {
            // File position - similar to LOF for now
            if let Some(OpCode::LoadConstant(QType::Integer(n))) = args.first() {
                bytecode.push(OpCode::Lof(n.to_string()));
            }
        }
        "INPUT$" => {
            // Read n characters from file or keyboard
            if args.len() >= 2 {
                // File version
                bytecode.push(OpCode::Input);
            } else {
                // Keyboard version
                bytecode.push(OpCode::Input);
            }
        }

        // System functions
        "FRE" => {
            // Return free memory based on argument type
            // FRE("") = string memory, FRE(-1) = array memory, FRE(-2) = stack memory
            let arg_type = if let Some(OpCode::LoadConstant(QType::Integer(n))) = args.first() {
                *n as i32
            } else if let Some(OpCode::LoadConstant(QType::String(_))) = args.first() {
                0
            } else {
                0
            };
            bytecode.push(OpCode::FreFunc(arg_type));
        }
        "CSRLIN" => {
            // Current cursor row
            bytecode.push(OpCode::CsrLinFunc);
        }
        "POS" => {
            // Current cursor column
            let arg = if let Some(OpCode::LoadConstant(QType::Integer(n))) = args.first() {
                *n as i32
            } else {
                0
            };
            bytecode.push(OpCode::PosFunc(arg));
        }
        "ENVIRON$" => {
            // Environment variable
            bytecode.push(OpCode::EnvironFunc);
        }
        "COMMAND$" => {
            // Command line arguments
            bytecode.push(OpCode::CommandFunc);
        }
        "INKEY$" => {
            // Get key press without waiting
            bytecode.push(OpCode::InKeyFunc);
        }
        "RANDOMIZE" => {
            // Initialize random number generator - noop for now
            if !args.is_empty() {
                bytecode.push(OpCode::Pop); // Pop seed if provided
            }
        }

        // Memory/Hardware functions (placeholders for compatibility)
        "PEEK" => {
            // Read memory byte
            if let Some(OpCode::LoadConstant(QType::Integer(addr))) = args.first() {
                bytecode.push(OpCode::PeekFunc(*addr as i32));
            }
        }
        "VARPTR" => {
            // Variable pointer - return placeholder address
            bytecode.push(OpCode::LoadConstant(QType::Long(0)));
        }
        "VARSEG" => {
            // Variable segment - return placeholder segment
            bytecode.push(OpCode::LoadConstant(QType::Integer(0)));
        }
        "SADD" => {
            // String address - return placeholder address
            bytecode.push(OpCode::LoadConstant(QType::Long(0)));
        }
        "VARPTR$" => {
            // Variable pointer as string
            bytecode.push(OpCode::LoadConstant(QType::String(String::from(
                "\0\0\0\0",
            ))));
        }

        // Graphics functions
        "POINT" => {
            // Get pixel color at coordinates
            if args.len() >= 2 {
                if let (
                    Some(OpCode::LoadConstant(QType::Integer(x))),
                    Some(OpCode::LoadConstant(QType::Integer(y))),
                ) = (args.first(), args.get(1))
                {
                    bytecode.push(OpCode::PointFunc(*x as i32, *y as i32));
                }
            }
        }
        "PMAP" => {
            // Map coordinates between physical and logical
            if args.len() >= 2 {
                if let (
                    Some(OpCode::LoadConstant(QType::Single(coord))),
                    Some(OpCode::LoadConstant(QType::Integer(func))),
                ) = (args.first(), args.get(1))
                {
                    bytecode.push(OpCode::PMapFunc(*coord as f64, *func as i32));
                }
            }
        }

        // Error handling functions
        "ERR" => {
            // Current error number
            bytecode.push(OpCode::Err);
        }
        "ERL" => {
            // Error line number
            bytecode.push(OpCode::Erl);
        }
        "ERDEV" => {
            // Device error code
            bytecode.push(OpCode::ErDev);
        }
        "ERDEV$" => {
            // Device error string
            bytecode.push(OpCode::ErDevStr);
        }

        _ => {}
    }

    Ok(bytecode)
}

/// Get the return type of a built-in function
pub fn get_builtin_return_type(name: &str) -> &'static str {
    match name.to_uppercase().as_str() {
        "LEFT$" | "RIGHT$" | "MID$" | "LCASE$" | "UCASE$" | "LTRIM$" | "RTRIM$" | "TRIM$"
        | "STR$" | "CHR$" | "SPACE$" | "STRING$" | "HEX$" | "OCT$" | "MKI$" | "MKL$" | "MKS$"
        | "MKD$" | "DATE$" | "TIME$" | "ENVIRON$" | "COMMAND$" | "INPUT$" | "INKEY$"
        | "VARPTR$" | "ERDEV$" => "STRING",

        "LEN" | "ASC" | "INSTR" | "SGN" | "INT" | "FIX" | "CINT" | "LBOUND" | "UBOUND" | "EOF"
        | "FREEFILE" | "LOC" | "CSRLIN" | "POS" | "CVI" | "ERR" | "ERL" | "ERDEV" | "PEEK"
        | "VARSEG" => "INTEGER",

        "ABS" | "SIN" | "COS" | "TAN" | "ATN" | "EXP" | "LOG" | "SQR" | "CLNG" | "CVL" | "FRE"
        | "LOF" | "VARPTR" => "LONG",

        "TIMER" | "RND" | "CSNG" | "CVS" => "SINGLE",

        "CDBL" | "CVD" | "VAL" => "DOUBLE",

        "CSTR" => "STRING",

        _ => "VARIANT",
    }
}
