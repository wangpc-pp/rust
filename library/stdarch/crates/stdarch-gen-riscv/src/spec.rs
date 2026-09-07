//! The `rvv.yml` spec schema and its expansion to concrete C prototypes.
//!
//! The file is a list of documentation *sections*. Each [`Section`] carries a
//! one-line `doc` heading and the operation families under it, so the heading is
//! written once per section instead of once per family.
//!
//! A *family* groups all intrinsics that share a base operation (e.g. `vadd`).
//! It is either a structured template (`form` + `args` + type sweep fields) or
//! a built-in `kind` generator for pseudo families and conversion matrices.
//! Both paths expand to the same `Proto` values the C-prototype parser produces,
//! and the section `doc` is carried onto each one as metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::adoc::Proto;
use crate::types::{vbool_vtype, vtype_of_rust};

/// LMUL fragments, narrowest-to-widest, so a signed shift moves between them.
const LMUL_ORDER: &[&str] = &["mf8", "mf4", "mf2", "m1", "m2", "m4", "m8"];

/// A data type for template expansion. Unlike [`crate::types::VType`] this keeps
/// the signed/unsigned distinction (`class`), which the C type spelling needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DType {
    /// `i` (int), `u` (uint), `f` (float), `b` (bfloat).
    class: char,
    sew: u32,
    /// Index into `LMUL_ORDER`.
    lmul: usize,
    /// Tuple size (1 for plain vectors).
    nf: u32,
}

impl DType {
    /// Parse a suffix like `i8mf8`, `u16m2`, `f32m1`, `bf16m4`, `i8m1x2`.
    fn parse(s: &str) -> Option<DType> {
        let (core, nf) = match s.split_once('x') {
            Some((c, n)) => (c, n.parse().ok()?),
            None => (s, 1),
        };
        let (class, rest) = if let Some(r) = core.strip_prefix("bf") {
            ('b', r)
        } else if let Some(r) = core.strip_prefix('i') {
            ('i', r)
        } else if let Some(r) = core.strip_prefix('u') {
            ('u', r)
        } else if let Some(r) = core.strip_prefix('f') {
            ('f', r)
        } else {
            return None;
        };
        let mpos = rest.find('m')?;
        let sew: u32 = rest[..mpos].parse().ok()?;
        let lmul = LMUL_ORDER.iter().position(|&l| l == &rest[mpos..])?;
        Some(DType {
            class,
            sew,
            lmul,
            nf,
        })
    }

    /// The `<sew><lmul>[x<nf>]` name-suffix fragment, e.g. `i8mf8`, `u16m2x2`.
    fn suffix(&self) -> String {
        let x = if self.nf > 1 {
            format!("x{}", self.nf)
        } else {
            String::new()
        };
        let class = match self.class {
            'b' => "bf",
            'i' => "i",
            'u' => "u",
            'f' => "f",
            _ => "?",
        };
        format!("{class}{}{}{x}", self.sew, LMUL_ORDER[self.lmul])
    }

    /// The RVV C type name, e.g. `vint8mf8_t`, `vuint16m2x2_t`, `vfloat32m1_t`.
    fn cty(&self) -> String {
        let prefix = match self.class {
            'i' => "vint",
            'u' => "vuint",
            'f' => "vfloat",
            'b' => "vbfloat",
            _ => "v?",
        };
        let x = if self.nf > 1 {
            format!("x{}", self.nf)
        } else {
            String::new()
        };
        format!("{prefix}{}{}{x}_t", self.sew, LMUL_ORDER[self.lmul])
    }

    /// The scalar element C type, e.g. `int8_t`, `_Float16`, `__bf16`.
    fn scalar(&self) -> &'static str {
        match (self.class, self.sew) {
            ('i', 8) => "int8_t",
            ('i', 16) => "int16_t",
            ('i', 32) => "int32_t",
            ('i', 64) => "int64_t",
            ('u', 8) => "uint8_t",
            ('u', 16) => "uint16_t",
            ('u', 32) => "uint32_t",
            ('u', 64) => "uint64_t",
            ('f', 16) => "_Float16",
            ('f', 32) => "float",
            ('f', 64) => "double",
            ('b', 16) => "__bf16",
            _ => "?",
        }
    }

    /// The minimal lane count `N = 64 * LMUL / SEW`.
    fn lanes(&self) -> u32 {
        let (num, den) = LMULS
            .iter()
            .find(|(l, _, _)| *l == LMUL_ORDER[self.lmul])
            .map(|(_, n, d)| (*n, *d))
            .unwrap_or((1, 1));
        (64 * num) / (den * self.sew)
    }

    /// The LMUL index for an unsigned index vector with element width `eew` and
    /// the same lane count as this vector. Returns `None` when that index vector
    /// would require an illegal LMUL.
    fn index_lmul(&self, eew: u32) -> Option<usize> {
        let lane = self.lanes().max(1);
        LMULS
            .iter()
            .position(|(_, num, den)| lane_count(eew, *num, *den) == Some(lane))
    }

    /// Apply a transform op-list (e.g. `c=u,e*2,l+1,nf=2`), returning the new type.
    ///
    /// LMUL ops: `l<+/-k>` shifts relative to `D`; `l=<lmul>` sets it absolutely
    /// (e.g. `l=m1` for reduction destinations, which are always `m1`).
    ///
    /// `ei<K>` produces the matching *index* type of a memory operation: an
    /// unsigned element of EEW `K` with the LMUL adjusted so the lane count is
    /// unchanged (`EMUL = LMUL * K / SEW`). Used by indexed loads/stores.
    fn transform(&self, ops: &str) -> DType {
        let mut d = *self;
        for op in ops.split(',') {
            if let Some(k) = op.strip_prefix("ei") {
                let k: u32 = k.parse().unwrap_or(d.sew);
                // Keep the lane count: scale the LMUL fragment by K/SEW.
                let lmul = d.index_lmul(k).unwrap_or(d.lmul);
                d.class = 'u';
                d.sew = k;
                d.lmul = lmul;
                d.nf = 1;
            } else if let Some(c) = op.strip_prefix("c=") {
                d.class = c.chars().next().unwrap_or(d.class);
            } else if let Some(k) = op.strip_prefix("e*") {
                d.sew *= k.parse::<u32>().unwrap_or(1);
            } else if let Some(k) = op.strip_prefix("e/") {
                d.sew /= k.parse::<u32>().unwrap_or(1);
            } else if let Some(l) = op.strip_prefix("l=") {
                d.lmul = LMUL_ORDER.iter().position(|&x| x == l).unwrap_or(d.lmul);
            } else if let Some(sh) = op.strip_prefix('l') {
                let sh: i32 = sh.parse().unwrap_or(0);
                d.lmul = (d.lmul as i32 + sh).clamp(0, LMUL_ORDER.len() as i32 - 1) as usize;
            } else if let Some(k) = op.strip_prefix("nf=") {
                d.nf = k.parse().unwrap_or(d.nf);
            }
        }
        d
    }
}

/// Restore the trailing `size_t vl` parameter elided from a template. Every RVV
/// intrinsic ends with it, so templates leave it off and it is appended before
/// the closing `)` — as the sole parameter when the arg list is otherwise empty,
/// else after a comma. A template that already ends with the `size_t vl`
/// parameter is left untouched (matched precisely so the `size_t * new_vl`
/// out-parameter of fault-first loads is not mistaken for it).
fn add_vl(tmpl: &str) -> String {
    let t = tmpl.trim_end();
    if t.ends_with("(size_t vl)") || t.ends_with(", size_t vl)") {
        return t.to_string();
    }
    let close = t.rfind(')').expect("template has no `)`");
    let inside = t[..close].rsplit('(').next().unwrap_or("").trim();
    let sep = if inside.is_empty() { "" } else { ", " };
    format!("{}{sep}size_t vl{}", &t[..close], &t[close..])
}

/// Families whose scalar (`vx`/`wx`) operand is a `size_t` shift amount rather
/// than an element-typed scalar (shifts and narrowing shifts/clips).
const SHIFT_FAMILIES: &[&str] = &[
    "vsll", "vsra", "vsrl", "vssra", "vssrl", "vnsra", "vnsrl", "vnclip", "vnclipu",
];

