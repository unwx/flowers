use crate::art::color::{IWantHuePaletteFactory, PaletteFactory};
use crate::art::dynamic::{DynGradient, DynNoise};
use crate::art::flower::layer::{LayerFactory, ValvateLayerFactory};
use crate::art::flower::petal::{PetalFactory, PolarPetalFactory};
use crate::art::gradient::{GradientConverter, LinearGradientConverter};
use crate::art::mosaic::Mosaic;
use crate::art::pattern::{NoiseFactory, RandomNoiseFactory};
use crate::art::TexturedArea;
use crate::color::convert::hsl_to_color;
use crate::color::palette::Palette;
use crate::color::LabImprovedCiede2000Distance;
use crate::constraint::{MAX_FLOWER_RADIUS, MIN_FLOWER_RADIUS};
use crate::math;
use crate::math::area::find_inner_areas;
use crate::math::curve::{scale, ClosedCurve};
use crate::math::definition::IntPointExtensions;
use crate::math::{interpolate, remap, Area};
use crate::util::macros::debug_assert_finite;
use crate::util::range::is_range_within_another_ref;
use crate::util::RandomF32RangeFactory;
use anyhow::{bail, Context, Result};
use linfa_clustering::KMeansInit;
use noise::RotatePoint;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::collections::HashSet;
use std::f32::consts::PI;
use std::ops::RangeInclusive;
use std::rc::Rc;

pub mod layer;
pub mod petal;

pub struct Flower {
    mosaic: Mosaic,
    layers: Vec<Layer>,
    radius: u16,
}

impl Flower {
    fn new(mosaic: Mosaic, layers: Vec<Layer>, radius: u16) -> Self {
        Self {
            mosaic,
            layers,
            radius,
        }
    }

    pub fn mosaic(&self) -> &Mosaic {
        &self.mosaic
    }

    pub fn layers(&self) -> &Vec<Layer> {
        &self.layers
    }

    pub fn radius(&self) -> u16 {
        self.radius
    }
}


pub struct Layer {
    petals: Vec<Petal>,
}

impl Layer {
    fn new() -> Self {
        Self { petals: Vec::new() }
    }

    fn push(&mut self, petal: Petal) {
        self.petals.push(petal);
    }

    pub fn petals(&self) -> &Vec<Petal> {
        &self.petals
    }
}


pub struct Petal {
    curve: ClosedCurve,
    texture: TexturedArea<DynGradient, DynNoise<f64, 2>>,
}

impl Petal {
    fn new(curve: ClosedCurve, texture: TexturedArea<DynGradient, DynNoise<f64, 2>>) -> Self {
        Self { curve, texture }
    }

    pub fn curve(&self) -> &ClosedCurve {
        &self.curve
    }

    pub fn texture(&self) -> &TexturedArea<DynGradient, DynNoise<f64, 2>> {
        &self.texture
    }
}


pub struct FlowerFactory {
    petal_factories: Vec<Rc<dyn PetalFactory>>,
    layer_factory: Box<dyn LayerFactory>,
    palette_factory: Box<dyn PaletteFactory>,
    noise_factory: Box<dyn NoiseFactory>,
    gradient_converter: Box<dyn GradientConverter>,

    layers_count: RangeInclusive<usize>,
    layer_size_delta: RangeInclusive<f32>,

    use_same_palette: RangeInclusive<f32>,
    use_same_noise: RangeInclusive<f32>,
}

