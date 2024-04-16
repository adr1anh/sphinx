use std::marker::PhantomData;

use crate::types::OuterDigest;
use p3_air::Air;
use p3_bn254_fr::Bn254Fr;
use p3_commit::TwoAdicMultiplicativeCoset;
use p3_field::TwoAdicField;
use wp1_core::stark::Com;
use wp1_core::{
    air::MachineAir,
    stark::{MachineStark, ShardCommitment, StarkGenericConfig, VerifyingKey},
};
use wp1_recursion_compiler::ir::Usize;
use wp1_recursion_compiler::ir::{Builder, Config};
use wp1_recursion_compiler::prelude::SymbolicVar;
use wp1_recursion_core::stark::config::outer_fri_config;
use wp1_recursion_program::commit::PolynomialSpaceVariable;
use wp1_recursion_program::folder::RecursiveVerifierConstraintFolder;

use crate::domain::{new_coset, TwoAdicMultiplicativeCosetVariable};
use crate::fri::verify_two_adic_pcs;
use crate::types::TwoAdicPcsMatsVariable;
use crate::types::TwoAdicPcsRoundVariable;
use crate::{challenger::MultiField32ChallengerVariable, types::RecursionShardProofVariable};

#[derive(Debug, Clone, Copy)]
pub struct StarkVerifierCircuit<C: Config, SC: StarkGenericConfig> {
    _phantom: PhantomData<(C, SC)>,
}

