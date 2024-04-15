use p3_air::Air;
use p3_commit::TwoAdicMultiplicativeCoset;
use p3_field::AbstractField;
use p3_field::TwoAdicField;
use wp1_core::air::MachineAir;
use wp1_core::stark::Com;
use wp1_core::stark::MachineStark;
use wp1_core::stark::StarkGenericConfig;
use wp1_core::stark::VerifyingKey;
use wp1_recursion_compiler::ir::Array;
use wp1_recursion_compiler::ir::Ext;
use wp1_recursion_compiler::ir::ExtConst;
use wp1_recursion_compiler::ir::Var;
use wp1_recursion_compiler::ir::{Builder, Config, Usize};
use wp1_recursion_core::runtime::DIGEST_SIZE;

use crate::challenger::CanObserveVariable;
use crate::challenger::DuplexChallengerVariable;
use crate::challenger::FeltChallenger;
use crate::commit::PolynomialSpaceVariable;
use crate::folder::RecursiveVerifierConstraintFolder;
use crate::fri::types::TwoAdicPcsMatsVariable;
use crate::fri::types::TwoAdicPcsRoundVariable;
use crate::fri::TwoAdicMultiplicativeCosetVariable;
use crate::types::ShardCommitmentVariable;
use crate::{commit::PcsVariable, fri::TwoAdicFriPcsVariable, types::ShardProofVariable};

pub const EMPTY: usize = 0x_1111_1111;

#[derive(Debug, Clone, Copy)]
pub struct StarkVerifier<C: Config, SC: StarkGenericConfig> {
    _phantom: std::marker::PhantomData<(C, SC)>,
}