/// Split a comma-separated argument list at top level, respecting `{...}` depth.
fn split_top_level(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in args.chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// The scalar spelling of a `{d...}` vector type: drop any LMUL (`l...`) op from
/// its transform list and take the `.s` accessor. E.g. `{d}` -> `{d.s}`,
/// `{d|e/2,l-1}` -> `{d|e/2.s}`.
fn scalarize(vty: &str) -> Option<String> {
    let inner = vty.strip_prefix("{d")?.strip_suffix('}')?;
    let ops = inner.strip_prefix('|').unwrap_or("");
    let kept: Vec<&str> = ops
        .split(',')
        .filter(|o| !o.is_empty() && !o.starts_with('l'))
        .collect();
    if kept.is_empty() {
        Some("{d.s}".to_string())
    } else {
        Some(format!("{{d|{}.s}}", kept.join(",")))
    }
}

/// The base operand letter of a form marker: its last char, or the char before a
/// trailing `m` (`vv`->`v`, `vx`->`x`, `wv`->`v`, `vxm`->`x`).
fn form_base(form: &str) -> char {
    let f = form.strip_suffix('m').unwrap_or(form);
    f.chars().last().unwrap_or('v')
}

/// Derive the prototype for another operand `form` from the family's primary
/// template. The primary template names its first operand `vs2` and its second
/// operand `vs1` (vector). The target form rewrites:
///   * the `_<form>_` marker in the name;
///   * `vs2` to the widened type `{d}` when the target is a widening form
///     (`w*`) — the primary `vv`/`vf` form spells the narrow `{d|e/2,l-1}`;
///   * the second operand: kept as the vector `vs1` for a `v` target; turned
///     into a scalar `rs1` (element-typed via [`scalarize`], or `size_t` for the
///     shift families) for an `x`/`f` target — except widening FP fused-multiply
///     forms keep the `vs1` name;
///   * the `v0` mask operand is dropped when going from a masked-carry form
///     (`vvm`) to its unmasked comparison form (`vv`).
fn derive_form(main: &str, fam: &str, form: &str) -> String {
    let open = main.find('(').expect("template has no `(`");
    let close = main.rfind(')').expect("template has no `)`");
    let head = &main[..open];
    let args = &main[open + 1..close];

    // head = `<ret> __riscv_<fam>_<mainform><tail>`; split off ret and the
    // `<mainform><tail>` suffix after `__riscv_<fam>_`.
    let sp = head
        .rfind(char::is_whitespace)
        .expect("no space in template head");
    let ret = head[..sp].trim();
    let name = head[sp..].trim();
    let prefix = format!("__riscv_{fam}_");
    let rest = name
        .strip_prefix(&prefix)
        .expect("template name lacks family prefix");
    let main_form: String = rest.chars().take_while(|c| *c != '_').collect();
    let tail = &rest[main_form.len()..]; // e.g. `_{d.suf}` or `_{d.suf}_{d.msuf}`

    let is_widen = form.starts_with('w');
    let target_second = form_base(form);
    let main_has_m = main_form.ends_with('m');
    let tgt_has_m = form.ends_with('m');
    let has_vd = split_top_level(args).iter().any(|a| a.ends_with(" vd"));

    let mut new_args = Vec::new();
    for a in split_top_level(args) {
        let (ty, pname) = a.rsplit_once(' ').expect("arg has no name");
        match pname {
            "vs2" => {
                let nt = if is_widen && ty == "{d|e/2,l-1}" {
                    "{d}"
                } else {
                    ty
                };
                new_args.push(format!("{nt} vs2"));
            }
            "vs1" | "rs1" => {
                if target_second == 'v' {
                    new_args.push(format!("{ty} vs1"));
                } else if fam == "vrgather" {
                    new_args.push("size_t vs1".to_string());
                } else if SHIFT_FAMILIES.contains(&fam) {
                    new_args.push("size_t rs1".to_string());
                } else {
                    let st = scalarize(ty).unwrap_or_else(|| ty.to_string());
                    // Widening FP fused-multiply keeps the `vs1` name.
                    let keep = target_second == 'f' && has_vd && ty.contains("e/2");
                    new_args.push(format!("{st} {}", if keep { "vs1" } else { "rs1" }));
                }
            }
            "v0" if main_has_m && !tgt_has_m => continue, // vvm -> vv drops the mask
            _ => new_args.push(a),
        }
    }
    format!("{ret} __riscv_{fam}_{form}{tail}({})", new_args.join(", "))
}

/// Render one template prototype string against the data type `d`, substituting
/// `{d...}` placeholders. See [`Family::templates`] for the placeholder syntax.
fn render_template(tmpl: &str, d: DType) -> String {
    let mut out = String::with_capacity(tmpl.len());
    let mut rest = tmpl;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find('}').expect("unterminated `{` in template");
        let expr = &after[..close];
        out.push_str(&render_placeholder(expr, d));
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Whether every `{d|ei<K>...}` placeholder in this template can be rendered for
/// the current data type. Indexed memory intrinsics only exist when the index
/// vector with EEW `K` has a legal LMUL at the same lane count as `d`.
fn template_type_is_legal(tmpl: &str, d: DType) -> bool {
    let mut rest = tmpl;
    while let Some(open) = rest.find("{d|") {
        let after = &rest[open + 3..];
        let Some(close) = after.find('}') else {
            break;
        };
        let expr = &after[..close];
        let ops = expr
            .rsplit_once('.')
            .filter(|(_, accessor)| matches!(*accessor, "suf" | "s" | "mask" | "msuf"))
            .map_or(expr, |(base, _)| base);
        if !transform_is_legal(d, ops) {
            return false;
        }
        rest = &after[close + 1..];
    }

    let mut rest = tmpl;
    while let Some(open) = rest.find("{d|ei") {
        let after = &rest[open + 5..]; // after "{d|ei"
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(eew) = digits.parse::<u32>() {
            if d.index_lmul(eew).is_none() {
                return false;
            }
        }
        rest = after;
    }
    true
}

/// Check a transformed type without clamping an out-of-range LMUL. Rendering
/// deliberately remains infallible, while expansion uses this predicate to
/// reject combinations such as `m8|l+1`.
fn transform_is_legal(d: DType, ops: &str) -> bool {
    let mut lmul = d.lmul as i32;
    for op in ops.split(',') {
        if let Some(l) = op.strip_prefix("l=") {
            let Some(index) = LMUL_ORDER.iter().position(|&candidate| candidate == l) else {
                return false;
            };
            lmul = index as i32;
        } else if let Some(shift) = op.strip_prefix('l') {
            let Ok(shift) = shift.parse::<i32>() else {
                return false;
            };
            lmul += shift;
            if !(0..LMUL_ORDER.len() as i32).contains(&lmul) {
                return false;
            }
        }
    }
    true
}

/// Render a single `{...}` placeholder body.
fn render_placeholder(expr: &str, d: DType) -> String {
    // Split off a trailing `.suf` / `.s` / `.mask` / `.msuf` accessor.
    let (base, accessor) = match expr.rsplit_once('.') {
        Some((b, acc @ ("suf" | "s" | "mask" | "msuf"))) => (b, Some(acc)),
        _ => (expr, None),
    };
    // `d` or `d|<ops>`.
    let ty = match base.split_once('|') {
        Some(("d", ops)) => d.transform(ops),
        _ => d, // bare `d`
    };
    match accessor {
        Some("suf") => ty.suffix(),
        Some("s") => ty.scalar().to_string(),
        // `.mask` — the `vboolK_t` companion; `.msuf` — its `bK` name suffix.
        Some("mask") => format!("vbool{}_t", 64 / ty.lanes().max(1)),
        Some("msuf") => format!("b{}", 64 / ty.lanes().max(1)),
        _ => ty.cty(),
    }
}

/// Synthesize the explicit-rounding (`_rm`) variant of a prototype: append `_rm`
/// to the name and insert a `size_t frm` parameter just before the trailing
/// `size_t vl` (every intrinsic ends with `vl`, so `frm` is the second-to-last
/// operand). The emitter promotes this iXLEN operand to a Rust `const` generic
/// while retaining this position for legacy calls. Used for the floating-point
/// arithmetic families.
fn synth_rounding(p: &Proto) -> Proto {
    let mut params = p.params.clone();
    let frm = ("size_t".to_string(), "frm".to_string());
    // Insert before the trailing `vl` operand if present, else at the end.
    let pos = params
        .iter()
        .position(|(_, n)| n == "vl")
        .unwrap_or(params.len());
    params.insert(pos, frm);
    Proto {
        ret: p.ret.clone(),
        name: format!("{}_rm", p.name),
        params,
        section: p.section.clone(),
    }
}

/// Synthesize the masked (`_m`) variant of an unmasked prototype: append `_m` to
/// the name and prepend a `vbool<K>_t vm` mask operand. `K` is `64 / N` where `N`
/// is the lane count of the masked element group — the operand named `vs2`
/// (present on almost every masked op), else the first vector/mask parameter,
/// else the return type. This matches the upstream masked C signatures.
///
/// Returns `None` if no lane count can be determined.
fn synth_masked(p: &Proto) -> Option<Proto> {
    let n = mask_lane_count(p)?;
    let k = 64 / n;
    let mask_ty = format!("vbool{k}_t");
    let mut params = Vec::with_capacity(p.params.len() + 1);
    params.push((mask_ty, "vm".to_string()));
    params.extend(p.params.iter().cloned());
    Some(Proto {
        ret: p.ret.clone(),
        name: format!("{}_m", p.name),
        params,
        section: p.section.clone(),
    })
}

/// The lane count that a masked variant's mask applies to.
fn mask_lane_count(p: &Proto) -> Option<u32> {
    let lane = |ty: &str| -> Option<u32> {
        let t = ty.trim().trim_start_matches("const ").trim();
        vtype_of_rust(t)
            .map(|v| v.n)
            .or_else(|| vbool_vtype(t).map(|v| v.n))
    };
    // Unzip writes half as many elements as its double-LMUL source supplies.
    // Its mask therefore follows the destination group, not `vs2`.
    if p.short()
        .split('_')
        .next()
        .is_some_and(|op| matches!(op, "vunzipe" | "vunzipo"))
    {
        return lane(&p.ret);
    }
    // Zvdot4a8i consumes packed 8-bit vectors with four times as many lanes as
    // its 32-bit accumulator. Its mask follows the accumulator/result lanes.
    if p.short()
        .split('_')
        .next()
        .is_some_and(|op| op.starts_with("vdot4a"))
    {
        return lane(&p.ret);
    }
    // Indexed segment operations also contain an index vector, whose EEW is
    // unrelated to the data SEW. Derive their mask from the data field group
    // explicitly: the returned tuple for loads and `vs3` for stores.
    if p.short()
        .split('_')
        .next()
        .is_some_and(|op| op.contains("seg"))
    {
        if p.ret != "void" {
            return lane(&p.ret);
        }
        if let Some((ty, _)) = p.params.iter().find(|(_, name)| name == "vs3") {
            return lane(ty);
        }
    }
    // Prefer the `vs2` source operand, then the first vector/mask parameter,
    // then the return type.
    if let Some((ty, _)) = p.params.iter().find(|(_, n)| n == "vs2") {
        if let Some(n) = lane(ty) {
            return Some(n);
        }
    }
    if let Some(n) = p.params.iter().find_map(|(t, _)| lane(t)) {
        return Some(n);
    }
    lane(&p.ret)
}

/// LMUL variants as (name-fragment, numerator, denominator), widest-first order
/// matching the `.adoc` listing.
pub const LMULS: &[(&str, u32, u32)] = &[
    ("mf8", 1, 8),
    ("mf4", 1, 4),
    ("mf2", 1, 2),
    ("m1", 1, 1),
    ("m2", 2, 1),
    ("m4", 4, 1),
    ("m8", 8, 1),
];

/// One operation family.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Family {
    /// Base name, e.g. `vadd`.
    pub name: String,

    /// Explicit non-`v` target features required by every intrinsic in this
    /// family, using the Rust target-feature spellings (e.g. `zvabd`).
    /// Kept as a plain list so the YAML stays compact:
    /// `required: [zvabd]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,

    /// Emit masked (`_m`) variants too.
    #[serde(default, skip_serializing_if = "is_false")]
    pub masked: bool,
    /// Emit explicit-rounding-mode (`_rm`) variants too, by inserting an
    /// `size_t frm` parameter just before the trailing `vl`. The emitter makes
    /// it a `const FRM: usize` and records this original parameter position in
    /// `rustc_legacy_const_generics`. Combined with `masked`, this yields the
    /// full base / `_rm` / `_m` / `_rm_m` set that the floating-point
    /// arithmetic families provide.
    #[serde(default, skip_serializing_if = "is_false")]
    pub rounding: bool,

    // -- template form (the most compact) --
    /// Element types this family sweeps over, as `<class><sew>` (`i8`, `u16`,
    /// `f32`, `bf16`). The generator forms `dtypes` as the cartesian product
    /// `elems` × `lmuls` (× the tuple size `nf`), keeping only LMULs that are
    /// legal for the element width and, for tuples, satisfy `LMUL * nf <= 8`.
    /// Preferred over the explicit `dtypes` list; when both are set `elems`
    /// wins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elems: Vec<String>,
    /// Candidate LMULs for the `elems` product (`mf8`..`m8`). Omitted means "all
    /// legal LMULs for each element width"; when present it is intersected with
    /// the legal set per width (so a narrowing family can simply drop `m8`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lmuls: Vec<String>,
    /// Tuple size(s) for the `elems` product (segment/tuple families). Either a
    /// single value (`nf: 2`) or a sweep list (`nf: [2, 3, 4, 5, 6, 7, 8]`); a
    /// swept family substitutes each value for a `{nf}` placeholder in its
    /// `name`, expanding one sub-family per `nf` in order. Defaults to `1`
    /// (plain vectors).
    #[serde(default, skip_serializing_if = "NfSpec::is_default")]
    pub nf: NfSpec,
    /// Element-width sweep, for the load/store families whose only difference
    /// between sibling entries is the memory element width (`vle8`/`vle16`/...).
    /// Each value is substituted for a `{sew}` placeholder in `name` and `args`,
    /// expanding one sub-family per width in order (the outermost sweep, so it
    /// nests around any `nf` sweep). When `elems` is left empty it is derived as
    /// the standard element types of that width (`8` -> `i8, u8`; `16` ->
    /// `f16, i16, u16`; ...); indexed families keep an explicit `elems` (the
    /// full set, independent of the *index* width) and use `{sew}` only to spell
    /// their `{d|ei<sew>}` index type.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sew: Vec<u32>,
    /// Primary data-type suffixes this family iterates over, e.g. `i8mf8`,
    /// `f32m2`. Each becomes the anchor type `D` for the templates below. Used
    /// only for the few families whose type sweep is too irregular for `elems`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dtypes: Vec<String>,
    /// A single prototype template, for families that have exactly one (the
    /// common case). Equivalent to a one-element `templates`; see it for the
    /// placeholder syntax.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Prototype templates, expanded once per data type (type-major: for each
    /// `D`, every template in order). Types are written as placeholders
    /// relative to `D`:
    ///   * `{d}` — `D` itself; `{d.suf}` — its name suffix; `{d.s}` — its scalar
    ///     element type;
    ///   * `{d|OPS}` — a transform of `D` (`.suf` variant `{d|OPS.suf}`), where
    ///     `OPS` is a comma list of `c=<i|u|f|b>` (class), `e*<k>` / `e/<k>`
    ///     (element widen/narrow), `l<+/-k>` (LMUL shift), `nf=<k>` (tuple size).
    /// Plain C tokens (`size_t`, `const {d.s} *`, literal `vbool*_t`, ...) are
    /// kept verbatim. The trailing `size_t vl` parameter that every intrinsic
    /// ends with is omitted from the template and appended automatically.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
    /// Extra operand forms to synthesize from the (single) `template` by
    /// rewriting its operand-form marker and second operand. Each entry is a
    /// form marker like `vx`, `vf`, `wv`, `wx`, `wf`, `vxm`. See
    /// [`derive_form`] for the rewrite rules (vector→scalar operand, widening
    /// `w*` first operand, shift/carry special cases).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suffixes: Vec<String>,

    // -- structured form (the most compact template form) --
    /// Operand-form marker of the primary prototype (`v`, `vv`, `vx`, `vf`,
    /// `wv`, `vvm`, ...). With `args` this replaces a full `template`: the C
    /// name is rebuilt as `__riscv_<name>_<form>_<retsuf>`, where `<retsuf>` is
    /// `{d.suf}` normally and `{d.suf}_{d.msuf}` when the return type is a mask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// The primary prototype's parameter list (without the trailing `size_t vl`,
    /// which is appended automatically), e.g. `{d} vs2, {d} vs1`. Placeholders
    /// are the same as in `templates`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    /// Return type of the structured `form`/`args` prototype. Defaults to `{d}`;
    /// set it for stores (`void`), comparisons (`{d.mask}`), etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ret: Option<String>,
    /// Mask-type sweep, for the families whose only type axis is the mask ratio
    /// (`vmand`, `vmsbf`, `vcpop`, `vlm`, ...). Values are `b1`..`b64`; when set
    /// the family iterates these instead of `elems`/`dtypes`, the placeholders
    /// `{m}` (→ `vbool<N>_t`) and `{m.suf}` (→ `b<N>`) are available, and the
    /// name suffix is `{m.suf}`. RVV mask types are defined in the spec's mask
    /// type table (`vbool<N>_t`, N = SEW/LMUL).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub masks: Vec<String>,
    /// Explicit name suffix (the RVV `{RETURN_TYPE}` component), for families
    /// whose name does not follow the default `{d.suf}` rule — e.g. reductions
    /// encode both input and output (`{d.suf}_{d|l=m1.suf}`) and `vmv_x_s`
    /// encodes input and scalar. Rendered with the same placeholders as `args`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namesuf: Option<String>,
    /// Built-in generator for pseudo families that are more naturally described
    /// by the RVV type system than by a single `form` template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The standard element types of a memory element width, in the order the plain
/// load/store families list them: `f` (if a float of that width exists), then
/// `i`, then `u`. Used to derive `elems` for a `{sew}`-swept family.
fn elems_for_sew(sew: u32) -> Vec<String> {
    let mut out = Vec::new();
    if matches!(sew, 16 | 32 | 64) {
        out.push(format!("f{sew}"));
    }
    out.push(format!("i{sew}"));
    out.push(format!("u{sew}"));
    if sew == 16 {
        out.push("bf16".to_string());
    }
    out
}

/// A family's tuple-size axis: either a single `nf` or a sweep list. A single
/// value (including the default `1`) expands the family once; a list expands one
/// sub-family per value, substituting each for a `{nf}` placeholder in the name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NfSpec {
    One(u32),
    List(Vec<u32>),
}

impl Default for NfSpec {
    fn default() -> Self {
        NfSpec::One(1)
    }
}

impl NfSpec {
    /// Whether this is the omittable default (`nf: 1`), so it is not serialized.
    fn is_default(&self) -> bool {
        matches!(self, NfSpec::One(1))
    }

    /// The single tuple size of an already-specialized family (post-sweep every
    /// family carries `NfSpec::One`). Defensively yields `1` for an unexpanded
    /// list, which never reaches the scalar consumers.
    fn scalar(&self) -> u32 {
        match self {
            NfSpec::One(n) => *n,
            NfSpec::List(_) => 1,
        }
    }
}

/// The whole spec file: an ordered list of documentation sections, each holding
/// the families that share one heading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub sections: Vec<Section>,
}

