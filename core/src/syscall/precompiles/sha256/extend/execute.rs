use crate::{
    runtime::{Register, Syscall},
    syscall::precompiles::{sha256::ShaExtendEvent, SyscallContext},
};

use super::ShaExtendChip;

impl Syscall for ShaExtendChip {
    fn num_extra_cycles(&self) -> u32 {
        48 * 20
    }

    fn execute(&self, rt: &mut SyscallContext<'_>) -> u32 {
        // Initialize the registers.
        let a0 = Register::X10;

        // Read `w_ptr` from register a0 or x5.
        // TODO: this is underconstrained.
        let w_ptr = rt.register_unsafe(a0);

        let clk_init = rt.clk;
        let w_ptr_init = w_ptr;
        let mut w_i_minus_15_reads = Vec::new();
        let mut w_i_minus_2_reads = Vec::new();
        let mut w_i_minus_16_reads = Vec::new();
        let mut w_i_minus_7_reads = Vec::new();
        let mut w_i_writes = Vec::new();
        for i in 16..64 {
            // Read w[i-15].
            let (record, w_i_minus_15) = rt.mr(w_ptr + (i - 15) * 4);
            w_i_minus_15_reads.push(record);
            rt.clk += 4;

            // Compute `s0`.
            let s0 =
                w_i_minus_15.rotate_right(7) ^ w_i_minus_15.rotate_right(18) ^ (w_i_minus_15 >> 3);

            // Read w[i-2].
            let (record, w_i_minus_2) = rt.mr(w_ptr + (i - 2) * 4);
            w_i_minus_2_reads.push(record);
            rt.clk += 4;

            // Compute `s1`.
            let s1 =
                w_i_minus_2.rotate_right(17) ^ w_i_minus_2.rotate_right(19) ^ (w_i_minus_2 >> 10);

            // Read w[i-16].
            let (record, w_i_minus_16) = rt.mr(w_ptr + (i - 16) * 4);
            w_i_minus_16_reads.push(record);
            rt.clk += 4;

            // Read w[i-7].
            let (record, w_i_minus_7) = rt.mr(w_ptr + (i - 7) * 4);
            w_i_minus_7_reads.push(record);
            rt.clk += 4;

            // Compute `w_i`.
            let w_i = s1
                .wrapping_add(w_i_minus_16)
                .wrapping_add(s0)
                .wrapping_add(w_i_minus_7);

            // Write w[i].
            w_i_writes.push(rt.mw(w_ptr + i * 4, w_i));
            rt.clk += 4;
        }

        // Push the SHA extend event.
        let shard = rt.current_shard();
        rt.record_mut().sha_extend_events.push(ShaExtendEvent {
            shard,
            clk: clk_init,
            w_ptr: w_ptr_init,
            w_i_minus_15_reads,
            w_i_minus_2_reads,
            w_i_minus_16_reads,
            w_i_minus_7_reads,
            w_i_writes,
        });

        w_ptr
    }
}
