use std::fmt;

/// QBasic data types
#[derive(Debug, Clone, PartialEq)]
pub enum QType {
    Integer(i16),
    Long(i32),
    Single(f32),
    Double(f64),
    String(String),
    UserDefined(Vec<u8>), // For TYPE...END TYPE structures
    Empty,
}

impl Eq for QType {}

impl QType {
    /// Convert QType to f64 for numeric operations
    pub fn to_f64(&self) -> f64 {
        match self {
            QType::Integer(i) => *i as f64,
            QType::Long(l) => *l as f64,
            QType::Single(s) => *s as f64,
            QType::Double(d) => *d,
            QType::String(s) => s.parse::<f64>().unwrap_or(0.0),
            QType::UserDefined(_) => 0.0,
            QType::Empty => 0.0,
        }
    }

    /// Convert QType to String representation
    pub fn as_string(&self) -> String {
        match self {
            QType::Integer(i) => i.to_string(),
            QType::Long(l) => l.to_string(),
            QType::Single(s) => s.to_string(),
            QType::Double(d) => d.to_string(),
            QType::String(s) => s.clone(),
            QType::UserDefined(_) => "[UserDefined]".to_string(),
            QType::Empty => String::new(),
        }
    }

    /// Get the type suffix character (%, &, !, #, $)
    pub fn type_suffix(&self) -> char {
        match self {
            QType::Integer(_) => '%',
            QType::Long(_) => '&',
            QType::Single(_) => '!',
            QType::Double(_) => '#',
            QType::String(_) => '$',
            QType::UserDefined(_) => ' ',
            QType::Empty => ' ',
        }
    }

    /// Create QType from suffix and value string
    pub fn from_suffix(suffix: char, value: &str) -> Option<Self> {
        match suffix {
            '%' => value.parse::<i16>().ok().map(QType::Integer),
            '&' => value.parse::<i32>().ok().map(QType::Long),
            '!' => value.parse::<f32>().ok().map(QType::Single),
            '#' => value.parse::<f64>().ok().map(QType::Double),
            '$' => Some(QType::String(value.to_string())),
            _ => None,
        }
    }

    /// Get default value for a type character
    pub fn default_for_type(c: char) -> Self {
        match c {
            '%' => QType::Integer(0),
            '&' => QType::Long(0),
            '!' => QType::Single(0.0),
            '#' => QType::Double(0.0),
            '$' => QType::String(String::new()),
            _ => QType::Empty,
        }
    }

    /// Check if the type is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            QType::Integer(_) | QType::Long(_) | QType::Single(_) | QType::Double(_)
        )
    }
}

impl fmt::Display for QType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QType::Integer(i) => write!(f, "{}", i),
            QType::Long(l) => write!(f, "{}", l),
            QType::Single(s) => write!(f, "{}", s),
            QType::Double(d) => write!(f, "{}", d),
            QType::String(s) => write!(f, "{}", s),
            QType::UserDefined(_) => write!(f, "[UserDefined]"),
            QType::Empty => write!(f, ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_f64() {
        assert_eq!(QType::Integer(42).to_f64(), 42.0);
        assert_eq!(QType::Long(1000).to_f64(), 1000.0);
        assert_eq!(QType::Single(2.5).to_f64(), 2.5f32 as f64);
        assert_eq!(QType::Double(2.5).to_f64(), 2.5);
        assert_eq!(QType::String("123".to_string()).to_f64(), 123.0);
        assert_eq!(QType::Empty.to_f64(), 0.0);
    }

    #[test]
    fn test_type_suffix() {
        assert_eq!(QType::Integer(0).type_suffix(), '%');
        assert_eq!(QType::Long(0).type_suffix(), '&');
        assert_eq!(QType::Single(0.0).type_suffix(), '!');
        assert_eq!(QType::Double(0.0).type_suffix(), '#');
        assert_eq!(QType::String(String::new()).type_suffix(), '$');
    }

    #[test]
    fn test_from_suffix() {
        assert_eq!(QType::from_suffix('%', "42"), Some(QType::Integer(42)));
        assert_eq!(QType::from_suffix('&', "1000"), Some(QType::Long(1000)));
        assert_eq!(
            QType::from_suffix('$', "hello"),
            Some(QType::String("hello".to_string()))
        );
    }

    #[test]
    fn test_default_for_type() {
        assert_eq!(QType::default_for_type('%'), QType::Integer(0));
        assert_eq!(QType::default_for_type('&'), QType::Long(0));
        assert_eq!(QType::default_for_type('$'), QType::String(String::new()));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", QType::Integer(42)), "42");
        assert_eq!(format!("{}", QType::String("test".to_string())), "test");
    }
}