/// One documentation section: a shared heading and the families under it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// One-line documentation heading shared by every family in the section.
    /// (Metadata only; it does not affect the generated bindings.)
    pub doc: String,
    /// The operation families in this section, in listing order.
    pub intrinsics: Vec<Family>,
}

impl Spec {
    /// Expand every family, tagging each prototype with its section heading.
    pub fn expand(&self) -> Vec<Proto> {
        // The Zvfbfa and Zvfbfwma surfaces are selected subsets of otherwise
        // standard floating-point families. Expand the ordinary grid first,
        // then clone the matching f16 prototypes using the vendored Clang 22
        // name manifests. This keeps rvv.yml compact while making additions or
        // removals in Clang visible as exact-set test failures.
        let mut out = Vec::new();
        let mut bf16 = Vec::new();
        for section in &self.sections {
            for family in &section.intrinsics {
                if matches!(family.kind.as_deref(), Some("zvfbfa" | "zvfbfwma")) {
                    bf16.push((section.doc.as_str(), family));
                } else {
                    out.extend(family.expand(&section.doc));
                }
            }
        }
        for (doc, family) in bf16 {
            out.extend(expand_bf16_surface(
                family.kind.as_deref().unwrap(),
                doc,
                &out,
            ));
        }
        out
    }

    /// Expand every family's explicit required-feature list to the concrete
    /// intrinsic short names it produces.
    pub fn required_features(&self) -> HashMap<String, Vec<String>> {
        let mut out = HashMap::new();
        for section in &self.sections {
            for family in &section.intrinsics {
                if family.required.is_empty() {
                    continue;
                }
                if let Some(kind @ ("zvfbfa" | "zvfbfwma")) = family.kind.as_deref() {
                    for name in bf16_surface_names(kind) {
                        out.insert(
                            name.trim_start_matches("__riscv_").to_string(),
                            family.required.clone(),
                        );
                    }
                    continue;
                }
                for proto in family.expand(&section.doc) {
                    out.insert(proto.short().to_string(), family.required.clone());
                }
            }
        }
        out
    }
}