impl<C: Config, SC> StarkVerifier<C, SC>
where
    C::F: TwoAdicField,
    SC: StarkGenericConfig<
        Val = C::F,
        Challenge = C::EF,
        Domain = TwoAdicMultiplicativeCoset<C::F>,
    >,
{
    pub fn verify_shard<A>(
        builder: &mut Builder<C>,
        vk: &VerifyingKey<SC>,
        pcs: &TwoAdicFriPcsVariable<C>,
        machine: &MachineStark<SC, A>,
        challenger: &mut DuplexChallengerVariable<C>,
        proof: &ShardProofVariable<C>,
        permutation_challenges: &[C::EF],
        sorted_indices: &Array<C, Var<C::N>>,
    ) where
        A: MachineAir<C::F> + for<'a> Air<RecursiveVerifierConstraintFolder<'a, C>>,
        C::F: TwoAdicField,
        C::EF: TwoAdicField,
        Com<SC>: Into<[SC::Val; DIGEST_SIZE]>,
    {
        let ShardProofVariable {
            commitment,
            opened_values,
            opening_proof,
            ..
        } = proof;

        let ShardCommitmentVariable {
            main_commit,
            permutation_commit,
            quotient_commit,
        } = commitment;

        let permutation_challenges_var = (0..2)
            .map(|_| challenger.sample_ext(builder))
            .collect::<Vec<_>>();

        for i in 0..2 {
            builder.assert_ext_eq(
                permutation_challenges_var[i],
                permutation_challenges[i].cons(),
            );
        }

        challenger.observe(builder, permutation_commit.clone());

        let alpha = challenger.sample_ext(builder);

        challenger.observe(builder, quotient_commit.clone());

        let zeta = challenger.sample_ext(builder);

        let num_shard_chips = opened_values.chips.len();
        let mut trace_domains =
            builder.dyn_array::<TwoAdicMultiplicativeCosetVariable<_>>(num_shard_chips);
        let mut quotient_domains =
            builder.dyn_array::<TwoAdicMultiplicativeCosetVariable<_>>(num_shard_chips);

        // TODO: note hardcoding of log_quotient_degree. The value comes from:
        //         let max_constraint_degree = 3;
        //         let log_quotient_degree = log2_ceil_usize(max_constraint_degree - 1);
        let log_quotient_degree_val = 1;
        let log_quotient_degree = C::N::from_canonical_usize(log_quotient_degree_val);
        let num_quotient_chunks_val = 1 << log_quotient_degree_val;

        let num_preprocessed_chips = vk.chip_information.len();

        let mut prep_mats: Array<_, TwoAdicPcsMatsVariable<_>> =
            builder.dyn_array(num_preprocessed_chips);
        let mut main_mats: Array<_, TwoAdicPcsMatsVariable<_>> = builder.dyn_array(num_shard_chips);
        let mut perm_mats: Array<_, TwoAdicPcsMatsVariable<_>> = builder.dyn_array(num_shard_chips);

        let num_quotient_mats: Usize<_> = builder.eval(num_shard_chips * num_quotient_chunks_val);
        let mut quotient_mats: Array<_, TwoAdicPcsMatsVariable<_>> =
            builder.dyn_array(num_quotient_mats);

        let mut qc_points = builder.dyn_array::<Ext<_, _>>(1);
        builder.set(&mut qc_points, 0, zeta);

        // TODO FIX: There is something weird going on here because the number of chips may not match
        // the number of chips in a shard.
        for (i, (name, domain, _)) in vk.chip_information.iter().enumerate() {
            let chip_idx = machine
                .chips()
                .iter()
                .rposition(|chip| &chip.name() == name)
                .unwrap();
            let index = builder.get(sorted_indices, chip_idx);
            let opening = builder.get(&opened_values.chips, index);

            let domain: TwoAdicMultiplicativeCosetVariable<_> = builder.eval_const(*domain);

            let mut trace_points = builder.dyn_array::<Ext<_, _>>(2);
            let zeta_next = domain.next_point(builder, zeta);

            builder.set(&mut trace_points, 0, zeta);
            builder.set(&mut trace_points, 1, zeta_next);

            let mut prep_values = builder.dyn_array::<Array<C, _>>(2);
            builder.set(&mut prep_values, 0, opening.preprocessed.local);
            builder.set(&mut prep_values, 1, opening.preprocessed.next);
            let main_mat = TwoAdicPcsMatsVariable::<C> {
                domain: domain.clone(),
                values: prep_values,
                points: trace_points.clone(),
            };
            builder.set(&mut prep_mats, i, main_mat);
        }

        builder.range(0, num_shard_chips).for_each(|i, builder| {
            let opening = builder.get(&opened_values.chips, i);
            let domain = pcs.natural_domain_for_log_degree(builder, Usize::Var(opening.log_degree));
            builder.set(&mut trace_domains, i, domain.clone());

            let log_quotient_size: Usize<_> =
                builder.eval(opening.log_degree + log_quotient_degree);
            let quotient_domain =
                domain.create_disjoint_domain(builder, log_quotient_size, Some(pcs.config.clone()));
            builder.set(&mut quotient_domains, i, quotient_domain.clone());

            // let trace_opening_points

            let mut trace_points = builder.dyn_array::<Ext<_, _>>(2);
            let zeta_next = domain.next_point(builder, zeta);
            builder.set(&mut trace_points, 0, zeta);
            builder.set(&mut trace_points, 1, zeta_next);

            // Get the main matrix.
            let mut main_values = builder.dyn_array::<Array<C, _>>(2);
            builder.set(&mut main_values, 0, opening.main.local);
            builder.set(&mut main_values, 1, opening.main.next);
            let main_mat = TwoAdicPcsMatsVariable::<C> {
                domain: domain.clone(),
                values: main_values,
                points: trace_points.clone(),
            };
            builder.set(&mut main_mats, i, main_mat);

            // Get the permutation matrix.
            let mut perm_values = builder.dyn_array::<Array<C, _>>(2);
            builder.set(&mut perm_values, 0, opening.permutation.local);
            builder.set(&mut perm_values, 1, opening.permutation.next);
            let perm_mat = TwoAdicPcsMatsVariable::<C> {
                domain: domain.clone(),
                values: perm_values,
                points: trace_points,
            };
            builder.set(&mut perm_mats, i, perm_mat);

            // Get the quotient matrices and values.

            let qc_domains = quotient_domain.split_domains(builder, log_quotient_degree_val);
            let num_quotient_chunks = C::N::from_canonical_usize(1 << log_quotient_degree_val);
            for (j, qc_dom) in qc_domains.into_iter().enumerate() {
                let qc_vals_array = builder.get(&opening.quotient, j);
                let mut qc_values = builder.dyn_array::<Array<C, _>>(1);
                builder.set(&mut qc_values, 0, qc_vals_array);
                let qc_mat = TwoAdicPcsMatsVariable::<C> {
                    domain: qc_dom,
                    values: qc_values,
                    points: qc_points.clone(),
                };
                let j_n = C::N::from_canonical_usize(j);
                let index: Var<_> = builder.eval(i * num_quotient_chunks + j_n);
                builder.set(&mut quotient_mats, index, qc_mat);
            }
        });

        // Create the pcs rounds.
        let mut rounds = builder.dyn_array::<TwoAdicPcsRoundVariable<_>>(4);
        let prep_commit_val: [SC::Val; DIGEST_SIZE] = vk.commit.clone().into();
        let prep_commit = builder.eval_const(prep_commit_val.to_vec());
        let prep_round = TwoAdicPcsRoundVariable {
            batch_commit: prep_commit,
            mats: prep_mats,
        };
        let main_round = TwoAdicPcsRoundVariable {
            batch_commit: main_commit.clone(),
            mats: main_mats,
        };
        let perm_round = TwoAdicPcsRoundVariable {
            batch_commit: permutation_commit.clone(),
            mats: perm_mats,
        };
        let quotient_round = TwoAdicPcsRoundVariable {
            batch_commit: quotient_commit.clone(),
            mats: quotient_mats,
        };
        builder.set(&mut rounds, 0, prep_round);
        builder.set(&mut rounds, 1, main_round);
        builder.set(&mut rounds, 2, perm_round);
        builder.set(&mut rounds, 3, quotient_round);

        // Verify the pcs proof
        pcs.verify(builder, &rounds, opening_proof, challenger);

        for (i, chip) in machine.chips().iter().enumerate() {
            let index = builder.get(sorted_indices, i);
            builder
                .if_ne(index, C::N::from_canonical_usize(EMPTY))
                .then(|builder| {
                    let values = builder.get(&opened_values.chips, index);
                    let trace_domain = builder.get(&trace_domains, index);
                    let quotient_domain: TwoAdicMultiplicativeCosetVariable<_> =
                        builder.get(&quotient_domains, index);
                    let qc_domains =
                        quotient_domain.split_domains(builder, chip.log_quotient_degree());
                    Self::verify_constraints(
                        builder,
                        chip,
                        &values,
                        &proof.public_values_digest,
                        &trace_domain,
                        &qc_domains,
                        zeta,
                        alpha,
                        permutation_challenges,
                    );
                });
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Instant;

    use crate::challenger::CanObserveVariable;
    use crate::challenger::FeltChallenger;
    use crate::hints::Hintable;
    use crate::stark::Ext;
    use crate::stark::EMPTY;
    use crate::types::ShardCommitmentVariable;
    use p3_challenger::{CanObserve, FieldChallenger};
    use p3_field::AbstractField;
    use rand::Rng;
    use wp1_core::air::PublicValuesDigest;
    use wp1_core::runtime::Program;
    use wp1_core::{
        stark::{RiscvAir, ShardProof, StarkGenericConfig},
        utils::BabyBearPoseidon2,
    };
    use wp1_recursion_compiler::ir::Array;
    use wp1_recursion_compiler::ir::Felt;
    use wp1_recursion_compiler::InnerConfig;
    use wp1_recursion_compiler::{
        asm::VmBuilder,
        ir::{Builder, ExtConst},
    };
    use wp1_recursion_core::runtime::{Runtime, DIGEST_SIZE};
    use wp1_recursion_core::stark::config::inner_fri_config;
    use wp1_recursion_core::stark::config::InnerChallenge;
    use wp1_recursion_core::stark::config::InnerVal;
    use wp1_sdk::{SP1Prover, SP1Stdin};

    use wp1_core::air::Word;

    use crate::{
        challenger::DuplexChallengerVariable,
        fri::{const_fri_config, TwoAdicFriPcsVariable},
        stark::StarkVerifier,
    };

    use wp1_core::stark::LocalProver;
    use wp1_recursion_core::stark::RecursionAir;
    use wp1_sdk::utils::setup_logger;

    type SC = BabyBearPoseidon2;
    type F = InnerVal;
    type EF = InnerChallenge;
    type C = InnerConfig;
    type A = RiscvAir<F>;

    #[test]
    fn test_permutation_challenges() {
        // Generate a dummy proof.
        setup_logger();
        let elf =
            include_bytes!("../../../examples/fibonacci/program/elf/riscv32im-succinct-zkvm-elf");

        let machine = A::machine(SC::default());
        let (_, vk) = machine.setup(&Program::from(elf));
        let mut challenger_val = machine.config().challenger();
        let proofs = SP1Prover::prove_with_config(elf, SP1Stdin::new(), machine.config().clone())
            .unwrap()
            .proof
            .shard_proofs;
        println!("Proof generated successfully");

        challenger_val.observe(vk.commit);

        for proof in proofs.iter() {
            challenger_val.observe(proof.commitment.main_commit);
        }

        let permutation_challenges = (0..2)
            .map(|_| challenger_val.sample_ext_element::<EF>())
            .collect::<Vec<_>>();

        // Observe all the commitments.
        let mut builder = Builder::<InnerConfig>::default();

        let mut challenger = DuplexChallengerVariable::new(&mut builder);

        let preprocessed_commit_val: [F; DIGEST_SIZE] = vk.commit.into();
        let preprocessed_commit: Array<C, _> = builder.eval_const(preprocessed_commit_val.to_vec());
        challenger.observe(&mut builder, preprocessed_commit);

        let mut witness_stream = Vec::new();
        for proof in proofs {
            witness_stream.extend(proof.write());
            let proof = ShardProof::<_>::read(&mut builder);
            let ShardCommitmentVariable { main_commit, .. } = proof.commitment;
            challenger.observe(&mut builder, main_commit);
        }

        // Sample the permutation challenges.
        let permutation_challenges_var = (0..2)
            .map(|_| challenger.sample_ext(&mut builder))
            .collect::<Vec<_>>();

        for i in 0..2 {
            builder.assert_ext_eq(
                permutation_challenges_var[i],
                permutation_challenges[i].cons(),
            );
        }

        let program = builder.compile();

        let mut runtime = Runtime::<F, EF, _>::new(&program, machine.config().perm.clone());
        runtime.witness_stream = witness_stream;
        runtime.run();
        println!(
            "The program executed successfully, number of cycles: {}",
            runtime.timestamp
        );
    }

    #[test]
    fn test_recursive_verify_shard() {
        // Generate a dummy proof.
        setup_logger();

        let elf =
            include_bytes!("../../../examples/fibonacci/program/elf/riscv32im-succinct-zkvm-elf");

        let machine = A::machine(SC::default());

        let (_, vk) = machine.setup(&Program::from(elf));
        let proof = SP1Prover::prove_with_config(elf, SP1Stdin::new(), machine.config().clone())
            .unwrap()
            .proof;
        let mut challenger_ver = machine.config().challenger();
        machine.verify(&vk, &proof, &mut challenger_ver).unwrap();
        println!("Proof generated successfully");

        let mut challenger_val = machine.config().challenger();
        challenger_val.observe(vk.commit);
        for proof in proof.shard_proofs.iter() {
            challenger_val.observe(proof.commitment.main_commit);
        }

        // Observe the public input digest
        let pv_digest_field_elms: Vec<F> =
            PublicValuesDigest::<Word<F>>::new(proof.public_values_digest).into();
        challenger_val.observe_slice(&pv_digest_field_elms);

        let permutation_challenges = (0..2)
            .map(|_| challenger_val.sample_ext_element::<EF>())
            .collect::<Vec<_>>();

        let time = Instant::now();
        let mut builder = Builder::<InnerConfig>::default();
        let config = const_fri_config(&mut builder, &inner_fri_config());
        let pcs = TwoAdicFriPcsVariable { config };

        let mut challenger = DuplexChallengerVariable::new(&mut builder);

        let preprocessed_commit_val: [F; DIGEST_SIZE] = vk.commit.into();
        let preprocessed_commit: Array<C, _> = builder.eval_const(preprocessed_commit_val.to_vec());
        challenger.observe(&mut builder, preprocessed_commit);

        let mut witness_stream = Vec::new();
        let mut shard_proofs = vec![];
        let mut sorted_indices = vec![];
        for proof_val in proof.shard_proofs {
            witness_stream.extend(proof_val.write());
            let sorted_indices_raw: Vec<usize> = machine
                .chips_sorted_indices(&proof_val)
                .into_iter()
                .map(|x| match x {
                    Some(x) => x,
                    None => EMPTY,
                })
                .collect();
            witness_stream.extend(sorted_indices_raw.write());
            let proof = ShardProof::<_>::read(&mut builder);
            let sorted_indices_arr = Vec::<usize>::read(&mut builder);
            builder
                .range(0, sorted_indices_arr.len())
                .for_each(|i, builder| {
                    let el = builder.get(&sorted_indices_arr, i);
                    builder.print_v(el);
                });
            let ShardCommitmentVariable { main_commit, .. } = &proof.commitment;
            challenger.observe(&mut builder, main_commit.clone());
            shard_proofs.push(proof);
            sorted_indices.push(sorted_indices_arr);
        }
        // Observe the public input digest
        let pv_digest_felt: Vec<Felt<F>> = pv_digest_field_elms
            .iter()
            .map(|x| builder.eval(*x))
            .collect();
        challenger.observe_slice(&mut builder, &pv_digest_felt);

        let code = builder.eval(InnerVal::two());
        builder.print_v(code);
        for (proof, sorted_indices) in shard_proofs.iter().zip(sorted_indices) {
            StarkVerifier::<C, SC>::verify_shard(
                &mut builder,
                &vk,
                &pcs,
                &machine,
                &mut challenger.clone(),
                proof,
                &permutation_challenges,
                &sorted_indices,
            );
        }

        let program = builder.compile();
        let elapsed = time.elapsed();
        println!("Building took: {:?}", elapsed);

        let time = Instant::now();
        let mut runtime = Runtime::<F, EF, _>::new(&program, machine.config().perm.clone());
        runtime.witness_stream = witness_stream;
        runtime.run();
        let elapsed = time.elapsed();
        runtime.print_stats();
        println!("Execution took: {:?}", elapsed);
    }

    #[test]
    #[ignore]
    fn test_kitchen_sink() {
        setup_logger();

        let time = Instant::now();
        let mut builder = VmBuilder::<F, EF>::default();

        let a: Felt<_> = builder.eval(F::from_canonical_u32(23));
        let b: Felt<_> = builder.eval(F::from_canonical_u32(17));
        let a_plus_b = builder.eval(a + b);
        let mut rng = rand::thread_rng();
        let a_ext_val = rng.gen::<EF>();
        let b_ext_val = rng.gen::<EF>();
        let a_ext: Ext<_, _> = builder.eval(a_ext_val.cons());
        let b_ext: Ext<_, _> = builder.eval(b_ext_val.cons());
        let a_plus_b_ext = builder.eval(a_ext + b_ext);
        builder.print_f(a_plus_b);
        builder.print_e(a_plus_b_ext);

        let program = builder.compile();
        let elapsed = time.elapsed();
        println!("Building took: {:?}", elapsed);

        let machine = A::machine(SC::default());
        let mut runtime = Runtime::<F, EF, _>::new(&program, machine.config().perm.clone());

        let time = Instant::now();
        runtime.run();
        let elapsed = time.elapsed();
        runtime.print_stats();
        println!("Execution took: {:?}", elapsed);

        let config = BabyBearPoseidon2::new();
        let machine = RecursionAir::machine(config);
        let (pk, vk) = machine.setup(&program);
        let mut challenger = machine.config().challenger();

        let record_clone = runtime.record.clone();
        machine.debug_constraints(&pk, record_clone, &mut challenger);

        let start = Instant::now();
        let mut challenger = machine.config().challenger();
        let proof = machine.prove::<LocalProver<_, _>>(&pk, runtime.record, &mut challenger);
        let duration = start.elapsed().as_secs();

        let mut challenger = machine.config().challenger();
        machine.verify(&vk, &proof, &mut challenger).unwrap();
        println!("proving duration = {}", duration);
    }
}
