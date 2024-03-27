use p3_challenger::CanObserve;
use p3_challenger::CanSample;
use p3_challenger::CanSampleBits;
use p3_field::AbstractField;
use p3_field::PrimeField32;
use wp1_core::stark::StarkGenericConfig;
use wp1_core::utils::BabyBearPoseidon2;
use wp1_recursion_compiler::asm::AsmConfig;
use wp1_recursion_compiler::asm::VmBuilder;
use wp1_recursion_compiler::ir::Array;
use wp1_recursion_compiler::ir::Ext;
use wp1_recursion_compiler::ir::Felt;
use wp1_recursion_compiler::ir::Usize;
use wp1_recursion_compiler::ir::Var;
use wp1_recursion_compiler::verifier::challenger::DuplexChallengerVariable;
use wp1_recursion_core::runtime::Runtime;
use wp1_recursion_core::runtime::POSEIDON2_WIDTH;

#[test]
fn test_compiler_challenger_1() {
    type SC = BabyBearPoseidon2;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;

    let config = SC::default();
    let mut challenger = config.challenger();
    let mut builder = VmBuilder::<F, EF>::default();

    challenger.observe(F::one());
    let result1: F = challenger.sample();
    challenger.observe(F::one());
    let result2: F = challenger.sample();
    challenger.observe(F::two());

    println!("DONE");

    let width: Var<_> = builder.eval(F::from_canonical_usize(POSEIDON2_WIDTH));
    let mut challenger = DuplexChallengerVariable::<AsmConfig<F, EF>> {
        sponge_state: builder.array(Usize::Var(width)),
        nb_inputs: builder.eval(F::zero()),
        input_buffer: builder.array(Usize::Var(width)),
        nb_outputs: builder.eval(F::zero()),
        output_buffer: builder.array(Usize::Var(width)),
    };
    let one: Felt<_> = builder.eval(F::one());

    challenger.observe(&mut builder, one);
    let element1 = challenger.sample(&mut builder);
    challenger.observe(&mut builder, one);
    let element2 = challenger.sample(&mut builder);

    let expected_result_1: Felt<_> = builder.eval(result1);
    builder.assert_felt_eq(expected_result_1, element1);

    let expected_result_2: Felt<_> = builder.eval(result2);
    builder.assert_felt_eq(expected_result_2, element2);

    let program = builder.compile();

    let mut runtime = Runtime::<F, EF, _>::new(&program, config.perm.clone());
    runtime.run();
    println!(
        "The program executed successfully, number of cycles: {}",
        runtime.clk.as_canonical_u32() / 4
    );
}

#[test]
fn test_compiler_challenger_2() {
    type SC = BabyBearPoseidon2;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;

    let config = SC::default();
    let mut challenger = config.challenger();
    let mut builder = VmBuilder::<F, EF>::default();

    for i in 0..73 {
        challenger.observe(F::from_canonical_usize(i));
        let _: F = challenger.sample();
    }
    let result = challenger.sample_bits(14);

    let width: Var<_> = builder.eval(F::from_canonical_usize(POSEIDON2_WIDTH));
    let mut challenger = DuplexChallengerVariable::<AsmConfig<F, EF>> {
        sponge_state: builder.array(Usize::Var(width)),
        nb_inputs: builder.eval(F::zero()),
        input_buffer: builder.array(Usize::Var(width)),
        nb_outputs: builder.eval(F::zero()),
        output_buffer: builder.array(Usize::Var(width)),
    };

    for i in 0..73 {
        let element = builder.eval(F::from_canonical_usize(i));
        challenger.observe(&mut builder, element);
        challenger.sample(&mut builder);
    }

    let element = challenger.sample_bits(&mut builder, Usize::Const(14));

    let expected_result: Var<_> = builder.eval(F::from_canonical_usize(result));
    builder.assert_var_eq(expected_result, element);

    let program = builder.compile();

    let mut runtime = Runtime::<F, EF, _>::new(&program, config.perm.clone());
    runtime.run();
    println!(
        "The program executed successfully, number of cycles: {}",
        runtime.clk.as_canonical_u32() / 4
    );
}

#[test]
fn test_compiler_challenger_3() {
    type SC = BabyBearPoseidon2;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;

    let config = SC::default();
    let mut challenger = config.challenger();
    let mut builder = VmBuilder::<F, EF>::default();

    challenger.observe(F::one());
    let result1 = challenger.sample_bits(16);

    let width: Var<_> = builder.eval(F::from_canonical_usize(POSEIDON2_WIDTH));
    let mut challenger = DuplexChallengerVariable::<AsmConfig<F, EF>> {
        sponge_state: builder.array(Usize::Var(width)),
        nb_inputs: builder.eval(F::zero()),
        input_buffer: builder.array(Usize::Var(width)),
        nb_outputs: builder.eval(F::zero()),
        output_buffer: builder.array(Usize::Var(width)),
    };
    let one: Felt<_> = builder.eval(F::one());
    challenger.observe(&mut builder, one);
    let element1 = challenger.sample_bits(&mut builder, Usize::Const(16));

    let expected_result_1: Var<_> = builder.eval(F::from_canonical_usize(result1));
    builder.assert_var_eq(expected_result_1, element1);

    let a: Var<_> = builder.eval(F::from_canonical_usize(1462788385));
    let b: Var<_> = builder.eval(F::from_canonical_usize(1462788385));
    builder.assert_var_eq(a, b);

    let program = builder.compile();

    let mut runtime = Runtime::<F, EF, _>::new(&program, config.perm.clone());
    runtime.run();
    println!(
        "The program executed successfully, number of cycles: {}",
        runtime.clk.as_canonical_u32() / 4
    );
}

#[test]
fn test_compiler_challenger_() {
    type SC = BabyBearPoseidon2;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;

    let config = SC::default();
    let mut challenger = config.challenger();
    let mut builder = VmBuilder::<F, EF>::default();

    for _ in 0..73 {
        challenger.observe(F::one());
        let _: EF = challenger.sample();
    }
    let result = challenger.sample_bits(14);

    let width: Var<_> = builder.eval(F::from_canonical_usize(POSEIDON2_WIDTH));
    let mut challenger = DuplexChallengerVariable::<AsmConfig<F, EF>> {
        sponge_state: builder.array(Usize::Var(width)),
        nb_inputs: builder.eval(F::zero()),
        input_buffer: builder.array(Usize::Var(width)),
        nb_outputs: builder.eval(F::zero()),
        output_buffer: builder.array(Usize::Var(width)),
    };

    let mut betas: Array<_, Ext<F, EF>> = builder.dyn_array(100);
    let end: Var<_> = builder.eval(F::from_canonical_usize(73));
    builder.range(0, end).for_each(|i, builder| {
        let element = builder.eval(F::one());
        challenger.observe(builder, element);
        let z = challenger.sample_ext(builder);
        builder.set(&mut betas, i, z);
    });

    let element = challenger.sample_bits(&mut builder, Usize::Const(14));

    let expected_result: Var<_> = builder.eval(F::from_canonical_usize(result));
    builder.assert_var_eq(expected_result, element);

    let program = builder.compile();

    let mut runtime = Runtime::<F, EF, _>::new(&program, config.perm.clone());
    runtime.run();
    println!(
        "The program executed successfully, number of cycles: {}",
        runtime.clk.as_canonical_u32() / 4
    );
}