fn bf16_surface_names(kind: &str) -> impl Iterator<Item = &'static str> {
    let text = match kind {
        "zvfbfa" => include_str!("../spec/vector/zvfbfa_non_policy.txt"),
        "zvfbfwma" => include_str!("../spec/vector/zvfbfwma_non_policy.txt"),
        _ => panic!("unknown BF16 surface kind `{kind}`"),
    };
    text.lines().filter(|line| !line.is_empty())
}

fn replace_f16_with_bf16(ty: &str) -> String {
    ty.replace("vfloat16", "vbfloat16")
        .replace("_Float16", "__bf16")
}

fn remove_bf16_name_component(name: &str, vector: bool) -> Option<String> {
    let components = if vector {
        &[
            "bf16mf8", "bf16mf4", "bf16mf2", "bf16m1", "bf16m2", "bf16m4", "bf16m8",
        ][..]
    } else {
        &["bf16"][..]
    };
    for component in components {
        let needle = format!("_{component}_");
        if name.contains(&needle) {
            return Some(name.replacen(&needle, "_", 1).replace("bf16", "f16"));
        }
    }
    None
}

fn expand_bf16_surface(kind: &str, doc: &str, ordinary: &[Proto]) -> Vec<Proto> {
    let by_name: HashMap<&str, &Proto> = ordinary.iter().map(|p| (p.name.as_str(), p)).collect();
    bf16_surface_names(kind)
        .map(|name| {
            let mut candidates = vec![name.replace("bf16", "f16")];
            if let Some(candidate) = remove_bf16_name_component(name, true) {
                candidates.push(candidate);
            }
            if let Some(candidate) = remove_bf16_name_component(name, false) {
                candidates.push(candidate);
            }
            if kind == "zvfbfwma" {
                candidates.push(
                    name.replace("vfwmaccbf16", "vfwmacc")
                        .replace("bf16", "f16"),
                );
            }
            let source = candidates
                .iter()
                .find_map(|candidate| by_name.get(candidate.as_str()).copied())
                .unwrap_or_else(|| panic!("no f16 prototype for {name}"));
            Proto {
                ret: replace_f16_with_bf16(&source.ret),
                name: name.to_string(),
                params: source
                    .params
                    .iter()
                    .map(|(ty, param)| (replace_f16_with_bf16(ty), param.clone()))
                    .collect(),
                section: doc.to_string(),
            }
        })
        .collect()
}

impl Family {
    /// Expand this family to its concrete prototypes, in `.adoc` order. `doc` is
    /// the owning section's heading, carried onto each [`Proto`].
    pub fn expand(&self, doc: &str) -> Vec<Proto> {
        // Element-width sweep (outermost, so it nests around any `nf` sweep):
        // `sew: [8, 16, 32, 64]` expands one sub-family per width, substituting
        // it for the `{sew}` placeholder in the name and args (e.g. `vle{sew}`,
        // `{d|ei{sew}}`). An empty `elems` is derived from the width (the plain
        // load/store families); an explicit `elems` is kept verbatim (the
        // indexed families, whose data types are independent of the index width
        // the sweep spells).
        if !self.sew.is_empty() {
            return self
                .sew
                .clone()
                .iter()
                .flat_map(|&sew| {
                    let s = sew.to_string();
                    let mut f = self.clone();
                    f.sew = Vec::new();
                    f.name = self.name.replace("{sew}", &s);
                    f.args = self.args.as_ref().map(|a| a.replace("{sew}", &s));
                    f.template = self.template.as_ref().map(|t| t.replace("{sew}", &s));
                    f.templates = self
                        .templates
                        .iter()
                        .map(|t| t.replace("{sew}", &s))
                        .collect();
                    if f.elems.is_empty() {
                        f.elems = elems_for_sew(sew);
                    }
                    f.expand(doc)
                })
                .collect();
        }

        // Tuple-size sweep: `nf: [2, 3, ...]` expands one sub-family per value,
        // substituting it for the `{nf}` placeholder in the name (which every
        // swept family carries, e.g. `vlseg{nf}e16`). Each sub-family is then a
        // plain `NfSpec::One` family expanded in order.
        if let NfSpec::List(nfs) = &self.nf {
            return nfs
                .iter()
                .flat_map(|&nf| {
                    let mut f = self.clone();
                    f.name = self.name.replace("{nf}", &nf.to_string());
                    f.nf = NfSpec::One(nf);
                    f.expand(doc)
                })
                .collect();
        }

        if self.template.is_none()
            && self.templates.is_empty()
            && self.form.is_none()
            && self.kind.is_none()
        {
            panic!("family `{}` has no template or kind", self.name);
        }

        // Otherwise gather the unmasked prototypes from a built-in family kind
        // or the compact templates × type product.
        let templates = self.template_list();
        let base: Vec<Proto> = if let Some(kind) = &self.kind {
            self.expand_kind(doc, kind)
        } else if !templates.is_empty() {
            self.expand_templates(doc, &templates)
        } else {
            panic!("family `{}` has no grid, template, or kind", self.name)
        };

        // ...then layer on the synthesized variants. Rounding first, so masking
        // then covers both the plain and `_rm` forms, giving the full
        // base / `_rm` / `_m` / `_rm_m` set.
        let mut protos = base.clone();
        if self.rounding {
            protos.extend(base.iter().map(synth_rounding));
        }
        if self.masked {
            let masked: Vec<Proto> = protos.iter().filter_map(synth_masked).collect();
            protos.extend(masked);
        }
        protos
    }

    /// The prototype templates, from the structured `form`/`args`/`ret` fields
    /// or the literal `template` / `templates`, each with the elided trailing
    /// `size_t vl` restored. If `suffixes` is set, the extra operand forms are
    /// derived from the first template (see [`derive_form`]) and appended.
    fn template_list(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        // Structured form: rebuild the C prototype from name + form + ret.
        if let Some(form) = &self.form {
            let args = self.args.as_deref().unwrap_or("");
            out.push(add_vl(&self.build_template(form, args)));
        }
        // Literal templates (the escape hatch for irregular families).
        out.extend(
            self.template
                .iter()
                .chain(self.templates.iter())
                .map(|t| add_vl(t)),
        );
        if !self.suffixes.is_empty() {
            let main = out
                .first()
                .unwrap_or_else(|| panic!("family `{}` has `suffixes` but no template", self.name))
                .clone();
            for suf in &self.suffixes {
                out.push(derive_form(&main, &self.name, suf));
            }
        }
        out
    }

