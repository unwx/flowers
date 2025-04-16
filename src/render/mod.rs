use crate::art::flower::Flower;
use crate::art::mosaic::Mosaic;
use crate::render::common::{clear_points, draw_texture};
use image::RgbaImage;

pub(crate) mod common;

pub(crate) fn draw_mosaic(mosaic: &Mosaic, image: &mut RgbaImage) {
    if let Some(area) = mosaic.invisible_area() {
        clear_points(area.flatten(), image);
    }

    for texture in mosaic.textures() {
        draw_texture(texture, image);
    }

    clear_points(mosaic.curve().iter().copied(), image);
}

pub(crate) fn draw_flower(flower: &Flower, image: &mut RgbaImage) {
    for layer in flower.layers() {
        for petal in layer.petals() {
            draw_texture(petal.texture(), image);
            clear_points(petal.curve().iter().copied(), image);
        }
    }

    draw_mosaic(flower.mosaic(), image);
}
