//! `RND` / `RANDOMIZE`: the QBasic 24-bit linear congruential generator.
//!
//! Replaces `func_rnd`, `sub_randomize` and the `rnd_seed` globals of
//! libqb.cpp. The error check (`new_error`) and the interactive
//! "Random-number seed" prompt stay in the C++ shims.

/// Seed a fresh program (and `RUN`) starts with.
pub const INITIAL_SEED: u32 = 327_680;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RndState {
    seed: u32,
    /// Seed at program start; `RANDOMIZE USING` keeps its low byte.
    seed_first: u32,
}

impl Default for RndState {
    fn default() -> Self {
        Self { seed: INITIAL_SEED, seed_first: INITIAL_SEED }
    }
}

impl RndState {
    /// `RND` (argument omitted: `n = None`) or `RND(n)`.
    pub fn rnd(&mut self, n: Option<f32>) -> f32 {
        let n = n.unwrap_or(1.0);
        // NaN compares unequal to 0, so it advances like a positive argument.
        if n != 0.0 {
            if n < 0.0 {
                let m = n.to_bits();
                self.seed = (m & 0x00FF_FFFF) + ((m & 0xFF00_0000) >> 24);
            }
            self.seed = self.seed.wrapping_mul(16_598_013).wrapping_add(12_820_163) & 0x00FF_FFFF;
        }
        // seed < 2^24, so the quotient is exact in f32.
        self.seed as f32 / 16_777_216.0
    }

    /// `RANDOMIZE seed`.
    pub fn randomize(&mut self, seed: f64) {
        self.seed = mix_seed(seed) | (self.seed & 0xFF);
    }

    /// `RANDOMIZE USING seed`: restarts a reproducible sequence.
    pub fn randomize_using(&mut self, seed: f64) {
        self.seed = mix_seed(seed) | (self.seed_first & 0xFF);
    }

    /// Seed typed at the `RANDOMIZE` prompt.
    pub fn randomize_prompt(&mut self, seed: i16) {
        self.seed = ((seed as u16 as u32) << 8) | (self.seed & 0xFF);
    }
}

/// High 32 bits of the double, folded to 16 bits, placed in bits 8..24.
fn mix_seed(seed: f64) -> u32 {
    let mut m = (seed.to_bits() >> 32) as u32;
    m ^= m >> 16;
    (m & 0xFFFF) << 8
}

// --- C ABI (include/qb_runtime.h); one generator per program ----------------------

static GLOBAL: std::sync::Mutex<RndState> =
    std::sync::Mutex::new(RndState { seed: INITIAL_SEED, seed_first: INITIAL_SEED });

fn with_global<T>(f: impl FnOnce(&mut RndState) -> T) -> T {
    let mut guard = GLOBAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

/// `RND` (`passed == 0`) or `RND(n)`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_rnd(n: f32, passed: i32) -> f32 {
    with_global(|s| s.rnd(if passed != 0 { Some(n) } else { None }))
}

/// `RANDOMIZE seed` (`using == 0`) or `RANDOMIZE USING seed`.
#[unsafe(no_mangle)]
pub extern "C" fn qb_randomize(seed: f64, using: i32) {
    with_global(|s| if using != 0 { s.randomize_using(seed) } else { s.randomize(seed) });
}

/// Seed entered at the interactive `RANDOMIZE` prompt.
#[unsafe(no_mangle)]
pub extern "C" fn qb_randomize_prompt(seed: i16) {
    with_global(|s| s.randomize_prompt(seed));
}

