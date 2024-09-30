use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::ops::{Deref, DerefMut};

pub struct RecoverableRng {
    gen: ChaCha8Rng,
    seed: u64,
}

impl RecoverableRng {
    pub fn new(seed: u64) -> Self {
        Self {
            gen: ChaCha8Rng::seed_from_u64(seed),
            seed,
        }
    }

    pub fn recover(&self) -> Self {
        Self::new(self.seed)
    }

    pub fn gen_option<F, T>(&mut self, func: F) -> Option<T>
    where
        F: FnOnce(&mut Self) -> T,
    {
        random_option(self, func)
    }
}

impl Deref for RecoverableRng {
    type Target = ChaCha8Rng;

    fn deref(&self) -> &Self::Target {
        &self.gen
    }
}

impl DerefMut for RecoverableRng {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.gen
    }
}

impl RngCore for RecoverableRng {
    fn next_u32(&mut self) -> u32 {
        self.gen.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.gen.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.gen.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.gen.try_fill_bytes(dest)
    }
}

pub fn random_option<R, F, T>(random: &mut R, func: F) -> Option<T>
where
    R: Rng,
    F: FnOnce(&mut R) -> T,
{
    if random.gen_bool(0.5) {
        Some(func(random))
    } else {
        None
    }
}