impl FlowerFactory {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        petal_factories: Vec<Rc<dyn PetalFactory>>,
        layer_factory: Box<dyn LayerFactory>,
        palette_factory: Box<dyn PaletteFactory>,
        noise_factory: Box<dyn NoiseFactory>,
        gradient_converter: Box<dyn GradientConverter>,
        layers_count: RangeInclusive<usize>,
        layer_size_delta: RangeInclusive<f32>,
        use_same_palette: RangeInclusive<f32>,
        use_same_noise: RangeInclusive<f32>,
    ) -> Result<Self> {
        if petal_factories.is_empty() {
            bail!("'petal_factories' cannot be empty");
        }
        if layers_count.is_empty() {
            bail!("'layers_count' cannot be empty");
        }
        if layer_size_delta.is_empty() {
            bail!("'layer_size_delta' cannot be empty");
        }
        if use_same_palette.is_empty() {
            bail!("'use_same_palette' cannot be empty");
        }
        if use_same_noise.is_empty() {
            bail!("'use_same_noise' cannot be empty");
        }

        {
            let layers_range = 1..=(petal_factories.len() - 1);
            if !is_range_within_another_ref(&layers_count, &layers_range) {
                bail!("'layers_count' must be within [{:?}] range", layers_range);
            }
        }
        if !is_range_within_another_ref(&layer_size_delta, &(0.0..=1.0)) {
            bail!("'layer_size_delta' must be within [0.0..=1.0] range");
        }

        Ok(Self {
            petal_factories,
            layer_factory,
            palette_factory,
            noise_factory,
            gradient_converter,
            layers_count,
            layer_size_delta,
            use_same_palette,
            use_same_noise,
        })
    }

    //noinspection DuplicatedCode
    pub fn new_random<R: Rng>(random: &mut R) -> Self {
        // TODO Enhancement: add more options

        let petal_factories = {
            let sharp = random.gen_bool(0.5);
            let width_factory = RandomF32RangeFactory::new(
                RandomF32RangeFactory::new(0.0..=1.0, 0.0, 0.3).random_range(random),
                0.0,
                0.1,
            );

            (0..5)
                .map(|_| {
                    #[rustfmt::skip]
                    let factory = PolarPetalFactory::try_new(
                        width_factory.random_range(random),
                        sharp
                    )
                    .expect("failed to create random PolarPetalFactory");

                    Rc::new(factory) as Rc<dyn PetalFactory>
                })
                .collect::<Vec<_>>()
        };

        let layer_factory = ValvateLayerFactory::try_new(
            RandomF32RangeFactory::new(0.05..=0.9, 0.0, 0.15).random_range(random),
            RandomF32RangeFactory::new((-PI / 24.0)..=(PI / 24.0), 0.0, 0.3).random_range(random),
        )
        .expect("failed to create random ValvateLayerFactory");

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
            1..=2,
            0..=0,
            0..=0,
            0.0..=0.1,
            0.75..=1.25
        )
        .expect("failed to create random RandomNoiseFactory");

        let gradient_converter = LinearGradientConverter;
        let layers_count = 1..=(petal_factories.len() - 1);

        #[rustfmt::skip]
        let layer_size_delta = RandomF32RangeFactory::new(
            0.0..=0.4,
            0.0,
            0.15
        )
        .random_range(random);

        let use_same_palette = -1.0..=1.0;
        let use_same_noise = -1.0..=1.0;

        Self::try_new(
            petal_factories,
            Box::new(layer_factory),
            Box::new(palette_factory),
            Box::new(noise_factory),
            Box::new(gradient_converter),
            layers_count,
            layer_size_delta,
            use_same_palette,
            use_same_noise,
        )
        .expect("failed to create random FlowerFactory")
    }


    pub fn petal_factories(&self) -> &Vec<Rc<dyn PetalFactory>> {
        &self.petal_factories
    }

    pub fn layer_factory(&self) -> &dyn LayerFactory {
        self.layer_factory.as_ref()
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

    pub fn layers_count(&self) -> RangeInclusive<usize> {
        self.layers_count.clone()
    }

    pub fn layer_size_delta(&self) -> RangeInclusive<f32> {
        self.layer_size_delta.clone()
    }

    pub fn use_same_palette(&self) -> RangeInclusive<f32> {
        self.use_same_palette.clone()
    }

    pub fn use_same_noise(&self) -> RangeInclusive<f32> {
        self.use_same_noise.clone()
    }


    pub fn random_flower<R: Rng>(
        &self,
        mosaic: Mosaic,
        radius: u16,
        random: &mut R,
    ) -> Result<Flower> {
        if !(MIN_FLOWER_RADIUS..=MAX_FLOWER_RADIUS).contains(&radius) {
            bail!("'radius' must be within [{MIN_FLOWER_RADIUS}..={MAX_FLOWER_RADIUS}] range");
        }
        if mosaic.radius() >= radius {
            bail!("'mosaic.radius' must be < 'flower.radius'");
        }

        let mut layers = self
            .create_layers(mosaic.radius(), radius, random)
            .with_context(|| {
                format!(
                    "failed to create flower layers. \
                    [mosaic_radius: {}, flower_radius: {}]",
                    mosaic.radius(),
                    radius,
                )
            })?;

        for layer in &mut layers {
            layer.shuffle(random);
        }

        layers = {
            let len = layers.len();
            Self::attach_areas(layers).with_context(|| {
                format!(
                    "failed to attach areas to the flower layers. \
                    [layers_count: {len}]"
                )
            })?
        };

        layers = Self::optimize_layers(layers);
        if layers.is_empty() {
            bail!("the resulting flower layers array is empty");
        }

        let layers = {
            let len = layers.len();
            self.attach_textures(layers, random).with_context(|| {
                format!(
                    "failed to attach textures to the flower layers. \
                    [layers_count: {len}]"
                )
            })?
        };
        Ok(Flower::new(mosaic, layers, radius))
    }

    fn create_layer<R: Rng>(
        &self,
        full_size: f32,
        mosaic_radius: u16,
        flower_radius: u16,
        past_full_size: Option<u16>,
        petal_factory: &dyn PetalFactory,
        random: &mut R,
    ) -> Result<(Vec<PetalStage>, u16)> {
        debug_assert_finite!(full_size);
        debug_assert!((0.0..=1.0).contains(&full_size));
        debug_assert!(mosaic_radius < flower_radius);

        let raw_layer = self.layer_factory.layer(petal_factory, random.next_u64())?;
        if raw_layer.is_empty() {
            bail!("the resulting raw layer is empty");
        }

        #[rustfmt::skip]
        let distance_from_origin = remap(
            full_size,
            0.0,
            1.0,
            (mosaic_radius as f32) * 0.8,
            0.0,
        ) as u16;

        let full_size = remap(
            full_size,
            0.0,
            1.0,
            mosaic_radius as f32,
            flower_radius as f32,
        ) as u16;
        let real_size = full_size - distance_from_origin;


        let mut layer = Vec::new();
        for (index, mut petal) in raw_layer.into_iter().enumerate() {
            if petal.is_empty() {
                bail!("failed to scale the layer: an empty petal. [index: {index}]");
            }

            {
                #[rustfmt::skip]
                let distance_from_origin_normalized = (distance_from_origin as f32) / (flower_radius as f32);
                let direction = *petal
                    .iter()
                    .max_by(|first, second| {
                        first.length_squared().total_cmp(&second.length_squared())
                    })
                    .expect("petal cannot be empty");

                let magnitude = direction.length();
                debug_assert_finite!(distance_from_origin_normalized, direction, magnitude);

                if magnitude == 0.0 {
                    bail!("failed to move a petal from the origin: 'petal.max_point.length' is 0.0. [index: {index}]");
                }
                for point in &mut petal {
                    *point += (direction / magnitude) * distance_from_origin_normalized;
                }
            }


            #[rustfmt::skip]
            let size_delta = {
                let delta = random.gen_range(self.layer_size_delta());
                if let Some(past_full_size) = past_full_size {
                    remap(
                        delta,
                        0.0,
                        1.0,
                        0.0,
                        (full_size as f32) - (past_full_size as f32),
                    ) as i16
                } else {
                    remap(
                        delta,
                        0.0,
                        1.0,
                        0.0,
                        (full_size - mosaic_radius) as f32
                    ) as i16
                }
            };
            let size = real_size
                .saturating_add_signed(-size_delta)
                .clamp(1, flower_radius);


            let curve = Some(petal)
                .map(|it| scale(it.as_slice(), size))
                .map(|it| interpolate(it.as_slice()))
                .and_then(|it| ClosedCurve::try_from(it).ok())
                .with_context(|| format!("failed to create a ClosedCurve from the interpolated petal. [index: {index}]"))?;

            let mut petal = PetalStage::default();
            petal.set_curve(curve);

            layer.push(petal);
        }

        Ok((layer, full_size))
    }

    fn create_layers<R: Rng>(
        &self,
        mosaic_radius: u16,
        flower_radius: u16,
        random: &mut R,
    ) -> Result<Vec<Vec<PetalStage>>> {
        let layers_count = random.gen_range(self.layers_count());
        debug_assert!(layers_count > 0);

        let full_layer_size_step = 1.0 / (layers_count as f32);
        let mut past_scaled_full_layer_size = None;
        let mut layers = Vec::new();

        for i in 0..layers_count {
            let (layer, scaled_full_layer_size) = self
                .create_layer(
                    ((i + 1) as f32) * full_layer_size_step,
                    mosaic_radius,
                    flower_radius,
                    past_scaled_full_layer_size,
                    self.petal_factories[i].as_ref(),
                    random,
                )
                .with_context(|| format!("failed to create a petals layer. [index: {i}]"))?;

            layers.push(layer);
            past_scaled_full_layer_size = Some(scaled_full_layer_size);
        }

        layers.reverse();
        Ok(layers)
    }

    fn attach_areas(layers: Vec<Vec<PetalStage>>) -> Result<Vec<Vec<PetalStage>>> {
        let mut populated_layers = (0..layers.len())
            .map(|_| Vec::new())
            .collect::<Vec<Vec<PetalStage>>>();

        for (layer_index, layer) in layers.into_iter().enumerate() {
            for (petal_index, mut petal) in layer.into_iter().enumerate() {
                let areas = find_inner_areas(petal.curve()).with_context(|| {
                    format!(
                        "failed to find inner areas in the petal curve. \
                        [layer_index: {layer_index}, petal_index: {petal_index}]"
                    )
                })?;

                if areas.is_empty() {
                    continue;
                }

                let area = math::area::merge(areas).with_context(|| {
                    format!(
                        "failed to merge petal inner areas into a single one. \
                        [layer_index: {layer_index}, petal_index: {petal_index}]"
                    )
                })?;
                petal.set_area(area);
                populated_layers[layer_index].push(petal);
            }
        }

        Ok(populated_layers
            .into_iter()
            .filter(|layer| !layer.is_empty())
            .collect())
    }

    fn optimize_layers(mut layers: Vec<Vec<PetalStage>>) -> Vec<Vec<PetalStage>> {
        enum CullingResult {
            Visible,
            Invisible,
            Culled(Area),
        }

        fn cull_back_area(back_area: &Area, front_areas: &[&Area]) -> CullingResult {
            let mut result = CullingResult::Visible;

            for front_area in front_areas {
                if !back_area.dirty_intersects(front_area) {
                    continue;
                }

                let visible_area = {
                    match result {
                        CullingResult::Visible => back_area.area_behind(front_area),
                        CullingResult::Culled(area) => area.area_behind(front_area),
                        _ => unreachable!(),
                    }
                };

                if let Some(area) = visible_area {
                    result = CullingResult::Culled(area);
                } else {
                    return CullingResult::Invisible;
                }
            }

            result
        }

        // Determinism:
        // This set is used only for contains/insert operations.
        let mut marked_for_removal = HashSet::new();

        // Starting from the largest layer,
        // finishing with the smallest.
        for current_layer_index in 0..layers.len() {
            for front_layer_index in current_layer_index..(layers.len() - 1) {
                for petal_index in 0..layers[current_layer_index].len() {
                    let culling_result = cull_back_area(
                        layers[current_layer_index][petal_index].area(),
                        layers[front_layer_index]
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| {
                                !marked_for_removal.contains(&(front_layer_index, *index))
                            })
                            .filter(|(index, _)| {
                                current_layer_index != front_layer_index || *index != petal_index
                            })
                            .map(|(_, petal)| petal.area())
                            .collect::<Vec<&Area>>()
                            .as_slice(),
                    );

                    match culling_result {
                        CullingResult::Invisible => {
                            marked_for_removal.insert((current_layer_index, petal_index));
                        }
                        CullingResult::Culled(area) => {
                            layers[current_layer_index][petal_index].set_area(area);
                        }
                        CullingResult::Visible => {}
                    }
                }
            }
        }

        layers
            .into_iter()
            .enumerate()
            .map(|(layer_index, layer)| {
                layer
                    .into_iter()
                    .enumerate()
                    .filter(|(petal_index, _)| {
                        !marked_for_removal.contains(&(layer_index, *petal_index))
                    })
                    .map(|(_, petal)| petal)
                    .collect::<Vec<_>>()
            })
            .filter(|layer| !layer.is_empty())
            .collect()
    }

    fn attach_textures<R: Rng>(
        &self,
        layers: Vec<Vec<PetalStage>>,
        random: &mut R,
    ) -> Result<Vec<Layer>> {
        // TODO Enhancement:
        //  Use a shared palette object for mosaic and flower generation.
        //  Remove hardcoded colors count.

        let colors = {
            let use_same_palette = random.gen_range(self.use_same_palette()) >= 0.0;
            if use_same_palette {
                let colors_count = random.gen_range(2..=3);
                let colors = self
                    .palette_factory
                    .palette(colors_count, random.next_u64())
                    .with_context(|| {
                        format!(
                            "failed to create a flower color palette. \
                            [requested_colors_count: {colors_count}]"
                        )
                    })?;

                Some(colors)
            } else {
                None
            }
        };
        let noise_structure_seed = {
            let use_same_noise = random.gen_range(self.use_same_noise()) >= 0.0;
            if use_same_noise {
                Some(random.next_u64())
            } else {
                None
            }
        };


        let mut populated_layers = (0..layers.len())
            .map(|_| Layer::new())
            .collect::<Vec<Layer>>();

        for (layer_index, layer) in layers.into_iter().enumerate() {
            let gradient = {
                let colors = {
                    if let Some(colors) = colors.clone() {
                        colors
                    } else {
                        let colors_count = 2;
                        self.palette_factory
                            .palette(colors_count, random.next_u64())
                            .with_context(|| {
                                format!(
                                    "failed to create a flower color palette. \
                                    [requested_colors_count: {colors_count}, layer_index: {layer_index}]"
                                )
                            })?
                    }
                };
                self.gradient_converter
                    .colors_to_gradient(
                        colors.into_iter().map(hsl_to_color).collect(),
                        random.next_u64(),
                    )
                    .with_context(|| {
                        format!(
                            "failed to convert the flower color palette to a gradient. \
                            [layer_index: {layer_index}]"
                        )
                    })?
            };

            for (petal_index, petal) in layer.into_iter().enumerate() {
                let (noise, noise_scale) = {
                    let noise_structure_seed =
                        noise_structure_seed.unwrap_or_else(|| random.next_u64());
                    let noise_output_seed = random.next_u32();

                    let (noise, scale) = self
                        .noise_factory
                        .noise(noise_output_seed, noise_structure_seed)
                        .with_context(|| {
                            format!(
                                "failed to create a flower petal noise. \
                                [output_seed: {noise_output_seed}, structure_seed: {noise_structure_seed}, \
                                layer_index: {layer_index}, petal_index: {petal_index}]"
                            )
                        })?;

                    let angle = petal
                        .curve()
                        .iter()
                        .max_by(|&&first, &&second| {
                            first
                                .as_point()
                                .length_squared()
                                .total_cmp(&second.as_point().length_squared())
                        })
                        .map(|point| point.as_point().to_angle())
                        .filter(|angle| angle.is_finite())
                        .with_context(|| {
                            format!(
                                "failed to decorate the flower petal noise with a rotation noise: \
                                failed to find the petal's angle. \
                                [layer_index: {layer_index}, petal_index: {petal_index}]"
                            )
                        })?;

                    let rotation = RotatePoint::new(noise).set_z_angle(angle.to_degrees() as f64);
                    (DynNoise::new(rotation), scale)
                };

                let petal = Petal::new(
                    petal.curve.expect("curve must be present"),
                    TexturedArea::new(
                        petal.area.expect("area must be present"),
                        Palette::new(gradient.clone()),
                        noise,
                        noise_scale,
                    ),
                );
                populated_layers[layer_index].push(petal);
            }
        }

        Ok(populated_layers)
    }
}


#[derive(Debug, Clone, Default)]
struct PetalStage {
    curve: Option<ClosedCurve>,
    area: Option<Area>,
}

impl PetalStage {
    pub fn curve(&self) -> &ClosedCurve {
        self.curve.as_ref().expect("requested curve is not ready")
    }

    pub fn area(&self) -> &Area {
        self.area.as_ref().expect("requested area is not ready")
    }

    pub fn set_curve(&mut self, curve: ClosedCurve) {
        self.curve = Some(curve)
    }

    pub fn set_area(&mut self, area: Area) {
        self.area = Some(area)
    }
}
