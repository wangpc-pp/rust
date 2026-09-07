//! Lowering of parsed C prototypes to Rust bindings.
//!
//! Every prototype becomes a `pub unsafe fn` with an equivalent Rust signature.
//! Its body is produced, in priority order, by:
//!
//!   1. **the lowering oracle** (`spec/vector/rvv_lowering.tsv`) — for the vast
//!      majority of intrinsics this gives the exact `llvm.riscv.*` name and the
//!      operand order verbatim from clang's own emitted IR, so we simply
//!      reconstruct the `extern` and the call;
//!   2. **pure-IR / subvector-tuple handlers** — `vundefined`, `vreinterpret`,
//!      `vget`/`vset`/`vcreate`/`vlmul_ext`/`vlmul_trunc`, whose result is an
//!      `insert`/`extract`/field access rather than a `riscv` intrinsic;
//!   3. **segment lowering** — compiler bridges convert Rust's scalable-vector
//!      tuple representation to LLVM's opaque `target("riscv.vector.tuple")`
//!      type, preserving each operation as one native segment instruction;
//!   4. a documented `unimplemented!()` stub for the few families with no LLVM
//!      intrinsic in this toolchain (the batched/quad dot-product extensions).

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::adoc::Proto;
use crate::types::*;

const LOWERING_FORMAT: &str = "rvv-lowering-v2";

/// One lowering recipe from the oracle: the concrete `llvm.riscv.*` name (with
/// `iXLEN` marking target-dependent integer overloads) and the ordered operand
/// sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub llvm: String,
    pub ops: Vec<Op>,
}

/// The LLVM integer type of a literal recipe operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntTy {
    /// An integer whose width is fixed on every target.
    Fixed(u32),
    /// An integer whose width follows the RISC-V XLEN.
    XLen,
}

/// An operand source in a recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// A poison/undef passthru value of the result type.
    Poison,
    /// A literal integer immediate whose type is carried by its operand slot.
    Const { value: i64, ty: IntTy },
    /// The C parameter at this 0-based index.
    Param(usize),
    /// The (sole) pointer parameter.
    Ptr,
}

impl Op {
    pub const fn fixed(value: i64, width: u32) -> Self {
        Self::Const {
            value,
            ty: IntTy::Fixed(width),
        }
    }

    pub const fn xlen(value: i64) -> Self {
        Self::Const {
            value,
            ty: IntTy::XLen,
        }
    }
}

