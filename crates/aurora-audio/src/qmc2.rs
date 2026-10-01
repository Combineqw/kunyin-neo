//! QQ Music QMC2 (mflac/mgg) streaming decryption.
//!
//! The decryptor is deliberately byte-offset based: a Range response can be
//! decrypted independently, and adjacent chunks produce the same bytes as a
//! single whole-file call.  This is the native counterpart of the Electron
//! protocol's existing TypeScript decryptor.

const V1_KEY_SIZE: usize = 128;
const V1_OFFSET_BOUNDARY: u64 = 0x7fff;
const FIRST_SEGMENT_SIZE: u64 = 0x80;
const OTHER_SEGMENT_SIZE: u64 = 0x1400;
const RC4_STREAM_CACHE_SIZE: usize = OTHER_SEGMENT_SIZE as usize + 512;
// The provider sends this prefix as base64/Latin-1 ASCII, matching the
// existing TypeScript decryptor's `Buffer.from(..., 'latin1')` comparison.
const EKEY_V2_PREFIX: &[u8] = b"UVFNdXNpYyBFbmNWMixLZXk6";
const EKEY_V2_KEY1: [u8; 16] = [
    0x33, 0x38, 0x36, 0x5a, 0x4a, 0x59, 0x21, 0x40, 0x23, 0x2a, 0x24, 0x25, 0x5e, 0x26, 0x29, 0x28,
];
const EKEY_V2_KEY2: [u8; 16] = [
    0x2a, 0x2a, 0x23, 0x21, 0x28, 0x23, 0x24, 0x25, 0x26, 0x5e, 0x61, 0x31, 0x63, 0x5a, 0x2c, 0x54,
];
const EKEY_SIMPLE_KEY: [u8; 8] = [105, 86, 70, 56, 43, 32, 21, 11];
const DELTA: u32 = 0x9e37_79b9;

#[derive(Debug, Clone)]
enum Cipher {
    Map {
        key: [u8; V1_KEY_SIZE],
    },
    Rc4 {
        key: Vec<u8>,
        hash: u32,
        stream: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub struct Qmc2Decryptor(Cipher);

impl Qmc2Decryptor {
    /// Decode an ekey and construct the corresponding QMC2 stream cipher.
    pub fn from_ekey(ekey: &str) -> Option<Self> {
        if ekey.len() < 12 {
            return None;
        }
        let encoded = ekey.as_bytes();
        let key = if encoded.len() > EKEY_V2_PREFIX.len() && encoded.starts_with(EKEY_V2_PREFIX) {
            ekey_decrypt_v2(&encoded[EKEY_V2_PREFIX.len()..])?
        } else {
            ekey_decrypt_v1(encoded)?
        };
        if key.is_empty() {
            return None;
        }
        Some(Self(if key.len() <= 300 {
            let mut compressed = [0_u8; V1_KEY_SIZE];
            for (i, slot) in compressed.iter_mut().enumerate() {
                let index = (i * i + 71_214) % key.len();
                let shift = ((index + 4) % 8) as u32;
                *slot = key[index].rotate_left(shift);
            }
            Cipher::Map { key: compressed }
        } else {
            let hash = rc4_hash(&key);
            let stream = rc4_init_stream(&key, RC4_STREAM_CACHE_SIZE);
            Cipher::Rc4 { key, hash, stream }
        }))
    }

    /// Decrypt bytes whose first byte has `file_offset` in the encrypted file.
    pub fn decrypt(&self, input: &[u8], file_offset: u64) -> Vec<u8> {
        let mut out = input.to_vec();
        match &self.0 {
            Cipher::Map { key } => {
                for (index, byte) in out.iter_mut().enumerate() {
                    let mut offset = file_offset.saturating_add(index as u64);
                    if offset > V1_OFFSET_BOUNDARY {
                        offset %= V1_OFFSET_BOUNDARY;
                    }
                    *byte ^= key[(offset as usize) % V1_KEY_SIZE];
                }
            }
            Cipher::Rc4 { key, hash, stream } => {
                decrypt_rc4(&mut out, file_offset, key, *hash, stream)
            }
        }
        out
    }
}

pub fn decrypt_qmc2_chunk(ekey: &str, file_offset: u64, input: &[u8]) -> Option<Vec<u8>> {
    Some(Qmc2Decryptor::from_ekey(ekey)?.decrypt(input, file_offset))
}

fn b2i(input: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        input[offset],
        input[offset + 1],
        input[offset + 2],
        input[offset + 3],
    ])
}

