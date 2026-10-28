use super::*;

pub(super) const DURATION: f32 = 2.6;
const FLAME_OUT: f32 = 1.;
const LANDING: f32 = 1.7;
const UNSTRAP: f32 = 1.9;

fn pack(x: f32, y: f32, flame: f32, fuel: f32, scale: f32) -> gpui_kit::Div {
    let mut pack = div().absolute().inset_0();
    for offset in [0., 9.] {
        pack = pack
            .child(block(x + offset, y, 7., 20., 0x9e91b5, scale).rounded(px(3. * scale)))
            .child(block(x + offset + 1., y + 2., 3., 13., 0xd5c7e6, scale).rounded_full())
            .child(block(x + offset, y + 8., 7., 3., 0x73d9ee, scale))
            .child(block(x + offset + 1., y + 18., 5., 4., 0x50465f, scale));
        if flame > 0. {
            pack = pack
                .child(
                    block(x + offset, y + 21., 7., flame, 0x74cfea, scale)
                        .rounded_b(px(4. * scale)),
                )
                .child(
                    block(x + offset + 2., y + 21., 3., flame * 0.65, GOLD, scale)
                        .rounded_b(px(2. * scale)),
                );
        }
    }
    pack = pack.child(block(x - 1., y - 7., 18., 4., 0x50465f, scale));
    if fuel > 0. {
        pack = pack.child(block(x, y - 6., 16. * fuel, 2., GOLD, scale));
    } else {
        pack = pack.child(block(x, y - 6., 2., 2., 0xff6b81, scale));
    }
    pack
}

pub(super) fn empty_pack(camera: f32, scale: f32) -> gpui_kit::Div {
    pack(START_X - 52. - camera, ROOFS[0].y - 22., 0., 0., scale)
}

pub(super) fn sequence(time: f32, camera: f32, scale: f32) -> gpui_kit::Div {
    let flight = (time / FLAME_OUT).min(1.);
    let fall = ((time - FLAME_OUT) / (LANDING - FLAME_OUT)).clamp(0., 1.);
    let x = START_X - 150. * (1. - flight).powi(2) - camera;
    let y = ROOFS[0].y - 96. * (1. - fall * fall) - 85. * (1. - flight);
    let squash = (1. - (time - LANDING) / 0.2).clamp(0., 1.);
    let mut scene = div().absolute().inset_0().child(silhouette(
        x - 20.,
        y - 36.,
        1.,
        0.,
        if time < LANDING { 0. } else { squash },
        scale,
    ));
    // Irregular bursts become shorter before the engines stop altogether.
    let flame = if time < FLAME_OUT && (time < 0.45 || !((time * 18.) as usize).is_multiple_of(3)) {
        22. * (1. - flight) + 4.
    } else {
        0.
    };
    let detach = ((time - UNSTRAP) / (DURATION - UNSTRAP)).clamp(0., 1.);
    scene = scene.child(pack(
        x - 14. - 38. * detach,
        y - 35. + 13. * detach,
        flame,
        1. - flight,
        scale,
    ));
    scene
}

pub(super) fn caption() -> gpui_kit::Div {
    v_flex()
        .absolute()
        .bottom(px(22.))
        .w_full()
        .items_center()
        .gap(px(6.))
        .text_color(rgb(GOLD))
        .child(control_prompt("Out of fuel and far from home."))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(0xb9adc9))
                .child("← → move · Space jump"),
        )
}
