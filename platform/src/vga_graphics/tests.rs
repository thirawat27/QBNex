use super::*;

fn graphics_mode_13() -> VGAGraphics {
    let mut gfx = VGAGraphics::new(DosMemory::new());
    gfx.set_screen_mode(13);
    gfx
}

#[test]
fn draw_down_command_uses_y_axis() {
    let mut gfx = graphics_mode_13();
    gfx.draw("C5 M10,10 D4");

    assert_eq!(gfx.get_pixel(10, 10), 5);
    assert_eq!(gfx.get_pixel(10, 14), 5);
    assert_eq!(gfx.get_pixel(14, 10), 0);
}

#[test]
fn draw_move_and_paint_parse_comma_arguments() {
    let mut gfx = graphics_mode_13();
    gfx.line(5, 5, 15, 5, 2);
    gfx.line(15, 5, 15, 15, 2);
    gfx.line(15, 15, 5, 15, 2);
    gfx.line(5, 15, 5, 5, 2);

    gfx.draw("BM10,10 P3,2");

    assert_eq!(gfx.get_pixel(10, 10), 3);
    assert_eq!(gfx.get_pixel(5, 5), 2);
}

#[test]
fn get_image_clamps_negative_coordinates() {
    let mut gfx = graphics_mode_13();
    gfx.pset(0, 0, 7);
    gfx.pset(1, 0, 8);

    let image = gfx.get_image(-20, -20, 1, 0);

    assert_eq!(u16::from_le_bytes([image[0], image[1]]), 2);
    assert_eq!(u16::from_le_bytes([image[2], image[3]]), 1);
    assert_eq!(image[4], 7);
    assert_eq!(image[5], 8);
}

#[test]
fn put_image_applies_raster_operation() {
    let mut gfx = graphics_mode_13();
    gfx.pset(20, 20, 0b0101);

    let sprite = vec![1, 0, 1, 0, 0b0011];
    gfx.put_image(20, 20, &sprite, "XOR");

    assert_eq!(gfx.get_pixel(20, 20), 0b0110);
}

#[test]
fn view_makes_coordinates_relative_to_viewport() {
    let mut gfx = graphics_mode_13();
    gfx.view(10, 10, 20, 20, 0, 0);

    gfx.pset(0, 0, 9);

    assert_eq!(gfx.get_pixel(0, 0), 9);
    assert_eq!(gfx.get_framebuffer()[10 * 320 + 10], 9);
}

#[test]
fn window_maps_logical_coordinates_into_viewport() {
    let mut gfx = graphics_mode_13();
    gfx.view(10, 10, 110, 110, 0, 0);
    gfx.window(-10.0, -10.0, 10.0, 10.0);

    gfx.pset(0, 0, 12);

    assert_eq!(gfx.get_pixel(0, 0), 12);
    assert_eq!(gfx.get_framebuffer()[60 * 320 + 60], 12);
}

#[test]
fn pmap_uses_active_viewport_and_window() {
    let mut gfx = graphics_mode_13();
    gfx.view(10, 10, 110, 110, 0, 0);
    gfx.window(-10.0, -10.0, 10.0, 10.0);

    assert_eq!(gfx.pmap(0.0, 0), 60.0);
    assert_eq!(gfx.pmap(0.0, 1), 60.0);
    assert_eq!(gfx.pmap(60.0, 2), 0.0);
    assert_eq!(gfx.pmap(60.0, 3), 0.0);
}

#[test]
fn circle_respects_window_transform() {
    let mut gfx = graphics_mode_13();
    gfx.view(10, 10, 110, 110, 0, 0);
    gfx.window(-10.0, -10.0, 10.0, 10.0);

    gfx.circle(0, 0, 5, 14);

    assert_eq!(gfx.get_framebuffer()[60 * 320 + 85], 14);
    assert_eq!(gfx.get_framebuffer()[85 * 320 + 60], 14);
}
