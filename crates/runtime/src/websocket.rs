//! RFC 6455 WebSocket Implementation
//! Includes pure-Rust SHA-1, Base64 encoding, RFC 6455 Handshake computation,
//! client-to-server XOR unmasking, and server-to-client frame formatting.
#![allow(clippy::needless_range_loop, clippy::type_complexity)]

use std::io::{Read, Write};

/// RFC 6455 Magic GUID for WebSocket Handshake (Section 4.2.2)
pub const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Compute pure-Rust RFC 3174 SHA-1 hash of `data`.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let bit_len = (data.len() as u64) * 8;

    // Pre-processing (padding)
    let mut msg = Vec::with_capacity(data.len() + 72);
    msg.extend_from_slice(data);
    msg.push(0x80);

    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit (64-byte) chunk
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            let offset = i * 4;
            w[i] = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut result = [0u8; 20];
    result[0..4].copy_from_slice(&h0.to_be_bytes());
    result[4..8].copy_from_slice(&h1.to_be_bytes());
    result[8..12].copy_from_slice(&h2.to_be_bytes());
    result[12..16].copy_from_slice(&h3.to_be_bytes());
    result[16..20].copy_from_slice(&h4.to_be_bytes());
    result
}

/// Standard Base64 encoder (RFC 4648).
pub fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(data.len().div_ceil(3) * 4);

    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as usize
        } else {
            0
        };

        output.push(CHARSET[(b0 >> 2) & 0x3F] as char);
        output.push(CHARSET[((b0 & 0x03) << 4) | ((b1 >> 4) & 0x0F)] as char);

        if i + 1 < data.len() {
            output.push(CHARSET[((b1 & 0x0F) << 2) | ((b2 >> 6) & 0x03)] as char);
        } else {
            output.push('=');
        }

        if i + 2 < data.len() {
            output.push(CHARSET[b2 & 0x3F] as char);
        } else {
            output.push('=');
        }

        i += 3;
    }

    output
}

/// Compute standard RFC 6455 `Sec-WebSocket-Accept` header from client's `Sec-WebSocket-Key`.
pub fn compute_websocket_accept(client_key: &str) -> String {
    let mut combined = String::with_capacity(client_key.trim().len() + WS_GUID.len());
    combined.push_str(client_key.trim());
    combined.push_str(WS_GUID);
    let hash = sha1(combined.as_bytes());
    base64_encode(&hash)
}

/// Opcode definitions for RFC 6455
pub mod opcodes {
    pub const CONTINUATION: u8 = 0x0;
    pub const TEXT: u8 = 0x1;
    pub const BINARY: u8 = 0x2;
    pub const CLOSE: u8 = 0x8;
    pub const PING: u8 = 0x9;
    pub const PONG: u8 = 0xA;
}

/// Read a single RFC 6455 WebSocket frame from a stream, decoding client XOR masking,
/// handling Ping/Pong heartbeats transparently, and returning the unmasked text payload.
pub fn read_websocket_frame<R: Read + Write>(stream: &mut R) -> std::io::Result<Option<String>> {
    loop {
        let mut header = [0u8; 2];
        match stream.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }

        let _fin = (header[0] & 0x80) != 0;
        let opcode = header[0] & 0x0F;
        let masked = (header[1] & 0x80) != 0;
        let mut payload_len = (header[1] & 0x7F) as usize;

        if payload_len == 126 {
            let mut ext = [0u8; 2];
            stream.read_exact(&mut ext)?;
            payload_len = u16::from_be_bytes(ext) as usize;
        } else if payload_len == 127 {
            let mut ext = [0u8; 8];
            stream.read_exact(&mut ext)?;
            payload_len = u64::from_be_bytes(ext) as usize;
        }

        let mask = if masked {
            let mut m = [0u8; 4];
            stream.read_exact(&mut m)?;
            Some(m)
        } else {
            None
        };

        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            stream.read_exact(&mut payload)?;
        }

        // Apply XOR unmasking if masked (mandatory for client-to-server frames per RFC 6455 §5.1)
        if let Some(mask_bytes) = mask {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask_bytes[i % 4];
            }
        }

        match opcode {
            opcodes::PING => {
                // Send Pong reply immediately (unmasked server frame) per RFC 6455 §5.5.3
                let pong_header = [0x8A, payload.len() as u8];
                stream.write_all(&pong_header)?;
                if !payload.is_empty() {
                    stream.write_all(&payload)?;
                }
                stream.flush()?;
                // Loop to read next data frame
                continue;
            }
            opcodes::PONG => {
                // Ignore Pong response and read next frame
                continue;
            }
            opcodes::CLOSE => {
                // Respond with Close frame acknowledgment per RFC 6455 §5.5.1
                let close_reply = [0x88, 0x00];
                let _ = stream.write_all(&close_reply);
                let _ = stream.flush();
                return Ok(None);
            }
            opcodes::TEXT | opcodes::CONTINUATION | opcodes::BINARY => {
                let text = String::from_utf8_lossy(&payload).into_owned();
                return Ok(Some(text));
            }
            _ => {
                // Unknown opcode, ignore or return empty
                continue;
            }
        }
    }
}

