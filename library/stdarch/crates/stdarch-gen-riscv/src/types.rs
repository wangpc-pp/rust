//! Shared RVV type machinery: parsing `<vsew>m<lmul>` descriptors, mapping C
//! types to Rust, computing the LLVM `<vscale x N x eT>` mangling fragment and
//! emitting the Rust definitions of the scalable-vector types.

use std::collections::BTreeSet;
use std::fmt::Write as _;

pub const FEATURE: &str = "#[unstable(feature = \"stdarch_riscv_vector\", issue = \"114544\")]";

/// One parsed `<vsew>m<lmul>`-style RVV element type descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VType {
    /// LLVM element letter: 'i' (integer/bit container), 'f' (IEEE float).
    pub letter: char,
    pub sew: u32,
    /// Minimal lane count `N` in `<vscale x N x eT>`.
    pub n: u32,
    /// Number of vectors for tuple/segment types (1 for plain vectors).
    pub nf: u32,
}

/// Parse an RVV data-type suffix like `i32m1`, `u8mf4`, `f64m8`, `bf16m1`,
/// (optionally with a `x<nf>` tuple marker) into a `VType`.
pub fn parse_vtype(suffix: &str) -> Option<VType> {
    // Strip optional tuple marker `x<nf>`.
    let (core, nf) = if let Some(idx) = suffix.find('x') {
        let nf: u32 = suffix[idx + 1..].parse().ok()?;
        (&suffix[..idx], nf)
    } else {
        (suffix, 1)
    };
    // Element class prefix.
    let (letter, rest) = if let Some(r) = core.strip_prefix('i') {
        ('i', r)
    } else if let Some(r) = core.strip_prefix('u') {
        ('i', r)
    } else if let Some(r) = core.strip_prefix("bf") {
        // Rust has no `bf16` scalar type, so public BF16 vectors use `u16`
        // elements. Generic bit-preserving operations must consequently use
        // LLVM's integer-vector overloads. Semantic BF16 conversions keep
        // their explicit `bf16` mangling in the lowering oracle.
        ('i', r)
    } else if let Some(r) = core.strip_prefix('f') {
        ('f', r)
    } else {
        return None;
    };
    // SEW then LMUL (`m1`/`m2`/../`mf2`/..).
    let mpos = rest.find('m')?;
    let sew: u32 = rest[..mpos].parse().ok()?;
    let lmul = &rest[mpos..];
    let (num, den) = lmul_ratio(lmul)?;
    let numerator = 64 * num;
    let denominator = den * sew;
    if numerator % denominator != 0 {
        return None;
    }
    let n = numerator / denominator;
    if n == 0 {
        return None;
    }
    Some(VType { letter, sew, n, nf })
}

/// LMUL fragment (`m1`/`mf2`/...) to its `(numerator, denominator)` ratio.
pub fn lmul_ratio(lmul: &str) -> Option<(u32, u32)> {
    Some(match lmul {
        "m1" => (1, 1),
        "m2" => (2, 1),
        "m4" => (4, 1),
        "m8" => (8, 1),
        "mf2" => (1, 2),
        "mf4" => (1, 4),
        "mf8" => (1, 8),
        _ => return None,
    })
}

/// The LLVM `<vscale x N x eT>` mangling fragment, e.g. `nxv2i32`.
pub fn nxv(v: VType) -> String {
    format!("nxv{}{}{}", v.n, v.letter, v.sew)
}

/// Map a C type spelling to the Rust type used in the binding.
pub fn c_type_to_rust(c: &str) -> String {
    let c = c.trim();
    // Pointers: `const T *` / `T *`.
    if let Some(inner) = c.strip_suffix('*') {
        let inner = inner.trim();
        let (is_const, base) = match inner.strip_prefix("const ") {
            Some(b) => (true, b.trim()),
            None => (false, inner),
        };
        let rbase = c_scalar_to_rust(base);
        return if is_const {
            format!("*const {rbase}")
        } else {
            format!("*mut {rbase}")
        };
    }
    if let Some(v) = rvv_ctype_to_rust(c) {
        return v;
    }
    c_scalar_to_rust(c)
}

/// Map a C scalar type to Rust.
pub fn c_scalar_to_rust(c: &str) -> String {
    match c.trim() {
        "size_t" => "usize".to_string(),
        "ptrdiff_t" => "isize".to_string(),
        "int8_t" => "i8".to_string(),
        "uint8_t" => "u8".to_string(),
        "int16_t" => "i16".to_string(),
        "uint16_t" => "u16".to_string(),
        "int32_t" => "i32".to_string(),
        "uint32_t" => "u32".to_string(),
        "int64_t" => "i64".to_string(),
        "uint64_t" => "u64".to_string(),
        "float" => "f32".to_string(),
        "double" => "f64".to_string(),
        "_Float16" => "f16".to_string(),
        "__bf16" => "u16".to_string(), // bf16 carried as u16 bits
        // Other plain C integer spellings used by source prototypes.
        "unsigned int" | "unsigned" => "u32".to_string(),
        "int" | "signed int" | "signed" => "i32".to_string(),
        "unsigned long" => "u64".to_string(),
        "long" => "i64".to_string(),
        "char" => "i8".to_string(),
        "unsigned char" => "u8".to_string(),
        other => other.to_string(),
    }
}

