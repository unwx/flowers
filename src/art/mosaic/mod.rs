use crate::art::color::{IWantHuePaletteFactory, PaletteFactory};
use crate::art::dynamic::{DynGradient, DynNoise};
use crate::art::gradient::{GradientConverter, LinearGradientConverter};
use crate::art::mosaic::area::{AreaGroup, AreaGroupDecorator, NoopGroupDecorator};
use crate::art::mosaic::curve::{CurveFactory, PolarCurveFactory};
use crate::art::pattern::{NoiseFactory, RandomNoiseFactory};
use crate::art::TexturedArea;
use crate::color::convert::hsl_to_color;
use crate::color::palette::Palette;
use crate::color::LabImprovedCiede2000Distance;
use crate::constraint::{MAX_MOSAIC_RADIUS, MIN_MOSAIC_RADIUS};
use crate::math;
use crate::math::area::find_inner_areas;
use crate::math::curve::{scale, ClosedCurve, MergeMode};
use crate::math::{interpolate, Area};
use crate::util::RandomF32RangeFactory;
use anyhow::{bail, Context, Result};
use colorgrad::Color;
use linfa_clustering::KMeansInit;
use rand::Rng;
use std::ops::RangeInclusive;

pub mod area;
pub mod curve;

pub struct Mosaic {
    curve: ClosedCurve,
    textures: Vec<TexturedArea<DynGradient, DynNoise<f64, 2>>>,
    invisible_area: Option<Area>,
    radius: u16,
}

impl Mosaic {
    fn new(
        curve: ClosedCurve,
        textures: Vec<TexturedArea<DynGradient, DynNoise<f64, 2>>>,
        invisible_area: Option<Area>,
        radius: u16,
    ) -> Self {
        Self {
            curve,
            textures,
            invisible_area,
            radius,
        }
    }

    pub fn curve(&self) -> &ClosedCurve {
        &self.curve
    }

    pub fn textures(&self) -> &Vec<TexturedArea<DynGradient, DynNoise<f64, 2>>> {
        &self.textures
    }

    pub fn invisible_area(&self) -> &Option<Area> {
        &self.invisible_area
    }

    pub fn radius(&self) -> u16 {
        self.radius
    }
}


pub struct MosaicFactory {
    curve_factory: Box<dyn CurveFactory>,
    palette_factory: Box<dyn PaletteFactory>,
    noise_factory: Box<dyn NoiseFactory>,
    gradient_converter: Box<dyn GradientConverter>,
    area_group_decorators: Vec<Box<dyn AreaGroupDecorator>>,
    curves_count: RangeInclusive<usize>,
}

impl MosaicFactory {
    pub fn try_new(
        curve_factory: Box<dyn CurveFactory>,
        palette_factory: Box<dyn PaletteFactory>,
        noise_factory: Box<dyn NoiseFactory>,
        gradient_converter: Box<dyn GradientConverter>,
        area_group_decorators: Vec<Box<dyn AreaGroupDecorator>>,
        curves_count: RangeInclusive<usize>,
    ) -> Result<Self> {
        if curves_count.is_empty() {
            bail!("'curves_count' cannot be empty");
        }
        if *curves_count.start() < 2 {
            bail!("'curves_count' must be >= 2");
        }

        Ok(Self {
            curve_factory,
            palette_factory,
            noise_factory,
            gradient_converter,
            area_group_decorators,
            curves_count,
        })
    }

    //noinspection DuplicatedCode
    pub fn new_random<R: Rng>(random: &mut R) -> MosaicFactory {
        // TODO Enhancement: add more options

        let curve_factory = PolarCurveFactory::new_random(random);
        let palette_factory = IWantHuePaletteFactory::try_new(
            RandomF32RangeFactory::new(0.0..=720.0, 30.0, 360.0),
            RandomF32RangeFactory::new(0.3..=1.0, 0.0, 0.4),
            RandomF32RangeFactory::new(0.0..=1.0, 0.2, 0.9),
            6,
            1500,
            KMeansInit::KMeansPlusPlus,
            3,
            100,
            0.5,
            LabImprovedCiede2000Distance,
        )
        .expect("failed to create random IWantHuePaletteFactory");

        #[rustfmt::skip]
        let noise_factory = RandomNoiseFactory::try_new(
            1..=3,
            0..=2,
            2..=4,
            0.0..=0.2,
            1.0..=1.75
        )
        .expect("failed to create random RandomNoiseFactory");

        let gradient_converter = LinearGradientConverter;
        let area_group_decorators = vec![];
        let curves_count = 2..=3;

        MosaicFactory::try_new(
            Box::new(curve_factory),
            Box::new(palette_factory),
            Box::new(noise_factory),
            Box::new(gradient_converter),
            area_group_decorators,
            curves_count,
        )
        .expect("failed to create random MosaicFactory")
    }


    pub fn curve_factory(&self) -> &dyn CurveFactory {
        self.curve_factory.as_ref()
    }

    pub fn palette_factory(&self) -> &dyn PaletteFactory {
        self.palette_factory.as_ref()
    }

    pub fn noise_factory(&self) -> &dyn NoiseFactory {
        self.noise_factory.as_ref()
    }

    pub fn gradient_converter(&self) -> &dyn GradientConverter {
        self.gradient_converter.as_ref()
    }

    pub fn area_group_decorators(&self) -> &Vec<Box<dyn AreaGroupDecorator>> {
        &self.area_group_decorators
    }

