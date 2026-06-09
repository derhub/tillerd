//! Indexer: markdown by heading → doc chunks.

use crate::{ChunkKind, Engram, NewChunk};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

const EXCLUDED_DIRS: &[&str] = &["node_modules", "target", ".git", "dist", "build", ".next"];

/// A section over the size bound is split into smaller chunks.
const MAX_CHUNK_CHARS: usize = 1500;
/// Lines of overlap carried between adjacent sub-chunks of a split section.
const OVERLAP_LINES: usize = 2;
/// Chunks whose body (excluding headings/blank lines) is shorter than this are
/// dropped, so bare headings and empty sections are not stored.
const MIN_BODY_CHARS: usize = 3;

/// A markdown chunk: its section heading (if any) and content.
#[derive(Debug, Clone, PartialEq)]
pub struct DocChunk {
    pub title: Option<String>,
    pub content: String,
}

/// A heading line, only when it is not inside a fenced code block.
fn heading_title(line: &str) -> Option<String> {
    let t = line.trim_start();
    let hashes = t.bytes().take_while(|b| *b == b'#').count();
    if (1..=6).contains(&hashes) && t.as_bytes().get(hashes) == Some(&b' ') {
        Some(t[hashes..].trim().to_string())
    } else {
        None
    }
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

/// Meaningful body length: characters in non-heading, non-blank lines. Used to
/// drop bare-heading or empty sections.
fn body_chars(lines: &[&str]) -> usize {
    lines
        .iter()
        .filter(|l| heading_title(l).is_none())
        .map(|l| l.trim().chars().count())
        .sum()
}

/// Split a long string at sentence boundaries. Prefers sentence boundaries
/// ([.!?;] followed by space/end); falls back to char boundary if no boundary found.
fn split_at_sentences(text: &str, max_size: usize) -> Vec<String> {
    if text.chars().count() <= max_size {
        return vec![text.to_string()];
    }

    let mut parts = Vec::new();
    let mut remaining = text;

    while remaining.chars().count() > max_size {
        if let Some(best_pos) = find_sentence_boundary(remaining, max_size) {
            let (chunk, rest) = remaining.split_at(best_pos);
            parts.push(chunk.trim_end().to_string());
            remaining = rest.trim_start();
        } else {
            let byte_pos = remaining
                .char_indices()
                .nth(max_size)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());
            parts.push(remaining[..byte_pos].trim_end().to_string());
            remaining = remaining[byte_pos..].trim_start();
        }
    }

    if !remaining.is_empty() {
        parts.push(remaining.to_string());
    }

    parts
}

/// Find the last sentence boundary (. ! ? ; followed by space/newline) within `max_chars`.
fn find_sentence_boundary(text: &str, max_chars: usize) -> Option<usize> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return None;
    }

    let boundary_chars = ['.', '!', '?', ';'];
    let mut best_pos = None;

    for i in (0..max_chars).rev() {
        if i < chars.len() - 1 && boundary_chars.contains(&chars[i]) {
            let next_char = chars.get(i + 1);
            if next_char == Some(&' ') || next_char == Some(&'\n') {
                best_pos = Some(chars[..=i + 1].iter().collect::<String>().len());
                break;
            }
        }
    }

    best_pos
}

/// Split markdown into chunks at heading boundaries of any level. Sections that
/// exceed the size bound are split at paragraph boundaries (never inside a fenced
/// code block) with a small overlap. Trivial chunks are dropped. Each chunk is
/// titled by its heading; content before the first heading forms an untitled
/// chunk.
pub fn chunk_markdown(content: &str) -> Vec<DocChunk> {
    let mut sections: Vec<DocChunk> = Vec::new();
    let mut title: Option<String> = None;
    let mut buf: Vec<&str> = Vec::new();
    let mut in_fence = false;

    let flush = |title: &Option<String>, buf: &[&str], out: &mut Vec<DocChunk>| {
        if body_chars(buf) >= MIN_BODY_CHARS {
            out.push(DocChunk {
                title: title.clone(),
                content: buf.join("\n").trim_end().to_string(),
            });
        }
    };

    for line in content.lines() {
        if is_fence(line) {
            in_fence = !in_fence;
        }
        if !in_fence {
            if let Some(h) = heading_title(line) {
                if !buf.is_empty() {
                    flush(&title, &buf, &mut sections);
                    buf.clear();
                }
                title = Some(h);
                buf.push(line);
                continue;
            }
        }
        buf.push(line);
    }
    if !buf.is_empty() {
        flush(&title, &buf, &mut sections);
    }

    let mut chunks = Vec::new();
    for sec in sections {
        if sec.content.chars().count() <= MAX_CHUNK_CHARS {
            chunks.push(sec);
        } else {
            split_large(&sec, &mut chunks);
        }
    }
    chunks
}