/// Restores the start-up seed (program start and `RUN`).
#[unsafe(no_mangle)]
pub extern "C" fn qb_rnd_reset() {
    with_global(|s| *s = RndState::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_abi_drives_one_global_generator() {
        qb_rnd_reset();
        close(qb_rnd(0.0, 0), 0.7055475);
        close(qb_rnd(123.0, 0), 0.533424); // n ignored when not passed
        close(qb_rnd(0.0, 1), 0.533424);
        qb_randomize(42.0, 1);
        let a = qb_rnd(0.0, 0);
        qb_randomize(42.0, 1);
        assert_eq!(qb_rnd(0.0, 0), a);
        qb_randomize(42.0, 0);
        qb_randomize_prompt(5);
        let mut expect = RndState::default();
        expect.randomize_using(42.0);
        expect.rnd(None);
        expect.randomize_using(42.0);
        expect.rnd(None);
        expect.randomize(42.0);
        expect.randomize_prompt(5);
        assert_eq!(with_global(|s| *s), expect);
        qb_rnd_reset();
        assert_eq!(with_global(|s| *s), RndState::default());
    }

    /// Values printed by the legacy C++ runtime (7 significant digits).
    fn close(got: f32, want: f64) {
        assert!((f64::from(got) - want).abs() < 5e-7 * want.abs().max(1e-2), "got {got}, want {want}");
    }

    #[test]
    fn fresh_sequence_matches_legacy() {
        let mut r = RndState::default();
        close(r.rnd(None), 0.7055475);
        close(r.rnd(None), 0.533424);
    }

    #[test]
    fn zero_argument_repeats_last_value() {
        let mut r = RndState::default();
        let a = r.rnd(None);
        assert_eq!(r.rnd(Some(0.0)), a);
        assert_eq!(r.rnd(Some(-0.0)), a);
    }

    #[test]
    fn negative_argument_reseeds_from_its_bits() {
        let mut r = RndState::default();
        r.rnd(None);
        r.rnd(None);
        close(r.rnd(Some(-1.0)), 0.224007);
        close(r.rnd(None), 3.584582e-2);
        // Same negative argument, same value, regardless of history.
        let mut other = RndState::default();
        assert_eq!(other.rnd(Some(-1.0)), RndState::default().rnd(Some(-1.0)));
    }

    #[test]
    fn positive_argument_advances_like_no_argument() {
        let mut a = RndState::default();
        let mut b = RndState::default();
        assert_eq!(a.rnd(Some(5.0)), b.rnd(None));
        assert_ne!(a.rnd(Some(5.0)), a.rnd(Some(5.0)));
    }

    #[test]
    fn randomize_variants_match_legacy_sequence() {
        let mut r = RndState::default();
        for n in [None, None, Some(0.0), Some(-1.0), None] {
            r.rnd(n);
        }
        r.randomize(42.0);
        close(r.rnd(None), 0.8157225);
        r.randomize_using(42.0);
        close(r.rnd(None), 0.3391077);
        r.randomize_using(7.5);
        close(r.rnd(None), 0.981518);
        r.randomize(-3.0);
        close(r.rnd(None), 0.5559155);
        close(r.rnd(None), 0.0432942510);
    }

    #[test]
    fn randomize_using_is_reproducible_but_randomize_is_not() {
        let mut r = RndState::default();
        r.randomize_using(42.0);
        let first = r.rnd(None);
        r.rnd(None);
        r.randomize_using(42.0);
        assert_eq!(r.rnd(None), first);
    }

    #[test]
    fn values_are_in_unit_interval_with_24_bit_resolution() {
        let mut r = RndState::default();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100_000 {
            let v = r.rnd(None);
            assert!((0.0..1.0).contains(&v));
            assert_eq!((f64::from(v) * 16_777_216.0).fract(), 0.0);
            seen.insert(v.to_bits());
        }
        // Full-period 24-bit LCG: no repeats within 100k draws.
        assert_eq!(seen.len(), 100_000);
    }

    #[test]
    fn prompt_seed_uses_low_16_bits() {
        let mut a = RndState::default();
        let mut b = RndState::default();
        a.randomize_prompt(-1);
        b.randomize_prompt(-1);
        assert_eq!(a, b);
        assert_ne!(a, RndState::default());
    }
}