/// RVV C vector type names (`vint32m1_t`, `vbool8_t`, ...) are used verbatim in
/// the Rust bindings (they are declared in `mod.rs`). Return `Some` when `c`
/// looks like one so we don't accidentally treat it as a scalar.
pub fn rvv_ctype_to_rust(c: &str) -> Option<String> {
    let c = c.trim();
    if (c.starts_with("vint")
        || c.starts_with("vuint")
        || c.starts_with("vfloat")
        || c.starts_with("vbfloat")
        || c.starts_with("vbool"))
        && c.ends_with("_t")
    {
        Some(c.to_string())
    } else {
        None
    }
}

/// Find the VType of a Rust RVV type name.
pub fn vtype_of_rust(r: &str) -> Option<VType> {
    let r = r.trim();
    let core = r.strip_suffix("_t")?;
    // FP8 types map to SEW=8, element `u8`.
    for fp8 in ["vfloat8e4m3", "vfloat8e5m2"] {
        if let Some(lmul) = core.strip_prefix(fp8) {
            return parse_vtype(&format!("u8{lmul}"));
        }
    }
    if let Some(rest) = core.strip_prefix("vint") {
        return parse_vtype(&format!("i{rest}"));
    }
    if let Some(rest) = core.strip_prefix("vuint") {
        return parse_vtype(&format!("u{rest}"));
    }
    if let Some(rest) = core.strip_prefix("vfloat") {
        return parse_vtype(&format!("f{rest}"));
    }
    if let Some(rest) = core.strip_prefix("vbfloat") {
        return parse_vtype(&format!("bf{rest}"));
    }
    None
}

/// The VType of a `vboolK_t` mask type (`<vscale x (64/K) x i1>`).
pub fn vbool_vtype(name: &str) -> Option<VType> {
    let k: u32 = name
        .strip_prefix("vbool")?
        .strip_suffix("_t")?
        .parse()
        .ok()?;
    let n = 64 / k;
    if n == 0 {
        return None;
    }
    Some(VType {
        letter: 'i',
        sew: 1,
        n,
        nf: 1,
    })
}

/// For a tuple type name like `vint32m1x2_t`, return the underlying single
/// vector name `vint32m1_t`.
pub fn single_vector_name(name: &str) -> Option<String> {
    let core = name.strip_suffix("_t")?;
    let idx = core.rfind('x')?;
    Some(format!("{}_t", &core[..idx]))
}

/// Collect every RVV type name used across all prototypes and emit their
/// definitions (`#[rustc_scalable_vector(N)]` structs, mask vectors, and
/// segment/tuple container types).
pub fn emit_type_defs<'a>(out: &mut String, type_names: impl Iterator<Item = &'a str>) {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for t in type_names {
        let t = t
            .trim()
            .trim_end_matches('*')
            .trim()
            .trim_start_matches("const ")
            .trim();
        if rvv_ctype_to_rust(t).is_some() {
            names.insert(t.to_string());
        }
    }
    for name in &names {
        if let Some(def) = type_def(name) {
            out.push_str(&def);
            out.push('\n');
        }
    }
}

