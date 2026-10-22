use triple_buffer::{Input, Output, triple_buffer};

/// Makes a latest-value cell (a triple buffer) holding `initial` and returns its two halves. The writer publishes whole
/// values; the reader always sees the newest published one. Neither side waits for the other, and nothing is copied
/// on publish: the buffers swap.
pub fn latest<T: Clone + Send>(initial: &T) -> (LatestWriter<T>, LatestReader<T>) {
    let (input, output) = triple_buffer(initial);
    (LatestWriter { input, published: 0 }, LatestReader { output })
}

/// The writing half of a [`latest`] cell. One thread writes.
pub struct LatestWriter<T: Send> {
    input: Input<T>,
    published: u64,
}

impl<T: Send> LatestWriter<T> {
    /// Publishes `value`, replacing the unread one if the reader has not looked yet.
    pub fn publish(&mut self, value: T) {
        self.input.write(value);
        self.published = self.published.wrapping_add(1);
    }

    /// The back buffer, for filling in place without allocating (it holds an older value: overwrite all of it). Call
    /// [`LatestWriter::publish_in_place`] when done.
    pub fn buffer_mut(&mut self) -> &mut T {
        self.input.input_buffer_mut()
    }

    /// Publishes what [`LatestWriter::buffer_mut`] holds.
    pub fn publish_in_place(&mut self) {
        self.input.publish();
        self.published = self.published.wrapping_add(1);
    }

    /// Values published so far.
    pub fn published(&self) -> u64 {
        self.published
    }

    /// Whether the reader took the last published value.
    pub fn consumed(&self) -> bool {
        self.input.consumed()
    }
}

/// The reading half of a [`latest`] cell. One thread reads.
pub struct LatestReader<T: Send> {
    output: Output<T>,
}

impl<T: Send> LatestReader<T> {
    /// The newest published value (the same one again if nothing new was published).
    pub fn read(&mut self) -> &T {
        self.output.read()
    }

    /// Whether a value newer than the last [`LatestReader::read`] is waiting.
    pub fn has_update(&self) -> bool {
        self.output.updated()
    }
}

impl<T: Send> std::fmt::Debug for LatestWriter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LatestWriter").field("published", &self.published).finish_non_exhaustive()
    }
}

impl<T: Send> std::fmt::Debug for LatestReader<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LatestReader").finish_non_exhaustive()
    }
}
