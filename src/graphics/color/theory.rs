use crate::graphics::color::convert::{hsl_to_rgb, rgb_to_hsl};
use crate::math::real::debug_assert_finite;
use palette::color_theory::{Analogous, Complementary, SplitComplementary, Tetradic, Triadic};
use palette::rgb::Rgb;
use palette::Clamp;
use palette::Hsl;
use rand::Rng;

pub const MIN_COLOR_LIGHT: f32 = 0.1;
pub const MAX_COLOR_LIGHT: f32 = 0.9;

#[must_use]
pub fn find_average_color_iter<'a, I>(iterator: I) -> Option<Rgb>
where
    I: Iterator<Item = &'a Rgb>,
{
    let mut count = 0;
    let mut red = 0.0;
    let mut green = 0.0;
    let mut blue = 0.0;

    for color in iterator {
        debug_assert_finite!(color);
        debug_assert_eq!(*color, color.clamp());

        red += color.red;
        green += color.green;
        blue += color.blue;
        count += 1;
    }

    let average = Rgb::new(
        red / count as f32,
        green / count as f32,
        blue / count as f32,
    );
    
    debug_assert_finite!(average);
    debug_assert_eq!(average, average.clamp());
    Some(average)
}

#[must_use]
pub fn random_color<R: Rng>(random: &mut R) -> Rgb {
    hsl_to_rgb(Hsl::new_srgb(
        random.gen_range(0.0..=1.0),
        random.gen_range(0.0..=1.0),
        random.gen_range(MIN_COLOR_LIGHT..=MAX_COLOR_LIGHT),
    ))
}

#[must_use]
pub fn random_palette<R: Rng>(size: usize, primary_color: Rgb, random: &mut R) -> Vec<Rgb> {
    debug_assert_finite!(primary_color);
    debug_assert_eq!(primary_color, primary_color.clamp());
    assert!(
        size <= 12,
        "palette can only contain up to 12 colors, size: {size}"
    );

    if size == 0 {
        return vec![];
    }
    if size == 1 {
        return vec![primary_color];
    }

    let mut palette = Vec::with_capacity(size + 3);
    palette.push(rgb_to_hsl(primary_color));

    while palette.len() < size {
        let base_color = palette[random.gen_range(0..palette.len())];
        let split = match random.gen_range(0..5) {
            0 => vec![base_color.complementary()],
            1 => {
                let c = base_color.split_complementary();
                vec![c.0, c.1]
            }
            2 => {
                let c = base_color.analogous();
                vec![c.0, c.1]
            }
            3 => {
                let c = base_color.triadic();
                vec![c.0, c.1]
            }
            4 => {
                let c = base_color.tetradic();
                vec![c.0, c.1, c.2]
            }
            _ => unreachable!(),
        };

        for color in split.iter().take(split.len().min(size - palette.len())) {
            debug_assert_finite!(color);

            if !palette.contains(color) {
                palette.push(*color);
            }
        }
    }

    palette.sort_unstable_by(|hsl1, hsl2| {
        f32::from(hsl1.hue)
            .partial_cmp(&f32::from(hsl2.hue))
            .expect("bug: HSL hue must be finite")
    });

    palette.into_iter().map(hsl_to_rgb).collect()
}
