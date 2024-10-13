use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct RestorableRng {
    delegate: ChaCha8Rng,
    seed: u64,
}

impl RestorableRng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            delegate: ChaCha8Rng::seed_from_u64(seed),
            seed,
        }
    }

    #[must_use]
    pub fn restore(&self) -> Self {
        Self::new(self.seed)
    }

    pub fn gen_option<F, T>(&mut self, func: F) -> Option<T>
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.gen_weighted_option(0.5, func)
    }

    pub fn gen_weighted_option<F, T>(&mut self, probability: f64, func: F) -> Option<T>
    where
        F: FnOnce(&mut Self) -> T,
    {
        if self.gen_bool(probability) {
            Some(func(self))
        } else {
            None
        }
    }
}

impl Deref for RestorableRng {
    type Target = ChaCha8Rng;

    fn deref(&self) -> &Self::Target {
        &self.delegate
    }
}

impl DerefMut for RestorableRng {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.delegate
    }
}

impl RngCore for RestorableRng {
    fn next_u32(&mut self) -> u32 {
        self.delegate.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.delegate.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.delegate.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.delegate.try_fill_bytes(dest)
    }
}