impl<C: Config, SC> StarkVerifierCircuit<C, SC>
where
    SC: StarkGenericConfig<
        Val = C::F,
        Challenge = C::EF,
        Domain = TwoAdicMultiplicativeCoset<C::F>,
    >,
{
    pub fn verify_shard<A>(
        builder: &mut Builder<C>,
        vk: &VerifyingKey<SC>,
        machine: &MachineStark<SC, A>,
        challenger: &mut MultiField32ChallengerVariable<C>,
        proof: &RecursionShardProofVariable<C>,
    ) where
        A: MachineAir<C::F> + for<'a> Air<RecursiveVerifierConstraintFolder<'a, C>>,
        C::F: TwoAdicField,
        C::EF: TwoAdicField,
        Com<SC>: Into<[Bn254Fr; 1]>,
        SymbolicVar<<C as Config>::N>: From<Bn254Fr>,
    {
        let RecursionShardProofVariable {
            commitment,
            opened_values,
            sorted_chips,
            sorted_indices,
            ..
        } = proof;

        let ShardCommitment {
            main_commit,
            permutation_commit,
            quotient_commit,
        } = commitment;

        let permutation_challenges = (0..2)
            .map(|_| challenger.sample_ext(builder))
            .collect::<Vec<_>>();

        challenger.observe_commitment(builder, *permutation_commit);

        let alpha = challenger.sample_ext(builder);

        challenger.observe_commitment(builder, *quotient_commit);

        let zeta = challenger.sample_ext(builder);

        let num_shard_chips = opened_values.chips.len();
        let mut trace_domains = Vec::new();
        let mut quotient_domains = Vec::new();

        let log_quotient_degree_val = 1;

        let mut prep_mats: Vec<TwoAdicPcsMatsVariable<_>> = Vec::new();
        let mut main_mats: Vec<TwoAdicPcsMatsVariable<_>> = Vec::new();
        let mut perm_mats: Vec<TwoAdicPcsMatsVariable<_>> = Vec::new();

        let mut quotient_mats = Vec::new();

        let qc_points = vec![zeta];

        for (name, domain, _) in vk.chip_information.iter() {
            let chip_idx = machine
                .chips()
                .iter()
                .rposition(|chip| &chip.name() == name)
                .unwrap();
            let index = sorted_indices[chip_idx];
            let opening = &opened_values.chips[index];

            let domain_var: TwoAdicMultiplicativeCosetVariable<_> = builder.constant(*domain);

            let mut trace_points = Vec::new();
            let zeta_next = domain_var.next_point(builder, zeta);

            trace_points.push(zeta);
            trace_points.push(zeta_next);

            let prep_values = vec![
                opening.preprocessed.local.clone(),
                opening.preprocessed.next.clone(),
            ];
            let prep_mat = TwoAdicPcsMatsVariable::<C> {
                domain: *domain,
                points: trace_points.clone(),
                values: prep_values,
            };
            prep_mats.push(prep_mat);
        }

        for i in 0..num_shard_chips {
            let opening = &opened_values.chips[i];
            let domain = new_coset(builder, opening.log_degree);
            trace_domains.push(domain.clone());

            let log_quotient_size = opening.log_degree + log_quotient_degree_val;
            let quotient_domain =
                domain.create_disjoint_domain(builder, Usize::Const(log_quotient_size), None);
            quotient_domains.push(quotient_domain.clone());

            let mut trace_points = Vec::new();
            let zeta_next = domain.next_point(builder, zeta);
            trace_points.push(zeta);
            trace_points.push(zeta_next);

            let main_values = vec![opening.main.local.clone(), opening.main.next.clone()];
            let main_mat = TwoAdicPcsMatsVariable::<C> {
                domain: TwoAdicMultiplicativeCoset {
                    log_n: domain.log_n,
                    shift: domain.shift,
                },
                values: main_values,
                points: trace_points.clone(),
            };
            main_mats.push(main_mat);

            let perm_values = vec![
                opening.permutation.local.clone(),
                opening.permutation.next.clone(),
            ];
            let perm_mat = TwoAdicPcsMatsVariable::<C> {
                domain: TwoAdicMultiplicativeCoset {
                    log_n: domain.clone().log_n,
                    shift: domain.clone().shift,
                },
                values: perm_values,
                points: trace_points,
            };
            perm_mats.push(perm_mat);

            let qc_domains = quotient_domain.split_domains(builder, log_quotient_degree_val);
            for (j, qc_dom) in qc_domains.into_iter().enumerate() {
                let qc_vals_array = opening.quotient[j].clone();
                let qc_values = vec![qc_vals_array];
                let qc_mat = TwoAdicPcsMatsVariable::<C> {
                    domain: TwoAdicMultiplicativeCoset {
                        log_n: qc_dom.clone().log_n,
                        shift: qc_dom.clone().shift,
                    },
                    values: qc_values,
                    points: qc_points.clone(),
                };
                quotient_mats.push(qc_mat);
            }
        }

        let mut rounds = Vec::new();
        let prep_commit_val: [Bn254Fr; 1] = vk.commit.clone().into();
        let prep_commit: OuterDigest<C> = [builder.eval(prep_commit_val[0])];
        let prep_round = TwoAdicPcsRoundVariable {
            batch_commit: prep_commit,
            mats: prep_mats,
        };
        let main_round = TwoAdicPcsRoundVariable {
            batch_commit: *main_commit,
            mats: main_mats,
        };
        let perm_round = TwoAdicPcsRoundVariable {
            batch_commit: *permutation_commit,
            mats: perm_mats,
        };
        let quotient_round = TwoAdicPcsRoundVariable {
            batch_commit: *quotient_commit,
            mats: quotient_mats,
        };
        rounds.push(prep_round);
        rounds.push(main_round);
        rounds.push(perm_round);
        rounds.push(quotient_round);
        let config = outer_fri_config();
        verify_two_adic_pcs(builder, &config, &proof.opening_proof, challenger, &rounds);

        for (i, sorted_chip) in sorted_chips.iter().enumerate() {
            for chip in machine.chips() {
                if chip.name() == *sorted_chip {
                    let values = &opened_values.chips[i];
                    let trace_domain = &trace_domains[i];
                    let quotient_domain = &quotient_domains[i];
                    let qc_domains =
                        quotient_domain.split_domains(builder, chip.log_quotient_degree());
                    Self::verify_constraints(
                        builder,
                        chip,
                        values,
                        &proof.public_values,
                        trace_domain,
                        &qc_domains,
                        zeta,
                        alpha,
                        &permutation_challenges,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use crate::{
        challenger::MultiField32ChallengerVariable, fri::tests::const_two_adic_pcs_proof,
        stark::StarkVerifierCircuit,
    };
    use p3_baby_bear::DiffusionMatrixBabybear;
    use p3_bn254_fr::Bn254Fr;
    use p3_challenger::CanObserve;
    use p3_field::PrimeField32;
    use serial_test::serial;
    use wp1_core::{
        air::{MachineAir, PublicValues, Word},
        stark::{LocalProver, MachineStark, ShardCommitment, ShardProof, StarkGenericConfig},
    };
    use wp1_recursion_compiler::{
        constraints::{gnark_ffi, ConstraintBackend},
        ir::{Builder, Config},
        OuterConfig,
    };
    use wp1_recursion_core::{
        cpu::Instruction,
        runtime::{Opcode, Program, Runtime},
        stark::{config::BabyBearPoseidon2Outer, RecursionAir},
    };
    use wp1_recursion_program::types::PublicValuesVariable;

    use crate::types::{
        ChipOpenedValuesVariable, OuterDigest, RecursionShardOpenedValuesVariable,
        RecursionShardProofVariable,
    };

    type SC = BabyBearPoseidon2Outer;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;
    type C = OuterConfig;
    type A = RecursionAir<F>;

    pub(crate) fn const_proof(
        builder: &mut Builder<C>,
        machine: &MachineStark<SC, A>,
        proof: &ShardProof<SC>,
    ) -> RecursionShardProofVariable<C>
    where
        C: Config<F = F, EF = EF>,
    {
        // Set up the public values.
        let public_values: PublicValuesVariable<OuterConfig> =
            builder.constant(proof.public_values);

        // Set up the commitments.
        let main_commit: [Bn254Fr; 1] = proof.commitment.main_commit.into();
        let permutation_commit: [Bn254Fr; 1] = proof.commitment.permutation_commit.into();
        let quotient_commit: [Bn254Fr; 1] = proof.commitment.quotient_commit.into();
        let main_commit: OuterDigest<C> = [builder.eval(main_commit[0])];
        let permutation_commit: OuterDigest<C> = [builder.eval(permutation_commit[0])];
        let quotient_commit: OuterDigest<C> = [builder.eval(quotient_commit[0])];

        let commitment = ShardCommitment {
            main_commit,
            permutation_commit,
            quotient_commit,
        };

        // Set up the opened values.
        let mut opened_values = Vec::new();
        for values in proof.opened_values.chips.iter() {
            let values: ChipOpenedValuesVariable<_> = builder.constant(values.clone());
            opened_values.push(values);
        }
        let opened_values = RecursionShardOpenedValuesVariable {
            chips: opened_values,
        };

        let opening_proof = const_two_adic_pcs_proof(builder, &proof.opening_proof);

        let chips = machine
            .shard_chips_ordered(&proof.chip_ordering)
            .map(|chip| chip.name())
            .collect::<Vec<_>>();

        let sorted_indices = machine
            .chips()
            .iter()
            .map(|chip| {
                proof
                    .chip_ordering
                    .get(&chip.name())
                    .copied()
                    .unwrap_or(usize::MAX)
            })
            .collect::<Vec<_>>();

        RecursionShardProofVariable {
            index: proof.index,
            commitment,
            opened_values,
            opening_proof,
            sorted_chips: chips,
            public_values,
            sorted_indices,
        }
    }

    pub(crate) fn basic_program<F: PrimeField32>() -> Program<F> {
        let zero = [F::zero(); 4];
        let one = [F::one(), F::zero(), F::zero(), F::zero()];
        Program::<F> {
            instructions: [Instruction::new(
                Opcode::ADD,
                F::from_canonical_u32(3),
                zero,
                one,
                F::zero(),
                F::zero(),
                false,
                true,
            )]
            .repeat(1 << 2),
        }
    }

    #[test]
    #[serial]
    fn test_recursive_verify_shard_v2() {
        type SC = BabyBearPoseidon2Outer;
        type F = <SC as StarkGenericConfig>::Val;
        type EF = <SC as StarkGenericConfig>::Challenge;
        type A = RecursionAir<F>;

        wp1_core::utils::setup_logger();
        let program = basic_program::<F>();
        let config = SC::new();
        let mut runtime = Runtime::<F, EF, DiffusionMatrixBabybear>::new_no_perm(&program);
        runtime.run();
        let machine = A::machine(config);
        let (pk, vk) = machine.setup(&program);
        let mut challenger = machine.config().challenger();
        let proofs = machine
            .prove::<LocalProver<_, _>>(&pk, runtime.record, &mut challenger)
            .shard_proofs;

        let mut runtime = Runtime::<F, EF, DiffusionMatrixBabybear>::new_no_perm(&program);
        runtime.run();
        let mut challenger = machine.config().challenger();
        let proof = machine.prove::<LocalProver<_, _>>(&pk, runtime.record, &mut challenger);
        let mut challenger = machine.config().challenger();
        machine.verify(&vk, &proof, &mut challenger).unwrap();

        let mut challenger_val = machine.config().challenger();
        challenger_val.observe(vk.commit);
        for proof in proofs.iter() {
            challenger_val.observe(proof.commitment.main_commit);
            let public_values_field = PublicValues::<Word<F>, F>::new(proof.public_values);
            challenger_val.observe_slice(&public_values_field.to_vec());
        }

        let mut builder = Builder::<OuterConfig>::default();
        let mut challenger = MultiField32ChallengerVariable::new(&mut builder);

        let preprocessed_commit_val: [Bn254Fr; 1] = vk.commit.into();
        let preprocessed_commit: OuterDigest<C> = [builder.eval(preprocessed_commit_val[0])];
        challenger.observe_commitment(&mut builder, preprocessed_commit);

        let mut shard_proofs = vec![];
        for proof_val in proofs {
            let proof = const_proof(&mut builder, &machine, &proof_val);
            let ShardCommitment { main_commit, .. } = &proof.commitment;
            challenger.observe_commitment(&mut builder, *main_commit);
            let public_values_elms = proof.public_values.to_vec(&mut builder);
            challenger.observe_slice(&mut builder, &public_values_elms);
            shard_proofs.push(proof);
        }

        #[allow(clippy::never_loop)]
        for proof in shard_proofs {
            StarkVerifierCircuit::<C, SC>::verify_shard(
                &mut builder,
                &vk,
                &machine,
                &mut challenger.clone(),
                &proof,
            );
            break;
        }

        let mut backend = ConstraintBackend::<OuterConfig>::default();
        let constraints = backend.emit(builder.operations);
        gnark_ffi::test_circuit(&constraints);
    }
}