/// Produce the Rust definition of a single RVV type name, or `None` if it
/// cannot be parsed.
pub fn type_def(name: &str) -> Option<String> {
    // Mask types: `vboolK_t` -> `<vscale x (64/K) x i1>`.
    if let Some(k) = name
        .strip_prefix("vbool")
        .and_then(|s| s.strip_suffix("_t"))
    {
        let k: u32 = k.parse().ok()?;
        let n = 64 / k;
        let header = type_header();
        return Some(format!(
            "{header}\n#[rustc_scalable_vector({n})]\npub struct {name}(bool);"
        ));
    }

    // Data / tuple types.
    let core = name.strip_suffix("_t")?;
    // FP8 (OFP8) types: `vfloat8e4m3<lmul>_t` / `vfloat8e5m2<lmul>_t`. SEW=8,
    // element carried as raw `u8` bits (no Rust fp8 primitive).
    for fp8 in ["vfloat8e4m3", "vfloat8e5m2"] {
        if let Some(lmul) = core.strip_prefix(fp8) {
            let v = parse_vtype(&format!("u8{lmul}"))?;
            let header = type_header();
            if v.nf == 1 {
                return Some(format!(
                    "{header}\n#[rustc_scalable_vector({n})]\npub struct {name}(u8);",
                    n = v.n,
                ));
            } else {
                let base = single_vector_name(name)?;
                let fields = std::iter::repeat(base)
                    .take(v.nf as usize)
                    .collect::<Vec<_>>()
                    .join(", ");
                return Some(format!(
                    "{header}\n#[rustc_scalable_vector]\npub struct {name}({fields});"
                ));
            }
        }
    }
    let (elem_rust, suffix) = if let Some(r) = core.strip_prefix("vint") {
        (
            format!("i{}", &r[..r.find(|c: char| c == 'm').unwrap_or(r.len())]),
            format!("i{r}"),
        )
    } else if let Some(r) = core.strip_prefix("vuint") {
        (
            format!("u{}", &r[..r.find(|c: char| c == 'm').unwrap_or(r.len())]),
            format!("u{r}"),
        )
    } else if let Some(r) = core.strip_prefix("vfloat") {
        (
            format!("f{}", &r[..r.find(|c: char| c == 'm').unwrap_or(r.len())]),
            format!("f{r}"),
        )
    } else if let Some(r) = core.strip_prefix("vbfloat") {
        // bf16 has no Rust primitive; carry the bits as u16.
        ("u16".to_string(), format!("bf{r}"))
    } else {
        return None;
    };
    let v = parse_vtype(&suffix)?;
    let header = type_header();
    if v.nf == 1 {
        Some(format!(
            "{header}\n#[rustc_scalable_vector({n})]\npub struct {name}({elem});",
            n = v.n,
            elem = elem_rust,
        ))
    } else {
        // Segment/tuple type: a container of `nf` identical scalable vectors.
        let base = single_vector_name(name)?;
        let fields = std::iter::repeat(base.clone())
            .take(v.nf as usize)
            .collect::<Vec<_>>()
            .join(", ");
        let _ = &elem_rust;
        Some(format!(
            "{header}\n#[rustc_scalable_vector]\npub struct {name}({fields});"
        ))
    }
}

fn type_header() -> String {
    format!("#[derive(Clone, Copy)]\n#[allow(non_camel_case_types)]\n{FEATURE}")
}

#[cfg(test)]
mod tests {
    use super::{nxv, parse_vtype, type_def};

    #[test]
    fn bf16_uses_i16_bit_container_mangling() {
        assert_eq!(nxv(parse_vtype("bf16mf4").unwrap()), "nxv1i16");
        assert_eq!(nxv(parse_vtype("bf16m8").unwrap()), "nxv32i16");
    }

    #[test]
    fn scalable_vector_types_do_not_derive_debug() {
        let mask = type_def("vbool32_t").unwrap();
        let vector = type_def("vint32m1_t").unwrap();
        let fp8_vector = type_def("vfloat8e4m3m1_t").unwrap();
        let bf16_vector = type_def("vbfloat16m1_t").unwrap();
        let tuple = type_def("vint32m1x2_t").unwrap();
        let fp8_tuple = type_def("vfloat8e4m3m1x2_t").unwrap();
        let bf16_tuple = type_def("vbfloat16m1x2_t").unwrap();

        for definition in [
            mask,
            vector,
            fp8_vector,
            bf16_vector,
            tuple,
            fp8_tuple,
            bf16_tuple.clone(),
        ] {
            assert_eq!(definition.lines().next(), Some("#[derive(Clone, Copy)]"));
            assert!(!definition.contains("Debug"));
        }
        assert!(bf16_tuple.contains("pub struct vbfloat16m1x2_t(vbfloat16m1_t, vbfloat16m1_t);"));
    }
}

/// The header comment prepended to `generated.rs`.
pub fn generated_header() -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "// This code is automatically generated. DO NOT MODIFY.\n\
         //\n\
         // Instead, edit `crates/stdarch-gen-riscv/spec/vector/rvv.yml` and re-run:\n\
         //\n\
         // ```\n\
         // cargo run -p stdarch-gen-riscv -- crates/stdarch-gen-riscv/spec/vector/rvv.yml\n\
         // ```\n\
         #![allow(improper_ctypes, non_snake_case, unused_variables, unused_unsafe, unused_braces, clippy::too_many_arguments)]\n\
         // Signed and unsigned RVV vector types lower to identical LLVM types, so\n\
         // the two `extern` declarations that share an `llvm.riscv.*` overload\n\
         // name are compatible even though Rust sees distinct nominal types.\n\
         #![allow(clashing_extern_declarations)]\n\n\
         #[cfg(test)]\n\
         use stdarch_test::assert_instr;\n\n\
         use super::*;\n\n"
    );
    out
}
