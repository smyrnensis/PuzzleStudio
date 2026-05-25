pub(crate) fn fnv_seed() -> u64 {
    0xcbf29ce484222325
}

pub(crate) fn fnv_mix(hash: u64, value: u64) -> u64 {
    hash.wrapping_mul(0x100000001b3) ^ value
}