/// Parse the oracle TSV into a map keyed by the full C intrinsic name.
/// Malformed, duplicate, and out-of-order rows are rejected instead of being
/// silently skipped or redirected to parameter zero.
pub fn load_oracle(text: &str) -> Result<HashMap<String, Recipe>, String> {
    let mut map = HashMap::new();
    let mut previous = None;
    let mut format = None;
    for (zero_line, raw) in text.lines().enumerate() {
        let line_no = zero_line + 1;
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            if let Some(value) = comment.trim().strip_prefix("format:") {
                let value = value.trim();
                if format.replace(value.to_string()).is_some() {
                    return Err(format!("line {line_no}: duplicate format header"));
                }
            }
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [name, llvm, operands] = fields.as_slice() else {
            return Err(format!(
                "line {line_no}: expected 3 tab-separated fields, found {}",
                fields.len()
            ));
        };
        if !name.starts_with("__riscv_")
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(format!("line {line_no}: invalid C intrinsic `{name}`"));
        }
        if !llvm.starts_with("llvm.riscv.")
            || !llvm
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_'))
        {
            return Err(format!("line {line_no}: invalid LLVM intrinsic `{llvm}`"));
        }
        if previous.is_some_and(|previous: &str| previous >= *name) {
            return Err(format!(
                "line {line_no}: oracle is not strictly sorted at `{name}`"
            ));
        }
        previous = Some(*name);
        if operands.is_empty() {
            return Err(format!("line {line_no}: empty operand recipe for `{name}`"));
        }
        let ops = operands
            .split(',')
            .map(|tok| {
                if tok == "poison" {
                    Ok(Op::Poison)
                } else if tok == "ptr" {
                    Ok(Op::Ptr)
                } else if let Some(constant) = tok.strip_prefix('#') {
                    parse_constant(constant)
                        .ok_or_else(|| format!("line {line_no}: invalid constant `{tok}`"))
                } else if !tok.is_empty() && tok.bytes().all(|c| c.is_ascii_digit()) {
                    tok.parse()
                        .map(Op::Param)
                        .map_err(|_| format!("line {line_no}: invalid parameter `{tok}`"))
                } else {
                    Err(format!("line {line_no}: invalid operand token `{tok}`"))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if map
            .insert(
                name.to_string(),
                Recipe {
                    llvm: llvm.to_string(),
                    ops,
                },
            )
            .is_some()
        {
            return Err(format!("line {line_no}: duplicate oracle entry `{name}`"));
        }
    }
    if map.is_empty() {
        return Err("lowering oracle has no entries".to_string());
    }
    match format.as_deref() {
        Some(LOWERING_FORMAT) => {}
        Some(other) => return Err(format!("unsupported lowering oracle format `{other}`")),
        None => return Err(format!("missing `# format: {LOWERING_FORMAT}` header")),
    }
    Ok(map)
}

fn parse_constant(text: &str) -> Option<Op> {
    let (ty, value) = text.split_once(':')?;
    let ty = if ty == "xlen" {
        IntTy::XLen
    } else {
        let width = ty.strip_prefix('i')?.parse().ok()?;
        if !matches!(width, 8 | 16 | 32 | 64) {
            return None;
        }
        IntTy::Fixed(width)
    };
    Some(Op::Const {
        value: value.parse().ok()?,
        ty,
    })
}

/// Emit one intrinsic into `out`. Returns `true` if a real body was produced and
/// `false` if it fell back to an `unimplemented!()` stub.
pub fn emit_one(
    out: &mut String,
    p: &Proto,
    features: &[String],
    oracle: &HashMap<String, Recipe>,
    families: &FamilyIndex,
) -> bool {
    let short = p.short();
    let rust_params: Vec<(String, String)> = p
        .params
        .iter()
        .map(|(t, n)| (c_type_to_rust(t), rust_param_name(n).to_string()))
        .collect();
    let ret_rust = if p.ret == "void" {
        None
    } else {
        Some(c_type_to_rust(&p.ret))
    };

    // 1. Pure-IR / subvector-tuple families (must precede the oracle: the oracle
    //    has no entries for these, and they need structural lowering anyway).
    if let Some((body, instr)) = special_body(short, &rust_params, ret_rust.as_deref()) {
        write_fn(
            out,
            short,
            &rust_params,
            ret_rust.as_deref(),
            features,
            &body,
            instr,
        );
        return true;
    }

    // 2. Segment loads/stores use compiler bridges because Rust cannot name
    //    LLVM's opaque RISC-V vector-tuple type directly.
    if is_segment(short) {
        if let Some((body, instr)) = segment_body(short, &rust_params, ret_rust.as_deref()) {
            write_fn(
                out,
                short,
                &rust_params,
                ret_rust.as_deref(),
                features,
                &body,
                instr,
            );
            return true;
        }
    }

    // 2b. Non-segment fault-only-first unit loads (`vle<eew>ff`). The oracle
    //     recipe links to `llvm.riscv.vleff`, whose result is an aggregate
    //     `{vector, iXLen}`. Route these through a compiler intrinsic that
    //     extracts both fields and writes the `size_t *new_vl` out-parameter.
    if is_unit_ff(short) {
        if let Some((body, instr)) = unit_ff_body(short, &rust_params, ret_rust.as_deref()) {
            write_fn(
                out,
                short,
                &rust_params,
                ret_rust.as_deref(),
                features,
                &body,
                instr,
            );
            return true;
        }
    }

    // Zvfbfmin's two conversion families use real LLVM `bf16` semantics.
    // Their recipes are compact and deterministic, while the remaining BF16
    // surface either uses structural/segment bridges or derives an `i16`
    // bit-container overload from an existing sibling.
    if let Some(r) = crate::zvfbfmin::lowering_recipe(p) {
        let body = emit_from_recipe(short, &r, &rust_params, ret_rust.as_deref());
        write_fn(
            out,
            short,
            &rust_params,
            ret_rust.as_deref(),
            features,
            &body,
            Some("nop"),
        );
        return true;
    }

    // 3. Programmatic vector-crypto recipes. These encode the same Clang 22
    //    lowering as the oracle, including multiple iXLEN operands in a single
    //    mangled name on RV32.
    if let Some(r) = crate::vector_crypto::lowering_recipe(p) {
        let body = emit_from_recipe(short, &r, &rust_params, ret_rust.as_deref());
        write_fn(
            out,
            short,
            &rust_params,
            ret_rust.as_deref(),
            features,
            &body,
            Some("nop"),
        );
        return true;
    }

    // 3a. The lowering oracle (exact clang-derived recipe).
    if let Some(r) = oracle.get(&p.name) {
        let body = emit_from_recipe(short, r, &rust_params, ret_rust.as_deref());
        write_fn(
            out,
            short,
            &rust_params,
            ret_rust.as_deref(),
            features,
            &body,
            Some("nop"),
        );
        return true;
    }

    // 3b. Family-pattern derivation: a few type variants (e.g. the `m8`-result
    //     `vzip`/`vpaire`/`vunzipe`) have no clang test, but a sibling in the
    //     same family does. Reuse the sibling's operand recipe and rebuild the
    //     type mangling from this prototype's own types.
    if let Some(r) = families.derive(p, &rust_params, ret_rust.as_deref()) {
        let body = emit_from_recipe(short, &r, &rust_params, ret_rust.as_deref());
        write_fn(
            out,
            short,
            &rust_params,
            ret_rust.as_deref(),
            features,
            &body,
            Some("nop"),
        );
        return true;
    }

    // 4. No lowering available (the batched/quad dot-product extensions have no
    //    LLVM intrinsic in this toolchain): documented stub.
    let stub_params: Vec<(String, String)> = rust_params
        .iter()
        .map(|(t, n)| {
            let name = if matches!(n.as_str(), "FRM" | "VXRM") {
                n.clone()
            } else {
                format!("_{n}")
            };
            (t.clone(), name)
        })
        .collect();
    let body =
        "// No corresponding `llvm.riscv.*` intrinsic exists in this toolchain.\nunimplemented!()";
    write_fn(
        out,
        short,
        &stub_params,
        ret_rust.as_deref(),
        features,
        body,
        None,
    );
    false
}

/// Whether an intrinsic is only available on RV64. LLVM does not support an
/// EEW=64 index vector for RV32 indexed memory operations. Match the concrete
/// operation token so this covers both unit and segment indexed loads/stores
/// without conflating a 64-bit data type with a 64-bit index.
pub fn requires_riscv64(p: &Proto) -> bool {
    operation_requires_riscv64(p.short())
}

fn operation_requires_riscv64(short: &str) -> bool {
    short
        .split('_')
        .next()
        .is_some_and(|operation| operation.contains("ei64"))
}

/// Rust const parameters conventionally use upper-case names. Keep the spec's
/// C-style operand names for parsing and legacy-call generation, but use the
/// const-generic spelling in emitted signatures and intrinsic calls.
fn rust_param_name(name: &str) -> &str {
    match name {
        "frm" => "FRM",
        "vxrm" => "VXRM",
        "uimm" => "UIMM",
        _ => name,
    }
}

/// Family key: base op, operand-form marker, and masked-ness.
fn family_key(p: &Proto) -> (String, String, bool) {
    let s = p.short();
    let masked = s.ends_with("_m");
    let parts: Vec<&str> = s.split('_').collect();
    let base = parts.first().copied().unwrap_or("").to_string();
    let form = parts.get(1).copied().unwrap_or("").to_string();
    (base, form, masked)
}

/// Index of one representative resolved intrinsic per family, used to derive
/// recipes for type variants the oracle happens not to cover.
#[derive(Default)]
pub struct FamilyIndex {
    /// (base, form, masked) -> (sample recipe, sample prototype types).
    map: HashMap<(String, String, bool), (Recipe, Vec<VType>, Option<VType>)>,
}

impl FamilyIndex {
    /// Record a resolved (proto, recipe) as the family representative.
    pub fn record(&mut self, p: &Proto, r: &Recipe) {
        let key = family_key(p);
        if self.map.contains_key(&key) {
            return;
        }
        let ptypes: Vec<VType> = p
            .params
            .iter()
            .filter_map(|(t, _)| {
                let t = t.trim().trim_start_matches("const ").trim();
                vtype_of_rust(t).or_else(|| vbool_vtype(t))
            })
            .collect();
        let rtype = if p.ret == "void" {
            None
        } else {
            vtype_of_rust(&p.ret).or_else(|| vbool_vtype(&p.ret))
        };
        self.map.insert(key, (r.clone(), ptypes, rtype));
    }

    /// Derive a recipe for `p` from its family representative by rewriting each
    /// `nxv` fragment of the sibling's mangled name.
    ///
    /// A slot's fragment need not equal the C type's `nxv` directly (e.g. the
    /// `vzip` result mangles at 2x the lane count), so we key by *role*: for
    /// each `nxv` fragment in the sibling name, we find which role (return type
    /// or a parameter type) it belongs to — allowing an integer lane-count
    /// scale — and emit that role's fragment for *our* prototype at the same
    /// scale.
    fn derive(
        &self,
        p: &Proto,
        _params: &[(String, String)],
        _ret: Option<&str>,
    ) -> Option<Recipe> {
        let (sample, s_ptypes, s_rtype) = self.map.get(&family_key(p))?;

        let our_ptypes: Vec<VType> = p
            .params
            .iter()
            .filter_map(|(t, _)| {
                let t = t.trim().trim_start_matches("const ").trim();
                vtype_of_rust(t).or_else(|| vbool_vtype(t))
            })
            .collect();
        let our_rtype = if p.ret == "void" {
            None
        } else {
            vtype_of_rust(&p.ret).or_else(|| vbool_vtype(&p.ret))
        };

        // Roles, in a fixed order: return type first, then each parameter type.
        // Each role pairs the sibling's type with ours.
        let mut roles: Vec<(VType, VType)> = Vec::new();
        if let (Some(sr), Some(orr)) = (s_rtype, our_rtype) {
            roles.push((*sr, orr));
        }
        for (s, o) in s_ptypes.iter().zip(our_ptypes.iter()) {
            roles.push((*s, *o));
        }

        // Rewrite each `.nxv...` fragment of the sibling name in turn.
        let rewritten: Vec<String> = sample
            .llvm
            .split('.')
            .map(|seg| {
                let Some((letter, sew, n)) = parse_nxv(seg) else {
                    return seg.to_string();
                };
                // Find the role whose sibling type matches (same element, and a
                // lane count that divides this fragment's `n`).
                for (s, o) in &roles {
                    if s.letter == letter && s.sew == sew && n % s.n == 0 {
                        let scale = n / s.n;
                        return format!("nxv{}{}{}", o.n * scale, o.letter, o.sew);
                    }
                }
                seg.to_string()
            })
            .collect();
        Some(Recipe {
            llvm: rewritten.join("."),
            ops: sample.ops.clone(),
        })
    }
}

/// Parse an `nxv<N><letter><sew>` fragment into `(letter, sew, n)`.
fn parse_nxv(seg: &str) -> Option<(char, u32, u32)> {
    let rest = seg.strip_prefix("nxv")?;
    let split = rest.find(|c: char| c == 'i' || c == 'f')?;
    let n: u32 = rest[..split].parse().ok()?;
    let letter = rest[split..].chars().next()?;
    let sew: u32 = rest[split + 1..].parse().ok()?;
    Some((letter, sew, n))
}

/// Look up the oracle recipe for a prototype (direct name or alias), if any.
pub fn oracle_recipe<'a>(p: &Proto, oracle: &'a HashMap<String, Recipe>) -> Option<&'a Recipe> {
    oracle.get(&p.name)
}

/// Reconstruct the `extern` block and call for an oracle recipe.
fn emit_from_recipe(
    short: &str,
    r: &Recipe,
    params: &[(String, String)],
    ret: Option<&str>,
) -> String {
    // Rust exposes BF16 vectors as `u16` bit containers. Use the matching
    // integer-container overloads supplied by the RISC-V backend for operations
    // whose Rust ABI is deliberately bit-based, avoiding unsupported scalable
    // vector `i16` <-> `bfloat` bitcasts.
    let llvm = if crate::zvfbfmin::uses_bit_container_mangling(short) {
        r.llvm.replace("bf16", "i16")
    } else {
        r.llvm.clone()
    };
    // Expand every explicitly marked iXLEN component into target-specific link
    // names while leaving fixed-width integer overloads unchanged.
    let (l32, l64) = architecture_link_names(&llvm);

    // Build the extern signature and the call argument list from the operand
    // sources. Poison operands take the result type; pointer/param operands take
    // their Rust types.
    let ret_ty = ret.unwrap_or("()");
    let ptr_param = params
        .iter()
        .find(|(t, _)| t.starts_with("*const") || t.starts_with("*mut"));

    let mut sig_args: Vec<String> = Vec::new();
    let mut call_args: Vec<String> = Vec::new();
    for (i, op) in r.ops.iter().enumerate() {
        let (ty, val) = match op {
            Op::Poison => (ret_ty.to_string(), "undef_vec!()".to_string()),
            Op::Const { value, ty } => (
                match ty {
                    IntTy::Fixed(width) => format!("i{width}"),
                    IntTy::XLen => "usize".to_string(),
                },
                value.to_string(),
            ),
            Op::Ptr => {
                let (t, n) = ptr_param.expect("recipe references ptr but no pointer param");
                (t.clone(), n.clone())
            }
            Op::Param(idx) => {
                let (t, n) = &params[*idx];
                (t.clone(), n.clone())
            }
        };
        sig_args.push(format!("a{i}: {ty}"));
        call_args.push(val);
    }

    let ret_decl = ret.map(|r| format!(" -> {r}")).unwrap_or_default();
    format!(
        "unsafe extern \"llvm-intrinsic\" {{\n    \
         #[cfg_attr(target_arch = \"riscv32\", link_name = \"{l32}\")]\n    \
         #[cfg_attr(target_arch = \"riscv64\", link_name = \"{l64}\")]\n    \
         fn f({}){ret_decl};\n}}\n\
         unsafe {{ f({}) }}",
        sig_args.join(", "),
        call_args.join(", "),
    )
}

