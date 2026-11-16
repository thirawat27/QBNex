//! Public procedures of the standard library modules (`http`,
//! `path`, `collections`, ...), generated from `compiler/stdlib` by
//! `tools/gen-library.ps1`. They are ordinary identifiers to the lexer; the
//! table only feeds hover and completion.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::library_data::LIBRARY;

#[derive(Clone, Debug)]
pub struct LibraryProc {
    /// As declared, suffix included (`Http_Get$`).
    pub name: &'static str,
    /// Module to `$IMPORT` (`http`).
    pub module: &'static str,
    pub is_function: bool,
    /// Declaration line (`FUNCTION Http_Get$ (url AS STRING, ...)`).
    pub syntax: &'static str,
    /// Comment above the declaration, if any.
    pub doc: &'static str,
}

fn base(name: &str) -> String {
    name.trim_end_matches(['%', '&', '!', '#', '$', '~', '`']).to_ascii_uppercase()
}

static TABLE: LazyLock<HashMap<String, LibraryProc>> = LazyLock::new(|| {
    LIBRARY
        .iter()
        .map(|&(name, module, is_function, syntax, doc)| (base(name), LibraryProc { name, module, is_function, syntax, doc }))
        .collect()
});

/// Looks a procedure up by name, case-insensitive, with or without its suffix.
pub fn lookup(name: &str) -> Option<&'static LibraryProc> {
    TABLE.get(&base(name))
}

/// Every library procedure, in no particular order.
pub fn all() -> impl Iterator<Item = &'static LibraryProc> {
    TABLE.values()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_public_procedures() {
        let h = lookup("http_get").unwrap();
        assert_eq!((h.name, h.module, h.is_function), ("Http_Get$", "http", true));
        assert!(lookup("Path_Join$").is_some_and(|p| p.module == "path"));
        assert!(lookup("List_Add").is_some_and(|p| p.module == "collections" && !p.is_function));
        assert!(lookup("qb_ext_http").is_none());
    }

    #[test]
    fn skips_internal_helpers() {
        for name in ["Coll_NewHandle", "Ext_Text$", "Core_SetError", "JsonAllocValue%", "UrlSchemeEnd&", "Path_Join2$", "Date_Part&"] {
            assert!(lookup(name).is_none(), "{name}");
        }
        assert!(lookup("Date_Parse#").is_some());
    }
}
