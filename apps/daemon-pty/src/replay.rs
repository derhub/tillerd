//! Replay buffer: bounded ring, evicts oldest chunks.

const CAPACITY: usize = 64 * 1024;

#[derive(Default)]
pub struct ReplayBuffer {
    chunks: std::collections::VecDeque<Vec<u8>>,
    total_bytes: usize,
}

impl ReplayBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, chunk: &[u8]) {
        self.chunks.push_back(chunk.to_vec());
        self.total_bytes += chunk.len();
        while self.total_bytes > CAPACITY {
            if let Some(dropped) = self.chunks.pop_front() {
                self.total_bytes -= dropped.len();
            } else {
                break;
            }
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.total_bytes);
        for c in &self.chunks {
            out.extend_from_slice(c);
        }
        out
    }

    #[allow(dead_code)] // buffered-byte count; used in tests
    pub fn len(&self) -> usize {
        self.total_bytes
    }

    #[allow(dead_code)] // paired with len() to satisfy clippy::len_without_is_empty
    pub fn is_empty(&self) -> bool {
        self.total_bytes == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_order() {
        let mut b = ReplayBuffer::new();
        b.push(b"hello ");
        b.push(b"world");
        assert_eq!(b.bytes(), b"hello world");
    }

    #[test]
    fn evicts_oldest_when_over_capacity() {
        let mut b = ReplayBuffer::new();
        let chunk = vec![b'x'; 40 * 1024];
        b.push(&chunk);
        b.push(&chunk); // 80 KB total — first chunk evicted
        assert!(b.len() <= CAPACITY);
        assert_eq!(b.len(), 40 * 1024);
    }
}
