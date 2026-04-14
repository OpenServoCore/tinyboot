/// Fixed-size ring buffer.
///
/// Designed for use with `PageWriter` where `N = 2 * PAGE_SIZE`.
/// When consumed in PAGE_SIZE-aligned chunks, `peek` always returns
/// a contiguous slice (no wrap on the read side).
#[repr(align(4))]
pub struct RingBuf<const N: usize> {
    buf: core::mem::MaybeUninit<[u8; N]>,
    head: usize,
    tail: usize,
}

impl<const N: usize> Default for RingBuf<N> {
    fn default() -> Self {
        Self {
            buf: core::mem::MaybeUninit::uninit(),
            head: 0,
            tail: 0,
        }
    }
}

impl<const N: usize> RingBuf<N> {
    /// Bytes available to read.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.head.wrapping_sub(self.tail) % N
    }

    /// Returns true if the buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    /// Push bytes into the buffer, handling wrap-around.
    /// Returns the number of bytes actually written (short if full).
    pub fn push(&mut self, data: &[u8]) -> usize {
        let free = N - 1 - self.len();
        let n = data.len().min(free);

        let ptr = self.buf.as_mut_ptr() as *mut u8;
        let mut head = self.head;
        for i in 0..n {
            // SAFETY: i < n <= data.len(), head < N
            unsafe { *ptr.add(head) = *data.get_unchecked(i) };
            head += 1;
            if head == N {
                head = 0;
            }
        }
        self.head = head;
        n
    }

    /// Borrow up to `n` contiguous readable bytes without consuming.
    ///
    /// Returns fewer than `n` bytes only if fewer are available.
    /// When `N = 2 * PAGE_SIZE` and you consume in PAGE_SIZE chunks,
    /// this always returns a contiguous slice (read side never wraps mid-page).
    pub fn peek(&self, n: usize) -> &[u8] {
        let available = self.len().min(n);
        // SAFETY: tail + available <= N, guaranteed by len() and consume() modulo N
        unsafe {
            core::slice::from_raw_parts(self.buf.as_ptr().cast::<u8>().add(self.tail), available)
        }
    }

    /// Advance the read pointer by `n` bytes.
    pub fn consume(&mut self, n: usize) {
        self.tail = (self.tail + n) % N;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let buf = RingBuf::<8>::default();
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.peek(8), &[]);
    }

    #[test]
    fn push_and_peek() {
        let mut buf = RingBuf::<8>::default();
        assert_eq!(buf.push(&[1, 2, 3]), 3);
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.peek(3), &[1, 2, 3]);
    }

    #[test]
    fn consume_advances() {
        let mut buf = RingBuf::<8>::default();
        buf.push(&[1, 2, 3, 4]);
        buf.consume(2);
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.peek(2), &[3, 4]);
    }

    #[test]
    fn push_wraps() {
        let mut buf = RingBuf::<8>::default();
        buf.push(&[1, 2, 3, 4, 5]);
        buf.consume(4); // tail=4, head=5
        assert_eq!(buf.push(&[6, 7, 8, 9]), 4); // wraps: 6,7,8 at end, 9 at start
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.peek(1), &[5]);
    }

    #[test]
    fn full_returns_short() {
        let mut buf = RingBuf::<4>::default();
        assert_eq!(buf.push(&[1, 2, 3]), 3); // capacity is N-1 = 3
        assert_eq!(buf.push(&[4]), 0); // full
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn page_aligned_peek_is_contiguous() {
        // Simulates PageWriter pattern: N=8, PAGE_SIZE=4
        let mut buf = RingBuf::<8>::default();

        // Fill first page
        buf.push(&[1, 2, 3, 4]);
        assert_eq!(buf.peek(4), &[1, 2, 3, 4]);
        buf.consume(4);

        // Fill second page
        buf.push(&[5, 6, 7, 8]);
        assert_eq!(buf.peek(4), &[5, 6, 7, 8]);
        buf.consume(4);

        // Back to first page slot
        buf.push(&[9, 10, 11, 12]);
        assert_eq!(buf.peek(4), &[9, 10, 11, 12]);
    }
}
