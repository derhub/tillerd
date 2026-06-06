//! Embedding: the [`Embedder`] boundary, the static-model implementation, and
//! brute-force cosine.

pub trait Embedder: Send + Sync {
    fn model_id(&self) -> &str;
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// Encode a vector as a little-endian f32 BLOB for storage.
pub fn encode_vec(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Decode a little-endian f32 BLOB back into a vector.
pub fn decode_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// A static, CPU-only embedding model. Downloaded from the model hub on first
/// use and cached locally; later runs load from the cache without network.
pub struct Model2VecEmbedder {
    model: model2vec_rs::model::StaticModel,
    id: String,
    dim: usize,
}

impl Model2VecEmbedder {
    /// Load by hub repo id (e.g. `minishlab/potion-retrieval-32M`). Downloads
    /// and caches on first use; loads from cache afterward. The repo id is also
    /// the model identity recorded with each embedding.
    pub fn from_repo(repo_id: impl Into<String>) -> anyhow::Result<Self> {
        let id = repo_id.into();
        let model = model2vec_rs::model::StaticModel::from_pretrained(&id, None, None, None)?;
        let dim = model.encode_single("probe").len();
        Ok(Self { model, id, dim })
    }

    /// Load from an already-present local model directory (no network).
    pub fn from_path(
        path: impl AsRef<std::path::Path>,
        id: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let p = path.as_ref();
        if !p.exists() {
            anyhow::bail!("embedding model not found at {}", p.display());
        }
        let model = model2vec_rs::model::StaticModel::from_pretrained(p, None, None, None)?;
        let dim = model.encode_single("probe").len();
        Ok(Self {
            model,
            id: id.into(),
            dim,
        })
    }
}

impl Embedder for Model2VecEmbedder {
    fn model_id(&self) -> &str {
        &self.id
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed(&self, text: &str) -> Vec<f32> {
        self.model.encode_single(text)
    }
}

/// Cosine similarity between two equal-length vectors.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Embed all chunks that lack an embedding for `embedder`'s model, up to
/// `limit`. Runs out-of-band of the write path. Returns the number embedded.
pub fn embed_pending(
    store: &crate::store::Store,
    embedder: &dyn Embedder,
    limit: i64,
) -> anyhow::Result<usize> {
    let pending = store.chunks_missing_embeddings(embedder.model_id(), limit)?;
    let n = pending.len();
    for (id, _kind, content) in pending {
        let vec = embedder.embed(&content);
        store.put_embedding(&id.to_string(), "chunk", embedder.model_id(), &vec)?;
    }
    Ok(n)
}

/// A deterministic, dependency-free embedder used only as a test double, so the
/// pipeline can be exercised without downloading the real model. It hashes
/// tokens into a fixed-width bag-of-words vector and L2-normalizes — lexical,
/// not semantic, but stable.
#[cfg(test)]
pub(crate) struct StubEmbedder {
    dim: usize,
    id: String,
}

#[cfg(test)]
impl StubEmbedder {
    pub fn with_id(dim: usize, id: impl Into<String>) -> Self {
        Self { dim, id: id.into() }
    }
}

#[cfg(test)]
impl Default for StubEmbedder {
    fn default() -> Self {
        Self::with_id(256, "stub")
    }
}

#[cfg(test)]
impl Embedder for StubEmbedder {
    fn model_id(&self) -> &str {
        &self.id
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0f32; self.dim];
        for token in text.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()) {
            let mut h: u64 = 1469598103934665603; // FNV-1a offset basis
            for b in token.to_ascii_lowercase().bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
            v[(h % self.dim as u64) as usize] += 1.0;
        }
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_embeds_to_fixed_dim_and_normalizes() {
        let v = StubEmbedder::with_id(64, "stub").embed("auth middleware token expiry");
        assert_eq!(v.len(), 64);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_is_one_for_identical_vectors() {
        let e = StubEmbedder::default();
        let v = e.embed("database connection pool");
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-5);
    }
}
