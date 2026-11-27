use after_effects as ae;

mod dither;
use dither::{Depth, Settings, World, WorldMut};

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Params {
    Style,
    Size,
    Levels,
    Brightness,
    Contrast,
    Mono,
    MonoColor,
    MonoOpacity,
    Background,
    TransparentBg,
}

#[derive(Default)]
struct Plugin {}

ae::define_effect!(Plugin, (), Params);

const MATRICES: [usize; 4] = [2, 4, 8, 16];

fn color_f(p: ae::Pixel8) -> [f32; 3] {
    [p.red as f32 / 255.0, p.green as f32 / 255.0, p.blue as f32 / 255.0]
}

fn read_settings(params: &mut ae::Parameters<Params>, in_data: &InData) -> Result<Settings, Error> {
    let style = params.get(Params::Style)?.as_popup()?.value();
    let size = params.get(Params::Size)?.as_slider()?.value().max(1) as f32;
    let levels = params.get(Params::Levels)?.as_slider()?.value().max(2) as u32;
    let brightness = params.get(Params::Brightness)?.as_float_slider()?.value() as f32 / 100.0;
    let contrast = params.get(Params::Contrast)?.as_float_slider()?.value() as f32;
    let mono = params.get(Params::Mono)?.as_checkbox()?.value();
    let mono_color = color_f(params.get(Params::MonoColor)?.as_color()?.value());
    let mono_opacity = params.get(Params::MonoOpacity)?.as_float_slider()?.value() as f32 / 100.0;
    let background = color_f(params.get(Params::Background)?.as_color()?.value());
    let transparent_bg = params.get(Params::TransparentBg)?.as_checkbox()?.value();

    // Size is given in full-resolution pixels; scale it for Half/Quarter preview.
    let dsx = f32::from(in_data.downsample_x());
    let dsy = f32::from(in_data.downsample_y());

    Ok(Settings {
        matrix: MATRICES[((style.max(1) - 1) as usize).min(MATRICES.len() - 1)],
        cell_x: (size * dsx).round().max(1.0) as usize,
        cell_y: (size * dsy).round().max(1.0) as usize,
        levels,
        brightness,
        contrast,
        mono,
        mono_color,
        mono_opacity,
        background,
        transparent_bg,
    })
}

fn depth_of(layer: &ae::Layer) -> Depth {
    match layer.world_type() {
        ae::aegp::WorldType::U15 => Depth::U16,
        ae::aegp::WorldType::F32 => Depth::F32,
        _ => Depth::U8,
    }
}

fn run(src: &ae::Layer, dst: &mut ae::Layer, s: &Settings) {
    let depth = depth_of(dst);
    let so = src.origin();
    let dor = dst.origin();
    let src_w = World {
        data: src.buffer(),
        stride: src.buffer_stride(),
        width: src.width(),
        height: src.height(),
        origin: (so.h, so.v),
    };
    let (dw, dh, dstride) = (dst.width(), dst.height(), dst.buffer_stride());
    let mut dst_w = WorldMut {
        data: dst.buffer_mut(),
        stride: dstride,
        width: dw,
        height: dh,
        origin: (dor.h, dor.v),
    };
    dither::process(&src_w, &mut dst_w, depth, s);
}

