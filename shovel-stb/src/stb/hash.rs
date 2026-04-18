/// Bob Jenkins' `lookup3` (`hashlittle`) with a fixed seed of
/// `0x75BCD15` (123456789), as used by the Yacht Club / WayForward engine.

const HASH_SEED: u32 = 0x75BCD15; // 123456789

/// Compute the STB cell hash for a given string.
///
/// Empty strings hash to `0` (the game's convention).
pub fn stb_hash(s: &str) -> u32 {
    if s.is_empty() {
        return 0;
    }
    hashlittle(s.as_bytes(), HASH_SEED)
}

fn hashlittle(key: &[u8], initval: u32) -> u32 {
    let len = key.len();
    let mut a: u32;
    let mut b: u32;
    let mut c: u32;

    a = 0xdeadbeef_u32
        .wrapping_add(len as u32)
        .wrapping_add(initval);
    b = a;
    c = a;

    let mut offset = 0;
    let mut remaining = len;

    while remaining > 12 {
        a = a.wrapping_add(u32::from_le_bytes(
            key[offset..offset + 4].try_into().unwrap(),
        ));
        b = b.wrapping_add(u32::from_le_bytes(
            key[offset + 4..offset + 8].try_into().unwrap(),
        ));
        c = c.wrapping_add(u32::from_le_bytes(
            key[offset + 8..offset + 12].try_into().unwrap(),
        ));

        a = a.wrapping_sub(c);
        a ^= c.rotate_left(4);
        c = c.wrapping_add(b);
        b = b.wrapping_sub(a);
        b ^= a.rotate_left(6);
        a = a.wrapping_add(c);
        c = c.wrapping_sub(b);
        c ^= b.rotate_left(8);
        b = b.wrapping_add(a);
        a = a.wrapping_sub(c);
        a ^= c.rotate_left(16);
        c = c.wrapping_add(b);
        b = b.wrapping_sub(a);
        b ^= a.rotate_left(19);
        a = a.wrapping_add(c);
        c = c.wrapping_sub(b);
        c ^= b.rotate_left(4);
        b = b.wrapping_add(a);

        offset += 12;
        remaining -= 12;
    }

    let tail = &key[offset..];
    if remaining >= 12 {
        c = c.wrapping_add((tail[11] as u32) << 24);
    }
    if remaining >= 11 {
        c = c.wrapping_add((tail[10] as u32) << 16);
    }
    if remaining >= 10 {
        c = c.wrapping_add((tail[9] as u32) << 8);
    }
    if remaining >= 9 {
        c = c.wrapping_add(tail[8] as u32);
    }
    if remaining >= 8 {
        b = b.wrapping_add((tail[7] as u32) << 24);
    }
    if remaining >= 7 {
        b = b.wrapping_add((tail[6] as u32) << 16);
    }
    if remaining >= 6 {
        b = b.wrapping_add((tail[5] as u32) << 8);
    }
    if remaining >= 5 {
        b = b.wrapping_add(tail[4] as u32);
    }
    if remaining >= 4 {
        a = a.wrapping_add((tail[3] as u32) << 24);
    }
    if remaining >= 3 {
        a = a.wrapping_add((tail[2] as u32) << 16);
    }
    if remaining >= 2 {
        a = a.wrapping_add((tail[1] as u32) << 8);
    }
    if remaining >= 1 {
        a = a.wrapping_add(tail[0] as u32);
    }

    if remaining == 0 {
        return c;
    }

    c ^= b;
    c = c.wrapping_sub(b.rotate_left(14));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(11));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(25));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(16));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(4));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(14));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(24));

    c
}
