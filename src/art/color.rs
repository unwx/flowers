use crate::color::theory::i_want_hue_color_palette;
use crate::util::range::is_range_within_another;
use crate::util::RandomF32RangeFactory;
use crate::Random;
use anyhow::{bail, Result};
use linfa_clustering::{KMeans, KMeansInit};
use linfa_nn::distance::Distance;
use palette::Okhsl;
use rand::{Rng, SeedableRng};

pub trait PaletteFactory {
    fn palette(&self, size: usize, seed: u64) -> Result<Vec<Okhsl>>;
}


#[derive(Debug, Copy, Clone)]
pub struct RandomPaletteFactory;

impl PaletteFactory for RandomPaletteFactory {
    fn palette(&self, size: usize, seed: u64) -> Result<Vec<Okhsl>> {
        let mut random = Random::seed_from_u64(seed);
        Ok((0..size)
            .map(|_| {
                Okhsl::new(
                    random.gen_range(0.0..=360.0),
                    random.gen_range(0.0..=1.0),
                    random.gen_range(0.0..=1.0),
                )
            })
            .collect())
    }
}


#[derive(Debug, Clone)]
pub struct DefinedPaletteFactory {
    colors: Vec<Okhsl>,
}

impl DefinedPaletteFactory {
    pub fn new(colors: Vec<Okhsl>) -> Self {
        assert!(!colors.is_empty(), "colors is empty");
        Self { colors }
    }

    pub fn colors(&self) -> &Vec<Okhsl> {
        &self.colors
    }

    pub fn into_colors(self) -> Vec<Okhsl> {
        self.colors
    }
}

impl PaletteFactory for DefinedPaletteFactory {
    fn palette(&self, size: usize, _: u64) -> Result<Vec<Okhsl>> {
        Ok((0..size)
            .map(|i| self.colors[i % self.colors.len()])
            .collect())
    }
}


// TODO Enhancement:
//  The current implementation iterates colors in ranges linearly only
//  (iterates all hue/saturation/lightness ranges simultaneously, O(n)),
//  is it worth trying more options?
#[derive(Debug, Clone)]
pub struct IWantHuePaletteFactory<D>
where
    D: Distance<f32> + Clone,
{
    hue_range_factory: RandomF32RangeFactory,
    saturation_range_factory: RandomF32RangeFactory,
    lightness_range_factory: RandomF32RangeFactory,

    colors_limit: usize,
    dataset_size: usize,
    init_method: KMeansInit<f32>,
    n_runs: usize,
    max_n_iterations: u64,
    tolerance: f32,
    distance_fn: D,
}

impl<D> IWantHuePaletteFactory<D>
where
    D: Distance<f32> + Clone,
{
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        hue_range_factory: RandomF32RangeFactory,
        saturation_range_factory: RandomF32RangeFactory,
        lightness_range_factory: RandomF32RangeFactory,
        colors_limit: usize,
        dataset_size: usize,
        init_method: KMeansInit<f32>,
        n_runs: usize,
        max_n_iterations: u64,
        tolerance: f32,
        distance_fn: D,
    ) -> Result<Self> {
        if !is_range_within_another(saturation_range_factory.boundaries(), 0.0..=1.0) {
            bail!("'saturation_range_factory' must be within [0.0..=1.0] range");
        }
        if !is_range_within_another(lightness_range_factory.boundaries(), 0.0..=1.0) {
            bail!("'lightness_range_factory' must be within [0.0..=1.0] range");
        }

        if dataset_size == 0 {
            bail!("'dataset_size' must be > 0");
        }
        if n_runs == 0 {
            bail!("'n_runs' must be > 0");
        }
        if max_n_iterations == 0 {
            bail!("'max_n_iterations' must be > 0");
        }
        if tolerance <= 0.0 {
            bail!("'tolerance' must be > 0.0");
        }

        Ok(Self {
            hue_range_factory,
            saturation_range_factory,
            lightness_range_factory,
            colors_limit,
            dataset_size,
            init_method,
            n_runs,
            max_n_iterations,
            tolerance,
            distance_fn,
        })
    }

    pub fn hue_range_factory(&self) -> RandomF32RangeFactory {
        self.hue_range_factory.clone()
    }

    pub fn saturation_range_factory(&self) -> RandomF32RangeFactory {
        self.saturation_range_factory.clone()
    }

    pub fn lightness_range_factory(&self) -> RandomF32RangeFactory {
        self.lightness_range_factory.clone()
    }

    pub fn dataset_size(&self) -> usize {
        self.dataset_size
    }

    pub fn init_method(&self) -> &KMeansInit<f32> {
        &self.init_method
    }

    pub fn n_runs(&self) -> usize {
        self.n_runs
    }

    pub fn max_n_iterations(&self) -> u64 {
        self.max_n_iterations
    }

    pub fn tolerance(&self) -> f32 {
        self.tolerance
    }

    pub fn distance_fn(&self) -> &D {
        &self.distance_fn
    }
}

impl<D> PaletteFactory for IWantHuePaletteFactory<D>
where
    D: Distance<f32> + Clone,
{
    fn palette(&self, size: usize, seed: u64) -> Result<Vec<Okhsl>> {
        // k-means clustering is expensive.
        if size > self.colors_limit {
            bail!(
                "cannot generate IWantHuePalette palette with more than '{}' colors: {}",
                self.colors_limit,
                size
            );
        }

        let mut random = Random::seed_from_u64(seed);

        let hue_range = self.hue_range_factory.random_range(&mut random);
        let saturation_range = self.saturation_range_factory.random_range(&mut random);
        let lightness_range = self.lightness_range_factory.random_range(&mut random);

        let kmeans_params = KMeans::params_with(size, random, self.distance_fn.clone())
            .init_method(self.init_method.clone())
            .n_runs(self.n_runs)
            .max_n_iterations(self.max_n_iterations)
            .tolerance(self.tolerance);

        i_want_hue_color_palette(
            hue_range,
            saturation_range,
            lightness_range,
            self.dataset_size,
            kmeans_params,
        )
    }
}