fn architecture_link_names(llvm: &str) -> (String, String) {
    if llvm.contains("iXLEN") {
        return (llvm.replace("iXLEN", "i32"), llvm.replace("iXLEN", "i64"));
    }
    (llvm.to_string(), llvm.to_string())
}

/// Emit the full `pub unsafe fn`.
fn write_fn(
    out: &mut String,
    name: &str,
    params: &[(String, String)],
    ret: Option<&str>,
    features: &[String],
    body: &str,
    assert_instr: Option<&str>,
) {
    let const_param = params.iter().enumerate().find(|(_, (_, param))| {
        matches!(param.as_str(), "FRM" | "VXRM")
            || crate::vector_crypto::immediate_max(name, param).is_some()
    });
    let args: Vec<String> = params
        .iter()
        .filter(|(_, param)| {
            !matches!(param.as_str(), "FRM" | "VXRM")
                && crate::vector_crypto::immediate_max(name, param).is_none()
        })
        .map(|(t, n)| format!("{n}: {t}"))
        .collect();
    let generics = const_param
        .map(|(_, (ty, name))| format!("<const {name}: {ty}>"))
        .unwrap_or_default();
    let ret_str = ret.map(|r| format!(" -> {r}")).unwrap_or_default();
    let feature_attr = features.join(",");
    if operation_requires_riscv64(name) {
        let _ = writeln!(out, "#[cfg(target_arch = \"riscv64\")]");
    }
    let _ = writeln!(out, "#[doc = \"{name}\"]");
    let _ = writeln!(out, "#[inline]");
    let _ = writeln!(out, "#[target_feature(enable = \"{feature_attr}\")]");
    let _ = writeln!(out, "{FEATURE}");
    if let Some(instr) = assert_instr {
        if let Some((_, (_, const_name))) = const_param {
            let _ = writeln!(
                out,
                "#[cfg_attr(test, assert_instr({instr}, {const_name} = 0))]"
            );
        } else {
            let _ = writeln!(out, "#[cfg_attr(test, assert_instr({instr}))]");
        }
    }
    if let Some((index, _)) = const_param {
        let _ = writeln!(out, "#[rustc_legacy_const_generics({index})]");
    }
    let _ = writeln!(
        out,
        "pub unsafe fn {name}{generics}({}){ret_str} {{",
        args.join(", ")
    );
    if let Some((_, (_, const_name))) = const_param {
        let max = match const_name.as_str() {
            "FRM" => 4,
            "VXRM" => 3,
            param => crate::vector_crypto::immediate_max(name, param)
                .expect("recognized const parameter"),
        };
        let _ = writeln!(out, "    static_assert!({const_name} <= {max});");
    }
    for line in body.lines() {
        let _ = writeln!(out, "    {line}");
    }
    let _ = writeln!(out, "}}");
}

/// The `undef_vec!` poison-passthru helper, emitted once near the top.
pub fn undef_vec_helper() -> &'static str {
    "macro_rules! undef_vec {\n    \
     () => {{\n        \
     union U<T: Copy> { u: (), v: T }\n        \
     unsafe { U { u: () }.v }\n    \
     }};\n\
     }\n\n"
}

// ---------------------------------------------------------------------------
// Pure-IR / subvector-tuple / config / scalar-move / mask families
// ---------------------------------------------------------------------------

/// Handle families the oracle does not (and cannot) cover with a `riscv`
/// intrinsic call. Returns `(body, assert_instr)`.
fn special_body(
    short: &str,
    params: &[(String, String)],
    ret: Option<&str>,
) -> Option<(String, Option<&'static str>)> {
    if short.starts_with("vundefined") {
        return Some(("undef_vec!()".to_string(), None));
    }
    if short.starts_with("vreinterpret") {
        return vreinterpret_body(params, ret?);
    }
    if short.starts_with("vlmul_ext_") {
        return lmul_ext_body(params, ret?);
    }
    if short.starts_with("vlmul_trunc_") {
        return lmul_trunc_body(params, ret?);
    }
    if short.starts_with("vget_") {
        return vget_body(params, ret?);
    }
    if short.starts_with("vset_") {
        return vset_body(params, ret?);
    }
    if short.starts_with("vcreate_") {
        return vcreate_body(params, ret?);
    }
    None
}

/// Reinterpret a data vector as another data vector, or between an LMUL=1
/// integer vector and a mask. Rust's layout size for a scalable boolean vector
/// is based on `bool`, so a generic transmute cannot represent the latter even
/// though RVV masks occupy vector-register bits. Route mask conversions through
/// the compiler bridge that implements Clang's full-mask insert/extract scheme.
fn vreinterpret_body(
    params: &[(String, String)],
    ret: &str,
) -> Option<(String, Option<&'static str>)> {
    let (src_ty, src) = (&params.first()?.0, &params.first()?.1);
    let src_is_mask = vbool_vtype(src_ty).is_some();
    let dst_is_mask = vbool_vtype(ret).is_some();

    if src_is_mask || dst_is_mask {
        if src_is_mask == dst_is_mask {
            return None;
        }
        return Some((
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vreinterpret::<{src_ty}, {ret}>({src}) }}"
            ),
            None,
        ));
    }

    Some((
        format!("unsafe {{ crate::intrinsics::transmute_unchecked({src}) }}"),
        None,
    ))
}

/// `vlmul_ext_v_<small>_<big>(value)`: widen LMUL by inserting into poison at 0.
fn lmul_ext_body(params: &[(String, String)], ret: &str) -> Option<(String, Option<&'static str>)> {
    let (val_ty, val) = (&params[0].0, &params[0].1);
    let (nbig, nsmall) = (nxv(vtype_of_rust(ret)?), nxv(vtype_of_rust(val_ty)?));
    let body = format!(
        "unsafe extern \"llvm-intrinsic\" {{\n    \
         #[link_name = \"llvm.vector.insert.{nbig}.{nsmall}\"]\n    \
         fn f(dst: {ret}, src: {val_ty}, idx: i64) -> {ret};\n}}\n\
         unsafe {{ f(undef_vec!(), {val}, 0) }}"
    );
    Some((body, None))
}

/// `vlmul_trunc_v_<big>_<small>(value)`: narrow LMUL by extracting at 0.
fn lmul_trunc_body(
    params: &[(String, String)],
    ret: &str,
) -> Option<(String, Option<&'static str>)> {
    let (val_ty, val) = (&params[0].0, &params[0].1);
    let (nbig, nsmall) = (nxv(vtype_of_rust(val_ty)?), nxv(vtype_of_rust(ret)?));
    let body = format!(
        "unsafe extern \"llvm-intrinsic\" {{\n    \
         #[link_name = \"llvm.vector.extract.{nsmall}.{nbig}\"]\n    \
         fn f(src: {val_ty}, idx: i64) -> {ret};\n}}\n\
         unsafe {{ f({val}, 0) }}"
    );
    Some((body, None))
}