impl AdobePluginGlobal for Plugin {
    fn params_setup(&self, params: &mut ae::Parameters<Params>, _: InData, _: OutData) -> Result<(), Error> {
        params.add(Params::Style, "Style", ae::PopupDef::setup(|f| {
            f.set_options(&["Bayer 2x2", "Bayer 4x4", "Bayer 8x8", "Bayer 16x16"]);
            f.set_default(4);
            f.set_value(f.default());
        }))?;
        params.add(Params::Size, "Size", ae::SliderDef::setup(|f| {
            f.set_valid_min(1);
            f.set_valid_max(256);
            f.set_slider_min(1);
            f.set_slider_max(16);
            f.set_default(2);
            f.set_value(f.default());
        }))?;
        params.add(Params::Levels, "Levels", ae::SliderDef::setup(|f| {
            f.set_valid_min(2);
            f.set_valid_max(64);
            f.set_slider_min(2);
            f.set_slider_max(16);
            f.set_default(3);
            f.set_value(f.default());
        }))?;
        params.add(Params::Brightness, "Brightness", ae::FloatSliderDef::setup(|f| {
            f.set_valid_min(0.0);
            f.set_valid_max(1000.0);
            f.set_slider_min(0.0);
            f.set_slider_max(200.0);
            f.set_default(100.0);
            f.set_precision(0);
            f.set_display_flags(ae::ValueDisplayFlag::PERCENT);
            f.set_value(f.default());
        }))?;
        params.add(Params::Contrast, "Contrast", ae::FloatSliderDef::setup(|f| {
            f.set_valid_min(0.0);
            f.set_valid_max(10.0);
            f.set_slider_min(0.0);
            f.set_slider_max(3.0);
            f.set_default(1.0);
            f.set_precision(2);
            f.set_value(f.default());
        }))?;
        params.add(Params::Mono, "Mono", ae::CheckBoxDef::setup(|f| {
            f.set_default(false);
            f.set_value(f.default());
        }))?;
        params.add(Params::MonoColor, "Mono color", ae::ColorDef::setup(|f| {
            f.set_default(ae::Pixel8 { red: 255, green: 255, blue: 255, alpha: 255 });
            f.set_value(f.default());
        }))?;
        params.add(Params::MonoOpacity, "Mono opacity", ae::FloatSliderDef::setup(|f| {
            f.set_valid_min(0.0);
            f.set_valid_max(100.0);
            f.set_slider_min(0.0);
            f.set_slider_max(100.0);
            f.set_default(100.0);
            f.set_precision(0);
            f.set_display_flags(ae::ValueDisplayFlag::PERCENT);
            f.set_value(f.default());
        }))?;
        params.add(Params::Background, "Mono background", ae::ColorDef::setup(|f| {
            f.set_default(ae::Pixel8 { red: 0, green: 0, blue: 0, alpha: 255 });
            f.set_value(f.default());
        }))?;
        params.add(Params::TransparentBg, "Transparent background", ae::CheckBoxDef::setup(|f| {
            f.set_default(false);
            f.set_value(f.default());
        }))?;
        Ok(())
    }

    fn handle_command(&self, cmd: ae::Command, in_data: InData, mut out_data: OutData, params: &mut ae::Parameters<Params>) -> Result<(), ae::Error> {
        match cmd {
            ae::Command::About => {
                out_data.set_return_msg("Bayer Dither v1.0\rOrdered dithering 2x2 to 16x16.\rPlumcat");
            }
            ae::Command::Render { in_layer, mut out_layer } => {
                let s = read_settings(params, &in_data)?;
                run(&in_layer, &mut out_layer, &s);
            }
            ae::Command::SmartPreRender { mut extra } => {
                let mut req = extra.output_request();
                req.preserve_rgb_of_zero_alpha = 1;
                if let Ok(in_result) = extra.callbacks().checkout_layer(0, 0, &req, in_data.current_time(), in_data.time_step(), in_data.time_scale()) {
                    let _ = extra.union_result_rect(in_result.result_rect.into());
                    let _ = extra.union_max_result_rect(in_result.max_result_rect.into());
                }
            }
            ae::Command::SmartRender { extra } => {
                let cb = extra.callbacks();
                let Some(input_world) = cb.checkout_layer_pixels(0)? else {
                    return Ok(());
                };
                let s = read_settings(params, &in_data)?;
                if let Ok(Some(mut output_world)) = cb.checkout_output() {
                    run(&input_world, &mut output_world, &s);
                }
                cb.checkin_layer_pixels(0)?;
            }
            _ => {}
        }
        Ok(())
    }
}
