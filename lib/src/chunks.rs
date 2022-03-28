//! Provides an chunks based iterator over a PortalFile
//!

/// An iterator of an mmap'd PortalFile
pub struct PortalChunks<'a, T: 'a> {
    v: &'a [T],
    chunk_size: usize,
}

impl<'a, T: 'a> PortalChunks<'a, T> {
    pub fn init(data: &'a [T], chunk_size: usize) -> PortalChunks<'a, T> {
        PortalChunks {
            v: data,
            chunk_size,
        }
    }
}

impl<'a> Iterator for PortalChunks<'a, u8> {
    type Item = &'a [u8];

    // The return type is `Option<T>`:
    //     * When the `Iterator` is finished, `None` is returned.
    //     * Otherwise, the next value is wrapped in `Some` and returned.
    fn next(&mut self) -> Option<Self::Item> {
        // return up to the next chunk size
        if self.v.is_empty() {
            return None;
        }

        // split to get the next chunk and move the slice along
        let chunksz = std::cmp::min(self.v.len(), self.chunk_size);
        let (beg, end) = self.v.split_at(chunksz);

        // update next slice
        self.v = end;
        Some(beg)
    }
}