fn tea_decrypt_block(block: &mut [u8], offset: usize, key: &[u32; 4]) {
    let mut y = b2i(block, offset);
    let mut z = b2i(block, offset + 4);
    let mut sum = DELTA.wrapping_mul(16);
    for _ in 0..16 {
        z = z.wrapping_sub(
            (y << 4).wrapping_add(key[2]) ^ y.wrapping_add(sum) ^ ((y >> 5).wrapping_add(key[3])),
        );
        y = y.wrapping_sub(
            (z << 4).wrapping_add(key[0]) ^ z.wrapping_add(sum) ^ ((z >> 5).wrapping_add(key[1])),
        );
        sum = sum.wrapping_sub(DELTA);
    }
    block[offset..offset + 4].copy_from_slice(&y.to_be_bytes());
    block[offset + 4..offset + 8].copy_from_slice(&z.to_be_bytes());
}

fn tea_decrypt(cipher: &[u8], key_bytes: &[u8; 16]) -> Option<Vec<u8>> {
    if cipher.len() < 16 || cipher.len() % 8 != 0 {
        return None;
    }
    let key = [
        b2i(key_bytes, 0),
        b2i(key_bytes, 4),
        b2i(key_bytes, 8),
        b2i(key_bytes, 12),
    ];
    let mut decoded = cipher.to_vec();
    let mut iv1 = [0_u8; 8];
    let mut iv2 = [0_u8; 8];
    for offset in (0..decoded.len()).step_by(8) {
        let next_iv1: [u8; 8] = decoded[offset..offset + 8].try_into().ok()?;
        for i in 0..8 {
            decoded[offset + i] ^= iv2[i];
        }
        tea_decrypt_block(&mut decoded, offset, &key);
        iv2.copy_from_slice(&decoded[offset..offset + 8]);
        for i in 0..8 {
            decoded[offset + i] ^= iv1[i];
        }
        iv1 = next_iv1;
    }
    let header = 3 + usize::from(decoded[0] & 7);
    if header + 7 > decoded.len() {
        return None;
    }
    Some(decoded[header..decoded.len() - 7].to_vec())
}

fn base64_decode(input: &[u8]) -> Option<Vec<u8>> {
    let mut values = Vec::with_capacity(input.len());
    for &byte in input {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => return None,
        };
        values.push(value);
    }
    if values.len() < 2 {
        return None;
    }
    let mut out = Vec::with_capacity(values.len() * 3 / 4);
    let mut index = 0;
    while index + 1 < values.len() {
        out.push((values[index] << 2) | (values[index + 1] >> 4));
        if index + 2 < values.len() {
            out.push((values[index + 1] << 4) | (values[index + 2] >> 2));
        }
        if index + 3 < values.len() {
            out.push((values[index + 2] << 6) | values[index + 3]);
        }
        index += 4;
    }
    Some(out)
}

fn ekey_decrypt_v1(encoded: &[u8]) -> Option<Vec<u8>> {
    let raw = base64_decode(encoded)?;
    if raw.len() < 12 {
        return None;
    }
    let mut tea_key = [0_u8; 16];
    for i in 0..8 {
        tea_key[i * 2] = EKEY_SIMPLE_KEY[i];
        tea_key[i * 2 + 1] = raw[i];
    }
    let payload = tea_decrypt(&raw[8..], &tea_key)?;
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&raw[..8]);
    out.extend_from_slice(&payload);
    Some(out)
}

fn ekey_decrypt_v2(encoded: &[u8]) -> Option<Vec<u8>> {
    let raw = base64_decode(encoded)?;
    let first = tea_decrypt(&raw, &EKEY_V2_KEY1)?;
    let second = tea_decrypt(&first, &EKEY_V2_KEY2)?;
    let end = second
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(second.len());
    ekey_decrypt_v1(&second[..end])
}

fn rc4_hash(key: &[u8]) -> u32 {
    let mut hash = 1_u32;
    for &byte in key {
        if byte == 0 {
            continue;
        }
        let next = hash.wrapping_mul(u32::from(byte));
        if next == 0 || next <= hash {
            break;
        }
        hash = next;
    }
    hash
}

fn rc4_segment_key(id: u64, seed: u8, hash: u32) -> u64 {
    if seed == 0 {
        return 0;
    }
    // The TypeScript implementation keeps this as a Number. The value can
    // exceed u32 for a small segment id/seed, so preserve it until the caller
    // applies the cipher's modulo or mask.
    ((f64::from(hash) / ((id + 1) as f64 * f64::from(seed))) * 100.0).floor() as u64
}

