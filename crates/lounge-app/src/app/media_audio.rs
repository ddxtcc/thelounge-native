use gpui::{Bounds, Pixels, px};
use gstreamer as gst;
use gstreamer::prelude::*;

pub(super) fn start_audio(url: &str) -> Result<gst::Pipeline, String> {
    gst::init().map_err(|error| error.to_string())?;
    let pipeline = gst::ElementFactory::make("playbin")
        .property("uri", url)
        .build()
        .map_err(|error| error.to_string())?
        .downcast::<gst::Pipeline>()
        .map_err(|_| "audio pipeline was not a playbin".to_string())?;
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|error| error.to_string())?;
    Ok(pipeline)
}

pub(super) fn audio_playing(pipeline: &gst::Pipeline) -> bool {
    pipeline.state(gst::ClockTime::ZERO).1 == gst::State::Playing
}

pub(super) fn audio_muted(pipeline: &gst::Pipeline) -> bool {
    pipeline.property::<bool>("mute")
}

pub(super) fn audio_event(pipeline: &gst::Pipeline) -> Option<Result<(), String>> {
    let bus = pipeline.bus()?;
    while let Some(message) = bus.pop() {
        match message.view() {
            gst::MessageView::Eos(_) => return Some(Ok(())),
            gst::MessageView::Error(error) => {
                return Some(Err(error.error().to_string()));
            }
            _ => {}
        }
    }
    None
}

pub(super) fn finish_audio(pipeline: &gst::Pipeline) {
    let _ = pipeline.set_state(gst::State::Paused);
    let _ = pipeline.seek_simple(
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
        gst::ClockTime::ZERO,
    );
}

pub(super) fn format_clock(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn fraction_at(bounds: Bounds<Pixels>, x: Pixels) -> Option<f32> {
    if bounds.size.width <= px(0.) {
        return None;
    }
    Some(((x - bounds.origin.x) / bounds.size.width).clamp(0., 1.))
}

pub(super) fn seek_audio(pipeline: &gst::Pipeline, bounds: Bounds<Pixels>, x: Pixels) {
    let Some(fraction) = fraction_at(bounds, x) else {
        return;
    };
    let Some(duration) = pipeline.query_duration::<gst::ClockTime>() else {
        return;
    };
    let target =
        gst::ClockTime::from_nseconds((duration.nseconds() as f64 * fraction as f64) as u64);
    let _ = pipeline.seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, target);
}

pub(super) fn set_audio_volume(pipeline: &gst::Pipeline, bounds: Bounds<Pixels>, x: Pixels) {
    let Some(fraction) = fraction_at(bounds, x) else {
        return;
    };
    pipeline.set_property("volume", f64::from(fraction));
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Bounds, point, px, size};

    #[test]
    fn pointer_fraction_is_clamped() {
        let bounds = Bounds::new(point(px(10.), px(0.)), size(px(100.), px(1.)));
        assert_eq!(fraction_at(bounds, px(60.)), Some(0.5));
        assert_eq!(fraction_at(bounds, px(0.)), Some(0.0));
        assert_eq!(fraction_at(bounds, px(200.)), Some(1.0));
        assert_eq!(
            fraction_at(
                Bounds::new(point(px(0.), px(0.)), size(px(0.), px(1.))),
                px(1.)
            ),
            None
        );
    }
}
