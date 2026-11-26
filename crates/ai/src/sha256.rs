//! SHA-256, because a model store that trusts a filename is not a store.
//!
//! Written out here rather than pulled in: it is eighty lines, every vector in the specification's
//! appendix is a test below, and a dependency that exists only to hash a model file would be a
//! supply-chain decision this project does not need to make. It is **not** a general-purpose
//! cryptography library and is used only to compare a downloaded file against a digest that shipped
//! with the manifest.

/// A digest, as bytes.
pub type Digest = [u8; 32];

/// The SHA-256 round constants: the first 32 bits of the fractional parts of the cube roots of the
/// first 64 primes.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be,
    0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa,
    0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85,
    0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f,
    0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The initial hash state: the first 32 bits of the fractional parts of the square roots of the first
/// eight primes.
const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// An incremental SHA-256, for files too large to read whole.
///
/// A 166 MB model does not belong in RAM just to be hashed. `update` as many chunks as come, then
/// `finish` pads and closes the stream. The answer is the one `digest` gives over the same bytes
/// concatenated, which a test pins.
#[derive(Debug, Clone)]
pub struct Hasher {
    state: [u32; 8],
    /// Bytes not yet compressed, at most one block.
    tail: [u8; 64],
    /// How many bytes of `tail` are real.
    filled: usize,
    /// Total bytes fed, before padding.
    length: u64,
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher {
    /// A fresh hash.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: H0,
            tail: [0u8; 64],
            filled: 0,
            length: 0,
        }
    }

    /// Feed more bytes.
    pub fn update(&mut self, data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        let mut data = data;
        if self.filled > 0 {
            let take = (64 - self.filled).min(data.len());
            self.tail[self.filled..self.filled + take].copy_from_slice(&data[..take]);
            self.filled += take;
            data = &data[take..];
            if self.filled == 64 {
                let block = self.tail;
                compress(&mut self.state, &block);
                self.filled = 0;
            }
        }
        let (chunks, rest) = data.as_chunks::<64>();
        for chunk in chunks {
            let mut block = [0u8; 64];
            block.copy_from_slice(chunk);
            compress(&mut self.state, &block);
        }
        if self.filled == 0 {
            self.tail[..rest.len()].copy_from_slice(rest);
            self.filled = rest.len();
        }
    }

    /// Close the stream: padding, the bit length, and the digest.
    #[must_use]
    pub fn finish(mut self) -> Digest {
        let rest = self.filled;
        let mut tail = [0u8; 128];
        tail[..rest].copy_from_slice(&self.tail[..rest]);
        tail[rest] = 0x80;
        let tail_blocks = if rest + 1 + 8 <= 64 { 1 } else { 2 };
        let bits = self.length.wrapping_mul(8);
        let length_at = tail_blocks * 64 - 8;
        tail[length_at..length_at + 8].copy_from_slice(&bits.to_be_bytes());
        for index in 0..tail_blocks {
            let mut block = [0u8; 64];
            block.copy_from_slice(&tail[index * 64..index * 64 + 64]);
            compress(&mut self.state, &block);
        }
        let mut out = [0u8; 32];
        for (index, word) in self.state.iter().enumerate() {
            out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

/// Hash `data`.
#[must_use]
pub fn digest(data: &[u8]) -> Digest {
    let mut state = H0;
    let mut block = [0u8; 64];

    // Every full block, straight through.
    let (chunks, rest) = data.as_chunks::<64>();
    for chunk in chunks {
        block.copy_from_slice(chunk);
        compress(&mut state, &block);
    }

    // The tail: what is left, a 1 bit, zeros, and the bit length as a big-endian u64.
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let tail_blocks = if rest.len() + 1 + 8 <= 64 { 1 } else { 2 };
    let bits = (data.len() as u64).wrapping_mul(8);
    let length_at = tail_blocks * 64 - 8;
    tail[length_at..length_at + 8].copy_from_slice(&bits.to_be_bytes());
    for index in 0..tail_blocks {
        block.copy_from_slice(&tail[index * 64..index * 64 + 64]);
        compress(&mut state, &block);
    }

    let mut out = [0u8; 32];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// One 64-byte block through the compression function.
fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for index in 0..16 {
        w[index] = u32::from_be_bytes([block[index * 4], block[index * 4 + 1], block[index * 4 + 2], block[index * 4 + 3]]);
    }
    for index in 16..64 {
        let s0 = w[index - 15].rotate_right(7) ^ w[index - 15].rotate_right(18) ^ (w[index - 15] >> 3);
        let s1 = w[index - 2].rotate_right(17) ^ w[index - 2].rotate_right(19) ^ (w[index - 2] >> 10);
        w[index] = w[index - 16].wrapping_add(s0).wrapping_add(w[index - 7]).wrapping_add(s1);
    }

    let mut h = *state;
    for index in 0..64 {
        let s1 = h[4].rotate_right(6) ^ h[4].rotate_right(11) ^ h[4].rotate_right(25);
        let choose = (h[4] & h[5]) ^ (!h[4] & h[6]);
        let temp1 = h[7]
            .wrapping_add(s1)
            .wrapping_add(choose)
            .wrapping_add(K[index])
            .wrapping_add(w[index]);
        let s0 = h[0].rotate_right(2) ^ h[0].rotate_right(13) ^ h[0].rotate_right(22);
        let majority = (h[0] & h[1]) ^ (h[0] & h[2]) ^ (h[1] & h[2]);
        let temp2 = s0.wrapping_add(majority);

        h[7] = h[6];
        h[6] = h[5];
        h[5] = h[4];
        h[4] = h[3].wrapping_add(temp1);
        h[3] = h[2];
        h[2] = h[1];
        h[1] = h[0];
        h[0] = temp1.wrapping_add(temp2);
    }
    for index in 0..8 {
        state[index] = state[index].wrapping_add(h[index]);
    }
}

/// Lowercase hex, which is how a manifest writes a digest.
#[must_use]
pub fn to_hex(digest: &Digest) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    out
}

/// Parse a hex digest. Uppercase is accepted; anything that is not 64 hex digits is not.
#[must_use]
pub fn from_hex(text: &str) -> Option<Digest> {
    let trimmed = text.trim();
    if trimmed.len() != 64 || !trimmed.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    let bytes = trimmed.as_bytes();
    for index in 0..32 {
        let high = char::from(bytes[index * 2]).to_digit(16)?;
        let low = char::from(bytes[index * 2 + 1]).to_digit(16)?;
        out[index] = ((high << 4) | low) as u8;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_of(data: &[u8]) -> String {
        to_hex(&digest(data))
    }

    #[test]
    fn the_fips_vectors_hold() {
        // The three vectors every implementation is checked against.
        assert_eq!(hex_of(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex_of(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            hex_of(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn the_block_boundaries_are_where_the_padding_changes() {
        // 55, 56 and 64 bytes are the three cases: padding fits, padding spills into a second block,
        // and a perfectly aligned block that still needs a whole extra block for the length.
        assert_eq!(
            hex_of(&[b'a'; 55]),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            hex_of(&[b'a'; 56]),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            hex_of(&[b'a'; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
        assert_eq!(
            hex_of(&[b'a'; 1000]),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn a_digest_survives_a_round_trip_through_hex() {
        let digest = digest(b"sortlens");
        let text = to_hex(&digest);
        assert_eq!(text.len(), 64);
        assert_eq!(from_hex(&text), Some(digest));
        assert_eq!(from_hex(&text.to_uppercase()), Some(digest), "uppercase is still hex");
        assert_eq!(from_hex(&text[..63]), None, "a short digest is not a digest");
        assert_eq!(from_hex("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"), None);
        assert_eq!(from_hex(""), None);
    }

    #[test]
    fn one_flipped_bit_changes_everything() {
        let mut data = vec![7u8; 100];
        let before = digest(&data);
        data[99] ^= 1;
        let after = digest(&data);
        assert_ne!(before, after);
        let differences = before.iter().zip(after.iter()).filter(|(a, b)| a != b).count();
        assert!(differences > 20, "a changed digest looks changed: {differences} bytes differ");
    }

    /// An incremental hash is the same hash, fed a few bytes at a time — the boundary cases land
    /// on 55/56/64/65, where the padding moves.
    #[test]
    fn the_incremental_hasher_matches_the_one_shot() {
        for length in [0usize, 1, 55, 56, 63, 64, 65, 127, 128, 129, 1000] {
            let data: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            for chunk in [1usize, 7, 64, 333] {
                let mut hasher = Hasher::new();
                for piece in data.chunks(chunk.max(1)) {
                    hasher.update(piece);
                }
                assert_eq!(hasher.finish(), digest(&data), "length {length} in chunks of {chunk}");
            }
        }
    }

    #[test]
    fn a_hasher_swallows_an_empty_update() {
        let mut hasher = Hasher::new();
        hasher.update(b"abc");
        hasher.update(b"");
        assert_eq!(hasher.finish(), digest(b"abc"));
    }
}