    /// Rebuild a full C prototype string from the structured fields. The name is
    /// `__riscv_<name>_<form>_<retsuf>`, where `<retsuf>` is the return-type
    /// component of the RVV naming scheme: `{m.suf}` for a mask-axis family,
    /// `{d.suf}_{d.msuf}` when the return type is a mask, else `{d.suf}`.
    fn build_template(&self, form: &str, args: &str) -> String {
        let ret = self.ret.as_deref().unwrap_or("{d}");
        let retsuf: &str = if let Some(ns) = &self.namesuf {
            ns
        } else if !self.masks.is_empty() {
            "{m.suf}"
        } else if ret == "{d.mask}" {
            "{d.suf}_{d.msuf}"
        } else {
            "{d.suf}"
        };
        format!("{ret} __riscv_{}_{form}_{retsuf}({args})", self.name)
    }

    /// Expand the template form: for each data type `D`, render every template
    /// (type-major ordering). The type list is either the explicit `dtypes` or,
    /// preferably, the `elems` × `lmuls` (× `nf`) product from [`Self::dtype_list`].
    fn expand_templates(&self, doc: &str, templates: &[String]) -> Vec<Proto> {
        // Mask-axis families sweep the bool ratios instead of data types.
        if !self.masks.is_empty() {
            return self.expand_masks(doc, templates);
        }
        let mut out = Vec::new();
        for d in self.dtype_list() {
            let dv = DType::parse(&d)
                .unwrap_or_else(|| panic!("bad dtype `{d}` in family `{}`", self.name));
            if !templates.iter().all(|t| template_type_is_legal(t, dv)) {
                continue;
            }
            for t in templates {
                let proto = render_template(t, dv);
                if let Some(p) = crate::adoc::parse_proto(&proto, doc) {
                    out.push(p);
                } else {
                    panic!("template `{t}` for dtype `{d}` did not parse: `{proto}`");
                }
            }
        }
        out
    }

    /// Expand a mask-axis family: for each `bN` in `masks`, render `{m}`
    /// (→ `vbool<N>_t`) and `{m.suf}` (→ `b<N>`) in every template.
    fn expand_masks(&self, doc: &str, templates: &[String]) -> Vec<Proto> {
        let mut out = Vec::new();
        for b in &self.masks {
            let n: u32 = b
                .strip_prefix('b')
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| panic!("bad mask `{b}` in family `{}`", self.name));
            let mty = format!("vbool{n}_t");
            for t in templates {
                let proto = t.replace("{m.suf}", &format!("b{n}")).replace("{m}", &mty);
                if let Some(p) = crate::adoc::parse_proto(&proto, doc) {
                    out.push(p);
                } else {
                    panic!("mask template `{t}` for `{b}` did not parse: `{proto}`");
                }
            }
        }
        out
    }

    /// The data types this template family sweeps over, as `<sew><lmul>[x<nf>]`
    /// suffixes. Prefers the `elems` × `lmuls` product (element-major, LMUL
    /// narrow-to-wide); the LMUL set is the legal LMULs for each element width,
    /// intersected with `lmuls` when given, and — for tuples — restricted to
    /// `LMUL * nf <= 8`. Falls back to the explicit `dtypes` list.
    fn dtype_list(&self) -> Vec<String> {
        if self.elems.is_empty() {
            return self.dtypes.clone();
        }
        let nf = self.nf.scalar().max(1);
        let mut out = Vec::new();
        for e in &self.elems {
            let (class, sew) =
                parse_elem(e).unwrap_or_else(|| panic!("bad elem `{e}` in family `{}`", self.name));
            for &(lmul, num, den) in LMULS {
                // Legal for this element width, and (for tuples) LMUL * nf <= 8.
                if lane_count(sew, num, den).is_none() || num * nf > 8 * den {
                    continue;
                }
                if !self.lmuls.is_empty() && !self.lmuls.iter().any(|l| l == lmul) {
                    continue;
                }
                let x = if nf > 1 {
                    format!("x{nf}")
                } else {
                    String::new()
                };
                out.push(format!("{class}{sew}{lmul}{x}"));
            }
        }
        out
    }

    fn expand_kind(&self, doc: &str, kind: &str) -> Vec<Proto> {
        if kind == "vector_crypto" {
            return crate::vector_crypto::expand_family(&self.name, doc)
                .unwrap_or_else(|| panic!("unknown vector-crypto family `{}`", self.name));
        }
        let protos = match kind {
            "vsetvl" => kind_vsetvl(&self.name, false),
            "vsetvlmax" => kind_vsetvl(&self.name, true),
            "vundefined" => kind_vundefined(&self.name),
            "vmv" => kind_vmv(&self.name),
            "vfmv" => kind_vfmv(&self.name),
            "vreinterpret" => kind_vreinterpret(&self.name),
            "vlmul" => kind_vlmul(&self.name),
            "vget" => kind_vget(&self.name),
            "vset" => kind_vset(&self.name),
            "vcreate" => kind_vcreate(&self.name),
            "vzip" => kind_vzip(&self.name),
            "vfwcvt_ofp8" => kind_vfwcvt_ofp8(&self.name),
            "vfncvt_ofp8" => kind_vfncvt_ofp8(&self.name),
            "vfcvt" => kind_fpcvt(&self.name, FPConvKind::Single),
            "vfwcvt" => kind_fpcvt(&self.name, FPConvKind::Widen),
            "vfncvt" => kind_fpcvt(&self.name, FPConvKind::Narrow),
            _ => panic!("unknown kind `{kind}` in family `{}`", self.name),
        };
        protos
            .into_iter()
            .filter_map(|p| crate::adoc::parse_proto(&p, doc))
            .collect()
    }
}

/// Minimal lane count `N = 64 * LMUL / SEW`, or `None` if illegal.
pub fn lane_count(sew: u32, num: u32, den: u32) -> Option<u32> {
    let numerator = 64 * num;
    let denominator = den * sew;
    if numerator % denominator != 0 {
        return None;
    }
    let n = numerator / denominator;
    if n == 0 { None } else { Some(n) }
}

/// Parse an element name (`i8`, `u16`, `f32`, `bf16`) into `(class_letter, sew)`,
/// where the class letter is the one used in a type suffix (`i`/`u`/`f`/`bf`).
fn parse_elem(e: &str) -> Option<(&'static str, u32)> {
    let (class, digits) = if let Some(r) = e.strip_prefix("bf") {
        ("bf", r)
    } else if let Some(r) = e.strip_prefix('i') {
        ("i", r)
    } else if let Some(r) = e.strip_prefix('u') {
        ("u", r)
    } else if let Some(r) = e.strip_prefix('f') {
        ("f", r)
    } else {
        return None;
    };
    Some((class, digits.parse().ok()?))
}

/// Element types supported by the structural vector/tuple pseudo-intrinsics.
/// This also drives the vector-to-vector `vmv` surface, for which Clang exposes
/// BF16 alongside the ordinary floating-point and integer element types.
const STRUCTURAL_ELEMS: &[&str] = &[
    "f16", "f32", "f64", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "bf16",
];

fn dtype_suffixes(elems: &[&str], nf: u32) -> Vec<String> {
    let mut out = Vec::new();
    for e in elems {
        let (class, sew) = parse_elem(e).unwrap_or_else(|| panic!("bad elem `{e}`"));
        for &(lmul, num, den) in LMULS {
            if lane_count(sew, num, den).is_none() || num * nf > 8 * den {
                continue;
            }
            let x = if nf > 1 {
                format!("x{nf}")
            } else {
                String::new()
            };
            out.push(format!("{class}{sew}{lmul}{x}"));
        }
    }
    out
}

fn dtype_ctype(suf: &str) -> String {
    DType::parse(suf)
        .unwrap_or_else(|| panic!("bad dtype suffix `{suf}`"))
        .cty()
}

fn kind_vsetvl(name: &str, max: bool) -> Vec<String> {
    let mut out = Vec::new();
    for sew in [8, 16, 32, 64] {
        for &(lmul, num, den) in LMULS {
            if lane_count(sew, num, den).is_none() {
                continue;
            }
            let args = if max { "void" } else { "size_t avl" };
            out.push(format!("size_t __riscv_{name}_e{sew}{lmul}({args})"));
        }
    }
    out
}

fn kind_vundefined(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for nf in 1..=8 {
        for suf in dtype_suffixes(STRUCTURAL_ELEMS, nf) {
            let ty = dtype_ctype(&suf);
            out.push(format!("{ty} __riscv_{name}_{suf}(void)"));
        }
    }
    out
}

fn kind_vmv(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for suf in dtype_suffixes(STRUCTURAL_ELEMS, 1) {
        let ty = dtype_ctype(&suf);
        out.push(format!(
            "{ty} __riscv_{name}_v_v_{suf}({ty} vs1, size_t vl)"
        ));
    }
    for suf in dtype_suffixes(&["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"], 1) {
        let ty = dtype_ctype(&suf);
        let scalar = DType::parse(&suf).unwrap().scalar();
        out.push(format!(
            "{ty} __riscv_{name}_v_x_{suf}({scalar} rs1, size_t vl)"
        ));
    }
    for suf in dtype_suffixes(&["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"], 1) {
        let ty = dtype_ctype(&suf);
        let scalar = DType::parse(&suf).unwrap().scalar();
        out.push(format!(
            "{scalar} __riscv_{name}_x_s_{suf}_{}({ty} vs2)",
            scalar_suffix(scalar)
        ));
        out.push(format!(
            "{ty} __riscv_{name}_s_x_{suf}({scalar} rs1, size_t vl)"
        ));
    }
    out
}

