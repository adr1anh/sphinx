use crate::runtime::Register;
use crate::runtime::Syscall;
use crate::runtime::{MemoryReadRecord, MemoryWriteRecord};
use crate::syscall::precompiles::blake3::{
    g_func, Blake3CompressInnerChip, Blake3CompressInnerEvent, G_INDEX, MSG_SCHEDULE,
    NUM_MSG_WORDS_PER_CALL, NUM_STATE_WORDS_PER_CALL, OPERATION_COUNT, ROUND_COUNT,
};
use crate::syscall::precompiles::SyscallContext;

impl Syscall for Blake3CompressInnerChip {
    fn num_extra_cycles(&self) -> u32 {
        (4 * ROUND_COUNT * OPERATION_COUNT) as u32
    }

    fn execute(&self, rt: &mut SyscallContext<'_>) -> u32 {
        // TODO: These pointers have to be constrained.
        let state_ptr = rt.register_unsafe(Register::X10);
        let message_ptr = rt.register_unsafe(Register::X11);

        let saved_clk = rt.clk;
        let mut message_reads =
            [[[MemoryReadRecord::default(); NUM_MSG_WORDS_PER_CALL]; OPERATION_COUNT]; ROUND_COUNT];
        let mut state_writes = [[[MemoryWriteRecord::default(); NUM_STATE_WORDS_PER_CALL];
            OPERATION_COUNT]; ROUND_COUNT];

        for round in 0..ROUND_COUNT {
            for operation in 0..OPERATION_COUNT {
                let state_index = G_INDEX[operation];
                let message_index: [usize; NUM_MSG_WORDS_PER_CALL] = [
                    MSG_SCHEDULE[round][2 * operation],
                    MSG_SCHEDULE[round][2 * operation + 1],
                ];

                let mut input = vec![];
                // Read the input to g.
                {
                    for index in state_index.iter() {
                        input.push(rt.word_unsafe(state_ptr + (*index as u32) * 4));
                    }
                    for i in 0..NUM_MSG_WORDS_PER_CALL {
                        let (record, value) = rt.mr(message_ptr + (message_index[i] as u32) * 4);
                        message_reads[round][operation][i] = record;
                        input.push(value);
                    }
                }

                // Call g.
                let results = g_func(input.try_into().unwrap());

                // Write the state.
                for i in 0..NUM_STATE_WORDS_PER_CALL {
                    state_writes[round][operation][i] =
                        rt.mw(state_ptr + (state_index[i] as u32) * 4, results[i]);
                }

                // Increment the clock for the next call of g.
                rt.clk += 4;
            }
        }

        let shard = rt.current_shard();

        rt.record_mut()
            .blake3_compress_inner_events
            .push(Blake3CompressInnerEvent {
                shard,
                clk: saved_clk,
                state_ptr,
                message_reads,
                state_writes,
                message_ptr,
            });

        state_ptr
    }
}
