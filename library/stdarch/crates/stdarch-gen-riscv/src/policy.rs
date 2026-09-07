//! Clang-derived RISC-V vector policy API surface.
//!
//! `rvv_policy.tsv` contains the concrete non-overloaded policy intrinsics that
//! Clang 23 actually exposes.  This module deliberately does not synthesize a
//! cartesian product of suffixes: an entry is added only when it is present in
//! the manifest *and* its corresponding non-policy API is already part of the
//! Rust surface.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::adoc::{Proto, parse_proto};
use crate::emit::{IntTy, Op, Recipe};

const FORMAT: &str = "rvv-policy-v2";
const SCOPES: &[&str] = &[
    "base",
    "vector-crypto",
    "zvabd",
    "zvdot4a8i",
    "zvfbfa",
    "zvfbfmin",
    "zvfbfwma",
    "zvfofp8min",
    "zvzip",
];

/// A policy variant encoded by the public intrinsic suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Tail undisturbed, unmasked.
    Tu,
    /// Tail undisturbed, mask agnostic.
    Tum,
    /// Tail undisturbed, mask undisturbed.
    Tumu,
    /// Tail agnostic, mask undisturbed.
    Mu,
}

impl Policy {
    /// The LLVM policy immediate for masked operations.
    pub fn immediate(self) -> Option<usize> {
        match self {
            Self::Tu => None,
            Self::Tum => Some(2),
            Self::Tumu => Some(0),
            Self::Mu => Some(1),
        }
    }

    pub fn is_masked(self) -> bool {
        !matches!(self, Self::Tu)
    }
}

/// The selected policy additions for the current Rust non-policy surface.
pub struct Surface {
    pub protos: Vec<Proto>,
    pub recipes: HashMap<String, Recipe>,
    /// Policy short name -> existing non-policy short name.
    pub sources: HashMap<String, String>,
    /// Number of selected entries per manifest scope.
    pub counts: BTreeMap<String, usize>,
}

struct Entry {
    scope: String,
    proto: Proto,
    recipe: Recipe,
}

/// Parse and select the checked-in policy manifest.
///
/// Every malformed row, operand token, duplicate name, or unsorted entry is a
/// hard error. Every manifest row must have its corresponding non-policy API in
/// `base`; this keeps the checked-in Clang surface and the generated Rust
/// surface in lockstep without synthesizing missing combinations.
pub fn select(text: &str, base: &[Proto]) -> Result<Surface, String> {
    let entries = parse_manifest(text)?;
    let mut by_clang_name = HashMap::new();
    for proto in base {
        if policy(proto.short()).is_some() {
            return Err(format!(
                "non-policy source unexpectedly contains policy API `{}`",
                proto.name
            ));
        }
        if let Some(previous) = by_clang_name.insert(proto.name.clone(), proto) {
            return Err(format!(
                "duplicate non-policy API `{}` (previous `{}`)",
                proto.name, previous.name
            ));
        }
    }

    let mut protos = Vec::new();
    let mut recipes = HashMap::new();
    let mut sources = HashMap::new();
    let mut counts = BTreeMap::new();
    let mut public_names = HashSet::new();

    for mut entry in entries {
        let source_clang_name = non_policy_name(&entry.proto.name)
            .ok_or_else(|| format!("manifest entry is not a policy API: `{}`", entry.proto.name))?;
        let source = by_clang_name.get(&source_clang_name).ok_or_else(|| {
            format!(
                "policy API `{}` has no non-policy source `{source_clang_name}`",
                entry.proto.name
            )
        })?;
        validate_against_source(&entry, source)?;
        if entry.scope == "zvfofp8min" {
            retarget_types_to_source(&mut entry.proto, source);
        }

        entry.proto.section = source.section.clone();
        if !public_names.insert(entry.proto.name.clone()) {
            return Err(format!(
                "duplicate selected policy API after Rust aliasing: `{}`",
                entry.proto.name
            ));
        }
        recipes.insert(entry.proto.name.clone(), entry.recipe);
        sources.insert(entry.proto.short().to_string(), source.short().to_string());
        *counts.entry(entry.scope).or_default() += 1;
        protos.push(entry.proto);
    }

    Ok(Surface {
        protos,
        recipes,
        sources,
        counts,
    })
}