/// `vget_v_<src>_<sub>(src, index)`: extract a sub-vector or read a tuple field.
fn vget_body(params: &[(String, String)], ret: &str) -> Option<(String, Option<&'static str>)> {
    let (src_ty, src) = (&params[0].0, &params[0].1);
    let idx = &params[1].1;
    let src_v = vtype_of_rust(src_ty)?;
    let sub_v = vtype_of_rust(ret)?;
    if src_v.nf > 1 {
        let arms = (0..src_v.nf)
            .map(|i| {
                format!(
                    "        {i} => crate::intrinsics::simd::scalable::sve_tuple_get::<_, _, {i}>({src}),"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Some((
            format!(
                "unsafe {{\n    match {idx} {{\n{arms}\n        _ => undef_vec!(),\n    }}\n}}"
            ),
            None,
        ));
    }
    let (nsrc, nsub) = (nxv(src_v), nxv(sub_v));
    let count = src_v.n / sub_v.n;
    let arms = (0..count)
        .map(|i| format!("        {i} => f({src}, {}),", i as u64 * sub_v.n as u64))
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "unsafe extern \"llvm-intrinsic\" {{\n    \
         #[link_name = \"llvm.vector.extract.{nsub}.{nsrc}\"]\n    \
         fn f(src: {src_ty}, idx: i64) -> {ret};\n}}\n\
         unsafe {{\n    match {idx} {{\n{arms}\n        _ => undef_vec!(),\n    }}\n}}"
    );
    Some((body, None))
}

/// `vset_v_<sub>_<dst>(dest, index, value)`: insert a sub-vector or set a field.
fn vset_body(params: &[(String, String)], ret: &str) -> Option<(String, Option<&'static str>)> {
    let (dst_ty, dst) = (&params[0].0, &params[0].1);
    let idx = &params[1].1;
    let (val_ty, val) = (&params[2].0, &params[2].1);
    let dst_v = vtype_of_rust(dst_ty)?;
    let sub_v = vtype_of_rust(val_ty)?;
    if dst_v.nf > 1 {
        let arms = (0..dst_v.nf)
            .map(|i| {
                format!(
                    "        {i} => crate::intrinsics::simd::scalable::sve_tuple_set::<_, _, {i}>({dst}, {val}),"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Some((
            format!("unsafe {{\n    match {idx} {{\n{arms}\n        _ => {dst},\n    }}\n}}"),
            None,
        ));
    }
    let (ndst, nsub) = (nxv(dst_v), nxv(sub_v));
    let count = dst_v.n / sub_v.n;
    let arms = (0..count)
        .map(|i| {
            format!(
                "        {i} => f({dst}, {val}, {}),",
                i as u64 * sub_v.n as u64
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "unsafe extern \"llvm-intrinsic\" {{\n    \
         #[link_name = \"llvm.vector.insert.{ndst}.{nsub}\"]\n    \
         fn f(dst: {dst_ty}, src: {val_ty}, idx: i64) -> {ret};\n}}\n\
         unsafe {{\n    match {idx} {{\n{arms}\n        _ => {dst},\n    }}\n}}"
    );
    Some((body, None))
}

/// `vcreate_v_...(v0, v1, ...)`: build a bigger vector or a tuple from pieces.
fn vcreate_body(params: &[(String, String)], ret: &str) -> Option<(String, Option<&'static str>)> {
    let ret_v = vtype_of_rust(ret)?;
    if ret_v.nf > 1 {
        let mut lines = String::new();
        let _ = writeln!(lines, "unsafe {{");
        let _ = writeln!(lines, "    let mut acc: {ret} = undef_vec!();");
        for (i, (_, n)) in params.iter().enumerate() {
            let _ = writeln!(
                lines,
                "    acc = crate::intrinsics::simd::scalable::sve_tuple_set::<_, _, {i}>(acc, {n});"
            );
        }
        let _ = writeln!(lines, "    acc");
        let _ = write!(lines, "}}");
        return Some((lines, None));
    }
    let sub_v = vtype_of_rust(&params[0].0)?;
    let (nret, nsub) = (nxv(ret_v), nxv(sub_v));
    let sub_ty = &params[0].0;
    let mut lines = String::new();
    let _ = writeln!(lines, "unsafe extern \"llvm-intrinsic\" {{");
    let _ = writeln!(
        lines,
        "    #[link_name = \"llvm.vector.insert.{nret}.{nsub}\"]"
    );
    let _ = writeln!(
        lines,
        "    fn f(dst: {ret}, src: {sub_ty}, idx: i64) -> {ret};"
    );
    let _ = writeln!(lines, "}}");
    let _ = writeln!(lines, "unsafe {{");
    let _ = writeln!(lines, "    let mut acc: {ret} = undef_vec!();");
    for (i, (_, n)) in params.iter().enumerate() {
        let _ = writeln!(
            lines,
            "    acc = f(acc, {n}, {});",
            i as u64 * sub_v.n as u64
        );
    }
    let _ = writeln!(lines, "    acc");
    let _ = write!(lines, "}}");
    Some((lines, None))
}

// ---------------------------------------------------------------------------
// Segment load/store lowering
// ---------------------------------------------------------------------------

/// A non-segment fault-only-first unit load (`vle8ff`, `vle32ff`, ...).
fn is_unit_ff(short: &str) -> bool {
    let op = short.split('_').next().unwrap_or("");
    op.starts_with("vle") && op.ends_with("ff") && !op.contains("seg")
}

/// Lower a non-segment `vle<eew>ff` fault-only-first load through the compiler
/// bridge, which extracts LLVM's `{vector, iXLen}` result and writes the new VL.
fn unit_ff_body(
    short: &str,
    params: &[(String, String)],
    ret: Option<&str>,
) -> Option<(String, Option<&'static str>)> {
    let ret = ret?;
    let ptr = params
        .iter()
        .find(|(t, _)| t.starts_with("*const"))?
        .1
        .clone();
    let new_vl = params
        .iter()
        .find(|(t, _)| t.starts_with("*mut usize"))?
        .1
        .clone();
    let mask = params.iter().find(|(t, _)| t.starts_with("vbool"));
    let load_policy = load_policy(short, params, mask.is_some())?;
    let body = if let Some((_, mask)) = mask {
        let immediate = load_policy.immediate?;
        format!(
            "unsafe {{ crate::intrinsics::simd::scalable::riscv_vleff_mask::<{ret}, _, _, {immediate}>({}, {ptr}, {mask}, {new_vl}, vl) }}",
            load_policy.passthru
        )
    } else {
        format!(
            "unsafe {{ crate::intrinsics::simd::scalable::riscv_vleff::<{ret}, _>({}, {ptr}, {new_vl}, vl) }}",
            load_policy.passthru
        )
    };
    Some((body, None))
}

/// A segment intrinsic name (`vlseg2e8`, `vsoxseg4ei16`, `vlsseg2e16ff`, ...).
fn is_segment(short: &str) -> bool {
    // Match `v{l,s}...seg<N>...` where a digit follows `seg`.
    if let Some(pos) = short.find("seg") {
        return short[pos + 3..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit());
    }
    false
}

/// Lower a segment load/store through a compiler bridge that preserves the
/// operation as one native RISC-V segment instruction.
fn segment_body(
    short: &str,
    params: &[(String, String)],
    ret: Option<&str>,
) -> Option<(String, Option<&'static str>)> {
    let kind = SegKind::parse(short)?;
    if kind.is_load {
        segment_load_body(short, &kind, params, ret?)
    } else {
        segment_store_body(&kind, params)
    }
}

/// Parsed shape of a segment op.
struct SegKind {
    is_load: bool,
    /// unit-stride / strided / indexed.
    mode: SegMode,
    /// fault-only-first (`ff`).
    ff: bool,
}

#[derive(Debug, PartialEq)]
enum SegMode {
    Unit,
    Strided,
    /// Ordered indexed accesses (`vloxseg`/`vsoxseg`).
    IndexedOrdered,
    /// Unordered indexed accesses (`vluxseg`/`vsuxseg`).
    IndexedUnordered,
}

impl SegKind {
    fn parse(short: &str) -> Option<SegKind> {
        let is_load = short.starts_with("vl");
        let mode = if short.contains("ssseg") || short.contains("lsseg") {
            SegMode::Strided
        } else if short.contains("oxseg") {
            SegMode::IndexedOrdered
        } else if short.contains("uxseg") {
            SegMode::IndexedUnordered
        } else {
            SegMode::Unit
        };
        // Fault-only-first is marked by an `ff` suffix on the *op* token (e.g.
        // `vlseg2e8ff_v_i8m1x2`); the full short name ends in the type suffix,
        // so test the base op token, not the whole name.
        let ff = short.split('_').next().unwrap_or("").ends_with("ff");
        Some(SegKind { is_load, mode, ff })
    }
}

fn segment_load_body(
    short: &str,
    kind: &SegKind,
    params: &[(String, String)],
    ret: &str,
) -> Option<(String, Option<&'static str>)> {
    let mask = params.iter().find(|(t, _)| t.starts_with("vbool"));
    let load_policy = load_policy(short, params, mask.is_some())?;
    // Fault-only-first loads additionally return the granted VL through the
    // `new_vl` out-parameter.
    if kind.ff {
        let ptr = params
            .iter()
            .find(|(t, _)| t.starts_with("*const"))?
            .1
            .clone();
        let new_vl = params
            .iter()
            .find(|(t, _)| t.starts_with("*mut usize"))?
            .1
            .clone();
        let body = if let Some((_, mask)) = mask {
            let immediate = load_policy.immediate?;
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vlsegff_mask::<{ret}, _, _, {immediate}>({}, {ptr}, {mask}, {new_vl}, vl) }}",
                load_policy.passthru
            )
        } else {
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vlsegff::<{ret}, _>({}, {ptr}, {new_vl}, vl) }}",
                load_policy.passthru
            )
        };
        return Some((body, None));
    }

    let ptr = params
        .iter()
        .find(|(t, _)| t.starts_with("*const"))?
        .1
        .as_str();
    let body = match (&kind.mode, mask) {
        (SegMode::Unit, None) => format!(
            "unsafe {{ crate::intrinsics::simd::scalable::riscv_vlseg::<{ret}, _>({}, {ptr}, vl) }}",
            load_policy.passthru
        ),
        (SegMode::Unit, Some((_, mask))) => {
            let immediate = load_policy.immediate?;
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vlseg_mask::<{ret}, _, _, {immediate}>({}, {ptr}, {mask}, vl) }}",
                load_policy.passthru
            )
        }
        (SegMode::Strided, None) => {
            let stride = params.iter().find(|(t, _)| t == "isize")?.1.as_str();
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vlsseg::<{ret}, _>({}, {ptr}, {stride}, vl) }}",
                load_policy.passthru
            )
        }
        (SegMode::Strided, Some((_, mask))) => {
            let stride = params.iter().find(|(t, _)| t == "isize")?.1.as_str();
            let immediate = load_policy.immediate?;
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vlsseg_mask::<{ret}, _, _, {immediate}>({}, {ptr}, {stride}, {mask}, vl) }}",
                load_policy.passthru
            )
        }
        (SegMode::IndexedOrdered | SegMode::IndexedUnordered, mask) => {
            let index = params
                .iter()
                .find(|(t, _)| {
                    t.starts_with("vuint") && vtype_of_rust(t).is_some_and(|v| v.nf == 1)
                })?
                .1
                .as_str();
            let bridge = if kind.mode == SegMode::IndexedOrdered {
                "riscv_vloxseg"
            } else {
                "riscv_vluxseg"
            };
            if let Some((_, mask)) = mask {
                let immediate = load_policy.immediate?;
                format!(
                    "unsafe {{ crate::intrinsics::simd::scalable::{bridge}_mask::<{ret}, _, _, _, {immediate}>({}, {ptr}, {index}, {mask}, vl) }}",
                    load_policy.passthru
                )
            } else {
                format!(
                    "unsafe {{ crate::intrinsics::simd::scalable::{bridge}::<{ret}, _, _>({}, {ptr}, {index}, vl) }}",
                    load_policy.passthru
                )
            }
        }
    };
    Some((body, None))
}

