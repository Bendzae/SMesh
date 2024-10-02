use crate::prelude::{HalfedgeId, HalfedgeOps, RunQuery, SMesh};

const MAX_LOOP_ITERATIONS: usize = 100;

impl SMesh {
    fn halfedge_loop(&self, h0: HalfedgeId) -> Vec<HalfedgeId> {
        let mut ret = vec![h0];
        let mut h = h0;

        let mut count = 0;

        loop {
            if count > MAX_LOOP_ITERATIONS {
                panic!("Max number of iterations reached. Is the mesh malformed?");
            }
            count += 1;

            h = h.next().run(self).expect("Halfedges should form a loop");
            if h == h0 {
                break;
            } else {
                ret.push(h);
            }
        }
        ret
    }
}