    pub fn curves_count(&self) -> RangeInclusive<usize> {
        self.curves_count.clone()
    }


    pub fn random_mosaic<R: Rng>(&self, radius: u16, random: &mut R) -> Result<Mosaic> {
        if !(MIN_MOSAIC_RADIUS..=MAX_MOSAIC_RADIUS).contains(&radius) {
            bail!("'radius' must be within [{MIN_MOSAIC_RADIUS}..={MAX_MOSAIC_RADIUS}] range");
        }

        let curve = self.create_curve(radius, random).with_context(|| {
            format!("failed to create a mosaic closed-curve. [radius: {radius}]")
        })?;

        let areas = find_inner_areas(&curve).with_context(|| {
            format!(
                "failed to find inner areas of the mosaic closed-curve. [closed_curve_len: {}]",
                curve.len()
            )
        })?;
        let invisible_area = math::area::merge(areas.clone());

        let area_group = {
            let len = areas.len();
            self.decorate_areas(areas, random).with_context(|| {
                format!(
                    "failed to decorate the mosaic areas. [areas_count: {}]",
                    len
                )
            })
        }?;
        let textures = {
            let groups_count = area_group.subgroups.len();
            let areas_count = area_group.subgroups.iter().flatten().count();

            self.attach_textures(area_group.subgroups, random)
                .with_context(|| {
                    format!(
                        "failed to attach textures to the mosaic areas. \
                        [groups_count: {groups_count}, areas_count: {areas_count}]"
                    )
                })
        }?;

        Ok(Mosaic::new(curve, textures, invisible_area, radius))
    }


    fn create_curve<R: Rng>(&self, radius: u16, random: &mut R) -> Result<ClosedCurve> {
        let parts_count = random.gen_range(self.curves_count());
        let mut parts = Vec::with_capacity(parts_count);

        for i in 0..parts_count {
            let part = self
                .curve_factory
                .curve(random.next_u64())
                .map(|curve| scale(curve.as_slice(), radius))
                .with_context(|| format!("failed to generate a mosaic curve part. [index: {i}]"))?;

            parts.push(part);
        }

        let merged = math::curve::merge(parts, MergeMode::Origin);
        let interpolated = interpolate(merged.as_slice());

        ClosedCurve::try_from(interpolated)
            .ok()
            .context("failed to create a ClosedCurve")
    }

    fn decorate_areas<R: Rng>(
        &self,
        initial_areas: Vec<Area>,
        random: &mut R,
    ) -> Result<AreaGroup> {
        let mut group = AreaGroup::new(vec![initial_areas]);

        let noop_decorator: Vec<Box<dyn AreaGroupDecorator>> = vec![Box::new(NoopGroupDecorator)];
        let area_group_decorators: &Vec<Box<dyn AreaGroupDecorator>> = {
            if !self.area_group_decorators.is_empty() {
                &self.area_group_decorators
            } else {
                &noop_decorator
            }
        };

        for (index, decorator) in area_group_decorators.iter().enumerate() {
            group = decorator
                .decorate_group(group, random.next_u64())
                .with_context(|| format!("failed to decorate the area group. [index: {index}]"))?;
        }

        Ok(group)
    }

    fn attach_textures<R: Rng>(
        &self,
        area_groups: Vec<Vec<Area>>,
        random: &mut R,
    ) -> Result<Vec<TexturedArea<DynGradient, DynNoise<f64, 2>>>> {
        if area_groups.is_empty() {
            return Ok(vec![]);
        }

        let groups_len = area_groups.len();
        let mut textures = Vec::with_capacity(groups_len);

        let mut colors: Vec<Color> = {
            let colors_count = if groups_len == 1 {
                random.gen_range(2..=4)
            } else {
                groups_len * 2
            };

            self.palette_factory
                .palette(colors_count, random.next_u64())
                .with_context(|| {
                    format!(
                        "failed to generate a mosaic color palette. \
                        [requested_palette_size: {colors_count}]"
                    )
                })?
                .into_iter()
                .map(hsl_to_color)
                .collect()
        };
        let colors_per_group = colors.len() / groups_len;


        for (group_index, group) in area_groups.into_iter().enumerate() {
            if group.is_empty() {
                continue;
            }

            let area = math::area::merge(group).with_context(|| {
                format!(
                    "failed to merge a mosaic area group into a single area. \
                    [group_index: {group_index}]"
                )
            })?;

            let palette = {
                let colors: Vec<Color> =
                    colors.drain((colors.len() - colors_per_group)..).collect();
                let colors_count = colors.len();
                debug_assert!(colors_count >= 2);

                let gradient = self
                    .gradient_converter
                    .colors_to_gradient(colors, random.next_u64())
                    .with_context(|| {
                        format!(
                            "failed to create a gradient from the mosaic color palette slice. \
                            [colors_count: {}, group_index: {}]",
                            colors_count, group_index
                        )
                    })?;

                Palette::new(gradient)
            };
            let (noise, noise_scale) = {
                let noise_output_seed = random.next_u32();
                let noise_structure_seed = random.next_u64();

                self.noise_factory
                    .noise(noise_output_seed, noise_structure_seed)
                    .with_context(|| {
                        format!(
                            "failed to create a mosaic noise. \
                            [structure_seed: {noise_structure_seed}, \
                            output_seed: {noise_output_seed}, \
                            group_index: {group_index}]"
                        )
                    })?
            };

            textures.push(TexturedArea::new(area, palette, noise, noise_scale))
        }

        Ok(textures)
    }
}