/// Write an RFC 6455 WebSocket Text frame to a stream (unmasked as required for server-to-client).
pub fn write_websocket_frame<W: Write>(stream: &mut W, text: &str) -> std::io::Result<usize> {
    let payload = text.as_bytes();
    let len = payload.len();

    let mut header = Vec::with_capacity(10);
    // FIN = 1, Opcode = 0x1 (Text)
    header.push(0x81);

    if len < 126 {
        // Mask bit is 0, payload len
        header.push(len as u8);
    } else if len <= 0xFFFF {
        header.push(126);
        header.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        header.push(127);
        header.extend_from_slice(&(len as u64).to_be_bytes());
    }

    stream.write_all(&header)?;
    stream.write_all(payload)?;
    stream.flush()?;
    Ok(header.len() + len)
}

/// Write an RFC 6455 WebSocket Text frame with client XOR masking (required for client-to-server per RFC 6455 §5.1).
pub fn write_websocket_client_frame<W: Write>(
    stream: &mut W,
    text: &str,
) -> std::io::Result<usize> {
    let payload = text.as_bytes();
    let len = payload.len();

    let mut header = Vec::with_capacity(14);
    // FIN = 1, Opcode = 0x1 (Text)
    header.push(0x81);

    // Client MUST set MASK bit (0x80)
    if len < 126 {
        header.push(0x80 | (len as u8));
    } else if len <= 0xFFFF {
        header.push(0x80 | 126);
        header.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        header.push(0x80 | 127);
        header.extend_from_slice(&(len as u64).to_be_bytes());
    }

    // 4-byte masking key
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mask: [u8; 4] = [
        ((ts >> 24) ^ 0x37) as u8,
        ((ts >> 16) ^ 0xFA) as u8,
        ((ts >> 8) ^ 0x21) as u8,
        (ts ^ 0x3D) as u8,
    ];
    header.extend_from_slice(&mask);

    // XOR mask the payload
    let mut masked_payload = Vec::with_capacity(len);
    for (i, byte) in payload.iter().enumerate() {
        masked_payload.push(byte ^ mask[i % 4]);
    }

    stream.write_all(&header)?;
    stream.write_all(&masked_payload)?;
    stream.flush()?;
    Ok(header.len() + len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_rfc6455_handshake_vector() {
        // Standard RFC 6455 Section 1.3 / 4.2.2 test vector
        let client_key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = compute_websocket_accept(client_key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn test_sha1_known_hashes() {
        let hash = sha1(b"abc");
        let hex = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_client_xor_unmasking() {
        // Standard RFC 6455 masked text frame example:
        // Text: "Hello"
        // Mask: 0x37 0xfa 0x21 0x3d
        // Masked: 0x7f 0x9f 0x4d 0x51 0x58
        let mut frame_data = vec![
            0x81, // FIN=1, Opcode=0x1
            0x85, // Mask=1, Len=5
            0x37, 0xfa, 0x21, 0x3d, // Masking key
            0x7f, 0x9f, 0x4d, 0x51, 0x58, // Masked "Hello"
        ];
        let mut cursor = Cursor::new(&mut frame_data);
        let decoded = read_websocket_frame(&mut cursor).unwrap();
        assert_eq!(decoded, Some("Hello".to_string()));
    }

    #[test]
    fn test_server_unmasked_frame_write() {
        let mut buffer = Vec::new();
        write_websocket_frame(&mut buffer, "Hello").unwrap();

        // Server frame MUST be unmasked:
        // Byte 0: 0x81 (FIN=1, Text)
        // Byte 1: 0x05 (Mask=0, Len=5)
        // Payload: "Hello"
        assert_eq!(buffer, vec![0x81, 0x05, b'H', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn test_client_masked_frame_write_and_read() {
        let mut buffer = Vec::new();
        write_websocket_client_frame(&mut buffer, "Hello, Syntra!").unwrap();

        // Byte 0: 0x81
        assert_eq!(buffer[0], 0x81);
        // Byte 1: Mask bit (0x80) set
        assert_eq!(buffer[1] & 0x80, 0x80);

        // Read and unmask using read_websocket_frame
        let mut cursor = Cursor::new(&mut buffer);
        let decoded = read_websocket_frame(&mut cursor).unwrap();
        assert_eq!(decoded, Some("Hello, Syntra!".to_string()));
    }
}
