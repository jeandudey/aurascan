use crate::OneEuroFilter;

#[derive(Debug)]
pub struct QuatFilter {
    f: [OneEuroFilter; 4],
    last: Option<[f64; 4]>,
}

impl QuatFilter {
    pub fn new(min_cutoff: f64, beta: f64, d_cutoff: f64) -> Self {
        Self {
            f: [
                OneEuroFilter::new(min_cutoff, beta, d_cutoff),
                OneEuroFilter::new(min_cutoff, beta, d_cutoff),
                OneEuroFilter::new(min_cutoff, beta, d_cutoff),
                OneEuroFilter::new(min_cutoff, beta, d_cutoff),
            ],
            last: None,
        }
    }

    pub fn filter(&mut self, mut q: [f64; 4], t: f64) -> [f64; 4] {
        if let Some(last) = self.last {
            let dot = q[0] * last[0] + q[1] * last[1] + q[2] * last[2] + q[3] + last[3];
            if dot < 0.0 {
                for c in q.iter_mut() {
                    *c = -*c;
                }
            }
        }

        let mut out = [0.0; 4];
        for i in 0..4 {
            out[i] = self.f[i].filter(q[i], t);
        }

        let n = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2] + out[3] * out[3]).sqrt();
        if n > 1e-9 {
            for c in out.iter_mut() {
                *c /= n;
            }
        }

        self.last = Some(out);
        out
    }
}
