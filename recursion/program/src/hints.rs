use p3_field::{AbstractExtensionField, AbstractField};
use wp1_core::{
    air::{PublicValues, Word},
    stark::{AirOpenedValues, ChipOpenedValues, ShardCommitment, ShardOpenedValues, ShardProof},
};
use wp1_recursion_compiler::{
    ir::{Array, Builder, Config, Ext, Felt, MemVariable, Var},
    InnerConfig,
};
use wp1_recursion_core::{
    air::Block,
    stark::config::{InnerChallenge, InnerDigest, InnerDigestHash, InnerPcsProof, InnerVal},
};
use wp1_sdk::utils::BabyBearPoseidon2;

use crate::types::{
    AirOpenedValuesVariable, ChipOpenedValuesVariable, PublicValuesVariable,
    ShardCommitmentVariable, ShardOpenedValuesVariable, ShardProofVariable,
};

pub trait Hintable<C: Config> {
    type HintVariable: MemVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable;

    fn write(&self) -> Vec<Vec<Block<C::F>>>;
}

type C = InnerConfig;

impl Hintable<C> for usize {
    type HintVariable = Var<InnerVal>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_var()
    }

    fn write(&self) -> Vec<Vec<Block<InnerVal>>> {
        vec![vec![Block::from(InnerVal::from_canonical_usize(*self))]]
    }
}

impl Hintable<C> for InnerVal {
    type HintVariable = Felt<InnerVal>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_felt()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![vec![Block::from(*self)]]
    }
}

impl Hintable<C> for InnerChallenge {
    type HintVariable = Ext<InnerVal, InnerChallenge>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_ext()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![vec![Block::try_from((*self).as_base_slice()).unwrap()]]
    }
}

impl Hintable<C> for Vec<usize> {
    type HintVariable = Array<C, Var<InnerVal>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_vars()
    }

    fn write(&self) -> Vec<Vec<Block<InnerVal>>> {
        vec![self
            .iter()
            .map(|x| Block::from(InnerVal::from_canonical_usize(*x)))
            .collect()]
    }
}

impl Hintable<C> for Vec<InnerVal> {
    type HintVariable = Array<C, Felt<InnerVal>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_felts()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![self.iter().map(|x| Block::from(*x)).collect()]
    }
}

impl Hintable<C> for Vec<InnerChallenge> {
    type HintVariable = Array<C, Ext<InnerVal, InnerChallenge>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        builder.hint_exts()
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        vec![self
            .iter()
            .map(|x| Block::try_from((*x).as_base_slice()).unwrap())
            .collect()]
    }
}

impl Hintable<C> for AirOpenedValues<InnerChallenge> {
    type HintVariable = AirOpenedValuesVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let local = Vec::<InnerChallenge>::read(builder);
        let next = Vec::<InnerChallenge>::read(builder);
        AirOpenedValuesVariable { local, next }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.local.write());
        stream.extend(self.next.write());
        stream
    }
}

impl Hintable<C> for Vec<Vec<InnerChallenge>> {
    type HintVariable = Array<C, Array<C, Ext<InnerVal, InnerChallenge>>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let len = builder.hint_var();
        let mut arr = builder.dyn_array(len);
        builder.range(0, len).for_each(|i, builder| {
            let hint = Vec::<InnerChallenge>::read(builder);
            builder.set(&mut arr, i, hint);
        });
        arr
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let len = InnerVal::from_canonical_usize(self.len());
        stream.push(vec![len.into()]);

        for arr in self.iter() {
            let comm = Vec::<InnerChallenge>::write(arr);
            stream.extend(comm);
        }

        stream
    }
}