/// Decode a policy suffix, including explicit-rounding names such as
/// `_rm_tumu` (the final suffix is sufficient and leaves `_rm` in the stem).
pub fn policy(short: &str) -> Option<Policy> {
    if short.ends_with("_tumu") {
        Some(Policy::Tumu)
    } else if short.ends_with("_tum") {
        Some(Policy::Tum)
    } else if short.ends_with("_mu") {
        Some(Policy::Mu)
    } else if short.ends_with("_tu") {
        Some(Policy::Tu)
    } else {
        None
    }
}

fn non_policy_name(name: &str) -> Option<String> {
    for suffix in ["_tumu", "_tum", "_mu"] {
        if let Some(stem) = name.strip_suffix(suffix) {
            return Some(format!("{stem}_m"));
        }
    }
    name.strip_suffix("_tu").map(str::to_string)
}

fn parse_manifest(text: &str) -> Result<Vec<Entry>, String> {
    let mut format = None;
    let mut entries = Vec::new();
    let mut names = HashSet::new();
    let mut previous_key: Option<(String, String)> = None;

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
        let [scope, prototype, llvm, operands] = fields.as_slice() else {
            return Err(format!(
                "line {line_no}: expected 4 tab-separated fields, found {}",
                fields.len()
            ));
        };
        if !SCOPES.contains(scope) {
            return Err(format!("line {line_no}: unknown policy scope `{scope}`"));
        }
        let proto = parse_proto(prototype, "")
            .ok_or_else(|| format!("line {line_no}: invalid C prototype `{prototype}`"))?;
        if policy(proto.short()).is_none() {
            return Err(format!(
                "line {line_no}: `{}` has no supported policy suffix",
                proto.name
            ));
        }
        if !llvm.starts_with("llvm.riscv.")
            || !llvm
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_'))
        {
            return Err(format!("line {line_no}: invalid LLVM intrinsic `{llvm}`"));
        }
        let recipe = Recipe {
            llvm: llvm.to_string(),
            ops: parse_operands(operands, &proto, line_no)?,
        };
        validate_recipe(&proto, &recipe, line_no)?;

        let key = (scope.to_string(), proto.name.clone());
        if previous_key
            .as_ref()
            .is_some_and(|previous| previous >= &key)
        {
            return Err(format!(
                "line {line_no}: manifest is not strictly sorted: `{scope}` `{}`",
                proto.name
            ));
        }
        previous_key = Some(key);
        if !names.insert(proto.name.clone()) {
            return Err(format!("line {line_no}: duplicate API `{}`", proto.name));
        }
        entries.push(Entry {
            scope: scope.to_string(),
            proto,
            recipe,
        });
    }

    match format.as_deref() {
        Some(FORMAT) => {}
        Some(other) => return Err(format!("unsupported policy manifest format `{other}`")),
        None => return Err("missing `# format: rvv-policy-v2` header".to_string()),
    }
    if entries.is_empty() {
        return Err("policy manifest has no entries".to_string());
    }
    Ok(entries)
}