struct LoadPolicy {
    passthru: String,
    immediate: Option<usize>,
}

/// Map the public RVV policy suffix to the passthru and masked policy consumed
/// by the compiler bridge. Unsuffixed APIs retain their poison/tama lowering.
fn load_policy(short: &str, params: &[(String, String)], masked: bool) -> Option<LoadPolicy> {
    match crate::policy::bridge_operands(short, params) {
        None => Some(LoadPolicy {
            passthru: "undef_vec!()".to_string(),
            immediate: masked.then_some(3),
        }),
        Some((passthru, None)) if !masked => Some(LoadPolicy {
            passthru: passthru.to_string(),
            immediate: None,
        }),
        Some((passthru, immediate @ Some(_))) if masked => Some(LoadPolicy {
            passthru: passthru.to_string(),
            immediate,
        }),
        _ => None,
    }
}

fn segment_store_body(
    kind: &SegKind,
    params: &[(String, String)],
) -> Option<(String, Option<&'static str>)> {
    let ptr = params
        .iter()
        .find(|(t, _)| t.starts_with("*mut"))?
        .1
        .as_str();
    let val = params
        .iter()
        .find(|(t, _)| vtype_of_rust(t).map(|v| v.nf > 1).unwrap_or(false))?;
    let value = val.1.as_str();
    let mask = params.iter().find(|(t, _)| t.starts_with("vbool"));
    let body = match (&kind.mode, mask) {
        (SegMode::Unit, None) => format!(
            "unsafe {{ crate::intrinsics::simd::scalable::riscv_vsseg({value}, {ptr}, vl) }}"
        ),
        (SegMode::Unit, Some((_, mask))) => format!(
            "unsafe {{ crate::intrinsics::simd::scalable::riscv_vsseg_mask({value}, {ptr}, {mask}, vl) }}"
        ),
        (SegMode::Strided, None) => {
            let stride = params.iter().find(|(t, _)| t == "isize")?.1.as_str();
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vssseg({value}, {ptr}, {stride}, vl) }}"
            )
        }
        (SegMode::Strided, Some((_, mask))) => {
            let stride = params.iter().find(|(t, _)| t == "isize")?.1.as_str();
            format!(
                "unsafe {{ crate::intrinsics::simd::scalable::riscv_vssseg_mask({value}, {ptr}, {stride}, {mask}, vl) }}"
            )
        }
        (SegMode::IndexedOrdered | SegMode::IndexedUnordered, mask) => {
            let index = params
                .iter()
                .find(|(t, _)| {
                    t.starts_with("vuint") && vtype_of_rust(t).is_some_and(|v| v.nf == 1)
                })?
                .1
                .as_str();
            let bridge = if kind.mode == SegMode::IndexedOrdered {
                "riscv_vsoxseg"
            } else {
                "riscv_vsuxseg"
            };
            if let Some((_, mask)) = mask {
                format!(
                    "unsafe {{ crate::intrinsics::simd::scalable::{bridge}_mask({value}, {ptr}, {index}, {mask}, vl) }}"
                )
            } else {
                format!(
                    "unsafe {{ crate::intrinsics::simd::scalable::{bridge}({value}, {ptr}, {index}, vl) }}"
                )
            }
        }
    };
    Some((body, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emit_test_proto(proto: Proto, recipe: Recipe) -> String {
        let mut oracle = HashMap::new();
        oracle.insert(proto.name.clone(), recipe);
        let mut out = String::new();
        assert!(emit_one(
            &mut out,
            &proto,
            &["v".to_string()],
            &oracle,
            &FamilyIndex::default()
        ));
        out
    }

    #[test]
    fn lowering_oracle_rejects_invalid_and_duplicate_rows() {
        assert!(load_oracle("garbage\n").is_err());
        assert!(load_oracle("__riscv_x\tllvm.riscv.x.i64\toops\n").is_err());
        assert!(load_oracle("__riscv_x\tllvm.riscv.x.i64\t#wat\n").is_err());
        assert!(
            load_oracle("# format: rvv-lowering-v2\n__riscv_x\tllvm.riscv.x.i64\t#i7:0\n").is_err()
        );
        assert!(
            load_oracle("# format: rvv-lowering-v2\n__riscv_x\tllvm.riscv.x.iXLEN\t#xlen:0\n")
                .is_ok()
        );
        assert!(
            load_oracle(concat!(
                "# format: rvv-lowering-v2\n",
                "__riscv_x\tllvm.riscv.x.i64\t0\n",
                "__riscv_x\tllvm.riscv.x.i64\t0\n"
            ))
            .is_err()
        );
        assert!(
            load_oracle(concat!(
                "# format: rvv-lowering-v2\n",
                "__riscv_y\tllvm.riscv.y.i64\t0\n",
                "__riscv_x\tllvm.riscv.x.i64\t0\n"
            ))
            .is_err()
        );
    }

    #[test]
    fn vreinterpret_uses_compiler_bridge_for_mask_data_conversions() {
        for (src, dst) in [
            ("vint8m1_t", "vbool1_t"),
            ("vuint16m1_t", "vbool8_t"),
            ("vbool64_t", "vint8m1_t"),
            ("vbool8_t", "vuint16m1_t"),
        ] {
            let params = vec![(src.to_string(), "src".to_string())];
            let body = vreinterpret_body(&params, dst).unwrap().0;
            assert_eq!(
                body,
                format!(
                    "unsafe {{ crate::intrinsics::simd::scalable::riscv_vreinterpret::<{src}, {dst}>(src) }}"
                )
            );
        }
    }

    #[test]
    fn vreinterpret_keeps_data_conversions_as_transmutes() {
        let params = vec![("vint8m1_t".to_string(), "src".to_string())];
        assert_eq!(
            vreinterpret_body(&params, "vuint16m1_t").unwrap().0,
            "unsafe { crate::intrinsics::transmute_unchecked(src) }"
        );
    }

    #[test]
    fn ofp8_bf16_conversions_use_integer_bit_container_mangling() {
        let narrow = Proto {
            ret: "vfloat8e4m3mf8_t".to_string(),
            name: "__riscv_vfncvt_f_f_w_bf16mf4_f8e4m3mf8".to_string(),
            params: vec![
                ("vbfloat16mf4_t".to_string(), "vs2".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            section: String::new(),
        };
        let narrow_recipe = Recipe {
            llvm: "llvm.riscv.vfncvt.f.f.w.nxv1i8.nxv1bf16.iXLEN".to_string(),
            ops: vec![Op::Poison, Op::Param(0), Op::xlen(7), Op::Param(1)],
        };
        let narrow_out = emit_test_proto(narrow, narrow_recipe);
        assert!(narrow_out.contains("llvm.riscv.vfncvt.f.f.w.nxv1i8.nxv1i16.i64"));

        let widen = Proto {
            ret: "vbfloat16mf4_t".to_string(),
            name: "__riscv_vfwcvt_f_f_v_f8e5m2mf8_bf16mf4".to_string(),
            params: vec![
                ("vfloat8e5m2mf8_t".to_string(), "vs2".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            section: String::new(),
        };
        let widen_recipe = Recipe {
            llvm: "llvm.riscv.vfwcvt.f.f.v.alt.nxv1bf16.nxv1i8.iXLEN".to_string(),
            ops: vec![Op::Poison, Op::Param(0), Op::Param(1)],
        };
        let widen_out = emit_test_proto(widen, widen_recipe);
        assert!(widen_out.contains("llvm.riscv.vfwcvt.f.f.v.alt.nxv1i16.nxv1i8.i64"));
    }

    #[test]
    fn ordinary_bf16_lowerings_keep_bfloat_mangling() {
        let proto = Proto {
            ret: "vbfloat16mf4_t".to_string(),
            name: "__riscv_vfncvtbf16_f_f_w_bf16mf4".to_string(),
            params: vec![
                ("vfloat32mf2_t".to_string(), "vs2".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            section: String::new(),
        };
        let recipe = Recipe {
            llvm: "llvm.riscv.vfncvtbf16.f.f.w.nxv1bf16.nxv1f32.iXLEN".to_string(),
            ops: vec![Op::Poison, Op::Param(0), Op::xlen(7), Op::Param(1)],
        };
        let out = emit_test_proto(proto, recipe);
        assert!(out.contains("llvm.riscv.vfncvtbf16.f.f.w.nxv1bf16.nxv1f32.i64"));
    }

    #[test]
    fn zvzip_bf16_uses_integer_bit_container_overloads() {
        for (short, llvm, expected) in [
            (
                "vzip_vv_bf16mf4",
                "llvm.riscv.vzip.nxv2bf16.nxv1bf16.i64",
                "llvm.riscv.vzip.nxv2i16.nxv1i16.i64",
            ),
            (
                "vpaire_vv_bf16mf4",
                "llvm.riscv.vpaire.nxv1bf16.i64",
                "llvm.riscv.vpaire.nxv1i16.i64",
            ),
            (
                "vpairo_vv_bf16mf4",
                "llvm.riscv.vpairo.nxv1bf16.i64",
                "llvm.riscv.vpairo.nxv1i16.i64",
            ),
            (
                "vunzipe_v_bf16mf4",
                "llvm.riscv.vunzipe.nxv1bf16.nxv2bf16.i64",
                "llvm.riscv.vunzipe.nxv1i16.nxv2i16.i64",
            ),
            (
                "vunzipo_v_bf16mf4",
                "llvm.riscv.vunzipo.nxv1bf16.nxv2bf16.i64",
                "llvm.riscv.vunzipo.nxv1i16.nxv2i16.i64",
            ),
        ] {
            let recipe = Recipe {
                llvm: llvm.to_string(),
                ops: Vec::new(),
            };
            let out = emit_from_recipe(short, &recipe, &[], Some("vbfloat16mf4_t"));
            assert!(out.contains(expected), "{short}: {out}");
            assert!(!out.contains("bf16"), "{short}: {out}");
        }
    }

    #[test]
    fn zvzip_missing_surface_is_derived_from_sibling_recipes() {
        let mut families = FamilyIndex::default();
        for (name, llvm, ret, params) in [
            (
                "__riscv_vzip_vv_i16mf2",
                "llvm.riscv.vzip.nxv4i16.nxv2i16.i64",
                "vint16m1_t",
                vec![
                    ("vint16mf2_t", "vs2"),
                    ("vint16mf2_t", "vs1"),
                    ("size_t", "vl"),
                ],
            ),
            (
                "__riscv_vpaire_vv_i16mf4",
                "llvm.riscv.vpaire.nxv1i16.i64",
                "vint16mf4_t",
                vec![
                    ("vint16mf4_t", "vs2"),
                    ("vint16mf4_t", "vs1"),
                    ("size_t", "vl"),
                ],
            ),
            (
                "__riscv_vunzipe_v_i16mf4",
                "llvm.riscv.vunzipe.nxv1i16.nxv2i16.i64",
                "vint16mf4_t",
                vec![("vint16mf2_t", "vs2"), ("size_t", "vl")],
            ),
        ] {
            let proto = Proto {
                ret: ret.to_string(),
                name: name.to_string(),
                params: params
                    .into_iter()
                    .map(|(ty, arg)| (ty.to_string(), arg.to_string()))
                    .collect(),
                section: String::new(),
            };
            families.record(
                &proto,
                &Recipe {
                    llvm: llvm.to_string(),
                    ops: if name.contains("vunzipe") {
                        vec![Op::Poison, Op::Param(0), Op::Param(1)]
                    } else {
                        vec![Op::Poison, Op::Param(0), Op::Param(1), Op::Param(2)]
                    },
                },
            );
        }

        for (proto, expected) in [
            (
                Proto {
                    ret: "vint16mf2_t".to_string(),
                    name: "__riscv_vzip_vv_i16mf4".to_string(),
                    params: vec![
                        ("vint16mf4_t".to_string(), "vs2".to_string()),
                        ("vint16mf4_t".to_string(), "vs1".to_string()),
                        ("size_t".to_string(), "vl".to_string()),
                    ],
                    section: String::new(),
                },
                "llvm.riscv.vzip.nxv2i16.nxv1i16.i64",
            ),
            (
                Proto {
                    ret: "vbfloat16mf4_t".to_string(),
                    name: "__riscv_vpaire_vv_bf16mf4".to_string(),
                    params: vec![
                        ("vbfloat16mf4_t".to_string(), "vs2".to_string()),
                        ("vbfloat16mf4_t".to_string(), "vs1".to_string()),
                        ("size_t".to_string(), "vl".to_string()),
                    ],
                    section: String::new(),
                },
                "llvm.riscv.vpaire.nxv1i16.i64",
            ),
            (
                Proto {
                    ret: "vbfloat16mf4_t".to_string(),
                    name: "__riscv_vunzipe_v_bf16mf4".to_string(),
                    params: vec![
                        ("vbfloat16mf2_t".to_string(), "vs2".to_string()),
                        ("size_t".to_string(), "vl".to_string()),
                    ],
                    section: String::new(),
                },
                "llvm.riscv.vunzipe.nxv1i16.nxv2i16.i64",
            ),
        ] {
            assert_eq!(families.derive(&proto, &[], None).unwrap().llvm, expected);
        }
    }

    #[test]
    fn typed_constants_drive_fixed_and_xlen_operand_types() {
        for (name, llvm, ops, expected) in [
            (
                "vneg_v_i8m1",
                "llvm.riscv.vrsub.nxv8i8.i8.iXLEN",
                vec![Op::Poison, Op::Param(0), Op::fixed(0, 8), Op::Param(1)],
                "a2: i8, a3: usize",
            ),
            (
                "vnot_v_i32m1",
                "llvm.riscv.vxor.nxv2i32.i32.iXLEN",
                vec![Op::Poison, Op::Param(0), Op::fixed(-1, 32), Op::Param(1)],
                "a2: i32, a3: usize",
            ),
            (
                "vwcvt_x_x_v_i16m1",
                "llvm.riscv.vwadd.nxv4i16.nxv4i8.i8.iXLEN",
                vec![Op::Poison, Op::Param(0), Op::fixed(0, 8), Op::Param(1)],
                "a2: i8, a3: usize",
            ),
            (
                "vncvt_x_x_w_i8m1",
                "llvm.riscv.vnsrl.nxv8i8.nxv8i16.iXLEN.iXLEN",
                vec![Op::Poison, Op::Param(0), Op::xlen(0), Op::Param(1)],
                "a2: usize, a3: usize",
            ),
        ] {
            let params = vec![
                ("vint8m1_t".to_string(), "src".to_string()),
                ("usize".to_string(), "vl".to_string()),
            ];
            let recipe = Recipe {
                llvm: llvm.to_string(),
                ops,
            };
            let out = emit_from_recipe(name, &recipe, &params, Some("vint8m1_t"));
            assert!(out.contains(expected), "{name}: {out}");
            assert!(out.contains(&llvm.replace("iXLEN", "i64")), "{name}: {out}");
            assert!(out.contains(&llvm.replace("iXLEN", "i32")), "{name}: {out}");
        }

        let controls = Recipe {
            llvm: "llvm.riscv.test.mask.nxv8i8.iXLEN".to_string(),
            ops: vec![Op::Poison, Op::xlen(7), Op::Param(0), Op::xlen(3)],
        };
        let out = emit_from_recipe(
            "test",
            &controls,
            &[("usize".to_string(), "vl".to_string())],
            Some("vint8m1_t"),
        );
        assert!(out.contains("a1: usize, a2: usize, a3: usize"));
    }

    #[test]
    fn only_ei64_indexed_memory_requires_riscv64() {
        for name in [
            "vloxei64_v_i32m1",
            "vluxei64_v_i32m1_m",
            "vsoxei64_v_i32m1",
            "vsuxei64_v_i32m1_m",
            "vloxseg2ei64_v_i32m1x2",
            "vluxseg2ei64_v_i32m1x2_m",
            "vsoxseg2ei64_v_i32m1x2",
            "vsuxseg2ei64_v_i32m1x2_m",
        ] {
            assert!(operation_requires_riscv64(name), "{name}");
        }
        for name in [
            "vloxei32_v_u64m1",
            "vloxseg2ei32_v_u64m1x2",
            "vle64_v_u64m1",
            "vadd_vv_u64m1",
        ] {
            assert!(!operation_requires_riscv64(name), "{name}");
        }
    }

    #[test]
    fn ei64_api_has_riscv64_cfg() {
        let proto = Proto {
            ret: "vint32m1_t".to_string(),
            name: "__riscv_vloxei64_v_i32m1".to_string(),
            params: vec![
                ("const int32_t *".to_string(), "rs1".to_string()),
                ("vuint64m2_t".to_string(), "rs2".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            section: String::new(),
        };
        let recipe = Recipe {
            llvm: "llvm.riscv.vloxei.nxv2i32.nxv2i64.i64".to_string(),
            ops: vec![Op::Poison, Op::Ptr, Op::Param(1), Op::Param(2)],
        };
        let out = emit_test_proto(proto, recipe);

        assert!(out.starts_with("#[cfg(target_arch = \"riscv64\")]\n"));
    }

    #[test]
    fn vxrm_is_an_ixlen_const_generic_at_its_legacy_unmasked_position() {
        let proto = Proto {
            ret: "vint32m1_t".to_string(),
            name: "__riscv_vaadd_vv_i32m1".to_string(),
            params: vec![
                ("vint32m1_t".to_string(), "vs2".to_string()),
                ("vint32m1_t".to_string(), "vs1".to_string()),
                ("size_t".to_string(), "vxrm".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            section: String::new(),
        };
        let recipe = Recipe {
            llvm: "llvm.riscv.vaadd.nxv2i32.nxv2i32.i64".to_string(),
            ops: vec![
                Op::Poison,
                Op::Param(0),
                Op::Param(1),
                Op::Param(2),
                Op::Param(3),
            ],
        };
        let out = emit_test_proto(proto, recipe);

        assert!(out.contains("#[rustc_legacy_const_generics(2)]"));
        assert!(out.contains("#[cfg_attr(test, assert_instr(nop, VXRM = 0))]"));
        assert!(out.contains(
            "pub unsafe fn vaadd_vv_i32m1<const VXRM: usize>(vs2: vint32m1_t, vs1: vint32m1_t, vl: usize) -> vint32m1_t"
        ));
        assert!(out.contains("static_assert!(VXRM <= 3);"));
        assert!(out.contains(
            "fn f(a0: vint32m1_t, a1: vint32m1_t, a2: vint32m1_t, a3: usize, a4: usize)"
        ));
        assert!(out.contains("f(undef_vec!(), vs2, vs1, VXRM, vl)"));
        assert!(!out.contains("vxrm: usize"));
    }

    #[test]
    fn frm_is_an_ixlen_const_generic_at_its_legacy_masked_position() {
        let proto = Proto {
            ret: "vfloat32m1_t".to_string(),
            name: "__riscv_vfadd_vv_f32m1_rm_m".to_string(),
            params: vec![
                ("vbool32_t".to_string(), "vm".to_string()),
                ("vfloat32m1_t".to_string(), "vs2".to_string()),
                ("vfloat32m1_t".to_string(), "vs1".to_string()),
                ("size_t".to_string(), "frm".to_string()),
                ("size_t".to_string(), "vl".to_string()),
            ],
            section: String::new(),
        };
        let recipe = Recipe {
            llvm: "llvm.riscv.vfadd.mask.nxv2f32.nxv2f32.i64".to_string(),
            ops: vec![
                Op::Poison,
                Op::Param(1),
                Op::Param(2),
                Op::Param(0),
                Op::Param(3),
                Op::Param(4),
                Op::xlen(3),
            ],
        };
        let out = emit_test_proto(proto, recipe);

        assert!(out.contains("#[rustc_legacy_const_generics(3)]"));
        assert!(out.contains("#[cfg_attr(test, assert_instr(nop, FRM = 0))]"));
        assert!(out.contains(
            "pub unsafe fn vfadd_vv_f32m1_rm_m<const FRM: usize>(vm: vbool32_t, vs2: vfloat32m1_t, vs1: vfloat32m1_t, vl: usize) -> vfloat32m1_t"
        ));
        assert!(out.contains("static_assert!(FRM <= 4);"));
        assert!(out.contains("a4: usize"));
        assert!(out.contains("f(undef_vec!(), vs2, vs1, vm, FRM, vl, 3)"));
        assert!(!out.contains("frm: usize"));
    }

    #[test]
    fn unit_ff_uses_compiler_bridge() {
        let unmasked_params = vec![
            ("*const i8".to_string(), "rs1".to_string()),
            ("*mut usize".to_string(), "new_vl".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let unmasked = unit_ff_body("vle8ff_v_i8m1", &unmasked_params, Some("vint8m1_t"))
            .unwrap()
            .0;
        assert!(unmasked.contains("riscv_vleff::<vint8m1_t, _>"));
        assert!(unmasked.contains("undef_vec!(), rs1, new_vl, vl"));

        let masked_params = vec![
            ("vbool8_t".to_string(), "vm".to_string()),
            ("*const i8".to_string(), "rs1".to_string()),
            ("*mut usize".to_string(), "new_vl".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let masked = unit_ff_body("vle8ff_v_i8m1_m", &masked_params, Some("vint8m1_t"))
            .unwrap()
            .0;
        assert!(masked.contains("riscv_vleff_mask::<vint8m1_t, _, _, 3>"));
        assert!(masked.contains("undef_vec!(), rs1, vm, new_vl, vl"));
        assert!(!masked.contains("vl, 3"));
    }

    #[test]
    fn unit_ff_policy_uses_real_passthru_and_immediate() {
        let tu_params = vec![
            ("vint8m1_t".to_string(), "maskedoff".to_string()),
            ("*const i8".to_string(), "rs1".to_string()),
            ("*mut usize".to_string(), "new_vl".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let tu = unit_ff_body("vle8ff_v_i8m1_tu", &tu_params, Some("vint8m1_t"))
            .unwrap()
            .0;
        assert!(tu.contains("riscv_vleff::<vint8m1_t, _>(maskedoff"));
        assert!(!tu.contains("undef_vec!()"));

        for (suffix, immediate) in [("tum", 2), ("tumu", 0), ("mu", 1)] {
            let params = vec![
                ("vbool8_t".to_string(), "mask".to_string()),
                ("vint8m1_t".to_string(), "maskedoff".to_string()),
                ("*const i8".to_string(), "rs1".to_string()),
                ("*mut usize".to_string(), "new_vl".to_string()),
                ("usize".to_string(), "vl".to_string()),
            ];
            let body = unit_ff_body(
                &format!("vle8ff_v_i8m1_{suffix}"),
                &params,
                Some("vint8m1_t"),
            )
            .unwrap()
            .0;
            assert!(body.contains(&format!("riscv_vleff_mask::<vint8m1_t, _, _, {immediate}>")));
            assert!(body.contains("(maskedoff, rs1, mask, new_vl, vl)"));
            assert!(!body.contains("undef_vec!()"));
        }
    }

    #[test]
    fn segment_ff_uses_compiler_bridge() {
        let unmasked_params = vec![
            ("*const i8".to_string(), "rs1".to_string()),
            ("*mut usize".to_string(), "new_vl".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let unmasked = segment_body("vlseg2e8ff_v_i8m1x2", &unmasked_params, Some("vint8m1x2_t"))
            .unwrap()
            .0;
        assert!(unmasked.contains("riscv_vlsegff::<vint8m1x2_t, _>"));
        assert!(unmasked.contains("undef_vec!(), rs1, new_vl, vl"));
        assert!(!unmasked.contains("llvm.riscv.vlse"));
        assert!(!unmasked.contains("*new_vl = vl"));

        let masked_params = vec![
            ("vbool32_t".to_string(), "vm".to_string()),
            ("*const i32".to_string(), "rs1".to_string()),
            ("*mut usize".to_string(), "new_vl".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let masked = segment_body(
            "vlseg3e32ff_v_i32m1x3_m",
            &masked_params,
            Some("vint32m1x3_t"),
        )
        .unwrap()
        .0;
        assert!(masked.contains("riscv_vlsegff_mask::<vint32m1x3_t, _, _, 3>"));
        assert!(masked.contains("undef_vec!(), rs1, vm, new_vl, vl"));
        assert!(!masked.contains("vl, 3"));
        assert!(!masked.contains("llvm.riscv.vlse.mask"));
        assert!(!masked.contains("*new_vl = vl"));
    }

    #[test]
    fn segment_policy_uses_real_passthru_and_immediate() {
        let tu_params = vec![
            ("vint8m1x2_t".to_string(), "maskedoff_tuple".to_string()),
            ("*const i8".to_string(), "rs1".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let tu = segment_body("vlseg2e8_v_i8m1x2_tu", &tu_params, Some("vint8m1x2_t"))
            .unwrap()
            .0;
        assert!(tu.contains("riscv_vlseg::<vint8m1x2_t, _>(maskedoff_tuple"));

        for (suffix, immediate) in [("tum", 2), ("tumu", 0), ("mu", 1)] {
            let params = vec![
                ("vbool8_t".to_string(), "mask".to_string()),
                ("vint8m1x2_t".to_string(), "maskedoff_tuple".to_string()),
                ("*const i8".to_string(), "rs1".to_string()),
                ("usize".to_string(), "vl".to_string()),
            ];
            let body = segment_body(
                &format!("vlseg2e8_v_i8m1x2_{suffix}"),
                &params,
                Some("vint8m1x2_t"),
            )
            .unwrap()
            .0;
            assert!(body.contains(&format!(
                "riscv_vlseg_mask::<vint8m1x2_t, _, _, {immediate}>"
            )));
            assert!(body.contains("(maskedoff_tuple, rs1, mask, vl)"));
        }
    }

    #[test]
    fn segment_ff_policy_preserves_passthru_policy_and_new_vl() {
        let tu_params = vec![
            ("vint8m1x2_t".to_string(), "maskedoff_tuple".to_string()),
            ("*const i8".to_string(), "rs1".to_string()),
            ("*mut usize".to_string(), "new_vl".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let tu = segment_body("vlseg2e8ff_v_i8m1x2_tu", &tu_params, Some("vint8m1x2_t"))
            .unwrap()
            .0;
        assert!(tu.contains("riscv_vlsegff::<vint8m1x2_t, _>(maskedoff_tuple"));
        assert!(tu.contains("rs1, new_vl, vl"));

        for (suffix, immediate) in [("tum", 2), ("tumu", 0), ("mu", 1)] {
            let params = vec![
                ("vbool8_t".to_string(), "mask".to_string()),
                ("vint8m1x2_t".to_string(), "maskedoff_tuple".to_string()),
                ("*const i8".to_string(), "rs1".to_string()),
                ("*mut usize".to_string(), "new_vl".to_string()),
                ("usize".to_string(), "vl".to_string()),
            ];
            let body = segment_body(
                &format!("vlseg2e8ff_v_i8m1x2_{suffix}"),
                &params,
                Some("vint8m1x2_t"),
            )
            .unwrap()
            .0;
            assert!(body.contains(&format!(
                "riscv_vlsegff_mask::<vint8m1x2_t, _, _, {immediate}>"
            )));
            assert!(body.contains("(maskedoff_tuple, rs1, mask, new_vl, vl)"));
            assert!(!body.contains("undef_vec!()"));
        }
    }

    #[test]
    fn indexed_segments_preserve_ordering() {
        let load_params = vec![
            ("*const i32".to_string(), "rs1".to_string()),
            ("vuint8m1_t".to_string(), "rs2".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let ordered_load =
            segment_body("vloxseg2ei8_v_i32m1x2", &load_params, Some("vint32m1x2_t"))
                .unwrap()
                .0;
        let unordered_load =
            segment_body("vluxseg2ei8_v_i32m1x2", &load_params, Some("vint32m1x2_t"))
                .unwrap()
                .0;
        assert!(ordered_load.contains("riscv_vloxseg::<vint32m1x2_t, _, _>"));
        assert!(unordered_load.contains("riscv_vluxseg::<vint32m1x2_t, _, _>"));
        assert!(!ordered_load.contains("llvm.riscv.vloxei"));
        assert!(!unordered_load.contains("llvm.riscv.vluxei"));
        assert!(!ordered_load.contains("byte_add"));
        assert!(!unordered_load.contains("byte_add"));

        let store_params = vec![
            ("*mut i32".to_string(), "rs1".to_string()),
            ("vuint8m1_t".to_string(), "rs2".to_string()),
            ("vint32m1x2_t".to_string(), "vs3".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let ordered_store = segment_body("vsoxseg2ei8_v_i32m1x2", &store_params, None)
            .unwrap()
            .0;
        let unordered_store = segment_body("vsuxseg2ei8_v_i32m1x2", &store_params, None)
            .unwrap()
            .0;
        assert!(ordered_store.contains("riscv_vsoxseg(vs3, rs1, rs2, vl)"));
        assert!(unordered_store.contains("riscv_vsuxseg(vs3, rs1, rs2, vl)"));
        assert!(!ordered_store.contains("llvm.riscv.vsoxei"));
        assert!(!unordered_store.contains("llvm.riscv.vsuxei"));
        assert!(!ordered_store.contains("byte_add"));
        assert!(!unordered_store.contains("byte_add"));
    }

    #[test]
    fn native_segments_cover_modes_and_masks() {
        let unmasked_load = vec![
            ("*const i32".to_string(), "rs1".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let masked_load = vec![
            ("vbool32_t".to_string(), "vm".to_string()),
            ("*const i32".to_string(), "rs1".to_string()),
            ("isize".to_string(), "rs2".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let unit_load = segment_body("vlseg2e32_v_i32m1x2", &unmasked_load, Some("vint32m1x2_t"))
            .unwrap()
            .0;
        let strided_load =
            segment_body("vlsseg2e32_v_i32m1x2_m", &masked_load, Some("vint32m1x2_t"))
                .unwrap()
                .0;
        assert!(unit_load.contains("riscv_vlseg::<vint32m1x2_t, _>"));
        assert!(strided_load.contains("riscv_vlsseg_mask::<vint32m1x2_t, _, _, 3>"));
        assert!(strided_load.contains("undef_vec!(), rs1, rs2, vm, vl"));

        let unmasked_store = vec![
            ("*mut i32".to_string(), "rs1".to_string()),
            ("isize".to_string(), "rs2".to_string()),
            ("vint32m1x2_t".to_string(), "vs3".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let masked_store = vec![
            ("vbool32_t".to_string(), "vm".to_string()),
            ("*mut i32".to_string(), "rs1".to_string()),
            ("vint32m1x2_t".to_string(), "vs3".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        let strided_store = segment_body("vssseg2e32_v_i32m1x2", &unmasked_store, None)
            .unwrap()
            .0;
        let unit_store = segment_body("vsseg2e32_v_i32m1x2_m", &masked_store, None)
            .unwrap()
            .0;
        assert!(strided_store.contains("riscv_vssseg(vs3, rs1, rs2, vl)"));
        assert!(unit_store.contains("riscv_vsseg_mask(vs3, rs1, vm, vl)"));
        for body in [&unit_load, &strided_load, &strided_store, &unit_store] {
            assert!(!body.contains("llvm.riscv.vlse"));
            assert!(!body.contains("llvm.riscv.vsse"));
            assert!(!body.contains("sve_tuple_"));
            assert!(!body.contains("byte_add"));
        }
    }
}
