//! Endpoint faces (hook, tool, admin, subscribe) over loopback.
//! Async read/write adapters for the shared contracts::framing codec.

pub mod admin;
pub mod dispatch;
pub mod hook;
pub mod mcp;
pub mod subscribe;
pub mod tool;

use contracts::framing::{encode_frame, HEADER_SIZE, MAX_FRAME_SIZE};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Read one length-prefixed frame, or `None` at a clean end of stream.
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut header = [0u8; HEADER_SIZE];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(header) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame exceeds max size",
        ));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

/// Write one length-prefixed frame and flush it.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> std::io::Result<()> {
    writer.write_all(&encode_frame(payload)).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn frames_roundtrip_through_encode_and_read() {
        let mut stream = Vec::new();
        stream.extend_from_slice(&encode_frame(b"alpha"));
        stream.extend_from_slice(&encode_frame(b"beta"));

        let mut reader: &[u8] = &stream;
        assert_eq!(
            read_frame(&mut reader).await.unwrap().as_deref(),
            Some(&b"alpha"[..])
        );
        assert_eq!(
            read_frame(&mut reader).await.unwrap().as_deref(),
            Some(&b"beta"[..])
        );
        assert_eq!(read_frame(&mut reader).await.unwrap(), None);
    }

    #[tokio::test]
    async fn read_frame_reports_none_on_a_clean_end_of_stream() {
        let mut reader: &[u8] = &[];
        assert_eq!(read_frame(&mut reader).await.unwrap(), None);
    }

    #[tokio::test]
    async fn read_frame_rejects_a_header_declaring_an_oversize_length() {
        // A length prefix just past the 1 MiB cap, with no payload behind it: the
        // codec must reject on the bound, not allocate and then hit end-of-stream.
        let oversize: u32 = (1 << 20) + 1;
        let header = oversize.to_be_bytes();

        let mut reader: &[u8] = &header;
        let err = read_frame(&mut reader).await.unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn write_frame_emits_the_length_prefixed_encoding() {
        let mut out = Vec::new();
        write_frame(&mut out, b"payload").await.unwrap();
        assert_eq!(out, encode_frame(b"payload"));
    }
}
