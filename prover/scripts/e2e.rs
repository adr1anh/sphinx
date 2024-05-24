use std::borrow::Borrow;
use std::path::PathBuf;

use clap::Parser;
use p3_baby_bear::BabyBear;
use p3_field::PrimeField;
use wp1_core::io::SP1Stdin;
use wp1_prover::utils::{babybear_bytes_to_bn254, babybears_to_bn254, words_to_bytes};
use wp1_prover::SP1Prover;
use wp1_recursion_circuit::stark::build_wrap_circuit;
use wp1_recursion_circuit::witness::Witnessable;
use wp1_recursion_compiler::ir::Witness;
use wp1_recursion_core::air::RecursionPublicValues;
use wp1_recursion_gnark_ffi::Groth16Prover;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    build_dir: String,
}

pub fn main() {
    wp1_core::utils::setup_logger();
    std::env::set_var("RECONSTRUCT_COMMITMENTS", "false");

    let args = Args::parse();
    let build_dir: PathBuf = args.build_dir.into();

    let elf = include_bytes!("../../tests/fibonacci/elf/riscv32im-succinct-zkvm-elf");

    tracing::info!("initializing prover");
    let prover = SP1Prover::new();

    tracing::info!("setup elf");
    let (pk, vk) = prover.setup(elf);

    tracing::info!("prove core");
    let stdin = SP1Stdin::new();
    let core_proof = prover.prove_core(&pk, &stdin).unwrap();

    tracing::info!("Compress");
    let reduced_proof = prover.compress(&vk, core_proof, vec![]).unwrap();

    tracing::info!("Shrink");
    let compressed_proof = prover.shrink(reduced_proof).unwrap();

    tracing::info!("wrap");
    let wrapped_proof = prover.wrap_bn254(compressed_proof).unwrap();

    tracing::info!("building verifier constraints");
    let constraints = tracing::info_span!("wrap circuit")
        .in_scope(|| build_wrap_circuit(&prover.wrap_vk, &wrapped_proof.proof));

    tracing::info!("building template witness");
    let pv: &RecursionPublicValues<_> = wrapped_proof.proof.public_values.as_slice().borrow();
    let vkey_hash = babybears_to_bn254(&pv.wp1_vk_digest);
    let committed_values_digest_bytes: [BabyBear; 32] = words_to_bytes(&pv.committed_value_digest)
        .try_into()
        .unwrap();
    let committed_values_digest = babybear_bytes_to_bn254(&committed_values_digest_bytes);

    let mut witness = Witness::default();
    wrapped_proof.proof.write(&mut witness);
    witness.write_commited_values_digest(committed_values_digest);
    witness.write_vkey_hash(vkey_hash);

    tracing::info!("sanity check gnark test");
    Groth16Prover::test(&constraints, witness.clone());

    tracing::info!("sanity check gnark build");
    Groth16Prover::build(&constraints, witness.clone(), &build_dir);

    tracing::info!("sanity check gnark prove");
    let groth16_prover = Groth16Prover::new();

    tracing::info!("gnark prove");
    let proof = groth16_prover.prove(witness.clone(), build_dir.clone());

    tracing::info!("verify gnark proof");
    groth16_prover.verify(
        &proof,
        &vkey_hash.as_canonical_biguint(),
        &committed_values_digest.as_canonical_biguint(),
        &build_dir,
    );

    println!(
        "{:?}",
        String::from_utf8(hex::encode(proof.encoded_proof.as_bytes()).into()).unwrap()
    );
}