fn kind_vfmv(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for suf in dtype_suffixes(&["f16", "f32", "f64"], 1) {
        let ty = dtype_ctype(&suf);
        let scalar = DType::parse(&suf).unwrap().scalar();
        let ss = scalar_suffix(scalar);
        out.push(format!(
            "{ty} __riscv_{name}_v_f_{suf}({scalar} rs1, size_t vl)"
        ));
        out.push(format!("{scalar} __riscv_{name}_f_s_{suf}_{ss}({ty} vs1)"));
        out.push(format!(
            "{ty} __riscv_{name}_s_f_{suf}({scalar} rs1, size_t vl)"
        ));
    }
    out
}

fn vreg_ratio(suf: &str) -> (u32, u32) {
    let d = DType::parse(suf).unwrap_or_else(|| panic!("bad dtype suffix `{suf}`"));
    let (_, num, den) = LMULS[d.lmul];
    (num * d.nf, den)
}

fn same_elem(a: &str, b: &str) -> bool {
    let da = DType::parse(a).unwrap();
    let db = DType::parse(b).unwrap();
    da.class == db.class && da.sew == db.sew
}

fn kind_vreinterpret(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let plain = dtype_suffixes(STRUCTURAL_ELEMS, 1);
    for src in &plain {
        for dst in &plain {
            if src == dst {
                continue;
            }
            let s = DType::parse(src).unwrap();
            let d = DType::parse(dst).unwrap();
            let ok = match (s.class, d.class) {
                ('f', 'i' | 'u') | ('i' | 'u', 'f') | ('i', 'u') | ('u', 'i') => {
                    s.sew == d.sew && s.lmul == d.lmul
                }
                ('b', 'i' | 'u') | ('i' | 'u', 'b') => s.sew == d.sew && s.lmul == d.lmul,
                ('i', 'i') | ('u', 'u') => s.lmul == d.lmul,
                _ => false,
            };
            if !ok {
                continue;
            }
            let src_ty = dtype_ctype(src);
            let dst_ty = dtype_ctype(dst);
            out.push(format!(
                "{dst_ty} __riscv_{name}_v_{src}_{dst}({src_ty} src)"
            ));
        }
    }
    for src in &plain {
        let s = DType::parse(src).unwrap();
        if !matches!(s.class, 'i' | 'u') {
            continue;
        }
        let m1 = LMUL_ORDER.iter().position(|&l| l == "m1").unwrap();
        if s.lmul != m1 {
            continue;
        }
        let src_ty = dtype_ctype(src);
        for b in ["b1", "b2", "b4", "b8", "b16", "b32", "b64"] {
            let n: u32 = b[1..].parse().unwrap();
            if s.sew > 8 * n {
                continue;
            }
            out.push(format!(
                "vbool{n}_t __riscv_{name}_v_{src}_{b}({src_ty} src)"
            ));
            out.push(format!(
                "{src_ty} __riscv_{name}_v_{b}_{src}(vbool{n}_t src)"
            ));
        }
    }
    out
}

fn kind_vlmul(_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let plain = dtype_suffixes(STRUCTURAL_ELEMS, 1);
    for src in &plain {
        for dst in &plain {
            if src == dst || !same_elem(src, dst) {
                continue;
            }
            let sd = DType::parse(src).unwrap();
            let dd = DType::parse(dst).unwrap();
            let op = if dd.lmul > sd.lmul { "ext" } else { "trunc" };
            let src_ty = dtype_ctype(src);
            let dst_ty = dtype_ctype(dst);
            out.push(format!(
                "{dst_ty} __riscv_vlmul_{op}_v_{src}_{dst}({src_ty} value)"
            ));
        }
    }
    out
}

fn kind_vget(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let plain = dtype_suffixes(STRUCTURAL_ELEMS, 1);
    for src in &plain {
        for dst in &plain {
            if src == dst || !same_elem(src, dst) {
                continue;
            }
            let sd = DType::parse(src).unwrap();
            let dd = DType::parse(dst).unwrap();
            let m1 = LMUL_ORDER.iter().position(|&l| l == "m1").unwrap();
            if sd.lmul <= dd.lmul || sd.lmul < m1 || dd.lmul < m1 {
                continue;
            }
            let src_ty = dtype_ctype(src);
            let dst_ty = dtype_ctype(dst);
            out.push(format!(
                "{dst_ty} __riscv_{name}_v_{src}_{dst}({src_ty} src, size_t index)"
            ));
        }
    }
    for nf in 2..=8 {
        for src in dtype_suffixes(STRUCTURAL_ELEMS, nf) {
            let base = src.split('x').next().unwrap();
            let src_ty = dtype_ctype(&src);
            let dst_ty = dtype_ctype(base);
            out.push(format!(
                "{dst_ty} __riscv_{name}_v_{src}_{base}({src_ty} src, size_t index)"
            ));
        }
    }
    out
}

fn kind_vset(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let plain = dtype_suffixes(STRUCTURAL_ELEMS, 1);
    for dst in &plain {
        for base in &plain {
            if base == dst || !same_elem(base, dst) {
                continue;
            }
            let bd = DType::parse(base).unwrap();
            let dd = DType::parse(dst).unwrap();
            if bd.lmul >= dd.lmul || bd.lmul < LMUL_ORDER.iter().position(|&l| l == "m1").unwrap() {
                continue;
            }
            let dst_ty = dtype_ctype(dst);
            let base_ty = dtype_ctype(base);
            out.push(format!("{dst_ty} __riscv_{name}_v_{base}_{dst}({dst_ty} dest, size_t index, {base_ty} value)"));
        }
    }
    for nf in 2..=8 {
        for dst in dtype_suffixes(STRUCTURAL_ELEMS, nf) {
            let base = dst.split('x').next().unwrap();
            let dst_ty = dtype_ctype(&dst);
            let base_ty = dtype_ctype(base);
            out.push(format!("{dst_ty} __riscv_{name}_v_{base}_{dst}({dst_ty} dest, size_t index, {base_ty} value)"));
        }
    }
    out
}

