//! A single C-prototype parser.
//!
//! Each rendered template line is a C declaration
//! (`<ret> __riscv_<name>(<args>)`, `;` optional). This module parses one such
//! line into a [`Proto`].

/// A parsed C intrinsic prototype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proto {
    pub ret: String,
    pub name: String,
    /// (c_type, param_name)
    pub params: Vec<(String, String)>,
    /// One-line documentation carried from the owning family.
    pub section: String,
}

impl Proto {
    /// The short name with the `__riscv_` prefix stripped.
    pub fn short(&self) -> &str {
        self.name.trim_start_matches("__riscv_")
    }
}

/// Parse a single `ret __riscv_name(params);` prototype.
pub fn parse_proto(s: &str, section: &str) -> Option<Proto> {
    let s = s.trim().trim_end_matches(';').trim();
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    let head = s[..open].trim();
    let args = &s[open + 1..close];

    // The head is `<ret type...> __riscv_<name>`; split on the last whitespace.
    let sp = head.rfind(char::is_whitespace)?;
    let ret = head[..sp].trim().to_string();
    let name = head[sp..].trim().to_string();
    if !name.starts_with("__riscv_") {
        return None;
    }

    let mut params = Vec::new();
    if args.trim() != "void" && !args.trim().is_empty() {
        for raw in args.split(',') {
            let raw = raw.trim();
            let sp = raw.rfind(char::is_whitespace)?;
            let ty = raw[..sp].trim().to_string();
            let pname = raw[sp..].trim().trim_start_matches('*').to_string();
            // `*` may attach to the name; keep it on the type instead.
            let (ty, pname) = if raw[sp..].trim_start().starts_with('*') {
                (format!("{ty} *"), pname)
            } else {
                (ty, pname)
            };
            params.push((ty, pname));
        }
    }
    Some(Proto {
        ret,
        name,
        params,
        section: section.to_string(),
    })
}