fn validate_recipe(proto: &Proto, recipe: &Recipe, line_no: usize) -> Result<(), String> {
    let variant = policy(proto.short()).expect("caller checked the suffix");
    if recipe.ops.iter().any(|op| matches!(op, Op::Poison)) {
        return Err(format!(
            "line {line_no}: policy API `{}` unexpectedly uses a poison passthru",
            proto.name
        ));
    }

    let passthru = usize::from(variant.is_masked());
    if proto.params.get(passthru).map(|(ty, _)| ty.as_str()) != Some(proto.ret.as_str())
        || !matches!(recipe.ops.first(), Some(Op::Param(index)) if *index == passthru)
    {
        return Err(format!(
            "line {line_no}: `{}` does not carry its result-typed passthru as LLVM operand 0",
            proto.name
        ));
    }

    if variant.is_masked() {
        let Some((mask_ty, mask_name)) = proto.params.first() else {
            return Err(format!(
                "line {line_no}: `{}` has no mask parameter",
                proto.name
            ));
        };
        if !mask_ty.starts_with("vbool") || !matches!(mask_name.as_str(), "mask" | "vm") {
            return Err(format!(
                "line {line_no}: `{}` must start with its control mask parameter",
                proto.name
            ));
        }
        if !recipe.llvm.contains(".mask")
            || recipe
                .ops
                .iter()
                .filter(|op| matches!(op, Op::Param(0)))
                .count()
                != 1
        {
            return Err(format!(
                "line {line_no}: `{}` recipe does not consume its mask through a masked LLVM intrinsic",
                proto.name
            ));
        }
        let expected = variant.immediate().expect("masked policy has an immediate") as i64;
        if let Some((actual, ty)) = explicit_policy(recipe)
            && (actual != expected || ty != IntTy::XLen)
        {
            return Err(format!(
                "line {line_no}: `{}` does not encode masked policy {expected} as an XLEN operand in the expected LLVM operand slot",
                proto.name
            ));
        }
    } else {
        if proto.params.first().is_some_and(|(ty, name)| {
            ty.starts_with("vbool") && matches!(name.as_str(), "mask" | "vm")
        }) {
            return Err(format!(
                "line {line_no}: unmasked `_tu` API `{}` starts with a control mask",
                proto.name
            ));
        }
        if recipe.llvm.contains(".mask") {
            return Err(format!(
                "line {line_no}: unmasked `_tu` API `{}` uses masked LLVM intrinsic `{}`",
                proto.name, recipe.llvm
            ));
        }

        // Clang's lowering is family-dependent here. Most `_tu` intrinsics
        // encode tail-undisturbed solely through operand 0, while operations
        // such as vslideup and two-source crypto also carry an explicit `2`.
        // Other constants can be data operands or a segment NF, so the exact
        // Clang-derived recipe remains authoritative.
    }

    validate_parameter_uses(proto, recipe, line_no)?;
    Ok(())
}

/// Return the explicit passthru parameter and masked policy for a special
/// compiler bridge (segment loads and fault-only-first loads).
pub fn bridge_operands<'a>(
    short: &str,
    params: &'a [(String, String)],
) -> Option<(&'a str, Option<usize>)> {
    let variant = policy(short)?;
    let passthru_index = usize::from(variant.is_masked());
    let (_, passthru) = params.get(passthru_index)?;
    Some((passthru, variant.immediate()))
}

fn explicit_policy(recipe: &Recipe) -> Option<(i64, IntTy)> {
    let index = recipe
        .ops
        .len()
        .checked_sub(usize::from(is_segment_load(&recipe.llvm)) + 1)?;
    match recipe.ops.get(index) {
        Some(Op::Const { value, ty }) => Some((*value, *ty)),
        _ => None,
    }
}

fn validate_parameter_uses(proto: &Proto, recipe: &Recipe, line_no: usize) -> Result<(), String> {
    let pointer_params: Vec<usize> = proto
        .params
        .iter()
        .enumerate()
        .filter_map(|(index, (ty, name))| (ty.contains('*') && name != "new_vl").then_some(index))
        .collect();
    let mut uses = vec![0usize; proto.params.len()];
    for op in &recipe.ops {
        match op {
            Op::Param(index) => uses[*index] += 1,
            Op::Ptr if pointer_params.len() == 1 => uses[pointer_params[0]] += 1,
            Op::Ptr => {
                return Err(format!(
                    "line {line_no}: `{}` has an ambiguous `ptr` operand",
                    proto.name
                ));
            }
            Op::Poison | Op::Const { .. } => {}
        }
    }

    for (index, ((_, name), count)) in proto.params.iter().zip(uses).enumerate() {
        // Fault-only-first bridges consume this out-parameter after the LLVM
        // aggregate call rather than passing it to the intrinsic itself.
        let valid = if name == "new_vl" {
            count == 0
        } else {
            count >= 1
        };
        if !valid {
            let expected = if name == "new_vl" {
                "zero"
            } else {
                "at least once"
            };
            return Err(format!(
                "line {line_no}: `{}` uses parameter {index} `{name}` {count} times; expected {expected}",
                proto.name
            ));
        }
    }
    Ok(())
}

