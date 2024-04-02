#![no_main]
wp1_zkvm::entrypoint!(main);

use hybrid_array::typenum::U24;
use wp1_zkvm::precompiles::bls12381::Bls12381;
use wp1_zkvm::precompiles::utils::AffinePoint;

#[wp1_derive::cycle_tracker]
pub fn main() {
    // generator.
    // 3685416753713387016781088315183077757961620795782546409894578378688607592378376318836054947676345821548104185464507
    // 1339506544944476473020471379941921221584933875938349620426543736416511423956333506472724655353366534992391756441569
    let a: [u8; 96] = [
        187, 198, 34, 219, 10, 240, 58, 251, 239, 26, 122, 249, 63, 232, 85, 108, 88, 172, 27, 23,
        63, 58, 78, 161, 5, 185, 116, 151, 79, 140, 104, 195, 15, 172, 169, 79, 140, 99, 149, 38,
        148, 215, 151, 49, 167, 211, 241, 23, 225, 231, 197, 70, 41, 35, 170, 12, 228, 138, 136,
        162, 68, 199, 60, 208, 237, 179, 4, 44, 203, 24, 219, 0, 246, 10, 208, 213, 149, 224, 245,
        252, 228, 138, 29, 116, 237, 48, 158, 160, 241, 160, 170, 227, 129, 244, 179, 8,
    ];

    let mut a_point = AffinePoint::<Bls12381, U24>::from_le_bytes(a);

    // scalar.
    // 3
    let scalar: [u32; 12] = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    println!("cycle-tracker-start: bn254_mul");
    a_point.mul_assign(&scalar);
    println!("cycle-tracker-end: bn254_mul");

    // 3 * generator.
    // 1527649530533633684281386512094328299672026648504329745640827351945739272160755686119065091946435084697047221031460
    // 487897572011753812113448064805964756454529228648704488481988876974355015977479905373670519228592356747638779818193
    let c: [u8; 96] = [
        36, 82, 78, 2, 201, 192, 210, 150, 155, 23, 162, 44, 11, 122, 116, 129, 249, 63, 91, 51,
        81, 10, 120, 243, 241, 165, 233, 155, 31, 214, 18, 177, 151, 150, 169, 236, 45, 33, 101,
        23, 19, 240, 209, 249, 8, 227, 236, 9, 209, 48, 174, 144, 5, 59, 71, 163, 92, 244, 74, 99,
        108, 37, 69, 231, 230, 59, 212, 15, 49, 39, 156, 157, 127, 9, 195, 171, 221, 12, 154, 166,
        12, 248, 197, 137, 51, 98, 132, 138, 159, 176, 245, 166, 211, 128, 43, 3,
    ];

    assert_eq!(a_point.to_le_bytes(), c);

    println!("done");
}