fn rc4_init_stream(key: &[u8], output_len: usize) -> Vec<u8> {
    let mut state: Vec<u8> = (0..key.len()).map(|value| value as u8).collect();
    let mut j = 0;
    for i in 0..key.len() {
        j = (j + usize::from(state[i]) + usize::from(key[i % key.len()])) % key.len();
        state.swap(i, j);
    }
    let mut output = vec![0_u8; output_len];
    let (mut si, mut sj) = (0, 0);
    for slot in &mut output {
        si = (si + 1) % key.len();
        sj = (sj + usize::from(state[si])) % key.len();
        state.swap(si, sj);
        *slot = state[(usize::from(state[si]) + usize::from(state[sj])) % key.len()];
    }
    output
}

fn decrypt_rc4(output: &mut [u8], file_offset: u64, key: &[u8], hash: u32, stream: &[u8]) {
    let mut position = 0;
    let mut offset = file_offset;
    if offset < FIRST_SEGMENT_SIZE {
        let count = output.len().min((FIRST_SEGMENT_SIZE - offset) as usize);
        for (index, byte) in output[..count].iter_mut().enumerate() {
            let absolute = offset + index as u64;
            let seed = key[(absolute as usize) % key.len()];
            let index = (rc4_segment_key(absolute, seed, hash) % key.len() as u64) as usize;
            *byte ^= key[index];
        }
        position += count;
        offset += count as u64;
    }
    let excess = offset % OTHER_SEGMENT_SIZE;
    if position < output.len() && excess != 0 {
        let count = (output.len() - position).min((OTHER_SEGMENT_SIZE - excess) as usize);
        let id = offset / OTHER_SEGMENT_SIZE;
        let seed = key[(id as usize) % key.len()];
        let skip = (rc4_segment_key(id, seed, hash) & 0x1ff) as usize;
        for (index, byte) in output[position..position + count].iter_mut().enumerate() {
            *byte ^= stream[skip + excess as usize + index];
        }
        position += count;
        offset += count as u64;
    }
    while position < output.len() {
        let count = (output.len() - position).min(OTHER_SEGMENT_SIZE as usize);
        let id = offset / OTHER_SEGMENT_SIZE;
        let seed = key[(id as usize) % key.len()];
        let skip = (rc4_segment_key(id, seed, hash) & 0x1ff) as usize;
        for (index, byte) in output[position..position + count].iter_mut().enumerate() {
            *byte ^= stream[skip + index];
        }
        position += count;
        offset += count as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_for_key(raw: &[u8]) -> Qmc2Decryptor {
        let mut compressed = [0_u8; V1_KEY_SIZE];
        for (i, slot) in compressed.iter_mut().enumerate() {
            let index = (i * i + 71_214) % raw.len();
            *slot = raw[index].rotate_left(((index + 4) % 8) as u32);
        }
        Qmc2Decryptor(Cipher::Map { key: compressed })
    }

    fn rc4_for_key(raw: &[u8]) -> Qmc2Decryptor {
        Qmc2Decryptor(Cipher::Rc4 {
            key: raw.to_vec(),
            hash: rc4_hash(raw),
            stream: rc4_init_stream(raw, RC4_STREAM_CACHE_SIZE),
        })
    }

    #[test]
    fn map_decryption_is_stable_across_range_chunks() {
        let cipher = map_for_key(b"native-qmc2-test-key");
        let plain: Vec<u8> = (0..4096).map(|value| value as u8).collect();
        let encrypted = cipher.decrypt(&plain, 0);
        let whole = cipher.decrypt(&encrypted, 0);
        assert_eq!(whole, plain);
        let mut joined = Vec::new();
        for (offset, size) in [(0, 17), (17, 128), (145, 409), (554, 3542)] {
            joined.extend(cipher.decrypt(&encrypted[offset..offset + size], offset as u64));
        }
        assert_eq!(joined, plain);
    }

    #[test]
    fn malformed_ekey_is_rejected() {
        assert!(Qmc2Decryptor::from_ekey("not-an-ekey").is_none());
    }

    #[test]
    fn rc4_decryption_is_stable_across_range_chunks() {
        let mut raw = vec![0_u8; 301];
        for (index, byte) in raw.iter_mut().enumerate() {
            *byte = ((index * 17 + 3) % 251) as u8;
        }
        let cipher = rc4_for_key(&raw);
        let plain: Vec<u8> = (0..10_000).map(|value| (value % 251) as u8).collect();
        let encrypted = cipher.decrypt(&plain, 0);
        assert_eq!(cipher.decrypt(&encrypted, 0), plain);

        let mut joined = Vec::new();
        for (offset, size) in [(0, 17), (17, 128), (145, 409), (554, 3542), (4096, 5904)] {
            joined.extend(cipher.decrypt(&encrypted[offset..offset + size], offset as u64));
        }
        assert_eq!(joined, plain);
    }

    #[test]
    fn rc4_segment_key_keeps_large_number_range() {
        let value = rc4_segment_key(0, 1, u32::MAX);
        assert!(value > u64::from(u32::MAX));
    }
}