fn is_segment_load(llvm: &str) -> bool {
    let operation = llvm.strip_prefix("llvm.riscv.").unwrap_or(llvm);
    operation.starts_with("vlseg")
        || operation.starts_with("vlsseg")
        || operation.starts_with("vloxseg")
        || operation.starts_with("vluxseg")
}

fn validate_against_source(entry: &Entry, source: &Proto) -> Result<(), String> {
    if !source_type_matches(&entry.scope, &entry.proto.ret, &source.ret) {
        return Err(format!(
            "policy API `{}` returns `{}`, but source `{}` returns `{}`",
            entry.proto.name, entry.proto.ret, source.name, source.ret
        ));
    }
    let variant = policy(entry.proto.short()).expect("manifest entries have policy suffixes");
    let policy_types: Vec<&str> = entry
        .proto
        .params
        .iter()
        .map(|(ty, _)| ty.as_str())
        .collect();
    let source_types: Vec<&str> = source.params.iter().map(|(ty, _)| ty.as_str()).collect();
    let passthru = usize::from(variant.is_masked());
    let inherited = policy_types
        .iter()
        .zip(&source_types)
        .all(|(policy, source)| source_type_matches(&entry.scope, policy, source))
        && policy_types.len() == source_types.len();
    let inserted = policy_types
        .get(passthru)
        .is_some_and(|ty| source_type_matches(&entry.scope, ty, &source.ret))
        && policy_types
            .iter()
            .enumerate()
            .filter_map(|(index, ty)| (index != passthru).then_some(*ty))
            .zip(&source_types)
            .all(|(policy, source)| source_type_matches(&entry.scope, policy, source))
        && policy_types.len() == source_types.len() + 1;
    if !inherited && !inserted {
        return Err(format!(
            "policy API `{}` is neither the source signature nor that signature with one result-typed passthru inserted at parameter {passthru}: source `{}`",
            entry.proto.name, source.name
        ));
    }
    Ok(())
}

/// Clang 22 models SiFive OFP8 values with unsigned-byte vector containers,
/// whereas Rust exposes distinct vector types for the E4M3 and E5M2 formats.
fn source_type_matches(scope: &str, clang: &str, rust: &str) -> bool {
    if clang == rust {
        return true;
    }
    if scope != "zvfofp8min" {
        return false;
    }
    let Some(lmul) = clang
        .strip_prefix("vuint8")
        .and_then(|ty| ty.strip_suffix("_t"))
    else {
        return false;
    };
    rust == format!("vfloat8e4m3{lmul}_t") || rust == format!("vfloat8e5m2{lmul}_t")
}

/// Preserve Clang's authoritative policy parameter names and recipe, but use
/// the public Rust types from the already-supported non-policy companion.
fn retarget_types_to_source(proto: &mut Proto, source: &Proto) {
    let variant = policy(proto.short()).expect("manifest entries have policy suffixes");
    let passthru = usize::from(variant.is_masked());
    let inserted = proto.params.len() == source.params.len() + 1;
    let mut source_types = source.params.iter().map(|(ty, _)| ty);
    for (index, (ty, _)) in proto.params.iter_mut().enumerate() {
        *ty = if inserted && index == passthru {
            source.ret.clone()
        } else {
            source_types
                .next()
                .expect("validated policy/source parameter shape")
                .clone()
        };
    }
    debug_assert!(source_types.next().is_none());
    proto.ret = source.ret.clone();
}

