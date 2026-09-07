//! Zvfbfmin-specific surface and lowering rules.
//!
//! Rust represents BF16 values as `u16` bit containers. Most Zvfbfmin APIs
//! only move those bits, so their LLVM overloads must use `i16`. The two
//! conversion families have BF16 numeric semantics and deliberately retain
//! LLVM's `bf16` mangling; rustc's intrinsic autocast bridges their public
//! `u16` vector types.

use crate::adoc::Proto;
use crate::emit::{Op, Recipe};
use crate::types::vtype_of_rust;

/// Whether a public prototype contains a BF16 scalar, vector, or tuple type.
pub fn uses_bf16(p: &Proto) -> bool {
    type_uses_bf16(&p.ret) || p.params.iter().any(|(ty, _)| type_uses_bf16(ty))
}

/// Whether a BF16 prototype needs the base Zvfbfmin feature explicitly.
///
/// Zvfbfa defines a separate arithmetic surface and does not imply Zvfbfmin in
/// Clang/LLVM. Zvfbfwma has its own feature with the appropriate implication
/// chain. All other BF16 APIs are the Zvfbfmin bit-preserving/conversion
/// surface and require Zvfbfmin.
pub fn requires_zvfbfmin(p: &Proto, explicit_required: &[String]) -> bool {
    uses_bf16(p)
        && !explicit_required
            .iter()
            .any(|feature| matches!(feature.as_str(), "zvfbfa" | "zvfbfwma" | "zvfofp8min"))
}

fn type_uses_bf16(ty: &str) -> bool {
    ty.contains("vbfloat16") || ty.contains("__bf16")
}

/// Whether this operation gives BF16 values numeric conversion semantics.
pub fn is_semantic_conversion(short: &str) -> bool {
    matches!(short.split('_').next(), Some("vfwcvtbf16" | "vfncvtbf16"))
}

/// Whether `short` belongs to either BF16 arithmetic extension.
fn is_semantic_arithmetic(short: &str) -> bool {
    if short.contains("f8e4m3") || short.contains("f8e5m2") {
        return false;
    }
    matches!(
        short.split('_').next(),
        Some(operation) if operation.starts_with("vf") || operation.starts_with("vmf")
    )
}

/// Whether a BF16-named operation should use an `i16` bit-container overload.
pub fn uses_bit_container_mangling(short: &str) -> bool {
    short.contains("bf16") && !is_semantic_conversion(short) && !is_semantic_arithmetic(short)
}

/// Construct the two Zvfbfmin semantic-conversion recipes.
///
/// These recipes cannot be derived from the ordinary f16/f32 families: their
/// intrinsic names are distinct and their BF16 slots must be mangled as
/// `nxv<N>bf16`, even though the Rust-facing vectors contain `u16`.
pub fn lowering_recipe(p: &Proto) -> Option<Recipe> {
    let short = p.short();
    if crate::policy::policy(short).is_some() {
        return None;
    }
    let operation = short.split('_').next()?;
    let masked = short.ends_with("_m");

    match operation {
        "vfwcvtbf16" => {
            let ret = vtype_of_rust(&p.ret)?;
            let suffix = if masked { ".mask" } else { "" };
            let ops = if masked {
                vec![
                    Op::Poison,
                    Op::Param(1),
                    Op::Param(0),
                    Op::Param(2),
                    Op::xlen(3),
                ]
            } else {
                vec![Op::Poison, Op::Param(0), Op::Param(1)]
            };
            Some(Recipe {
                llvm: format!(
                    "llvm.riscv.vfwcvtbf16.f.f.v{suffix}.nxv{}f32.nxv{}bf16.iXLEN",
                    ret.n, ret.n
                ),
                ops,
            })
        }
        "vfncvtbf16" => {
            let ret = vtype_of_rust(&p.ret)?;
            let rounding = short.ends_with("_rm") || short.ends_with("_rm_m");
            let suffix = if masked { ".mask" } else { "" };
            let ops = match (rounding, masked) {
                (false, false) => {
                    vec![Op::Poison, Op::Param(0), Op::xlen(7), Op::Param(1)]
                }
                (false, true) => vec![
                    Op::Poison,
                    Op::Param(1),
                    Op::Param(0),
                    Op::xlen(7),
                    Op::Param(2),
                    Op::xlen(3),
                ],
                (true, false) => {
                    vec![Op::Poison, Op::Param(0), Op::Param(1), Op::Param(2)]
                }
                (true, true) => vec![
                    Op::Poison,
                    Op::Param(1),
                    Op::Param(0),
                    Op::Param(2),
                    Op::Param(3),
                    Op::xlen(3),
                ],
            };
            Some(Recipe {
                llvm: format!(
                    "llvm.riscv.vfncvtbf16.f.f.w{suffix}.nxv{}bf16.nxv{}f32.iXLEN",
                    ret.n, ret.n
                ),
                ops,
            })
        }
        _ => clang_22_arithmetic_recipe(p),
    }
}