impl Hintable<C> for ChipOpenedValues<InnerChallenge> {
    type HintVariable = ChipOpenedValuesVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let preprocessed = AirOpenedValues::<InnerChallenge>::read(builder);
        let main = AirOpenedValues::<InnerChallenge>::read(builder);
        let permutation = AirOpenedValues::<InnerChallenge>::read(builder);
        let quotient = Vec::<Vec<InnerChallenge>>::read(builder);
        let cumulative_sum = InnerChallenge::read(builder);
        let log_degree = builder.hint_var();
        ChipOpenedValuesVariable {
            preprocessed,
            main,
            permutation,
            quotient,
            cumulative_sum,
            log_degree,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.preprocessed.write());
        stream.extend(self.main.write());
        stream.extend(self.permutation.write());
        stream.extend(self.quotient.write());
        stream.extend(self.cumulative_sum.write());
        stream.extend(self.log_degree.write());
        stream
    }
}

impl Hintable<C> for Vec<ChipOpenedValues<InnerChallenge>> {
    type HintVariable = Array<C, ChipOpenedValuesVariable<C>>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let len = builder.hint_var();
        let mut arr = builder.dyn_array(len);
        builder.range(0, len).for_each(|i, builder| {
            let hint = ChipOpenedValues::<InnerChallenge>::read(builder);
            builder.set(&mut arr, i, hint);
        });
        arr
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();

        let len = InnerVal::from_canonical_usize(self.len());
        stream.push(vec![len.into()]);

        for arr in self.iter() {
            let comm = ChipOpenedValues::<InnerChallenge>::write(arr);
            stream.extend(comm);
        }

        stream
    }
}

impl Hintable<C> for ShardOpenedValues<InnerChallenge> {
    type HintVariable = ShardOpenedValuesVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let chips = Vec::<ChipOpenedValues<InnerChallenge>>::read(builder);
        ShardOpenedValuesVariable { chips }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.chips.write());
        stream
    }
}

impl Hintable<C> for ShardCommitment<InnerDigestHash> {
    type HintVariable = ShardCommitmentVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let main_commit = InnerDigest::read(builder);
        let permutation_commit = InnerDigest::read(builder);
        let quotient_commit = InnerDigest::read(builder);
        ShardCommitmentVariable {
            main_commit,
            permutation_commit,
            quotient_commit,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        let h: InnerDigest = self.main_commit.into();
        stream.extend(h.write());
        let h: InnerDigest = self.permutation_commit.into();
        stream.extend(h.write());
        let h: InnerDigest = self.quotient_commit.into();
        stream.extend(h.write());
        stream
    }
}

impl Hintable<C> for PublicValues<u32, u32> {
    type HintVariable = PublicValuesVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let committed_values_digest = Vec::<InnerVal>::read(builder);
        let shard = builder.hint_felt();
        let start_pc = builder.hint_felt();
        let next_pc = builder.hint_felt();
        let exit_code = builder.hint_felt();
        PublicValuesVariable {
            committed_values_digest,
            shard,
            start_pc,
            next_pc,
            exit_code,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        type F = <C as Config>::F;

        let mut stream = Vec::new();
        stream.extend(
            self.committed_value_digest
                .into_iter()
                .flat_map(|x| Word::from(x).0)
                .collect::<Vec<F>>()
                .write(),
        );
        stream.extend(F::from_canonical_u32(self.shard).write());
        stream.extend(F::from_canonical_u32(self.start_pc).write());
        stream.extend(F::from_canonical_u32(self.next_pc).write());
        stream.extend(F::from_canonical_u32(self.exit_code).write());
        stream
    }
}

impl Hintable<C> for ShardProof<BabyBearPoseidon2> {
    type HintVariable = ShardProofVariable<C>;

    fn read(builder: &mut Builder<C>) -> Self::HintVariable {
        let index = builder.hint_var();
        let commitment = ShardCommitment::read(builder);
        let opened_values = ShardOpenedValues::read(builder);
        let opening_proof = InnerPcsProof::read(builder);
        let public_values = PublicValues::read(builder);
        ShardProofVariable {
            index,
            commitment,
            opened_values,
            opening_proof,
            public_values,
        }
    }

    fn write(&self) -> Vec<Vec<Block<<C as Config>::F>>> {
        let mut stream = Vec::new();
        stream.extend(self.index.write());
        stream.extend(self.commitment.write());
        stream.extend(self.opened_values.write());
        stream.extend(self.opening_proof.write());
        stream.extend(self.public_values.write());

        stream
    }
}
