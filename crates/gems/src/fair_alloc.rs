/// Algorithm for temporally fair allocation of a discrete quantity requiring only an iterator
/// over capacity requesters (and corresponding state).
#[derive(Default)]
pub struct FairAlloc {
    capacity: usize,
    round_op_threshold: f64,

    sum_request: usize,
    sum_debt: f64,

    available: usize,
    request: f64,
    fullfillment: f64,
}

impl FairAlloc {
    /// Total capacity which can be allocated
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.set_capacity(capacity);
        self
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
    }

    /// Due to rounding errors perfect allocation might fail. This threshold will allow slightly
    /// more optimistic allocation which will more likely fill all capacity.
    pub fn with_round_op_threshold(mut self, round_op_threshold: f64) -> Self {
        self.round_op_threshold = round_op_threshold;
        self
    }

    /// The first pass over request
    pub fn with_warmup<'a>(
        mut self,
        items: impl Iterator<Item = (usize, &'a mut FairAllocState)>,
    ) -> Self {
        self.warmup(items);
        self
    }

    pub fn warmup<'a>(&mut self, items: impl Iterator<Item = (usize, &'a mut FairAllocState)>) {
        self.warmup_begin();

        for (request, s) in items {
            self.warmup_request(request, s);
        }

        self.warmup_end();
    }

    pub fn warmup_begin(&mut self) {
        self.sum_debt = 0.;
        self.sum_request = 0;
    }

    pub fn warmup_request(&mut self, request: usize, state: &mut FairAllocState) {
        self.sum_request += request;
        self.sum_debt += state.debt;
    }

    pub fn warmup_end(&mut self) {
        self.available = self.capacity.min(self.sum_request);
        self.fullfillment =
            ((self.available as f64 - self.sum_debt) / self.sum_request as f64).min(1.);

        self.request = self.sum_debt + self.fullfillment * self.sum_request as f64;
    }

    /// The second pass over request consuming the instances and producing the allocation
    pub fn allocate<'a>(
        &mut self,
        items: impl Iterator<Item = (usize, &'a mut FairAllocState)>,
    ) -> impl Iterator<Item = usize> {
        items
            .into_iter()
            .map(move |(request, s)| s.allocate(request, self))
    }
}

/// State tracked for each requester necessary to implement temporally fair allocation
#[derive(Debug, Default)]
pub struct FairAllocState {
    debt: f64,
}

impl FairAllocState {
    pub fn allocate(&mut self, request: usize, glob: &mut FairAlloc) -> usize {
        // This is our temporally fair share
        self.debt += glob.fullfillment * request as f64;

        // Rounded down to nearest non-negative integer
        let mut n = self.debt.floor().max(0.) as usize;

        glob.request -= f64::from(self.debt);

        // Value rounded down is guaranteed to be available
        glob.available -= n;
        self.debt -= n as f64;

        // Check if we can take one more, i.e. round up
        println!("{} / {}", glob.request, glob.available);
        if glob.request + 1. <= glob.available as f64 + glob.round_op_threshold && n + 1 <= request
        {
            n += 1;
            glob.available -= 1;
            self.debt -= 1.;
        }

        n
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fair_alloc_under_cap() {
        let mut state = (0..5)
            .map(|_| FairAllocState::default())
            .collect::<Vec<_>>();
        let mut total = vec![0; 5];

        let request = vec![1, 3, 5, 1, 9];
        println!("request: {request:?}",);

        for _ in 0..100 {
            for (t, n) in total.iter_mut().zip(
                FairAlloc::default()
                    .with_capacity(20)
                    .with_warmup(request.iter().cloned().zip(state.iter_mut()))
                    .allocate(request.iter().cloned().zip(state.iter_mut())),
            ) {
                *t += n;
            }
        }

        println!("state: {state:?}",);
        println!("total: {total:?}",);

        for (&t, &d) in total.iter().zip(request.iter()) {
            assert_eq!(t, d * 100);
        }
    }

    #[test]
    fn test_fair_alloc_over_cap() {
        for len in 1..10 {
            println!(">>>>> len: {len}");

            let mut state = (0..len)
                .map(|_| FairAllocState::default())
                .collect::<Vec<_>>();
            let mut total = vec![0; len];

            let request = vec![1; len];
            println!("request: {request:?}",);

            let iter = 3 * len;

            for _ in 0..iter {
                let count = FairAlloc::default()
                    .with_capacity(len - 1)
                    .with_round_op_threshold(0.00001)
                    .with_warmup(request.iter().cloned().zip(state.iter_mut()))
                    .allocate(request.iter().cloned().zip(state.iter_mut()))
                    .collect::<Vec<_>>();

                for (t, n) in total.iter_mut().zip(count.iter()) {
                    *t += n;
                }

                println!("count: {count:?}",);
                println!("state: {state:?}",);

                assert_eq!(count.iter().sum::<usize>(), len - 1);
            }

            println!("state: {state:?}",);
            println!("total: {total:?}",);

            for &t in total.iter() {
                assert_eq!(t, iter * (len - 1) / len);
            }
        }
    }
}