fn parse_operands(text: &str, proto: &Proto, line_no: usize) -> Result<Vec<Op>, String> {
    if text.is_empty() {
        return Err(format!("line {line_no}: empty operand recipe"));
    }
    text.split(',')
        .map(|token| match token {
            "poison" => Ok(Op::Poison),
            "ptr" => {
                if proto.params.iter().any(|(ty, _)| ty.contains('*')) {
                    Ok(Op::Ptr)
                } else {
                    Err(format!(
                        "line {line_no}: `ptr` operand but `{}` has no pointer parameter",
                        proto.name
                    ))
                }
            }
            token if token.starts_with("#xlen:") => token[6..]
                .parse::<i64>()
                .map(Op::xlen)
                .map_err(|_| format!("line {line_no}: invalid constant operand `{token}`")),
            token if token.starts_with("#i") => {
                let (width, value) = token[2..].split_once(':').ok_or_else(|| {
                    format!("line {line_no}: invalid constant operand `{token}`")
                })?;
                let width = width.parse::<u32>().map_err(|_| {
                    format!("line {line_no}: invalid constant operand `{token}`")
                })?;
                if !matches!(width, 8 | 16 | 32 | 64) {
                    return Err(format!(
                        "line {line_no}: invalid constant operand `{token}`"
                    ));
                }
                let value = value.parse::<i64>().map_err(|_| {
                    format!("line {line_no}: invalid constant operand `{token}`")
                })?;
                Ok(Op::fixed(value, width))
            }
            token if !token.is_empty() && token.bytes().all(|c| c.is_ascii_digit()) => {
                let index = token.parse::<usize>().map_err(|_| {
                    format!("line {line_no}: invalid parameter operand `{token}`")
                })?;
                if index >= proto.params.len() {
                    return Err(format!(
                        "line {line_no}: parameter operand {index} is out of range for `{}` ({} parameters)",
                        proto.name,
                        proto.params.len()
                    ));
                }
                Ok(Op::Param(index))
            }
            token => Err(format!("line {line_no}: invalid operand token `{token}`")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::spec::Spec;

    fn proto(text: &str) -> Proto {
        parse_proto(text, "section").unwrap()
    }

    #[test]
    fn suffix_maps_to_existing_non_policy_api() {
        for (name, expected, kind) in [
            (
                "__riscv_vadd_vv_i32m1_tu",
                "__riscv_vadd_vv_i32m1",
                Policy::Tu,
            ),
            (
                "__riscv_vadd_vv_i32m1_tum",
                "__riscv_vadd_vv_i32m1_m",
                Policy::Tum,
            ),
            (
                "__riscv_vadd_vv_i32m1_tumu",
                "__riscv_vadd_vv_i32m1_m",
                Policy::Tumu,
            ),
            (
                "__riscv_vadd_vv_i32m1_mu",
                "__riscv_vadd_vv_i32m1_m",
                Policy::Mu,
            ),
            (
                "__riscv_vfadd_vv_f32m1_rm_tu",
                "__riscv_vfadd_vv_f32m1_rm",
                Policy::Tu,
            ),
            (
                "__riscv_vfadd_vv_f32m1_rm_tumu",
                "__riscv_vfadd_vv_f32m1_rm_m",
                Policy::Tumu,
            ),
        ] {
            assert_eq!(policy(name), Some(kind));
            assert_eq!(non_policy_name(name).as_deref(), Some(expected));
        }
    }

    #[test]
    fn selection_uses_manifest_signature_and_recipe() {
        let base = vec![proto(
            "vint32m1_t __riscv_vdot4a_vv_i32m1_m(vbool32_t vm, vint32m1_t vd, vuint32m1_t vs2, vuint32m1_t vs1, size_t vl)",
        )];
        let manifest = concat!(
            "# format: rvv-policy-v2\n",
            "zvdot4a8i\tvint32m1_t __riscv_vdot4a_vv_i32m1_tumu(vbool32_t mask, vint32m1_t maskedoff, vuint32m1_t op1, vuint32m1_t op2, size_t vl)\t",
            "llvm.riscv.vdot4a.mask.nxv2i32.nxv2i32.iXLEN\t1,2,3,0,4,#xlen:0\n",
        );
        let surface = select(manifest, &base).unwrap();
        assert_eq!(surface.protos.len(), 1);
        assert_eq!(surface.protos[0].name, "__riscv_vdot4a_vv_i32m1_tumu");
        assert_eq!(surface.protos[0].params[0].1, "mask");
        assert_eq!(surface.protos[0].params[1].1, "maskedoff");
        assert_eq!(surface.sources["vdot4a_vv_i32m1_tumu"], "vdot4a_vv_i32m1_m");
        assert_eq!(surface.recipes.len(), 1);
    }

    #[test]
    fn malformed_manifest_is_rejected() {
        let base = vec![proto(
            "vint32m1_t __riscv_vadd_vv_i32m1(vint32m1_t op1, vint32m1_t op2, size_t vl)",
        )];
        for bad in [
            "base\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.nxv2i32.nxv2i32.i64\t0,1,2,3\n",
            "# format: rvv-policy-v2\nbase\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.nxv2i32.nxv2i32.i64\toops\n",
            "# format: rvv-policy-v2\nbase\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.nxv2i32.nxv2i32.i64\t0,1,2,9\n",
        ] {
            assert!(select(bad, &base).is_err(), "accepted: {bad}");
        }
    }

    #[test]
    fn validates_real_unmasked_policy_recipe_shapes() {
        let base = vec![
            proto("vint32m1_t __riscv_vadd_vv_i32m1(vint32m1_t op1, vint32m1_t op2, size_t vl)"),
            proto(
                "vint32m1_t __riscv_vslideup_vx_i32m1(vint32m1_t dest, vint32m1_t src, size_t offset, size_t vl)",
            ),
        ];
        let manifest = concat!(
            "# format: rvv-policy-v2\n",
            "base\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.nxv2i32.nxv2i32.i64\t0,1,2,3\n",
            "base\tvint32m1_t __riscv_vslideup_vx_i32m1_tu(vint32m1_t dest, vint32m1_t src, size_t offset, size_t vl)\tllvm.riscv.vslideup.nxv2i32.iXLEN\t0,1,2,3,#xlen:2\n",
        );
        assert_eq!(select(manifest, &base).unwrap().protos.len(), 2);
    }

    #[test]
    fn validates_masked_policy_immediates() {
        let base = vec![proto(
            "vint32m1_t __riscv_vadd_vv_i32m1_m(vbool32_t mask, vint32m1_t op1, vint32m1_t op2, size_t vl)",
        )];
        for (suffix, immediate) in [("tum", 2), ("tumu", 0), ("mu", 1)] {
            let manifest = format!(
                "# format: rvv-policy-v2\nbase\tvint32m1_t __riscv_vadd_vv_i32m1_{suffix}(vbool32_t mask, vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.mask.nxv2i32.nxv2i32.iXLEN\t1,2,3,0,4,#xlen:{immediate}\n"
            );
            assert_eq!(select(&manifest, &base).unwrap().protos.len(), 1);

            let fixed_width =
                manifest.replace(&format!("#xlen:{immediate}"), &format!("#i64:{immediate}"));
            assert!(select(&fixed_width, &base).is_err());
        }
    }

    #[test]
    fn rejects_inconsistent_policy_shape() {
        let base = vec![proto(
            "vint32m1_t __riscv_vadd_vv_i32m1(vint32m1_t op1, vint32m1_t op2, size_t vl)",
        )];
        for row in [
            // `_tu` cannot lower through a masked intrinsic.
            "base\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.mask.nxv2i32.nxv2i32.i64\t0,1,2,3\n",
            // Every public operand except an FOF `new_vl` must be consumed.
            "base\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.nxv2i32.nxv2i32.i64\t0,1,3\n",
            // Scopes are a closed manifest contract.
            "typo\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.nxv2i32.nxv2i32.i64\t0,1,2,3\n",
        ] {
            let manifest = format!("# format: rvv-policy-v2\n{row}");
            assert!(select(&manifest, &base).is_err(), "accepted: {row}");
        }
    }

    #[test]
    fn rejects_policy_api_in_non_policy_source() {
        let base = vec![proto(
            "vint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)",
        )];
        let manifest = concat!(
            "# format: rvv-policy-v2\n",
            "base\tvint32m1_t __riscv_vadd_vv_i32m1_tu(vint32m1_t maskedoff, vint32m1_t op1, vint32m1_t op2, size_t vl)\tllvm.riscv.vadd.nxv2i32.nxv2i32.i64\t0,1,2,3\n",
        );
        assert!(select(manifest, &base).is_err());
    }

    #[test]
    fn ofp8_policy_surface_is_an_exact_bidirectional_extension() {
        let spec: Spec = serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap();
        let base = spec.expand();
        let manifest = include_str!("../spec/vector/rvv_policy.tsv");
        let ofp8_manifest = manifest
            .lines()
            .filter(|line| line.starts_with('#') || line.starts_with("zvfofp8min\t"))
            .collect::<Vec<_>>()
            .join("\n");
        let surface = select(&ofp8_manifest, &base).unwrap();

        let sources: Vec<_> = base
            .iter()
            .filter(|proto| proto.short().contains("f8e4m3") || proto.short().contains("f8e5m2"))
            .collect();
        assert_eq!(sources.len(), 200);
        assert_eq!(surface.counts.get("zvfofp8min"), Some(&400));

        let expected: BTreeSet<_> = sources
            .iter()
            .flat_map(|source| {
                if let Some(stem) = source.short().strip_suffix("_m") {
                    ["_tum", "_tumu", "_mu"]
                        .into_iter()
                        .map(|suffix| format!("{stem}{suffix}"))
                        .collect::<Vec<_>>()
                } else {
                    vec![format!("{}_tu", source.short())]
                }
            })
            .collect();
        let actual: BTreeSet<_> = surface
            .sources
            .iter()
            .filter_map(|(policy, source)| {
                (source.contains("f8e4m3") || source.contains("f8e5m2")).then_some(policy.clone())
            })
            .collect();
        let missing: Vec<_> = expected.difference(&actual).cloned().collect();
        let unexpected: Vec<_> = actual.difference(&expected).cloned().collect();
        assert!(
            missing.is_empty() && unexpected.is_empty(),
            "Zvfofp8min policy surface differs from the 200 non-policy sources\nmissing: {missing:#?}\nunexpected: {unexpected:#?}"
        );
        assert_eq!(actual.len(), 400);

        for proto in surface
            .protos
            .iter()
            .filter(|proto| actual.contains(proto.short()))
        {
            assert!(
                proto.ret.contains("float8")
                    || proto.params.iter().any(|(ty, _)| ty.contains("float8")),
                "{} lost its public OFP8 type",
                proto.name
            );
            assert!(
                !proto.ret.starts_with("vuint8")
                    && proto.params.iter().all(|(ty, _)| !ty.starts_with("vuint8")),
                "{} leaked Clang's u8 OFP8 container type",
                proto.name
            );
        }
    }

    #[test]
    fn special_bridge_operands_follow_clang_policy_order() {
        let unmasked = vec![
            ("vint8m1_t".to_string(), "maskedoff".to_string()),
            ("*const i8".to_string(), "base".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        assert_eq!(
            bridge_operands("vle8ff_v_i8m1_tu", &unmasked),
            Some(("maskedoff", None))
        );

        let masked = vec![
            ("vbool8_t".to_string(), "mask".to_string()),
            ("vint8m1x2_t".to_string(), "maskedoff".to_string()),
            ("*const i8".to_string(), "base".to_string()),
            ("usize".to_string(), "vl".to_string()),
        ];
        for (suffix, immediate) in [("tum", 2), ("tumu", 0), ("mu", 1)] {
            assert_eq!(
                bridge_operands(&format!("vlseg2e8_v_i8m1x2_{suffix}"), &masked),
                Some(("maskedoff", Some(immediate)))
            );
        }
    }
}
