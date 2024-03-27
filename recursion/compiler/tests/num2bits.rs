use p3_field::AbstractField;
use wp1_core::stark::StarkGenericConfig;
use wp1_core::utils::BabyBearPoseidon2;
use wp1_recursion_compiler::asm::VmBuilder;
use wp1_recursion_core::runtime::Runtime;

#[test]
fn test_compiler_for_loops() {
    type SC = BabyBearPoseidon2;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;
    let mut builder = VmBuilder::<F, EF>::default();

    let f = builder.eval(F::from_canonical_usize(1462788387));
    builder.num2bits_f(f);

    let program = builder.compile();

    let config = SC::default();
    let mut runtime = Runtime::<F, EF, _>::new(&program, config.perm.clone());
    runtime.run();
}
