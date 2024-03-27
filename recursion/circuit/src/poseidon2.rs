//! An implementation of Poseidon2 over BN254.

use wp1_recursion_compiler::ir::{Builder, Config, DslIR, Var};

pub trait P2CircuitBuilder<C: Config> {
    fn p2_permute_mut(&mut self, state: [Var<C::N>; 3]);
}

impl<C: Config> P2CircuitBuilder<C> for Builder<C> {
    fn p2_permute_mut(&mut self, state: [Var<C::N>; 3]) {
        self.push(DslIR::Poseidon2PermuteBn254(state))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Write;
    use std::marker::PhantomData;

    use ff::PrimeField as FFPrimeField;
    use p3_baby_bear::BabyBear;
    use p3_bn254_fr::FFBn254Fr;
    use p3_bn254_fr::{Bn254Fr, DiffusionMatrixBN254};
    use p3_field::{extension::BinomialExtensionField, AbstractField};
    use p3_poseidon2::Poseidon2;
    use p3_symmetric::Permutation;
    use wp1_recursion_compiler::gnark::GnarkBackend;
    use wp1_recursion_compiler::ir::{Builder, Config, Var};
    use zkhash::ark_ff::BigInteger;
    use zkhash::ark_ff::PrimeField;
    use zkhash::fields::bn256::FpBN256 as ark_FpBN256;
    use zkhash::poseidon2::poseidon2_instance_bn256::RC3;

    use crate::poseidon2::P2CircuitBuilder;

    #[derive(Clone)]
    struct GnarkConfig;

    impl Config for GnarkConfig {
        type N = Bn254Fr;
        type F = BabyBear;
        type EF = BinomialExtensionField<BabyBear, 4>;
    }

    fn bn254_from_ark_ff(input: ark_FpBN256) -> Bn254Fr {
        let bytes = input.into_bigint().to_bytes_le();

        let mut res = <FFBn254Fr as ff::PrimeField>::Repr::default();

        for (i, digit) in res.0.iter_mut().enumerate() {
            *digit = bytes[i];
        }

        let value = FFBn254Fr::from_repr(res);

        if value.is_some().into() {
            Bn254Fr {
                value: value.unwrap(),
            }
        } else {
            panic!("Invalid field element")
        }
    }

    #[test]
    fn test_p2_permute_mut() {
        type F = Bn254Fr;
        const WIDTH: usize = 3;
        const D: u64 = 5;
        const ROUNDS_F: usize = 8;
        const ROUNDS_P: usize = 56;

        let round_constants: Vec<[F; WIDTH]> = RC3
            .iter()
            .map(|vec| {
                vec.iter()
                    .cloned()
                    .map(bn254_from_ark_ff)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect();

        let poseidon2: Poseidon2<Bn254Fr, DiffusionMatrixBN254, WIDTH, D> =
            Poseidon2::new(ROUNDS_F, ROUNDS_P, round_constants, DiffusionMatrixBN254);

        let input: [Bn254Fr; 3] = [
            Bn254Fr::from_canonical_u32(0),
            Bn254Fr::from_canonical_u32(1),
            Bn254Fr::from_canonical_u32(2),
        ];
        let mut output = input;
        poseidon2.permute_mut(&mut output);

        let mut builder = Builder::<GnarkConfig>::default();
        let a: Var<_> = builder.eval(input[0]);
        let b: Var<_> = builder.eval(input[1]);
        let c: Var<_> = builder.eval(input[2]);
        builder.p2_permute_mut([a, b, c]);

        builder.assert_var_eq(a, output[0]);
        builder.assert_var_eq(b, output[1]);
        builder.assert_var_eq(c, output[2]);

        let mut backend = GnarkBackend::<GnarkConfig> {
            nb_backend_vars: 0,
            used: HashMap::new(),
            phantom: PhantomData,
        };
        let result = backend.compile(builder.operations);
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}/build/verifier.go", manifest_dir);
        let mut file = File::create(path).unwrap();
        file.write_all(result.as_bytes()).unwrap();
    }
}