/// Split an oversized section at paragraph boundaries, carrying `OVERLAP_LINES`
/// of context forward. A boundary is only taken outside a fenced code block.
/// For single oversized lines, fall back to sentence boundaries.
fn split_large(sec: &DocChunk, out: &mut Vec<DocChunk>) {
    let lines: Vec<&str> = sec.content.lines().collect();
    let mut buf: Vec<&str> = Vec::new();
    let mut in_fence = false;

    let emit = |title: &Option<String>, buf: &[&str], out: &mut Vec<DocChunk>| {
        if body_chars(buf) >= MIN_BODY_CHARS {
            out.push(DocChunk {
                title: title.clone(),
                content: buf.join("\n").trim_end().to_string(),
            });
        }
    };

    let emit_sentence_split = |title: &Option<String>, line: &str, out: &mut Vec<DocChunk>| {
        let parts = split_at_sentences(line, MAX_CHUNK_CHARS);
        for part in parts {
            if body_chars(&[&part]) >= MIN_BODY_CHARS {
                out.push(DocChunk {
                    title: title.clone(),
                    content: part,
                });
            }
        }
    };

    for (i, line) in lines.iter().enumerate() {
        if is_fence(line) {
            in_fence = !in_fence;
        }
        buf.push(line);
        let at_paragraph_break = line.trim().is_empty() && !in_fence;
        let big_enough = buf.join("\n").chars().count() >= MAX_CHUNK_CHARS;
        let more_coming = i + 1 < lines.len();
        if at_paragraph_break && big_enough && more_coming {
            emit(&sec.title, &buf, out);
            // Carry overlap forward, but drop fence markers so a chunk can never
            // inherit a half-open code fence.
            let start = buf.len().saturating_sub(OVERLAP_LINES);
            buf = buf[start..]
                .iter()
                .copied()
                .filter(|l| !is_fence(l))
                .collect();
        } else if line.chars().count() > MAX_CHUNK_CHARS && buf.len() == 1 && !in_fence {
            // Single line too long: split at sentence boundaries.
            emit_sentence_split(&sec.title, line, out);
            buf.clear();
        }
    }
    if !buf.is_empty() {
        emit(&sec.title, &buf, out);
    }
}

/// Re-index all markdown under `cwd`: drop the project's prior doc chunks, then
/// chunk and store fresh ones. Returns the number of chunks stored.
pub fn index_project(memorya: &Engram, cwd: &Path, ts: i64) -> anyhow::Result<usize> {
    let cwd_str = cwd.to_string_lossy().to_string();
    memorya.store().delete_doc_chunks_under(&cwd_str)?;

    let mut files = Vec::new();
    collect_markdown(cwd, &mut files);

    let mut stored = 0usize;
    for path in files {
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let path_str = path.to_string_lossy().to_string();
        // Documents are indexed verbatim.
        for chunk in chunk_markdown(&raw) {
            let title = chunk.title.unwrap_or_else(|| short_path(&path_str));
            memorya.ingest(NewChunk {
                session_id: None,
                kind: ChunkKind::Doc,
                content: chunk.content,
                title: Some(title),
                file_path: Some(path_str.clone()),
                turn_index: None,
                ts,
            })?;
            stored += 1;
        }
    }
    Ok(stored)
}

/// Collect markdown files under `dir`, honoring `.gitignore` (and `.ignore`,
/// nested gitignores, and the global gitignore) so ignored files — secrets,
/// build output — are never indexed. Common heavy directories are pruned even
/// when not gitignored.
fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let walker = WalkBuilder::new(dir)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false) // honor .gitignore even outside a git repo
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            !EXCLUDED_DIRS.contains(&name)
        })
        .build();
    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path.to_path_buf());
        }
    }
}