fn kind_vcreate(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let plain = dtype_suffixes(STRUCTURAL_ELEMS, 1);
    for dst in &plain {
        for base in &plain {
            if base == dst || !same_elem(base, dst) {
                continue;
            }
            let bd = DType::parse(base).unwrap();
            let dd = DType::parse(dst).unwrap();
            if bd.lmul >= dd.lmul || bd.lmul < LMUL_ORDER.iter().position(|&l| l == "m1").unwrap() {
                continue;
            }
            let (bn, bd_den) = vreg_ratio(base);
            let (dn, dd_den) = vreg_ratio(dst);
            if (dn * bd_den) % (bn * dd_den) != 0 {
                continue;
            }
            let count = (dn * bd_den) / (bn * dd_den);
            let dst_ty = dtype_ctype(dst);
            let base_ty = dtype_ctype(base);
            let args = (0..count)
                .map(|i| format!("{base_ty} v{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(format!("{dst_ty} __riscv_{name}_v_{base}_{dst}({args})"));
        }
    }
    for nf in 2..=8 {
        for dst in dtype_suffixes(STRUCTURAL_ELEMS, nf) {
            let base = dst.split('x').next().unwrap();
            let dst_ty = dtype_ctype(&dst);
            let base_ty = dtype_ctype(base);
            let args = (0..nf)
                .map(|i| format!("{base_ty} v{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(format!("{dst_ty} __riscv_{name}_v_{dst}({args})"));
        }
    }
    out
}

/// Element order follows increasing SEW; types with the same SEW stay in
/// integer, IEEE floating-point, bfloat order.
const ZVZIP_ELEMS: &[&str] = &[
    "i8", "u8", "i16", "u16", "f16", "bf16", "i32", "u32", "f32", "i64", "u64", "f64",
];

/// `vzip.vv` doubles the destination LMUL and uses that destination type in
/// the public name suffix.
fn kind_vzip(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for src_suffix in dtype_suffixes(ZVZIP_ELEMS, 1) {
        let src = DType::parse(&src_suffix).unwrap();
        let Some(dst_lmul) = shifted_lmul(src.lmul, 1) else {
            continue;
        };
        let mut dst = src;
        dst.lmul = dst_lmul;
        let dst_suffix = dst.suffix();
        out.push(format!(
            "{} __riscv_{name}_vv_{dst_suffix}({} vs2, {} vs1, size_t vl)",
            dst.cty(),
            src.cty(),
            src.cty(),
        ));
    }
    out
}

const OFP8_FORMATS: &[&str] = &["e4m3", "e5m2"];

fn ofp8_suffix(format: &str, d: DType) -> String {
    format!("f8{format}{}", LMUL_ORDER[d.lmul])
}

fn ofp8_ctype(format: &str, d: DType) -> String {
    format!("vfloat8{format}{}_t", LMUL_ORDER[d.lmul])
}

fn push_ofp8_variants(
    out: &mut Vec<String>,
    operation: &str,
    name_suffix: &str,
    ret: &str,
    src: &str,
    mask_anchor: DType,
    policies: PolicySet,
) {
    let mask = format!("vbool{}_t", 64 / mask_anchor.lanes());
    for &policy in policy_suffixes(policies) {
        let mut args = Vec::new();
        if matches!(policy, "_m" | "_rm_m") {
            args.push(format!("{mask} vm"));
        }
        args.push(format!("{src} vs2"));
        if matches!(policy, "_rm" | "_rm_m") {
            args.push("size_t frm".to_string());
        }
        args.push("size_t vl".to_string());
        out.push(format!(
            "{ret} __riscv_{operation}_{name_suffix}{policy}({})",
            args.join(", ")
        ));
    }
}

/// Zvfofp8min widening conversions from either OFP8 format to BF16.
fn kind_vfwcvt_ofp8(name: &str) -> Vec<String> {
    debug_assert_eq!(name, "vfwcvt");
    let mut out = Vec::new();
    for &format in OFP8_FORMATS {
        for dst_suffix in dtype_suffixes(&["u8"], 1) {
            let dst = DType::parse(&dst_suffix).unwrap();
            let Some(wide_lmul) = shifted_lmul(dst.lmul, 1) else {
                continue;
            };
            let mut wide = dst;
            wide.class = 'b';
            wide.sew = 16;
            wide.lmul = wide_lmul;
            let fp8_suffix = ofp8_suffix(format, dst);
            push_ofp8_variants(
                &mut out,
                "vfwcvt_f_f_v",
                &format!("{fp8_suffix}_{}", wide.suffix()),
                &wide.cty(),
                &ofp8_ctype(format, dst),
                dst,
                PolicySet::Mask,
            );
        }
    }
    out
}

/// Zvfofp8min narrowing conversions from BF16/FP32 to either OFP8 format,
/// including saturating and explicit-rounding forms. Source widths are kept in
/// numeric order (BF16 before FP32).
fn kind_vfncvt_ofp8(name: &str) -> Vec<String> {
    debug_assert_eq!(name, "vfncvt");
    let mut out = Vec::new();
    for &(src_class, src_sew, lmul_shift, form) in &[('b', 16, 1, "w"), ('f', 32, 2, "q")] {
        for &format in OFP8_FORMATS {
            for dst_suffix in dtype_suffixes(&["u8"], 1) {
                let dst = DType::parse(&dst_suffix).unwrap();
                let Some(src_lmul) = shifted_lmul(dst.lmul, lmul_shift) else {
                    continue;
                };
                let mut src = dst;
                src.class = src_class;
                src.sew = src_sew;
                src.lmul = src_lmul;
                let fp8_suffix = ofp8_suffix(format, dst);
                let name_suffix = if form == "w" {
                    format!("{}_{}", src.suffix(), fp8_suffix)
                } else {
                    fp8_suffix
                };
                for operation in [
                    format!("vfncvt_f_f_{form}"),
                    format!("vfncvt_sat_f_f_{form}"),
                ] {
                    push_ofp8_variants(
                        &mut out,
                        &operation,
                        &name_suffix,
                        &ofp8_ctype(format, dst),
                        &src.cty(),
                        dst,
                        PolicySet::Round,
                    );
                }
            }
        }
    }
    out
}

#[derive(Clone, Copy)]
enum FPConvKind {
    Single,
    Widen,
    Narrow,
}

#[derive(Clone, Copy)]
enum PolicySet {
    Mask,
    Round,
}

fn policy_suffixes(p: PolicySet) -> &'static [&'static str] {
    match p {
        PolicySet::Mask => &["", "_m"],
        PolicySet::Round => &["", "_m", "_rm", "_rm_m"],
    }
}

fn shifted_lmul(lmul: usize, sh: i32) -> Option<usize> {
    let v = lmul as i32 + sh;
    (0..LMUL_ORDER.len() as i32)
        .contains(&v)
        .then_some(v as usize)
}

fn conv_src(dst: DType, src_class: char, scale: i32) -> Option<DType> {
    let mut src = dst;
    src.class = src_class;
    match scale {
        0 => {}
        -1 => {
            src.sew /= 2;
            src.lmul = shifted_lmul(src.lmul, -1)?;
        }
        1 => {
            src.sew *= 2;
            src.lmul = shifted_lmul(src.lmul, 1)?;
        }
        _ => return None,
    }
    matches!(src.sew, 8 | 16 | 32 | 64).then_some(src)
}

fn kind_fpcvt(name: &str, kind: FPConvKind) -> Vec<String> {
    let rules: &[(&str, &[&str], char, i32, PolicySet)] = match kind {
        FPConvKind::Single => &[
            ("x_f_v", &["i16", "i32", "i64"], 'f', 0, PolicySet::Round),
            ("rtz_x_f_v", &["i16", "i32", "i64"], 'f', 0, PolicySet::Mask),
            ("xu_f_v", &["u16", "u32", "u64"], 'f', 0, PolicySet::Round),
            (
                "rtz_xu_f_v",
                &["u16", "u32", "u64"],
                'f',
                0,
                PolicySet::Mask,
            ),
            ("f_x_v", &["f16", "f32", "f64"], 'i', 0, PolicySet::Round),
            ("f_xu_v", &["f16", "f32", "f64"], 'u', 0, PolicySet::Round),
        ],
        FPConvKind::Widen => &[
            ("f_x_v", &["f16", "f32", "f64"], 'i', -1, PolicySet::Mask),
            ("f_xu_v", &["f16", "f32", "f64"], 'u', -1, PolicySet::Mask),
            ("f_f_v", &["f32", "f64"], 'f', -1, PolicySet::Mask),
            ("x_f_v", &["i32", "i64"], 'f', -1, PolicySet::Round),
            ("rtz_x_f_v", &["i32", "i64"], 'f', -1, PolicySet::Mask),
            ("xu_f_v", &["u32", "u64"], 'f', -1, PolicySet::Round),
            ("rtz_xu_f_v", &["u32", "u64"], 'f', -1, PolicySet::Mask),
        ],
        FPConvKind::Narrow => &[
            ("x_f_w", &["i8", "i16", "i32"], 'f', 1, PolicySet::Round),
            ("rtz_x_f_w", &["i8", "i16", "i32"], 'f', 1, PolicySet::Mask),
            ("xu_f_w", &["u8", "u16", "u32"], 'f', 1, PolicySet::Round),
            ("rtz_xu_f_w", &["u8", "u16", "u32"], 'f', 1, PolicySet::Mask),
            ("f_x_w", &["f16", "f32"], 'i', 1, PolicySet::Round),
            ("f_xu_w", &["f16", "f32"], 'u', 1, PolicySet::Round),
            ("f_f_w", &["f16", "f32"], 'f', 1, PolicySet::Round),
            ("rod_f_f_w", &["f16", "f32"], 'f', 1, PolicySet::Mask),
        ],
    };

    let mut out = Vec::new();
    for &(form, elems, src_class, scale, policies) in rules {
        for dst in dtype_suffixes(elems, 1) {
            let dst_d = DType::parse(&dst).unwrap();
            let Some(src_d) = conv_src(dst_d, src_class, scale) else {
                continue;
            };
            if lane_count(src_d.sew, LMULS[src_d.lmul].1, LMULS[src_d.lmul].2).is_none() {
                continue;
            }
            let ret = dst_d.cty();
            let src = src_d.cty();
            for &pol in policy_suffixes(policies) {
                let mut args = format!("{src} vs2");
                if matches!(pol, "_rm" | "_rm_m") {
                    args.push_str(", size_t frm");
                }
                if matches!(pol, "_m" | "_rm_m") {
                    args = format!("{} vm, {args}", render_placeholder("d.mask", dst_d));
                }
                out.push(format!(
                    "{ret} __riscv_{name}_{form}_{dst}{pol}({args}, size_t vl)"
                ));
            }
        }
    }
    out
}

fn scalar_suffix(scalar: &str) -> &'static str {
    match scalar {
        "int8_t" => "i8",
        "int16_t" => "i16",
        "int32_t" => "i32",
        "int64_t" => "i64",
        "uint8_t" => "u8",
        "uint16_t" => "u16",
        "uint32_t" => "u32",
        "uint64_t" => "u64",
        "_Float16" => "f16",
        "float" => "f32",
        "double" => "f64",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_operands_are_ixlen() {
        let spec: Spec = serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap();
        let protos = spec.expand();
        let mut count = 0;
        for proto in protos {
            for (ty, name) in proto.params {
                if name == "frm" || name == "vxrm" {
                    assert_eq!(ty, "size_t", "{} has a non-iXLEN {name}", proto.name);
                    count += 1;
                }
            }
        }
        assert!(count > 0);
    }

    #[test]
    fn mask_scalar_results_are_ixlen() {
        let spec: Spec = serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap();
        let protos = spec.expand();
        for name in ["__riscv_vcpop_m_b1", "__riscv_vcpop_m_b1_m"] {
            assert_eq!(
                protos.iter().find(|p| p.name == name).unwrap().ret,
                "size_t"
            );
        }
        for name in ["__riscv_vfirst_m_b1", "__riscv_vfirst_m_b1_m"] {
            assert_eq!(
                protos.iter().find(|p| p.name == name).unwrap().ret,
                "ptrdiff_t"
            );
        }
    }

    #[test]
    fn masked_lane_count_uses_the_masked_data_group() {
        let spec: Spec = serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap();
        let protos = spec.expand();
        let indexed_segment = protos
            .iter()
            .find(|p| p.name == "__riscv_vluxseg2ei32_v_i8mf8x2_m")
            .unwrap();
        assert_eq!(
            indexed_segment.params[0],
            ("vbool64_t".to_string(), "vm".to_string())
        );

        // A reduction's destination has a different lane count. Its mask must
        // follow the reduced `vs2` group, so unconditionally preferring the
        // return type would incorrectly produce `vbool8_t` here.
        let reduction = protos
            .iter()
            .find(|p| p.name == "__riscv_vredsum_vs_i8mf8_i8m1_m")
            .unwrap();
        assert_eq!(
            reduction.params[0],
            ("vbool64_t".to_string(), "vm".to_string())
        );
    }

    #[test]
    fn indexed_type_keeps_the_data_lane_count() {
        let data = DType::parse("i32m1").unwrap();
        let index = data.transform("ei8");
        assert_eq!(data.lanes(), 2);
        assert_eq!(index.suffix(), "u8mf4");
        assert_eq!(index.lanes(), data.lanes());
    }

    #[test]
    fn zvdot4a8i_uses_packed_i32_carriers_and_accumulator_masks() {
        let spec: Spec = serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap();
        let protos = spec.expand();

        let params = |name: &str| {
            protos
                .iter()
                .find(|p| p.name == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .params
                .clone()
        };
        assert_eq!(
            params("__riscv_vdot4a_vv_i32m1"),
            vec![
                ("vint32m1_t".into(), "vd".into()),
                ("vuint32m1_t".into(), "vs2".into()),
                ("vuint32m1_t".into(), "vs1".into()),
                ("size_t".into(), "vl".into()),
            ]
        );
        assert_eq!(
            params("__riscv_vdot4asu_vv_i32m1")[1..3],
            [
                ("vuint32m1_t".into(), "vs2".into()),
                ("vuint32m1_t".into(), "vs1".into()),
            ]
        );
        assert_eq!(
            params("__riscv_vdot4aus_vx_i32m1")[1..3],
            [
                ("vuint32m1_t".into(), "vs2".into()),
                ("uint32_t".into(), "rs1".into()),
            ]
        );
        assert_eq!(
            params("__riscv_vdot4au_vv_u32m1")[1..3],
            [
                ("vuint32m1_t".into(), "vs2".into()),
                ("vuint32m1_t".into(), "vs1".into()),
            ]
        );

        let masked = params("__riscv_vdot4a_vv_i32m1_m");
        assert_eq!(masked[0], ("vbool32_t".into(), "vm".into()));
        assert_eq!(masked[2].0, "vuint32m1_t");
    }

    #[test]
    fn zvzip_matches_the_clang_non_policy_surface() {
        let spec: Spec = serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap();
        let protos = spec.expand();
        let zvzip: Vec<_> = protos
            .iter()
            .filter(|p| {
                matches!(
                    p.short().split('_').next(),
                    Some("vzip" | "vpaire" | "vpairo" | "vunzipe" | "vunzipo")
                )
            })
            .collect();
        let oracle = include_str!("../spec/vector/rvv_lowering.tsv");
        let expected_names: std::collections::BTreeSet<_> = oracle
            .lines()
            .filter_map(|line| line.split_once('\t').map(|(name, _)| name))
            .filter(|name| {
                matches!(
                    name.trim_start_matches("__riscv_").split('_').next(),
                    Some("vzip" | "vpaire" | "vpairo" | "vunzipe" | "vunzipo")
                )
            })
            .collect();
        let actual_names: std::collections::BTreeSet<_> =
            zvzip.iter().map(|p| p.name.clone()).collect();

        let expected_names: std::collections::BTreeSet<_> = expected_names
            .into_iter()
            .map(str::to_string)
            .chain(
                ["vpaire_vv", "vpairo_vv"]
                    .into_iter()
                    .flat_map(|family| {
                        ["bf16mf4", "bf16mf2", "bf16m1", "bf16m2", "bf16m4", "bf16m8"]
                            .into_iter()
                            .flat_map(move |suffix| {
                                [
                                    format!("__riscv_{family}_{suffix}"),
                                    format!("__riscv_{family}_{suffix}_m"),
                                ]
                            })
                    })
                    .chain(
                        ["vunzipe_v", "vunzipo_v"]
                            .into_iter()
                            .flat_map(|family| {
                                ["bf16mf4", "bf16mf2", "bf16m1", "bf16m2", "bf16m4"]
                                    .into_iter()
                                    .flat_map(move |suffix| {
                                        [
                                            format!("__riscv_{family}_{suffix}"),
                                            format!("__riscv_{family}_{suffix}_m"),
                                        ]
                                    })
                            })
                            .chain(
                                ["bf16mf2", "bf16m1", "bf16m2", "bf16m4", "bf16m8"]
                                    .into_iter()
                                    .flat_map(|suffix| {
                                        [
                                            format!("__riscv_vzip_vv_{suffix}"),
                                            format!("__riscv_vzip_vv_{suffix}_m"),
                                        ]
                                    }),
                            ),
                    ),
            )
            .collect();

        assert_eq!(zvzip.len(), 578);
        assert_eq!(actual_names, expected_names);
        for (family, expected) in [
            ("vzip", 106),
            ("vpaire", 130),
            ("vpairo", 130),
            ("vunzipe", 106),
            ("vunzipo", 106),
        ] {
            assert_eq!(
                zvzip
                    .iter()
                    .filter(|p| p.short().split('_').next() == Some(family))
                    .count(),
                expected,
                "{family}",
            );
        }

        let zip = zvzip
            .iter()
            .find(|p| p.name == "__riscv_vzip_vv_i8mf4")
            .unwrap();
        assert_eq!(zip.ret, "vint8mf4_t");
        assert_eq!(zip.params[0].0, "vint8mf8_t");

        let unsigned_zip = zvzip
            .iter()
            .find(|p| p.name == "__riscv_vzip_vv_u16mf2_m")
            .unwrap();
        assert_eq!(unsigned_zip.ret, "vuint16mf2_t");
        assert_eq!(unsigned_zip.params[0].0, "vbool64_t");
        assert_eq!(unsigned_zip.params[1].0, "vuint16mf4_t");
        assert_eq!(unsigned_zip.params[2].0, "vuint16mf4_t");

        let unzip_masked = zvzip
            .iter()
            .find(|p| p.name == "__riscv_vunzipe_v_i8mf8_m")
            .unwrap();
        assert_eq!(unzip_masked.ret, "vint8mf8_t");
        assert_eq!(unzip_masked.params[0].0, "vbool64_t");
        assert_eq!(unzip_masked.params[1].0, "vint8mf4_t");
    }

    #[test]
    fn ofp8_conversions_match_the_clang_non_policy_surface_and_scope() {
        let spec: Spec = serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap();
        let protos = spec.expand();
        let required_features = spec.required_features();
        let ofp8: Vec<_> = protos
            .iter()
            .filter(|p| p.short().contains("f8e4m3") || p.short().contains("f8e5m2"))
            .collect();
        let oracle = include_str!("../spec/vector/rvv_lowering.tsv");
        let expected_names: std::collections::BTreeSet<_> = oracle
            .lines()
            .filter_map(|line| line.split_once('\t').map(|(name, _)| name))
            .filter(|name| name.contains("f8e4m3") || name.contains("f8e5m2"))
            .collect();
        let actual_names: std::collections::BTreeSet<_> =
            ofp8.iter().map(|p| p.name.as_str()).collect();

        assert_eq!(ofp8.len(), 200);
        assert_eq!(actual_names, expected_names);
        for proto in &ofp8 {
            assert_eq!(
                required_features.get(proto.short()),
                Some(&vec!["zvfofp8min".to_string()]),
                "{}",
                proto.name
            );
        }
        for (prefix, expected) in [
            ("vfncvt_f_f_q", 40),
            ("vfncvt_f_f_w", 48),
            ("vfncvt_sat_f_f_q", 40),
            ("vfncvt_sat_f_f_w", 48),
            ("vfwcvt_f_f_v", 24),
        ] {
            assert_eq!(
                ofp8.iter()
                    .filter(|p| p.short().starts_with(prefix))
                    .count(),
                expected,
                "{prefix}",
            );
        }

        let narrow = ofp8
            .iter()
            .find(|p| p.name == "__riscv_vfncvt_f_f_w_bf16mf4_f8e4m3mf8_rm_m")
            .unwrap();
        assert_eq!(narrow.ret, "vfloat8e4m3mf8_t");
        assert_eq!(narrow.params[0].0, "vbool64_t");
        assert_eq!(narrow.params[1].0, "vbfloat16mf4_t");
        assert_eq!(narrow.params[2], ("size_t".to_string(), "frm".to_string()));

        let widen = ofp8
            .iter()
            .find(|p| p.name == "__riscv_vfwcvt_f_f_v_f8e5m2m4_bf16m8")
            .unwrap();
        assert_eq!(widen.ret, "vbfloat16m8_t");
        assert_eq!(widen.params[0].0, "vfloat8e5m2m4_t");
    }
}
