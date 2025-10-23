/// Generic buffer with automatic flushing based on size and time
///
/// This buffer accumulates items and automatically determines when to flush
/// based on either reaching a maximum item count or exceeding a time threshold.
///
/// # Type Parameters
/// * `T` - The type of items to buffer
///
/// # Examples
/// ```
/// use scotty::tasks::timed_buffer::TimedBuffer;
///
/// let mut buffer = TimedBuffer::new(10, 100);  // 10 items or 100ms
/// buffer.push("item1");
/// buffer.push("item2");
///
/// if buffer.should_flush() {
///     let items = buffer.flush();
///     // Process items...
/// }
/// ```
pub struct TimedBuffer<T> {
    items: Vec<T>,
    last_flush: tokio::time::Instant,
    max_items: usize,
    max_delay_ms: u64,
}

impl<T> TimedBuffer<T> {
    /// Create a new TimedBuffer with specified limits
    ///
    /// # Arguments
    /// * `max_items` - Maximum number of items before auto-flush
    /// * `max_delay_ms` - Maximum milliseconds to wait before auto-flush
    pub fn new(max_items: usize, max_delay_ms: u64) -> Self {
        Self {
            items: Vec::with_capacity(max_items),
            last_flush: tokio::time::Instant::now(),
            max_items,
            max_delay_ms,
        }
    }

    /// Add an item to the buffer
    pub fn push(&mut self, item: T) {
        self.items.push(item);
    }

    /// Check if the buffer should be flushed
    ///
    /// Returns true if either:
    /// - The buffer has reached max_items
    /// - max_delay_ms has elapsed since last flush
    pub fn should_flush(&self) -> bool {
        self.items.len() >= self.max_items
            || self.last_flush.elapsed() >= tokio::time::Duration::from_millis(self.max_delay_ms)
    }

    /// Flush the buffer and return all items
    ///
    /// Resets the internal timer and empties the buffer.
    /// Returns all buffered items.
    pub fn flush(&mut self) -> Vec<T> {
        self.last_flush = tokio::time::Instant::now();
        std::mem::take(&mut self.items)
    }

    /// Check if the buffer contains any items
    pub fn has_data(&self) -> bool {
        !self.items.is_empty()
    }

    /// Get the number of items currently in the buffer
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the buffer is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_size_flush() {
        let mut buffer = TimedBuffer::new(3, 1000);
        buffer.push("a");
        buffer.push("b");
        assert!(!buffer.should_flush());

        buffer.push("c");
        assert!(buffer.should_flush());

        let items = buffer.flush();
        assert_eq!(items, vec!["a", "b", "c"]);
        assert!(buffer.is_empty());
    }

    #[tokio::test]
    async fn test_buffer_time_flush() {
        let mut buffer = TimedBuffer::new(100, 50);
        buffer.push("a");
        assert!(!buffer.should_flush());

        tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;
        assert!(buffer.should_flush());

        let items = buffer.flush();
        assert_eq!(items, vec!["a"]);
    }

    #[test]
    fn test_buffer_utility_methods() {
        let mut buffer = TimedBuffer::new(10, 100);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert!(!buffer.has_data());

        buffer.push(1);
        buffer.push(2);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 2);
        assert!(buffer.has_data());
    }
}
