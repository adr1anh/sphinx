use p3_air::Air;
use p3_commit::LagrangeSelectors;
use p3_field::AbstractExtensionField;
use p3_field::AbstractField;
use p3_field::TwoAdicField;
use wp1_core::air::MachineAir;
use wp1_core::air::PV_DIGEST_NUM_WORDS;
use wp1_core::air::WORD_SIZE;
use wp1_core::stark::AirOpenedValues;
use wp1_core::stark::{MachineChip, StarkGenericConfig};
use wp1_recursion_compiler::ir::Felt;
use wp1_recursion_compiler::prelude::Config;
use wp1_recursion_compiler::prelude::ExtConst;
use wp1_recursion_compiler::prelude::{Builder, Ext, SymbolicExt};

use crate::commit::PolynomialSpaceVariable;
use crate::folder::RecursiveVerifierConstraintFolder;
use crate::fri::TwoAdicMultiplicativeCosetVariable;
use crate::stark::StarkVerifier;
use crate::types::ChipOpenedValuesVariable;
use crate::types::ChipOpening;
use crate::types::PublicValuesDigestVariable;

impl<C: Config, SC> StarkVerifier<C, SC>
where
    SC: StarkGenericConfig<Val = C::F, Challenge = C::EF>,
    C::F: TwoAdicField,
{
    fn eval_constrains<A>(
        builder: &mut Builder<C>,
        chip: &MachineChip<SC, A>,
        opening: &ChipOpening<C>,
        public_values_digest: &PublicValuesDigestVariable<C>,
        selectors: &LagrangeSelectors<Ext<C::F, C::EF>>,
        alpha: Ext<C::F, C::EF>,
        permutation_challenges: &[Ext<C::F, C::EF>],
    ) -> Ext<C::F, C::EF>
    where
        A: for<'b> Air<RecursiveVerifierConstraintFolder<'b, C>>,
    {
        let mut unflatten = |v: &[Ext<C::F, C::EF>]| {
            v.chunks_exact(SC::Challenge::D)
                .map(|chunk| {
                    builder.eval(
                        chunk
                            .iter()
                            .enumerate()
                            .map(|(e_i, &x)| x * C::EF::monomial(e_i).cons())
                            .sum::<SymbolicExt<_, _>>(),
                    )
                })
                .collect::<Vec<Ext<_, _>>>()
        };
        let perm_opening = AirOpenedValues {
            local: unflatten(&opening.permutation.local),
            next: unflatten(&opening.permutation.next),
        };

        let zero: Ext<SC::Val, SC::Challenge> = builder.eval(SC::Val::zero());
        let mut public_values: Vec<Felt<C::F>> = Vec::new();
        for i in 0..PV_DIGEST_NUM_WORDS {
            for j in 0..WORD_SIZE {
                let el = builder.get(public_values_digest, i * WORD_SIZE + j);
                public_values.push(el);
            }
        }
        let mut folder = RecursiveVerifierConstraintFolder {
            builder,
            preprocessed: opening.preprocessed.view(),
            main: opening.main.view(),
            perm: perm_opening.view(),
            perm_challenges: permutation_challenges,
            cumulative_sum: opening.cumulative_sum,
            public_values: &public_values,
            is_first_row: selectors.is_first_row,
            is_last_row: selectors.is_last_row,
            is_transition: selectors.is_transition,
            alpha,
            accumulator: zero,
        };

        chip.eval(&mut folder);
        folder.accumulator
    }

    fn recompute_quotient(
        builder: &mut Builder<C>,
        opening: &ChipOpening<C>,
        qc_domains: &[TwoAdicMultiplicativeCosetVariable<C>],
        zeta: Ext<C::F, C::EF>,
    ) -> Ext<C::F, C::EF> {
        let zps = qc_domains
            .iter()
            .enumerate()
            .map(|(i, domain)| {
                qc_domains
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, other_domain)| {
                        // Calculate: other_domain.zp_at_point(zeta)
                        //     * other_domain.zp_at_point(domain.first_point()).inverse()
                        let first_point: Ext<_, _> = builder.eval(domain.first_point());
                        other_domain.zp_at_point(builder, zeta)
                            * other_domain.zp_at_point(builder, first_point).inverse()
                    })
                    .product::<SymbolicExt<_, _>>()
            })
            .collect::<Vec<SymbolicExt<_, _>>>()
            .into_iter()
            .map(|x| builder.eval(x))
            .collect::<Vec<Ext<_, _>>>();

        builder.eval(
            opening
                .quotient
                .iter()
                .enumerate()
                .map(|(ch_i, ch)| {
                    assert_eq!(ch.len(), C::EF::D);
                    ch.iter()
                        .enumerate()
                        .map(|(e_i, &c)| zps[ch_i] * C::EF::monomial(e_i) * c)
                        .sum::<SymbolicExt<_, _>>()
                })
                .sum::<SymbolicExt<_, _>>(),
        )
    }

    /// Reference: `[sp1_core::stark::Verifier::verify_constraints]`
    pub fn verify_constraints<A>(
        builder: &mut Builder<C>,
        chip: &MachineChip<SC, A>,
        opening: &ChipOpenedValuesVariable<C>,
        public_values_digest: &PublicValuesDigestVariable<C>,
        trace_domain: &TwoAdicMultiplicativeCosetVariable<C>,
        qc_domains: &[TwoAdicMultiplicativeCosetVariable<C>],
        zeta: Ext<C::F, C::EF>,
        alpha: Ext<C::F, C::EF>,
        permutation_challenges: &[Ext<C::F, C::EF>],
    ) where
        A: MachineAir<C::F> + for<'a> Air<RecursiveVerifierConstraintFolder<'a, C>>,
    {
        let opening = ChipOpening::from_variable(builder, chip, opening);
        let sels = trace_domain.selectors_at_point(builder, zeta);

        let folded_constraints = Self::eval_constrains(
            builder,
            chip,
            &opening,
            public_values_digest,
            &sels,
            alpha,
            permutation_challenges,
        );

        let quotient: Ext<_, _> = Self::recompute_quotient(builder, &opening, qc_domains, zeta);

        // Assert that the quotient times the zerofier is equal to the folded constraints.
        builder.assert_ext_eq(folded_constraints * sels.inv_zeroifier, quotient);
    }
}

