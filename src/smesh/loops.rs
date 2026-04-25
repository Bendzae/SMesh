//! Halfedge-loop walks (following `next` pointers).

use crate::prelude::{HalfedgeId, HalfedgeOps, RunQuery, SMesh};

/// Safety cap on loop walks: if a face's `next`-chain exceeds this many steps
/// the mesh is treated as malformed. Bumping this is only sensible for
/// hand-crafted n-gons with very high valence.
const MAX_LOOP_ITERATIONS: usize = 100;

impl SMesh {
    /// Collect every halfedge reachable from `h0` by repeatedly following
    /// `next`, stopping when we return to `h0`.
    ///
    /// On a well-formed face this yields the full boundary of the face (or,
    /// if `h0` is a boundary halfedge, the full boundary loop of the mesh
    /// hole). For face boundaries this is equivalent to `face.halfedges(mesh)`.
    ///
    /// # Panics
    /// Panics if the chain does not close within 100 steps — indicating
    /// broken halfedge connectivity.
    pub fn halfedge_loop(&self, h0: HalfedgeId) -> Vec<HalfedgeId> {
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
