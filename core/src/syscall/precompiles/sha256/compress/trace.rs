use std::borrow::BorrowMut;

use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;

use crate::{
    air::{MachineAir, Word},
    memory::MemoryCols,
    runtime::ExecutionRecord,
};

use super::{
    columns::{ShaCompressCols, NUM_SHA_COMPRESS_COLS},
    ShaCompressChip, SHA_COMPRESS_K,
};

impl<F: PrimeField32> MachineAir<F> for ShaCompressChip {
    type Record = ExecutionRecord;

    fn name(&self) -> String {
        "ShaCompress".to_string()
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        let mut rows = Vec::new();

        let mut new_byte_lookup_events = Vec::new();
        for i in 0..input.sha_compress_events.len() {
            let mut event = input.sha_compress_events[i].clone();

            let og_h = event.h;
            let mut v = [0u32; 8].map(Word::from);

            let mut octet_num_idx = 0;

            // Load a, b, c, d, e, f, g, h.
            for j in 0..8usize {
                let mut row = [F::zero(); NUM_SHA_COMPRESS_COLS];
                let cols: &mut ShaCompressCols<F> = row.as_mut_slice().borrow_mut();

                cols.shard = F::from_canonical_u32(event.shard);
                cols.clk = F::from_canonical_u32(event.clk);
                cols.w_ptr = F::from_canonical_u32(event.w_ptr);
                cols.h_ptr = F::from_canonical_u32(event.h_ptr);

                cols.octet[j] = F::one();
                cols.octet_num[octet_num_idx] = F::one();

                cols.mem
                    .populate_read(event.h_read_records[j], &mut new_byte_lookup_events);
                cols.mem_addr = F::from_canonical_u32(event.h_ptr + (j * 4) as u32);

                cols.a = v[0];
                cols.b = v[1];
                cols.c = v[2];
                cols.d = v[3];
                cols.e = v[4];
                cols.f = v[5];
                cols.g = v[6];
                cols.h = v[7];

                match j {
                    0 => cols.a = *cols.mem.value(),
                    1 => cols.b = *cols.mem.value(),
                    2 => cols.c = *cols.mem.value(),
                    3 => cols.d = *cols.mem.value(),
                    4 => cols.e = *cols.mem.value(),
                    5 => cols.f = *cols.mem.value(),
                    6 => cols.g = *cols.mem.value(),
                    7 => cols.h = *cols.mem.value(),
                    _ => panic!("unsupported j"),
                };

                v[0] = cols.a;
                v[1] = cols.b;
                v[2] = cols.c;
                v[3] = cols.d;
                v[4] = cols.e;
                v[5] = cols.f;
                v[6] = cols.g;
                v[7] = cols.h;

                cols.is_real = F::one();
                cols.start = cols.is_real * cols.octet_num[0] * cols.octet[0];
                rows.push(row);
            }

            // Peforms the compress operation.
            for j in 0..64 {
                if j % 8 == 0 {
                    octet_num_idx += 1;
                }
                let mut row = [F::zero(); NUM_SHA_COMPRESS_COLS];
                let cols: &mut ShaCompressCols<F> = row.as_mut_slice().borrow_mut();

                cols.is_compression = F::one();
                cols.octet[j % 8] = F::one();
                cols.octet_num[octet_num_idx] = F::one();

                cols.shard = F::from_canonical_u32(event.shard);
                cols.clk = F::from_canonical_u32(event.clk);
                cols.w_ptr = F::from_canonical_u32(event.w_ptr);
                cols.h_ptr = F::from_canonical_u32(event.h_ptr);
                cols.mem
                    .populate_read(event.w_i_read_records[j], &mut new_byte_lookup_events);
                cols.mem_addr = F::from_canonical_u32(event.w_ptr + (j * 4) as u32);

                let a = event.h[0];
                let b = event.h[1];
                let c = event.h[2];
                let d = event.h[3];
                let e = event.h[4];
                let f = event.h[5];
                let g = event.h[6];
                let h = event.h[7];
                cols.a = Word::from(a);
                cols.b = Word::from(b);
                cols.c = Word::from(c);
                cols.d = Word::from(d);
                cols.e = Word::from(e);
                cols.f = Word::from(f);
                cols.g = Word::from(g);
                cols.h = Word::from(h);

                let e_rr_6 = cols.e_rr_6.populate(output, e, 6);
                let e_rr_11 = cols.e_rr_11.populate(output, e, 11);
                let e_rr_25 = cols.e_rr_25.populate(output, e, 25);
                let s1_intermeddiate = cols.s1_intermediate.populate(output, e_rr_6, e_rr_11);
                let s1 = cols.s1.populate(output, s1_intermeddiate, e_rr_25);

                let e_and_f = cols.e_and_f.populate(output, e, f);
                let e_not = cols.e_not.populate(output, e);
                let e_not_and_g = cols.e_not_and_g.populate(output, e_not, g);
                let ch = cols.ch.populate(output, e_and_f, e_not_and_g);

                // TODO: This is a hack to avoid calling Add5Operation::populate. We currently don't
                // call Add5Operation::eval due to the complexity of getting the inputs at the right
                // index, which is definitely feasible but we haven't gotten to yet. If we call
                // Add5Operation::populate, it creates interactions, and without the matching eval
                // call, it will cause a panic. We plan to fix this really soon.
                //
                // This is how we should call populate here.
                //
                // let temp1 = cols
                // .temp1
                // .populate(output, h, s1, ch, event.w[j], SHA_COMPRESS_K[j]);
                //
                let temp1 = h
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(event.w[j])
                    .wrapping_add(SHA_COMPRESS_K[j]);
                cols.temp1.value = Word::from(temp1);

                let a_rr_2 = cols.a_rr_2.populate(output, a, 2);
                let a_rr_13 = cols.a_rr_13.populate(output, a, 13);
                let a_rr_22 = cols.a_rr_22.populate(output, a, 22);
                let s0_intermediate = cols.s0_intermediate.populate(output, a_rr_2, a_rr_13);
                let s0 = cols.s0.populate(output, s0_intermediate, a_rr_22);

                let a_and_b = cols.a_and_b.populate(output, a, b);
                let a_and_c = cols.a_and_c.populate(output, a, c);
                let b_and_c = cols.b_and_c.populate(output, b, c);
                let maj_intermediate = cols.maj_intermediate.populate(output, a_and_b, a_and_c);
                let maj = cols.maj.populate(output, maj_intermediate, b_and_c);

                let temp2 = cols.temp2.populate(output, s0, maj);

                let d_add_temp1 = cols.d_add_temp1.populate(output, d, temp1);
                let temp1_add_temp2 = cols.temp1_add_temp2.populate(output, temp1, temp2);

                event.h[7] = g;
                event.h[6] = f;
                event.h[5] = e;
                event.h[4] = d_add_temp1;
                event.h[3] = c;
                event.h[2] = b;
                event.h[1] = a;
                event.h[0] = temp1_add_temp2;

                cols.is_real = F::one();
                cols.start = cols.is_real * cols.octet_num[0] * cols.octet[0];

                rows.push(row);
            }

            let mut v: [u32; 8] = (0..8)
                .map(|i| event.h[i])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            octet_num_idx += 1;
            // Store a, b, c, d, e, f, g, h.
            for j in 0..8usize {
                let mut row = [F::zero(); NUM_SHA_COMPRESS_COLS];
                let cols: &mut ShaCompressCols<F> = row.as_mut_slice().borrow_mut();

                cols.shard = F::from_canonical_u32(event.shard);
                cols.clk = F::from_canonical_u32(event.clk);
                cols.w_ptr = F::from_canonical_u32(event.w_ptr);
                cols.h_ptr = F::from_canonical_u32(event.h_ptr);

                cols.octet[j] = F::one();
                cols.octet_num[octet_num_idx] = F::one();

                cols.finalize_add.populate(output, og_h[j], event.h[j]);
                cols.mem
                    .populate_write(event.h_write_records[j], &mut new_byte_lookup_events);
                cols.mem_addr = F::from_canonical_u32(event.h_ptr + (j * 4) as u32);

                v[j] = event.h[j];
                cols.a = Word::from(v[0]);
                cols.b = Word::from(v[1]);
                cols.c = Word::from(v[2]);
                cols.d = Word::from(v[3]);
                cols.e = Word::from(v[4]);
                cols.f = Word::from(v[5]);
                cols.g = Word::from(v[6]);
                cols.h = Word::from(v[7]);

                match j {
                    0 => cols.finalized_operand = cols.a,
                    1 => cols.finalized_operand = cols.b,
                    2 => cols.finalized_operand = cols.c,
                    3 => cols.finalized_operand = cols.d,
                    4 => cols.finalized_operand = cols.e,
                    5 => cols.finalized_operand = cols.f,
                    6 => cols.finalized_operand = cols.g,
                    7 => cols.finalized_operand = cols.h,
                    _ => panic!("unsupported j"),
                };

                cols.is_real = F::one();
                cols.start = cols.is_real * cols.octet_num[0] * cols.octet[0];

                rows.push(row);
            }
        }

        output.add_byte_lookup_events(new_byte_lookup_events);

        let nb_rows = rows.len();
        let mut padded_nb_rows = nb_rows.next_power_of_two();
        if padded_nb_rows == 2 || padded_nb_rows == 1 {
            padded_nb_rows = 4;
        }

        for _ in nb_rows..padded_nb_rows {
            let row = [F::zero(); NUM_SHA_COMPRESS_COLS];
            rows.push(row);
        }

        // Convert the trace to a row major matrix.
        RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_SHA_COMPRESS_COLS,
        )
    }

    fn included(&self, shard: &Self::Record) -> bool {
        !shard.sha_compress_events.is_empty()
    }
}
