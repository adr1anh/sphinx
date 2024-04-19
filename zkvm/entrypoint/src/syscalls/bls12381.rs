#[cfg(target_os = "zkvm")]
use core::arch::asm;

/// Adds two Bls12381 points.
///
/// The result is stored in the first point.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_add(p: *mut u32, q: *const u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_ADD,
            in("a0") p,
            in("a1") q,
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}

/// Double a Bls12381 point.
///
/// The result is stored in the first point.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_double(p: *mut u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_DOUBLE,
            in("a0") p,
            in("a1") 0,
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
/// Adds two BLS12381 Fp field elements
///
/// The result is stored by overwriting the first argument.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_fp_add(p: *mut u32, q: *const u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_FP_ADD,
            in("a0") p,
            in("a1") q
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_fp_sub(p: *mut u32, q: *const u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_FP_SUB,
            in("a0") p,
            in("a1") q
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_fp_mul(p: *mut u32, q: *const u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_FP_MUL,
            in("a0") p,
            in("a1") q
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_fp2_add(p: *mut u32, q: *const u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_FP2_ADD,
            in("a0") p,
            in("a1") q
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_fp2_sub(p: *mut u32, q: *const u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_FP2_SUB,
            in("a0") p,
            in("a1") q
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_fp2_mul(p: *mut u32, q: *const u32) {
    #[cfg(target_os = "zkvm")]
    unsafe {
        asm!(
            "ecall",
            in("t0") crate::syscalls::BLS12381_FP2_MUL,
            in("a0") p,
            in("a1") q
        );
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}

/// Decompresses a compressed BLS12-381 point.
///
/// The first half of the input array should contain the X coordinate.
/// The second half of the input array will be overwritten with the Y coordinate.
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn syscall_bls12381_decompress(point: &mut [u8; 96], is_odd: bool) {
    #[cfg(target_os = "zkvm")]
    {
        // Handle infinity point case specifically for bls12-381 crate.
        // It is expected that service bits are already masked by the caller,
        // so infinite point is just 96 zeroes

        let (prefix, aligned, suffix) = unsafe { point.align_to::<u128>() };
        let only_zeroes = prefix.iter().all(|&x| x == 0)
            && suffix.iter().all(|&x| x == 0)
            && aligned.iter().all(|&x| x == 0);

        if only_zeroes {
            // our point is infinite point, so skipping the precompile invocation and return expected value
            // of uncompressed infinite point, which is array of zeroes with zero element set to 64.
            point[0] = 64;
        } else {
            // Memory system/FpOps are little endian so we'll just flip the whole array before/after
            point.reverse();
            let p = point.as_mut_ptr();
            unsafe {
                asm!(
                "ecall",
                in("t0") crate::syscalls::BLS12381_DECOMPRESS,
                in("a0") p,
                in("a1") is_odd as u8,
                );
            }
            point.reverse();
        }
    }

    #[cfg(not(target_os = "zkvm"))]
    unreachable!()
}