#[cfg(test)]
mod tests {
    use itertools::{izip, Itertools};
    use serde::{de::DeserializeOwned, Serialize};
    use wp1_core::{
        air::{PublicValuesDigest, Word, PV_DIGEST_NUM_WORDS, WORD_SIZE},
        runtime::Program,
        stark::{
            Chip, Com, Dom, MachineStark, OpeningProof, PcsProverData, RiscvAir, ShardCommitment,
            ShardMainData, ShardProof, StarkGenericConfig, Verifier,
        },
        utils::BabyBearPoseidon2,
    };
    use wp1_recursion_core::runtime::Runtime;
    use wp1_sdk::{SP1Prover, SP1Stdin, SP1Verifier};

    use p3_challenger::{CanObserve, FieldChallenger};
    use p3_field::PrimeField32;
    use wp1_recursion_compiler::{asm::VmBuilder, ir::Felt, prelude::ExtConst};

    use p3_commit::{Pcs, PolynomialSpace};

    use crate::{
        commit::PolynomialSpaceVariable,
        fri::TwoAdicMultiplicativeCosetVariable,
        stark::StarkVerifier,
        types::{ChipOpenedValuesVariable, ChipOpening},
    };

    #[allow(clippy::type_complexity)]
    fn get_shard_data<'a, SC>(
        machine: &'a MachineStark<SC, RiscvAir<SC::Val>>,
        proof: &'a ShardProof<SC>,
        challenger: &mut SC::Challenger,
    ) -> (
        Vec<&'a Chip<SC::Val, RiscvAir<SC::Val>>>,
        Vec<Dom<SC>>,
        Vec<Vec<Dom<SC>>>,
        Vec<SC::Challenge>,
        SC::Challenge,
        SC::Challenge,
    )
    where
        SC: StarkGenericConfig + Default,
        SC::Challenger: Clone,
        OpeningProof<SC>: Send + Sync,
        Com<SC>: Send + Sync,
        PcsProverData<SC>: Send + Sync,
        ShardMainData<SC>: Serialize + DeserializeOwned,
        SC::Val: PrimeField32,
    {
        let ShardProof {
            commitment,
            opened_values,
            ..
        } = proof;

        let ShardCommitment {
            permutation_commit,
            quotient_commit,
            ..
        } = commitment;

        // Extract verification metadata.
        let pcs = machine.config().pcs();

        let permutation_challenges = (0..2)
            .map(|_| challenger.sample_ext_element::<SC::Challenge>())
            .collect::<Vec<_>>();

        challenger.observe(permutation_commit.clone());

        let alpha = challenger.sample_ext_element::<SC::Challenge>();

        // Observe the quotient commitments.
        challenger.observe(quotient_commit.clone());

        let zeta = challenger.sample_ext_element::<SC::Challenge>();

        let chips = machine
            .shard_chips_ordered(&proof.chip_ordering)
            .collect::<Vec<_>>();

        let log_degrees = opened_values
            .chips
            .iter()
            .map(|val| val.log_degree)
            .collect::<Vec<_>>();

        let log_quotient_degrees = chips
            .iter()
            .map(|chip| chip.log_quotient_degree())
            .collect::<Vec<_>>();

        let trace_domains = log_degrees
            .iter()
            .map(|log_degree| pcs.natural_domain_for_degree(1 << log_degree))
            .collect::<Vec<_>>();

        let quotient_chunk_domains = trace_domains
            .iter()
            .zip_eq(log_degrees)
            .zip_eq(log_quotient_degrees)
            .map(|((domain, log_degree), log_quotient_degree)| {
                let quotient_degree = 1 << log_quotient_degree;
                let quotient_domain =
                    domain.create_disjoint_domain(1 << (log_degree + log_quotient_degree));
                quotient_domain.split_domains(quotient_degree)
            })
            .collect::<Vec<_>>();

        (
            chips,
            trace_domains,
            quotient_chunk_domains,
            permutation_challenges,
            alpha,
            zeta,
        )
    }

    #[test]
    fn test_verify_constraints_parts() {
        type SC = BabyBearPoseidon2;
        type F = <SC as StarkGenericConfig>::Val;
        type EF = <SC as StarkGenericConfig>::Challenge;
        type A = RiscvAir<F>;

        // Generate a dummy proof.
        wp1_core::utils::setup_logger();
        let elf =
            include_bytes!("../../../examples/fibonacci/program/elf/riscv32im-succinct-zkvm-elf");

        let machine = A::machine(SC::default());
        let (_, vk) = machine.setup(&Program::from(elf));
        let mut challenger = machine.config().challenger();
        let proof = SP1Prover::prove_with_config(elf, SP1Stdin::new(), machine.config().clone())
            .unwrap()
            .proof;
        println!("Proof generated successfully");

        challenger.observe(vk.commit);
        for proof in proof.shard_proofs.iter() {
            challenger.observe(proof.commitment.main_commit);
        }

        // Observe the public input digest
        let pv_digest_field_elms: Vec<F> =
            PublicValuesDigest::<Word<F>>::new(proof.public_values_digest).into();
        challenger.observe_slice(&pv_digest_field_elms);

        // Run the verify inside the DSL and compare it to the calculated value.
        let mut builder = VmBuilder::<F, EF>::default();

        for proof in proof.shard_proofs.into_iter().take(1) {
            let (
                chips,
                trace_domains_vals,
                quotient_chunk_domains_vals,
                permutation_challenges,
                alpha_val,
                zeta_val,
            ) = get_shard_data(&machine, &proof, &mut challenger);

            // Set up the public values digest.
            let mut public_values_digest = builder.dyn_array(PV_DIGEST_NUM_WORDS * WORD_SIZE);
            for i in 0..PV_DIGEST_NUM_WORDS {
                for j in 0..WORD_SIZE {
                    let word_val: Felt<_> = builder.eval(proof.public_values_digest[i][j]);
                    builder.set(&mut public_values_digest, i * WORD_SIZE + j, word_val);
                }
            }

            for (chip, trace_domain_val, qc_domains_vals, values_vals) in izip!(
                chips.iter(),
                trace_domains_vals,
                quotient_chunk_domains_vals,
                proof.opened_values.chips.iter(),
            ) {
                // Compute the expected folded constraints value.
                let sels_val = trace_domain_val.selectors_at_point(zeta_val);
                let folded_constraints_val = Verifier::<SC, _>::eval_constraints(
                    chip,
                    values_vals,
                    &sels_val,
                    alpha_val,
                    &permutation_challenges,
                    proof.public_values_digest,
                );

                // Compute the folded constraints value in the DSL.
                let values_var: ChipOpenedValuesVariable<_> =
                    builder.eval_const(values_vals.clone());
                let values = ChipOpening::from_variable(&mut builder, chip, &values_var);
                let alpha = builder.eval(alpha_val.cons());
                let zeta = builder.eval(zeta_val.cons());
                let trace_domain: TwoAdicMultiplicativeCosetVariable<_> =
                    builder.eval_const(trace_domain_val);
                let sels = trace_domain.selectors_at_point(&mut builder, zeta);

                let permutation_challenges = permutation_challenges
                    .iter()
                    .map(|c| builder.eval(c.cons()))
                    .collect::<Vec<_>>();
                let folded_constraints = StarkVerifier::<_, SC>::eval_constrains(
                    &mut builder,
                    chip,
                    &values,
                    &public_values_digest,
                    &sels,
                    alpha,
                    permutation_challenges.as_slice(),
                );

                // Assert that the two values are equal.
                builder.assert_ext_eq(folded_constraints, folded_constraints_val.cons());

                // Compute the expected quotient value.
                let quotient_val =
                    Verifier::<SC, A>::recompute_quotient(values_vals, &qc_domains_vals, zeta_val);

                let qc_domains = qc_domains_vals
                    .iter()
                    .map(|domain| {
                        builder.eval_const::<TwoAdicMultiplicativeCosetVariable<_>>(*domain)
                    })
                    .collect::<Vec<_>>();
                let quotient = StarkVerifier::<_, SC>::recompute_quotient(
                    &mut builder,
                    &values,
                    &qc_domains,
                    zeta,
                );

                // Assert that the two values are equal.
                builder.assert_ext_eq(quotient, quotient_val.cons());

                // Assert that the constraint-quotient relation holds.
                builder.assert_ext_eq(folded_constraints * sels.inv_zeroifier, quotient);
            }
        }

        let program = builder.compile();

        let mut runtime = Runtime::<F, EF, _>::new(&program, machine.config().perm.clone());
        runtime.run();
        println!(
            "The program executed successfully, number of cycles: {}",
            runtime.clk.as_canonical_u32() / 4
        );
    }

    #[test]
    fn test_verify_constraints_whole() {
        type SC = BabyBearPoseidon2;
        type F = <SC as StarkGenericConfig>::Val;
        type EF = <SC as StarkGenericConfig>::Challenge;
        type A = RiscvAir<F>;

        // Generate a dummy proof.
        wp1_core::utils::setup_logger();
        let elf =
            include_bytes!("../../../examples/fibonacci/program/elf/riscv32im-succinct-zkvm-elf");

        let machine = A::machine(SC::default());
        let (_, vk) = machine.setup(&Program::from(elf));
        let mut challenger = machine.config().challenger();
        let proof =
            SP1Prover::prove_with_config(elf, SP1Stdin::new(), machine.config().clone()).unwrap();
        SP1Verifier::verify_with_config(elf, &proof, machine.config().clone()).unwrap();

        let proof = proof.proof;
        println!("Proof generated and verified successfully");

        challenger.observe(vk.commit);

        for proof in proof.shard_proofs.iter() {
            challenger.observe(proof.commitment.main_commit);
        }

        // Observe the public input digest
        let pv_digest_field_elms: Vec<F> =
            PublicValuesDigest::<Word<F>>::new(proof.public_values_digest).into();
        challenger.observe_slice(&pv_digest_field_elms);

        // Run the verify inside the DSL and compare it to the calculated value.
        let mut builder = VmBuilder::<F, EF>::default();

        for proof in proof.shard_proofs.into_iter().take(1) {
            let (
                chips,
                trace_domains_vals,
                quotient_chunk_domains_vals,
                permutation_challenges,
                alpha_val,
                zeta_val,
            ) = get_shard_data(&machine, &proof, &mut challenger);

            for (chip, trace_domain_val, qc_domains_vals, values_vals) in izip!(
                chips.iter(),
                trace_domains_vals,
                quotient_chunk_domains_vals,
                proof.opened_values.chips.iter(),
            ) {
                let opening = builder.eval_const(values_vals.clone());
                let alpha = builder.eval(alpha_val.cons());
                let zeta = builder.eval(zeta_val.cons());
                let trace_domain = builder.eval_const(trace_domain_val);
                let qc_domains = qc_domains_vals
                    .iter()
                    .map(|domain| builder.eval_const(*domain))
                    .collect::<Vec<_>>();

                // Set up the public values digest.
                let mut public_values_digest = builder.dyn_array(PV_DIGEST_NUM_WORDS * WORD_SIZE);
                for i in 0..PV_DIGEST_NUM_WORDS {
                    for j in 0..WORD_SIZE {
                        let word_val: Felt<_> = builder.eval(proof.public_values_digest[i][j]);
                        builder.set(&mut public_values_digest, i * WORD_SIZE + j, word_val);
                    }
                }
                let permutation_challenges = permutation_challenges
                    .iter()
                    .map(|c| builder.eval(c.cons()))
                    .collect::<Vec<_>>();

                StarkVerifier::<_, SC>::verify_constraints::<A>(
                    &mut builder,
                    chip,
                    &opening,
                    &public_values_digest,
                    &trace_domain,
                    &qc_domains,
                    zeta,
                    alpha,
                    &permutation_challenges,
                )
            }
        }

        let program = builder.compile();

        let mut runtime = Runtime::<F, EF, _>::new(&program, machine.config().perm.clone());
        runtime.run();
        println!(
            "The program executed successfully, number of cycles: {}",
            runtime.clk.as_canonical_u32() / 4
        );
    }
}