/// Exact non-policy semantic lowering extracted from Clang 22.
///
/// Keeping this separate from the generic oracle is intentional: BF16 public
/// types are represented by Rust as `u16`, while these arithmetic intrinsics
/// must retain `bf16` in every numeric LLVM overload slot.
fn clang_22_arithmetic_recipe(p: &Proto) -> Option<Recipe> {
    let (stem, source) = if p.short().starts_with("vfwmaccbf16_") {
        (
            p.name
                .replace("vfwmaccbf16", "vfwmacc")
                .replace("bf16", "f16"),
            include_str!("../spec/vector/rvv_lowering.tsv"),
        )
    } else if p.short().contains("bf16") && is_semantic_arithmetic(p.short()) {
        let names = include_str!("../spec/vector/zvfbfa_non_policy.txt");
        if !names.lines().any(|name| name == p.name) {
            return None;
        }
        let direct = p.name.replace("bf16", "f16");
        let remove = |components: &[&str]| {
            components.iter().find_map(|component| {
                let needle = format!("_{component}_");
                p.name
                    .contains(&needle)
                    .then(|| p.name.replacen(&needle, "_", 1).replace("bf16", "f16"))
            })
        };
        let candidates = [
            Some(direct),
            remove(&[
                "bf16mf8", "bf16mf4", "bf16mf2", "bf16m1", "bf16m2", "bf16m4", "bf16m8",
            ]),
            remove(&["bf16"]),
        ];
        let source = include_str!("../spec/vector/rvv_lowering.tsv");
        let stem = candidates.into_iter().flatten().find(|candidate| {
            source
                .lines()
                .any(|line| line.split('\t').next() == Some(candidate))
        })?;
        (stem, source)
    } else {
        return None;
    };

    source.lines().find_map(|line| {
        let mut fields = line.split('\t');
        if fields.next()? != stem {
            return None;
        }
        let mut llvm = fields
            .next()?
            .replace("nxv1f16", "nxv1bf16")
            .replace("nxv2f16", "nxv2bf16")
            .replace("nxv4f16", "nxv4bf16")
            .replace("nxv8f16", "nxv8bf16")
            .replace("nxv16f16", "nxv16bf16")
            .replace("nxv32f16", "nxv32bf16")
            .replace(".f16", ".bf16");
        if p.short().starts_with("vfwmaccbf16_") {
            llvm = llvm.replacen("llvm.riscv.vfwmacc", "llvm.riscv.vfwmaccbf16", 1);
        }
        let ops = fields
            .next()?
            .split(',')
            .map(|op| match op {
                "poison" => Some(Op::Poison),
                "ptr" => Some(Op::Ptr),
                _ if op.starts_with("#xlen:") => op[6..].parse().ok().map(Op::xlen),
                _ if op.starts_with("#i") => {
                    let (width, value) = op[2..].split_once(':')?;
                    Some(Op::fixed(value.parse().ok()?, width.parse().ok()?))
                }
                _ => op.parse().ok().map(Op::Param),
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Recipe { llvm, ops })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::spec::Spec;

    fn spec() -> Spec {
        serde_yaml::from_str(include_str!("../spec/vector/rvv.yml")).unwrap()
    }

    fn expected_names(file: &'static str) -> BTreeSet<&'static str> {
        file.lines().filter(|line| !line.is_empty()).collect()
    }

    fn actual_names(spec: &Spec, feature: &str) -> Vec<String> {
        let required = spec.required_features();
        spec.expand()
            .into_iter()
            .filter(|p| {
                required
                    .get(p.short())
                    .is_some_and(|features| features.iter().any(|item| item == feature))
            })
            .map(|p| p.name)
            .collect()
    }

    #[test]
    fn surface_exactly_matches_vendored_clang_22() {
        let spec = spec();
        let zvfbfa = actual_names(&spec, "zvfbfa")
            .into_iter()
            .collect::<BTreeSet<_>>();
        let zvfbfwma = actual_names(&spec, "zvfbfwma")
            .into_iter()
            .collect::<BTreeSet<_>>();
        let actual: Vec<_> = spec
            .expand()
            .into_iter()
            .filter(|p| uses_bf16(p))
            .filter(|p| !p.short().contains("f8e4m3") && !p.short().contains("f8e5m2"))
            .filter(|p| {
                !matches!(
                    p.short().split('_').next(),
                    Some("vzip" | "vpaire" | "vpairo" | "vunzipe" | "vunzipo")
                )
            })
            .map(|p| p.name)
            .filter(|name| !zvfbfa.contains(name) && !zvfbfwma.contains(name))
            .collect();
        let base_movement: BTreeSet<_> = ["vmerge_vvm", "vmv_v_v"]
            .into_iter()
            .flat_map(|operation| {
                ["mf4", "mf2", "m1", "m2", "m4", "m8"]
                    .into_iter()
                    .map(move |lmul| format!("__riscv_{operation}_bf16{lmul}"))
            })
            .collect();
        let actual_base_movement: BTreeSet<_> = actual
            .iter()
            .filter(|name| base_movement.contains(*name))
            .cloned()
            .collect();
        let actual_names: BTreeSet<_> = actual
            .iter()
            .filter(|name| !base_movement.contains(*name))
            .map(String::as_str)
            .collect();
        let expected = expected_names(include_str!("../spec/vector/zvfbfmin_non_policy.txt"));

        assert_eq!(expected.len(), 1544);
        assert_eq!(actual_base_movement, base_movement);
        assert_eq!(actual.len(), expected.len() + base_movement.len());
        assert_eq!(actual_names, expected);
        assert_eq!(
            actual.iter().filter(|name| name.contains("seg")).count(),
            1042
        );
        assert_eq!(
            actual
                .iter()
                .filter(|name| name.contains("vfwcvtbf16") || name.contains("vfncvtbf16"))
                .count(),
            30
        );
    }

    #[test]
    fn zvfbfa_surface_exactly_matches_vendored_clang_22() {
        let spec = spec();
        let actual = actual_names(&spec, "zvfbfa");
        let actual_names: BTreeSet<_> = actual.iter().map(String::as_str).collect();
        let expected = expected_names(include_str!("../spec/vector/zvfbfa_non_policy.txt"));
        assert_eq!(expected.len(), 1144);
        assert_eq!(actual.len(), actual_names.len(), "duplicate Zvfbfa API");
        assert_eq!(actual_names, expected);
    }

    #[test]
    fn zvfbfwma_surface_exactly_matches_vendored_clang_22() {
        let spec = spec();
        let actual = actual_names(&spec, "zvfbfwma");
        let actual_names: BTreeSet<_> = actual.iter().map(String::as_str).collect();
        let expected = expected_names(include_str!("../spec/vector/zvfbfwma_non_policy.txt"));
        assert_eq!(expected.len(), 40);
        assert_eq!(actual.len(), actual_names.len(), "duplicate Zvfbfwma API");
        assert_eq!(actual_names, expected);
    }

    #[test]
    fn conversion_recipes_keep_semantic_bf16_mangling() {
        let protos = spec().expand();
        let recipe = |name: &str| {
            let proto = protos.iter().find(|p| p.name == name).unwrap();
            lowering_recipe(proto).unwrap()
        };

        let widen = recipe("__riscv_vfwcvtbf16_f_f_v_f32mf2_m");
        assert_eq!(
            widen.llvm,
            "llvm.riscv.vfwcvtbf16.f.f.v.mask.nxv1f32.nxv1bf16.iXLEN"
        );
        assert!(matches!(
            widen.ops.as_slice(),
            [
                Op::Poison,
                Op::Param(1),
                Op::Param(0),
                Op::Param(2),
                Op::Const { value: 3, .. }
            ]
        ));

        let narrow = recipe("__riscv_vfncvtbf16_f_f_w_bf16mf4_rm_m");
        assert_eq!(
            narrow.llvm,
            "llvm.riscv.vfncvtbf16.f.f.w.mask.nxv1bf16.nxv1f32.iXLEN"
        );
        assert!(matches!(
            narrow.ops.as_slice(),
            [
                Op::Poison,
                Op::Param(1),
                Op::Param(0),
                Op::Param(2),
                Op::Param(3),
                Op::Const { value: 3, .. }
            ]
        ));
    }

    #[test]
    fn arithmetic_recipes_keep_every_semantic_bf16_slot() {
        let protos = spec().expand();
        let recipe = |name: &str| {
            let proto = protos.iter().find(|p| p.name == name).unwrap();
            lowering_recipe(proto).unwrap()
        };

        assert_eq!(
            recipe("__riscv_vfadd_vf_bf16mf4").llvm,
            "llvm.riscv.vfadd.nxv1bf16.bf16.iXLEN"
        );
        assert_eq!(
            recipe("__riscv_vfwadd_vv_bf16mf4_f32mf2_rm_m").llvm,
            "llvm.riscv.vfwadd.mask.nxv1f32.nxv1bf16.nxv1bf16.iXLEN"
        );
        assert_eq!(
            recipe("__riscv_vfclass_v_bf16mf4_u16mf4_m").llvm,
            "llvm.riscv.vfclass.mask.nxv1bf16.iXLEN"
        );
        assert_eq!(
            recipe("__riscv_vfwmaccbf16_vf_f32mf2_rm_m").llvm,
            "llvm.riscv.vfwmaccbf16.mask.nxv1f32.bf16.nxv1bf16.iXLEN"
        );
    }
}
