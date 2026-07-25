mod catalog;
mod music;
mod prng;
mod runtime;
mod sfx;

pub use catalog::*;
pub use music::*;
pub use puzzle_audio_contract::*;
pub use runtime::*;
pub use sfx::*;

/// Returns the deterministic six-digit seed used by editor random-preset
/// authoring. This keeps the JavaScript-compatible UTF-16 hash and Mulberry32
/// sequence owned beside the synthesis generators.
pub fn random_audio_preset_seed(seed: &str) -> String {
    prng::Mulberry32::from_text(seed)
        .int_inclusive(100_000, 999_999)
        .to_string()
}

#[cfg(test)]
mod preset_tests {
    #[test]
    fn random_preset_seed_matches_the_javascript_generator_golden() {
        assert_eq!(super::random_audio_preset_seed("123456"), "689390");
    }
}