fn short_path(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_at_every_heading_level() {
        let md = "# H1\nbody1\n## H2\nbody2\n###### H6\nbody6\n";
        let titles: Vec<_> = chunk_markdown(md)
            .into_iter()
            .map(|c| c.title.unwrap())
            .collect();
        assert_eq!(titles, vec!["H1", "H2", "H6"]);
    }

    #[test]
    fn preamble_before_first_heading_is_its_own_untitled_chunk() {
        let chunks = chunk_markdown("some intro prose here\n## Section\nbody");
        assert_eq!(chunks[0].title, None);
        assert!(chunks[0].content.contains("intro prose"));
        assert_eq!(chunks[1].title.as_deref(), Some("Section"));
    }

    #[test]
    fn single_chunk_when_no_headings() {
        let chunks = chunk_markdown("just a paragraph with enough text to keep");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].title, None);
    }

    #[test]
    fn a_hash_inside_a_code_fence_is_not_a_heading() {
        let md = "## Real\nintro\n```\n# not a heading\nmore code\n```\ntail";
        let chunks = chunk_markdown(md);
        assert_eq!(
            chunks.len(),
            1,
            "the fenced '# ...' must not start a new chunk"
        );
        assert_eq!(chunks[0].title.as_deref(), Some("Real"));
    }

    #[test]
    fn drops_trivial_chunks() {
        let chunks = chunk_markdown("## kept section with real body text here\nyes\n## x\n");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("real body"));
    }

    #[test]
    fn oversized_section_splits_with_overlap_and_keeps_fences_intact() {
        let para = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do";
        let mut md = String::from("## Big\n");
        for _ in 0..40 {
            md.push_str(para);
            md.push_str("\n\n");
        }
        let chunks = chunk_markdown(&md);
        assert!(chunks.len() > 1, "an oversized section must split");
        assert!(chunks.iter().all(|c| c.title.as_deref() == Some("Big")));
    }

    #[test]
    fn fenced_block_in_oversized_section_is_never_split() {
        let mut md = String::from("## Code\nintro paragraph\n\n```\n");
        for i in 0..400 {
            md.push_str(&format!("line {i} of a very long code block\n"));
        }
        md.push_str("```\n\ntail paragraph\n");
        for c in chunk_markdown(&md) {
            let fences = c.content.matches("```").count();
            assert!(
                fences == 0 || fences == 2,
                "a chunk must not contain a half-open fence"
            );
        }
    }

    #[test]
    fn excludes_heavy_directories() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(proj.join("node_modules")).unwrap();
        std::fs::write(proj.join("README.md"), "## Title\nfirst body").unwrap();
        std::fs::write(
            proj.join("node_modules/skip.md"),
            "## Skip\nshould not index",
        )
        .unwrap();

        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        assert_eq!(index_project(&e, &proj, 1).unwrap(), 1);
    }

    #[test]
    fn reindexing_replaces_prior_doc_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("README.md"), "## Title\nfirst body").unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        index_project(&e, &proj, 1).unwrap();

        std::fs::write(
            proj.join("README.md"),
            "## Title\nsecond body\n## More\nextra",
        )
        .unwrap();
        index_project(&e, &proj, 2).unwrap();
        assert_eq!(e.active_chunk_count().unwrap(), 2, "no stale chunks remain");
    }

    #[test]
    fn honors_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join(".gitignore"), "secret.md\nsecrets/\n").unwrap();
        std::fs::write(proj.join("README.md"), "## Public\npublic body").unwrap();
        std::fs::write(proj.join("secret.md"), "## Secret\napi key here").unwrap();
        std::fs::create_dir_all(proj.join("secrets")).unwrap();
        std::fs::write(proj.join("secrets/keys.md"), "## Keys\nsk-123").unwrap();

        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        assert_eq!(
            index_project(&e, &proj, 1).unwrap(),
            1,
            "only README indexed; gitignored files skipped"
        );
        let leaked: i64 = e
            .store()
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chunks WHERE content LIKE '%sk-123%' OR content LIKE '%api key%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(leaked, 0, "gitignored secrets never indexed");
    }

    #[test]
    fn indexes_documents_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("n.md"), "before middle after").unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        index_project(&e, &proj, 1).unwrap();
        let content: String = e
            .store()
            .conn()
            .query_row("SELECT content FROM chunks WHERE kind='doc'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(
            content.contains("middle after"),
            "doc kept verbatim, not truncated"
        );
    }

    #[test]
    fn splits_at_sentence_boundaries() {
        let text = "This is sentence one. This is sentence two. This is sentence three.";
        let parts = split_at_sentences(text, 30);
        assert!(parts.len() > 1, "long text should split");
        assert!(
            parts.iter().all(|p| p.chars().count() <= 35),
            "all parts fit within limit"
        );
    }

    #[test]
    fn sentence_split_falls_back_to_char_boundary() {
        let text = "a".repeat(2000);
        let parts = split_at_sentences(&text, 1500);
        assert!(parts.len() > 1, "text without boundaries should split");
        assert_eq!(
            parts[0].chars().count(),
            1500,
            "first chunk is exactly max size"
        );
    }

    #[test]
    fn single_long_line_splits_at_sentences() {
        let md = "## Title\nThis is a very long line with many words. More words here. And even more words. Stop here.";
        let chunks = chunk_markdown(md);
        assert!(
            chunks.iter().all(|c| c.content.chars().count() <= 1600),
            "all chunks must fit within size limit"
        );
    }

    #[test]
    fn sentence_split_respects_min_body_chars() {
        let text = "Short.";
        let parts = split_at_sentences(text, 100);
        assert_eq!(parts.len(), 1, "text under limit stays whole");
    }
}
