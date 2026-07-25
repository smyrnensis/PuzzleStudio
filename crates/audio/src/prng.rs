//! Deterministic random primitives shared by audio generators.
//!
//! Seed hashing deliberately follows JavaScript string semantics: FNV-1a is
//! applied to UTF-16 code units, not UTF-8 bytes.

pub(crate) fn fnv1a_utf16(text: &str) -> u32 {
    text.encode_utf16().fold(2_166_136_261_u32, |hash, unit| {
        (hash ^ u32::from(unit)).wrapping_mul(16_777_619)
    })
}

#[derive(Clone, Debug)]
pub(crate) struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    pub(crate) fn from_text(seed: &str) -> Self {
        Self {
            state: fnv1a_utf16(seed),
        }
    }

    pub(crate) fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut value = self.state;
        value = (value ^ (value >> 15)).wrapping_mul(value | 1);
        value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
        value ^ (value >> 14)
    }

    pub(crate) fn uniform(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }

    pub(crate) fn int_inclusive(&mut self, min: u32, max: u32) -> u32 {
        min + (self.uniform() * f64::from(max - min + 1)).floor() as u32
    }

    pub(crate) fn index(&mut self, len: usize) -> usize {
        (self.uniform() * len as f64).floor() as usize
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Rc4 {
    state: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    pub(crate) fn from_js_integer(seed: i32) -> Self {
        let key = seed.to_string().into_bytes();
        let mut state = [0_u8; 256];
        for (index, value) in state.iter_mut().enumerate() {
            *value = index as u8;
        }
        let mut j = 0_u8;
        for i in 0..256 {
            j = j.wrapping_add(state[i]).wrapping_add(key[i % key.len()]);
            state.swap(i, usize::from(j));
        }
        Self { state, i: 0, j: 0 }
    }

    fn next_byte(&mut self) -> u8 {
        self.i = self.i.wrapping_add(1);
        self.j = self.j.wrapping_add(self.state[usize::from(self.i)]);
        self.state.swap(usize::from(self.i), usize::from(self.j));
        let index = self.state[usize::from(self.i)].wrapping_add(self.state[usize::from(self.j)]);
        self.state[usize::from(index)]
    }

    pub(crate) fn uniform_56(&mut self) -> f64 {
        let mut output = 0_f64;
        for _ in 0..7 {
            output = output * 256.0 + f64::from(self.next_byte());
        }
        output / (2_f64.powi(56) - 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_javascript_utf16_fnv_and_mulberry_vectors() {
        assert_eq!(fnv1a_utf16("123456"), 2_576_725_674);
        let mut ascii = Mulberry32::from_text("123456");
        assert_eq!(
            (0..5).map(|_| ascii.next_u32()).collect::<Vec<_>>(),
            [
                2_812_683_243,
                3_838_267_589,
                2_672_882_843,
                590_295_174,
                2_961_239_792
            ]
        );

        assert_eq!(fnv1a_utf16("🎵"), 1_143_895_498);
        let mut non_bmp = Mulberry32::from_text("🎵");
        assert_eq!(
            (0..5).map(|_| non_bmp.next_u32()).collect::<Vec<_>>(),
            [
                2_506_370_590,
                668_106_349,
                920_797_650,
                816_920_876,
                3_054_525_191
            ]
        );
    }
}
