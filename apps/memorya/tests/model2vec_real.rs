//! Real embedding model test. #[ignore]d (downloads ~130MB model).

use memorya::embed::{cosine, Embedder, Model2VecEmbedder};

const REPO: &str = "minishlab/potion-retrieval-32M";

#[test]
#[ignore = "downloads ~130MB model from the hub"]
fn model_loads_and_reports_dim() {
    let e = Model2VecEmbedder::from_repo(REPO).unwrap();
    assert!(e.dim() > 0, "model reports an embedding dimension");
    assert_eq!(e.model_id(), REPO);
}

#[test]
#[ignore = "downloads ~130MB model from the hub"]
fn semantically_related_text_ranks_above_unrelated() {
    let e = Model2VecEmbedder::from_repo(REPO).unwrap();
    let q = e.embed("which database does the project use");
    let near = e.embed("the service stores its data in a PostgreSQL database");
    let far = e.embed("the cat dozed on the warm windowsill all afternoon");
    let s_near = cosine(&q, &near);
    let s_far = cosine(&q, &far);
    assert!(
        s_near > s_far,
        "related ({s_near:.3}) must outrank unrelated ({s_far:.3})"
    );
}
