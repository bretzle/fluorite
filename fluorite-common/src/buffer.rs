#[derive(Debug)]
pub struct CircularBuffer<T, const N: usize> {
    pub data: [T; N],
    pub idx: usize,
}

impl<T, const N: usize> CircularBuffer<T, N> {
    pub fn new(data: [T; N]) -> Self {
        Self { data, idx: 0 }
    }

    pub fn push(&mut self, val: T) {
        if self.idx == N {
            self.idx = 0;
        }

        self.data[self.idx] = val;
		self.idx += 1;
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.data.iter()
    }
}
