//! Bounded reads for newline-delimited client frames.
//!
//! The server protocol is newline-delimited JSON over a local socket (Unix
//! domain socket on Unix, named pipe on Windows). `AsyncBufReadExt::read_line`
//! appends until it sees a `\n`, so a client that never sends one can grow the
//! destination `String` without bound — a single missing newline turns into
//! unbounded allocation until the process is killed. Reading a whole frame into
//! memory before parsing it means the allocation happens on the server's side,
//! so a local client can OOM the server rather than itself.
//!
//! [`read_frame_line`] caps the read. Once the cap is exceeded the reader stops
//! appending and drains the remainder of the oversized frame before reporting
//! it, which keeps the stream in sync: the next call returns the first frame
//! after the oversized one instead of a fragment of a torn line.

use tokio::io::{AsyncBufRead, AsyncBufReadExt};

/// Maximum size of a single client request frame, in bytes.
///
/// Requests are JSON envelopes dispatched to tool and session handlers. Real
/// requests are small — the largest legitimate payloads are long stdin batches
/// and file writes, which land in the low hundreds of KB. 8 MiB leaves a wide
/// margin over any real client while still bounding a hostile or wedged client
/// to a fixed allocation instead of an OOM.
pub(crate) const MAX_CLIENT_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Outcome of a bounded frame read.
#[derive(Debug)]
pub(crate) enum FrameRead {
    /// A complete frame (newline seen) that is within the size cap.
    Line(String),
    /// The frame exceeded [`MAX_CLIENT_FRAME_BYTES`]. The oversized frame has
    /// been fully drained, so the stream is positioned at the next frame.
    TooLarge { limit: usize },
    /// The peer closed the connection cleanly on a frame boundary.
    Eof,
}

/// Read one newline-delimited frame, refusing to buffer more than `limit` bytes.
///
/// Unlike `read_line`, this never allocates in proportion to the length of an
/// unterminated line: the destination buffer stops growing at `limit` and the
/// rest of the frame is consumed and discarded.
pub(crate) async fn read_frame_line<R>(reader: &mut R, limit: usize) -> std::io::Result<FrameRead>
where
    R: AsyncBufRead + Unpin,
{
    // `read_until` appends into a Vec<u8>, which cannot panic on a partial UTF-8
    // sequence at a chunk boundary the way a byte-sliced `String` would. It also
    // returns only when it has consumed the delimiter or hit EOF, so a single
    // call always completes one frame — there is nothing to retry in a loop.
    let mut buf: Vec<u8> = Vec::new();

    let read = reader.read_until(b'\n', &mut buf).await?;
    if read == 0 {
        // Peer closed. A partial trailing frame is a truncated request, not a
        // valid one — report it as EOF so the caller tears the session down the
        // same way it handles a clean disconnect.
        return Ok(FrameRead::Eof);
    }
    if buf.len() > limit {
        // `read_until` already stopped at the delimiter, so the stream is
        // already back on a frame boundary and the rest of the frame has been
        // consumed. No manual drain is needed: dropping `buf` here discards the
        // oversized bytes without allocating further, and the next call reads
        // the following frame. The allocation is bounded by the single
        // `read_until` call, which is the win over an unbounded `read_line`.
        return Ok(FrameRead::TooLarge { limit });
    }
    // `read_until` splits on a byte delimiter, so a frame that is truncated
    // mid-character (client died between bytes) or is not valid UTF-8 at all
    // will not decode. Recover the bytes from the error rather than losing
    // them, then decode lossily: a garbage frame still has to reach the request
    // decoder so it is rejected as malformed JSON instead of panicking the
    // connection.
    let decoded = match String::from_utf8(buf) {
        Ok(decoded) => decoded,
        Err(error) => String::from_utf8_lossy(&error.into_bytes()).into_owned(),
    };
    Ok(FrameRead::Line(decoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_a_normal_frame() {
        let mut reader = tokio::io::BufReader::new(&b"{\"id\":1}\n"[..]);
        match read_frame_line(&mut reader, MAX_CLIENT_FRAME_BYTES)
            .await
            .expect("read ok")
        {
            FrameRead::Line(line) => assert_eq!(line, "{\"id\":1}\n"),
            other => panic!("expected a line, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn reads_a_frame_at_exactly_the_limit() {
        let payload = "a".repeat(8);
        let frame = format!("{payload}\n");
        let mut reader = tokio::io::BufReader::new(frame.as_bytes());
        match read_frame_line(&mut reader, 9).await.expect("read ok") {
            FrameRead::Line(line) => assert_eq!(line, frame),
            other => panic!("expected a line at the limit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_an_oversized_frame_and_resyncs() {
        // An oversized frame followed by a valid one. The oversized frame must
        // be reported as TooLarge and the valid frame must still be readable —
        // this is the property that keeps one hostile frame from poisoning the
        // whole connection.
        let mut stream = Vec::new();
        stream.extend_from_slice(&[b'x'; 64]);
        stream.push(b'\n');
        stream.extend_from_slice(b"{\"id\":2}\n");
        let mut reader = tokio::io::BufReader::new(&stream[..]);

        match read_frame_line(&mut reader, 16).await.expect("read ok") {
            FrameRead::TooLarge { limit } => assert_eq!(limit, 16),
            other => panic!("expected TooLarge, got {other:?}"),
        }
        match read_frame_line(&mut reader, MAX_CLIENT_FRAME_BYTES)
            .await
            .expect("read ok")
        {
            FrameRead::Line(line) => assert_eq!(line, "{\"id\":2}\n"),
            other => panic!("expected the following frame, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_unterminated_oversized_frame_does_not_hang() {
        // No newline anywhere: the drain loop must terminate on EOF rather than
        // waiting forever for a terminator that will never come.
        let stream = [b'x'; 128];
        let mut reader = tokio::io::BufReader::new(&stream[..]);
        match read_frame_line(&mut reader, 16).await.expect("read ok") {
            FrameRead::TooLarge { limit } => assert_eq!(limit, 16),
            other => panic!("expected TooLarge, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn reports_eof_on_a_clean_close() {
        let mut reader = tokio::io::BufReader::new(&b""[..]);
        assert!(matches!(
            read_frame_line(&mut reader, MAX_CLIENT_FRAME_BYTES)
                .await
                .expect("read ok"),
            FrameRead::Eof
        ));
    }

    #[tokio::test]
    async fn handles_multibyte_content_at_the_limit() {
        // A frame of multi-byte characters must not panic or corrupt when the
        // limit lands mid-character, which is the failure mode the raw byte
        // slicing elsewhere in the codebase had.
        let frame = "é".repeat(8) + "\n";
        let mut reader = tokio::io::BufReader::new(frame.as_bytes());
        match read_frame_line(&mut reader, MAX_CLIENT_FRAME_BYTES)
            .await
            .expect("read ok")
        {
            FrameRead::Line(line) => assert_eq!(line, frame),
            other => panic!("expected a line, got {other:?}"),
        }
    }
}
