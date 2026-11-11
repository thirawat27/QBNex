//! What the debuggee sends when it stops, and how values are shown.

use serde::{Deserialize, Serialize};

use qb_exec::debug::{self, VarScope};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "k", content = "v")]
pub enum Val {
    Int(i64),
    UInt(u64),
    Single(f32),
    Double(f64),
    Float(f64),
    /// raw bytes of the string
    Str(Vec<u8>),
    Array { dims: Vec<(i64, i64)>, elems: Vec<Val>, total: u64 },
    Udt { ty: String, fields: Vec<(String, Val)> },
    Unavailable(String),
}

impl From<debug::Value> for Val {
    fn from(v: debug::Value) -> Val {
        use debug::Value as V;
        match v {
            V::Int(i) => Val::Int(i),
            V::UInt(u) => Val::UInt(u),
            V::Single(f) => Val::Single(f),
            V::Double(f) => Val::Double(f),
            V::Float(f) => Val::Float(f),
            V::Str(s) => Val::Str(s),
            V::Array { dims, elems, total } => Val::Array { dims, elems: elems.into_iter().map(Val::from).collect(), total },
            V::Udt { ty, fields } => Val::Udt { ty, fields: fields.into_iter().map(|(n, v)| (n, Val::from(v))).collect() },
            V::Unavailable(s) => Val::Unavailable(s),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Var {
    pub qir: String,
    /// "global" | "static" | "param" | "local"
    pub scope: String,
    pub val: Val,
}

impl From<debug::VarValue> for Var {
    fn from(v: debug::VarValue) -> Var {
        let scope = match v.kind {
            VarScope::Global => "global",
            VarScope::Static => "static",
            VarScope::Param => "param",
            VarScope::Local => "local",
        };
        Var { qir: v.qir_name, scope: scope.into(), val: v.value.into() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub func: String,
    pub line: u32,
    pub locals: Vec<Var>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub reason: String,
    pub line: u32,
    /// innermost first
    pub frames: Vec<Frame>,
    pub globals: Vec<Var>,
}

/// Text of a string value: UTF-8 when valid, else CP437.
pub fn text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => qb_syntax::encoding::decode_cp437(bytes),
    }
}

fn float(f: f64) -> String {
    if f.is_finite() && f == f.trunc() && f.abs() < 1e15 { format!("{f:.0}") } else { format!("{f}") }
}

impl Val {
    /// One-line display.
    pub fn display(&self) -> String {
        match self {
            Val::Int(i) => i.to_string(),
            Val::UInt(u) => u.to_string(),
            Val::Single(f) => {
                if f.is_finite() && *f == f.trunc() && f.abs() < 1e7 { format!("{f:.0}") } else { format!("{f}") }
            }
            Val::Double(f) | Val::Float(f) => float(*f),
            Val::Str(s) => format!("{:?}", text(s)),
            Val::Array { dims, total, .. } => {
                let d: Vec<String> = dims.iter().map(|(lo, hi)| format!("{lo} TO {hi}")).collect();
                format!("({}) [{total}]", d.join(", "))
            }
            Val::Udt { ty, .. } => format!("{{{}}}", ty.strip_prefix("T_").unwrap_or(ty)),
            Val::Unavailable(why) => format!("<{why}>"),
        }
    }

    /// Type name for the variables view.
    pub fn type_name(&self) -> &'static str {
        match self {
            Val::Int(_) => "integer",
            Val::UInt(_) => "unsigned",
            Val::Single(_) => "SINGLE",
            Val::Double(_) => "DOUBLE",
            Val::Float(_) => "_FLOAT",
            Val::Str(_) => "STRING",
            Val::Array { .. } => "array",
            Val::Udt { .. } => "TYPE",
            Val::Unavailable(_) => "",
        }
    }

    pub fn is_composite(&self) -> bool {
        matches!(self, Val::Array { .. } | Val::Udt { .. })
    }

    /// Children of an array (as `(i, j)`) or UDT (field names, raw).
    pub fn children(&self) -> Vec<(String, Val)> {
        match self {
            Val::Array { dims, elems, .. } => {
                let mut idx: Vec<i64> = dims.iter().map(|d| d.0).collect();
                let mut out = Vec::new();
                for e in elems {
                    let name = format!("({})", idx.iter().map(i64::to_string).collect::<Vec<_>>().join(", "));
                    out.push((name, e.clone()));
                    for (i, d) in dims.iter().enumerate() {
                        if idx[i] < d.1 {
                            idx[i] += 1;
                            break;
                        }
                        idx[i] = d.0;
                    }
                }
                out
            }
            Val::Udt { fields, .. } => fields.clone(),
            _ => Vec::new(),
        }
    }

    /// Element at `index` (one subscript per dimension).
    pub fn element(&self, index: &[i64]) -> Option<&Val> {
        let Val::Array { dims, elems, .. } = self else { return None };
        if index.len() != dims.len() {
            return None;
        }
        let mut off = 0i64;
        let mut mul = 1i64;
        for (i, (lo, hi)) in index.iter().zip(dims) {
            if i < lo || i > hi {
                return None;
            }
            off += (i - lo) * mul;
            mul *= hi - lo + 1;
        }
        elems.get(usize::try_from(off).ok()?)
    }

    pub fn field(&self, name: &str) -> Option<&Val> {
        let Val::Udt { fields, .. } = self else { return None };
        fields.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_and_indexes() {
        assert_eq!(Val::Single(1.5).display(), "1.5");
        assert_eq!(Val::Single(3.0).display(), "3");
        assert_eq!(Val::Str(b"hi".to_vec()).display(), "\"hi\"");
        let a = Val::Array { dims: vec![(0, 1), (1, 2)], elems: (0..4).map(Val::Int).collect(), total: 4 };
        assert_eq!(a.display(), "(0 TO 1, 1 TO 2) [4]");
        assert_eq!(a.element(&[1, 2]), Some(&Val::Int(3)));
        assert_eq!(a.children()[1].0, "(1, 1)");
        assert_eq!(a.children()[2].0, "(0, 2)");
        let back: Val = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        assert_eq!(back, a);
    }
}
