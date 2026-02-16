use std::collections::VecDeque;

/// Fixed-capacity circular buffer for time-series data.
///
/// Backed by a `VecDeque`. When the buffer is full, the oldest element is
/// evicted on each push. Iteration order is oldest → newest.
#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    buf: VecDeque<T>,
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    /// Create an empty ring buffer with the given maximum capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "RingBuffer capacity must be > 0");
        Self {
            buf: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a value, evicting the oldest entry when at capacity.
    pub fn push(&mut self, value: T) {
        if self.buf.len() == self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(value);
    }

    /// Iterate from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buf.iter()
    }

    /// The most recently pushed value, if any.
    pub fn last(&self) -> Option<&T> {
        self.buf.back()
    }

    /// Number of elements currently stored.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the buffer contains no elements.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Maximum number of elements the buffer can hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Whether the buffer is at full capacity.
    pub fn is_full(&self) -> bool {
        self.buf.len() == self.capacity
    }

    /// Remove all elements.
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_under_capacity() {
        let mut rb = RingBuffer::new(5);
        rb.push(1);
        rb.push(2);
        rb.push(3);

        assert_eq!(rb.len(), 3);
        assert!(!rb.is_full());
        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&1, &2, &3]);
    }

    #[test]
    fn push_over_capacity_evicts_oldest() {
        let mut rb = RingBuffer::new(3);
        for i in 1..=5 {
            rb.push(i);
        }

        assert_eq!(rb.len(), 3);
        assert!(rb.is_full());
        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&3, &4, &5]);
    }

    #[test]
    fn empty_buffer() {
        let rb: RingBuffer<f64> = RingBuffer::new(10);

        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
        assert_eq!(rb.last(), None);
        assert_eq!(rb.iter().count(), 0);
    }

    #[test]
    fn single_element() {
        let mut rb = RingBuffer::new(3);
        rb.push(42);

        assert_eq!(rb.len(), 1);
        assert_eq!(rb.last(), Some(&42));
        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&42]);
    }

    #[test]
    fn last_returns_newest() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.push(4); // evicts 1

        assert_eq!(rb.last(), Some(&4));
    }

    #[test]
    fn clear_resets() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);

        rb.clear();

        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);
        assert_eq!(rb.capacity(), 3);
    }

    #[test]
    fn is_full_transitions() {
        let mut rb = RingBuffer::new(2);
        assert!(!rb.is_full());

        rb.push(1);
        assert!(!rb.is_full());

        rb.push(2);
        assert!(rb.is_full());

        rb.push(3); // evicts 1, still full
        assert!(rb.is_full());
    }

    #[test]
    fn capacity_preserved() {
        let rb: RingBuffer<u8> = RingBuffer::new(100);
        assert_eq!(rb.capacity(), 100);
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn zero_capacity_panics() {
        let _ = RingBuffer::<i32>::new(0);
    }
}
