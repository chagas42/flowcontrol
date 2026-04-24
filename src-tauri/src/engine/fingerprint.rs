//! Pair fingerprint derived from the TCP handshake's two nonces.
//!
//! Both peers compute identical bytes by concatenating
//! `(client_nonce, server_nonce)` in the same order before hashing.
//! Guards against typing the wrong peer on the same LAN — not an MITM defense.

use blake3::Hasher;

/// Derive a 5-byte fingerprint from the TCP connection's shared nonces.
/// Both peers MUST call this with the same (client_nonce, server_nonce)
/// ordering to produce the same bytes.
pub fn derive(client_nonce: &[u8; 16], server_nonce: &[u8; 16]) -> [u8; 5] {
    let mut h = Hasher::new();
    h.update(b"flowcontrol-pair-v1");
    h.update(client_nonce);
    h.update(server_nonce);
    let out = h.finalize();
    let mut fp = [0u8; 5];
    fp.copy_from_slice(&out.as_bytes()[..5]);
    fp
}

/// Render "23-AB-45-CD" style string for the UI.
/// Crockford-ish alphabet: no I/L/O/0/1 to avoid ambiguity.
pub fn render(fp: &[u8; 5]) -> String {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
    let mut out = String::with_capacity(11);
    for (i, chunk) in fp.chunks(2).enumerate() {
        if i > 0 {
            out.push('-');
        }
        let n = if chunk.len() == 2 {
            ((chunk[0] as u16) << 8) | chunk[1] as u16
        } else {
            (chunk[0] as u16) << 8
        };
        out.push(ALPHABET[((n >> 11) & 0x1F) as usize] as char);
        out.push(ALPHABET[((n >> 6) & 0x1F) as usize] as char);
        if chunk.len() == 2 {
            out.push(ALPHABET[((n >> 1) & 0x1F) as usize] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_zero_bytes_produces_three_groups() {
        // 5 bytes chunk-by-2 → (2,2,1). Chunks of len 2 emit 3 chars each;
        // the trailing 1-byte chunk emits 2. Total: 3+3+2 = 8 chars + 2
        // separators = 10 chars. (The spec's `with_capacity(11)` is an
        // off-by-one — the function overallocates by 1.)
        let s = render(&[0u8; 5]);
        assert_eq!(s.len(), 10);
        assert_eq!(s.matches('-').count(), 2);
    }

    #[test]
    fn same_nonces_yield_same_fingerprint() {
        let client = [1u8; 16];
        let server = [2u8; 16];
        assert_eq!(derive(&client, &server), derive(&client, &server));
    }

    #[test]
    fn nonce_order_matters() {
        let a = [1u8; 16];
        let b = [2u8; 16];
        // The whole point of the protocol: both peers must use the same
        // (client, server) ordering; swapping produces a different fingerprint.
        assert_ne!(derive(&a, &b), derive(&b, &a));
    }
}
