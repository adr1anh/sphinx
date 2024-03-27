use crate::{commit::PolynomialSpaceVariable, folder::RecursiveVerifierConstraintFolder};
use itertools::{izip, Itertools};
use p3_air::Air;
use p3_field::TwoAdicField;
use wp1_core::stark::{MachineChip, ShardCommitment, StarkGenericConfig};
use wp1_recursion_compiler::ir::ExtConst;
use wp1_recursion_compiler::{
    ir::{Builder, Config, Usize},
    verifier::challenger::DuplexChallengerVariable,
};

use crate::{commit::PcsVariable, fri::TwoAdicFriPcsVariable, types::ShardProofVariable};

#[derive(Debug, Clone, Copy)]
pub struct StarkVerifier<C: Config, SC: StarkGenericConfig> {
    _phantom: std::marker::PhantomData<(C, SC)>,
}

impl<C: Config, SC: StarkGenericConfig> StarkVerifier<C, SC>
where
    SC: StarkGenericConfig<Val = C::F, Challenge = C::EF>,
{
    pub fn verify_shard<A>(
        builder: &mut Builder<C>,
        pcs: &TwoAdicFriPcsVariable<C>,
        chips: &[&MachineChip<SC, A>],
        challenger: &mut DuplexChallengerVariable<C>,
        proof: &ShardProofVariable<C>,
        permutation_challenges: &[C::EF],
    ) where
        A: for<'b> Air<RecursiveVerifierConstraintFolder<'b, C>>,
        C::F: TwoAdicField,
        C::EF: TwoAdicField,
    {
        let ShardProofVariable {
            commitment,
            opened_values,
            ..
        } = proof;

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
            .map(|log_degree| pcs.natural_domain_for_log_degree(builder, *log_degree))
            .collect::<Vec<_>>();

        let ShardCommitment {
            main_commit: _,
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

        challenger.observe_commitment(builder, permutation_commit.clone());

        let alpha = challenger.sample_ext(builder);

        challenger.observe_commitment(builder, quotient_commit.clone());

        let zeta = challenger.sample_ext(builder);

        let quotient_chunk_domains = trace_domains
            .iter()
            .zip_eq(log_degrees)
            .zip_eq(log_quotient_degrees)
            .map(|((domain, log_degree), log_quotient_degree)| {
                let log_quotient_size: Usize<_> = builder.eval(log_degree + log_quotient_degree);
                let quotient_domain = domain.create_disjoint_domain(builder, log_quotient_size);
                quotient_domain.split_domains(builder, log_quotient_degree)
            })
            .collect::<Vec<_>>();

        for (chip, trace_domain, qc_domains, values) in izip!(
            chips.iter(),
            trace_domains,
            quotient_chunk_domains,
            opened_values.chips.iter(),
        ) {
            Self::verify_constraints(
                builder,
                chip,
                values,
                trace_domain,
                qc_domains,
                zeta,
                alpha,
                permutation_challenges,
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use p3_challenger::{CanObserve, FieldChallenger};
    use wp1_core::{
        air::MachineAir,
        stark::{RiscvAir, ShardCommitment, ShardProof, StarkGenericConfig},
        utils::BabyBearPoseidon2,
        SP1Prover, SP1Stdin,
    };
    use wp1_recursion_compiler::{
        asm::{AsmConfig, VmBuilder},
        ir::{Builder, Config, ExtConst, Usize},
        verifier::{
            challenger::DuplexChallengerVariable,
            fri::types::{Commitment, DIGEST_SIZE},
        },
    };
    use wp1_recursion_core::runtime::Runtime;

    use crate::{
        fri::{
            const_fri_config, const_two_adic_pcs_proof, default_fri_config, TwoAdicFriPcsVariable,
        },
        stark::StarkVerifier,
        types::{ChipOpening, ShardOpenedValuesVariable, ShardProofVariable},
    };

    type SC = BabyBearPoseidon2;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;
    type C = AsmConfig<F, EF>;
    type A = RiscvAir<F>;

    pub(crate) fn const_proof<C>(
        builder: &mut Builder<C>,
        proof: ShardProof<SC>,
    ) -> ShardProofVariable<C>
    where
        C: Config<F = F, EF = EF>,
    {
        let index = builder.materialize(Usize::Const(proof.index));

        // Set up the commitments.
        let mut main_commit: Commitment<_> = builder.dyn_array(DIGEST_SIZE);
        let mut permutation_commit: Commitment<_> = builder.dyn_array(DIGEST_SIZE);
        let mut quotient_commit: Commitment<_> = builder.dyn_array(DIGEST_SIZE);

        let main_commit_val: [_; DIGEST_SIZE] = proof.commitment.main_commit.into();
        let perm_commit_val: [_; DIGEST_SIZE] = proof.commitment.permutation_commit.into();
        let quotient_commit_val: [_; DIGEST_SIZE] = proof.commitment.quotient_commit.into();
        for (i, ((main_val, perm_val), quotient_val)) in main_commit_val
            .into_iter()
            .zip(perm_commit_val)
            .zip(quotient_commit_val)
            .enumerate()
        {
            builder.set(&mut main_commit, i, main_val);
            builder.set(&mut permutation_commit, i, perm_val);
            builder.set(&mut quotient_commit, i, quotient_val);
        }

        let commitment = ShardCommitment {
            main_commit,
            permutation_commit,
            quotient_commit,
        };

        // Set up the opened values.
        let opened_values = ShardOpenedValuesVariable {
            chips: proof
                .opened_values
                .chips
                .iter()
                .map(|values| ChipOpening::from_constant(builder, values))
                .collect(),
        };

        let opening_proof = const_two_adic_pcs_proof(builder, proof.opening_proof);

        ShardProofVariable {
            index: Usize::Var(index),
            commitment,
            opened_values,
            opening_proof,
        }
    }

    #[test]
    fn test_permutation_challenges() {
        // Generate a dummy proof.
        wp1_core::utils::setup_logger();
        let elf =
            include_bytes!("../../../examples/fibonacci/program/elf/riscv32im-succinct-zkvm-elf");

        let machine = A::machine(SC::default());
        let mut challenger_val = machine.config().challenger();
        let proofs = SP1Prover::prove_with_config(elf, SP1Stdin::new(), machine.config().clone())
            .unwrap()
            .proof
            .shard_proofs;
        println!("Proof generated successfully");

        proofs.iter().for_each(|proof| {
            challenger_val.observe(proof.commitment.main_commit);
        });

        let permutation_challenges = (0..2)
            .map(|_| challenger_val.sample_ext_element::<EF>())
            .collect::<Vec<_>>();

        // Observe all the commitments.
        let mut builder = VmBuilder::<F, EF>::default();

        let mut challenger = DuplexChallengerVariable::new(&mut builder);

        for proof in proofs {
            let proof = const_proof(&mut builder, proof);
            let ShardCommitment { main_commit, .. } = proof.commitment;
            challenger.observe_commitment(&mut builder, main_commit);
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
        runtime.run();
        println!(
            "The program executed successfully, number of cycles: {}",
            runtime.timestamp
        );
    }

    #[test]
    fn test_verify_shard() {
        // Generate a dummy proof.
        wp1_core::utils::setup_logger();
        let elf =
            include_bytes!("../../../examples/fibonacci/program/elf/riscv32im-succinct-zkvm-elf");

        let machine = A::machine(SC::default());
        let mut challenger_val = machine.config().challenger();
        let proofs = SP1Prover::prove_with_config(elf, SP1Stdin::new(), machine.config().clone())
            .unwrap()
            .proof
            .shard_proofs;
        println!("Proof generated successfully");

        proofs.iter().for_each(|proof| {
            challenger_val.observe(proof.commitment.main_commit);
        });

        let permutation_challenges = (0..2)
            .map(|_| challenger_val.sample_ext_element::<EF>())
            .collect::<Vec<_>>();

        // Observe all the commitments.
        let mut builder = VmBuilder::<F, EF>::default();
        let config = const_fri_config(&mut builder, default_fri_config());
        let pcs = TwoAdicFriPcsVariable { config };

        let mut challenger = DuplexChallengerVariable::new(&mut builder);

        let mut shard_proofs = vec![];
        let mut shard_chips = vec![];
        for proof_val in proofs {
            let chips = machine
                .chips()
                .iter()
                .filter(|chip| proof_val.chip_ids.contains(&chip.name()))
                .collect::<Vec<_>>();
            let proof = const_proof(&mut builder, proof_val);
            let ShardCommitment { main_commit, .. } = &proof.commitment;
            challenger.observe_commitment(&mut builder, main_commit.clone());
            shard_proofs.push(proof);
            shard_chips.push(chips);
        }

        for (proof, chip) in shard_proofs.into_iter().zip(shard_chips) {
            StarkVerifier::<C, SC>::verify_shard(
                &mut builder,
                &pcs,
                &chip,
                &mut challenger.clone(),
                &proof,
                &permutation_challenges,
            );
        }

        let program = builder.compile();

        let mut runtime = Runtime::<F, EF, _>::new(&program, machine.config().perm.clone());
        runtime.run();
        println!(
            "The program executed successfully, number of cycles: {}",
            runtime.timestamp
        );
    }
}
