use std::sync::{OnceLock, atomic::{AtomicU32, Ordering::Relaxed}};

static DISPLAY_SCALE: AtomicU32 = AtomicU32::new(1.0f32.to_bits());
static SCALE: OnceLock<f32> = OnceLock::new();

#[cfg(target_os = "ios")]
pub fn set_display(w: f64, h: f64) {
    if w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite() {
        DISPLAY_SCALE.store(((w.max(h) / w.min(h) / (16.0 / 9.0)) as f32).clamp(1.0, 2.0).to_bits(), Relaxed);
    }
}

pub fn scale() -> f32 {
    *SCALE.get_or_init(|| {
        let scale = if std::env::var("NX_ULTRAWIDE").as_deref() == Ok("1") {
            f32::from_bits(DISPLAY_SCALE.load(Relaxed))
        } else { 1.0 };
        eprintln!("[gfx] ultrawide aspect scale {scale:.6}");
        scale
    })
}

// These are MK8DX 4.0.1's observed UI vertex programs, not scene or resolve programs.
pub fn is_ui(hash: u64, w: u32, h: u32) -> bool {
    w >= 640 && h >= 360 && w * 9 == h * 16 && matches!(hash,
        0x77264c28fdc0921b | 0x8a167a1501240ee5 | 0xff58c483a97cf8ef |
        0xddb6b360577ab6ff | 0x52454081518e814e | 0x305d4f5c810d7403 |
        0x684ee55e995f1d67 | 0x861a8981f7511caa)
}

pub fn center_x(x: f32, width: f32, scale: f32) -> f32 {
    width * 0.5 + (x - width * 0.5) / scale
}

pub fn cover_y(y: f32, height: f32, scale: f32) -> f32 {
    height * 0.5 + (y - height * 0.5) * scale
}

pub fn starts_menu_art(hash: u64, depth_stencil: u32, texture: Option<(u32, u32, bool)>) -> bool {
    match texture {
        Some((8, 1048, _)) => matches!(hash, 0x77264c28fdc0921b | 0x8a167a1501240ee5) &&
            matches!(menu_depth_state(depth_stencil), 0x40 | 0x48),
        Some(texture) if is_menu_illustration(texture) => hash == 0x77264c28fdc0921b && menu_depth_state(depth_stencil) == 0x40,
        _ => false,
    }
}

// The main menu uses both full-resolution artwork and half-resolution artwork
// (records, amiibo, MKTV, help and Toy-Con). Both have the same overlay passes.
pub fn is_menu_illustration(texture: (u32, u32, bool)) -> bool {
    matches!(texture, (1920, 1080, true) | (960, 540, true))
}

pub fn is_menu_fade(hash: u64, depth_stencil: u32, texture: Option<(u32, u32, bool)>) -> bool {
    matches!(hash, 0x77264c28fdc0921b | 0x8a167a1501240ee5) &&
        menu_depth_state(depth_stencil) == 0x40 && texture == Some((8, 8, true))
}

pub fn starts_intro_art(vs: u64, fs: u64, depth_stencil: u32, texture: Option<(u32, u32, bool)>) -> bool {
    // The opening animation composites its artwork with a separate mask program.
    vs == 0xff58c483a97cf8ef && fs == 0xcc03897b25330f90 &&
        matches!(menu_depth_state(depth_stencil), 0x40 | 0x48) &&
        matches!(texture, Some((1280, 720 | 420, false)))
}

pub fn is_intro_composite(vs: u64, fs: u64) -> bool {
    // This pass samples the already covered title framebuffer, not original art.
    vs == 0x305d4f5c810d7403 && fs == 0x16182a4ea0da75ec
}

pub fn is_menu_backdrop(depth: u32, mask: (u32, u32), background: (u32, u32, u64)) -> bool {
    // Menu masks sample the downscaled screen, which already contains the
    // widescreen background. Applying the UI correction again creates seams.
    matches!(menu_depth_state(depth), 0x40 | 0x48) &&
        matches!(mask, (80, 45 | 39) | (4, 45) | (8, 8) | (16, 16)) &&
        background == (320, 180, 90)
}

pub fn is_frame_copy(vs: u64, fs: u64, depth: u32, color: u32, texture: (u32, u32, u64)) -> bool {
    let (w, h, format) = texture;
    vs == 0x77264c28fdc0921b && fs == 0x5bb31027ff12d02e &&
        menu_depth_state(depth) == 0x40 && color == 0x08030000 &&
        format == 90 && w >= 640 && h >= 360 && w * 9 == h * 16
}

pub fn menu_depth_state(depth_stencil: u32) -> u32 {
    // The renderer decodes flags and depth comparison from the low byte only.
    // Guest state padding changes during transitions (e.g. 0x40 -> 0x3f800040).
    depth_stencil & 0xff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_resolution_menu_art_keeps_the_same_overlay_space() {
        for texture in [(1920, 1080, true), (960, 540, true)] {
            assert!(is_menu_illustration(texture));
            assert!(starts_menu_art(0x77264c28fdc0921b, 0x40, Some(texture)));
        }
        for texture in [(960, 540, false), (1280, 720, false), (170, 105, true), (900, 1024, false)] {
            assert!(!is_menu_illustration(texture));
        }
        assert!(!starts_menu_art(0x77264c28fdc0921b, 0x48, Some((960, 540, true))));
    }

    #[test]
    fn backdrop_masks_use_screen_space_without_a_main_menu_seed() {
        for depth in [0x40, 0x48, 0x500fef48] {
            for mask in [(80, 45), (80, 39), (4, 45), (8, 8), (16, 16)] {
                assert!(is_menu_backdrop(depth, mask, (320, 180, 90)));
            }
        }
        // Ordinary controls, previews and depth-tested geometry retain their
        // existing placement even when they share a menu shader or texture size.
        assert!(!is_menu_backdrop(0x48, (170, 105), (320, 180, 90)));
        assert!(!is_menu_backdrop(0x48, (8, 8), (320, 180, 71)));
        assert!(!is_menu_backdrop(0x48, (8, 8), (1280, 720, 90)));
        assert!(!is_menu_backdrop(0x44, (8, 8), (320, 180, 90)));
    }

    #[test]
    fn opaque_transition_frame_copy_keeps_full_viewport() {
        let vs = 0x77264c28fdc0921b;
        let fs = 0x5bb31027ff12d02e;
        assert!(is_frame_copy(vs, fs, 0x3af2b940, 0x8030000, (1280, 720, 90)));
        assert!(!is_frame_copy(vs, fs, 0x40, 0x8030100, (1280, 720, 90)));
        assert!(!is_frame_copy(vs, fs, 0x48, 0x8030000, (1280, 720, 90)));
        assert!(!is_frame_copy(vs, fs, 0x40, 0x8030000, (1280, 720, 71)));
        assert!(!is_frame_copy(vs, fs, 0x40, 0x8030000, (170, 105, 90)));
    }

    #[test]
    fn track_gradient_starts_cover_but_track_previews_do_not() {
        let hash = 0x8a167a1501240ee5;
        for state in [0x40, 0x48, 0x3f800040] {
            assert!(starts_menu_art(hash, state, Some((8, 1048, true))));
        }
        for texture in [(170, 105, true), (400, 400, true), (1920, 1080, true), (1280, 720, false)] {
            assert!(!starts_menu_art(hash, 0x40, Some(texture)));
        }
        assert!(!starts_menu_art(hash, 0x44, Some((8, 1048, true))));
        let scale = 1.222159;
        let y0 = cover_y(0.0, 1080.0, scale);
        let y1 = cover_y(1080.0, 1080.0, scale);
        assert!(y0 < 0.0 && y1 > 1080.0);
        assert!(((y1 - y0) / 1080.0 - scale).abs() < 0.00001);
    }

    #[test]
    fn intro_framebuffer_is_not_scaled_as_raw_artwork() {
        assert!(is_intro_composite(0x305d4f5c810d7403, 0x16182a4ea0da75ec));
        assert!(!is_intro_composite(0xff58c483a97cf8ef, 0xcc03897b25330f90));
        assert!(!is_intro_composite(0x305d4f5c810d7403, 0x1b73271e7a83233c));
    }

    #[test]
    fn opening_animation_is_artwork_not_a_menu_preview() {
        for state in [0x40, 0x500fef48] {
            for height in [720, 420] {
                assert!(starts_intro_art(0xff58c483a97cf8ef, 0xcc03897b25330f90, state, Some((1280, height, false))));
            }
        }
        assert!(!starts_intro_art(0xff58c483a97cf8ef, 0x1b73271e7a83233c, 0x40, Some((1280, 720, false))));
        assert!(!starts_intro_art(0xff58c483a97cf8ef, 0xcc03897b25330f90, 0x44, Some((1280, 720, false))));
        assert!(!starts_intro_art(0x77264c28fdc0921b, 0x5bb31027ff12d02e, 0x40, Some((1280, 720, false))));
    }

    #[test]
    fn mode_transition_background_and_fade_keep_full_coverage() {
        assert!(starts_menu_art(0x77264c28fdc0921b, 0x48, Some((8, 1048, true))));
        for hash in [0x77264c28fdc0921b, 0x8a167a1501240ee5] {
            assert!(is_menu_fade(hash, 0xac461340, Some((8, 8, true))));
            assert!(!is_menu_fade(hash, 0x500fef48, Some((8, 8, true))));
            assert!(!is_menu_fade(hash, 0xac461340, Some((72, 72, true))));
        }
    }

    #[test]
    fn title_background_does_not_require_gradient_draw() {
        assert!(starts_menu_art(0x77264c28fdc0921b, 0x40, Some((1920, 1080, true))));
        assert!(starts_menu_art(0x77264c28fdc0921b, 0x40, Some((8, 1048, false))));
        // A resolve, preview panel, or ordinary icon must not start a cover pass.
        assert!(!starts_menu_art(0xf8211eae2605bb61, 0x40, Some((1920, 1080, true))));
        assert!(!starts_menu_art(0x77264c28fdc0921b, 0x40, Some((1920, 1080, false))));
        assert!(!starts_menu_art(0x77264c28fdc0921b, 0x500fef48, Some((1920, 1080, true))));
        assert!(!starts_menu_art(0x77264c28fdc0921b, 0x40, Some((170, 105, true))));
    }

    #[test]
    fn transition_padding_does_not_change_background_classification() {
        for state in [0x40, 0x3f800040, 0x500fef40] {
            assert!(starts_menu_art(0x77264c28fdc0921b, state, Some((1920, 1080, true))));
            assert!(starts_menu_art(0x77264c28fdc0921b, state, Some((8, 1048, false))));
            assert_eq!(menu_depth_state(state), 0x40);
        }
        for state in [0x41, 0x42, 0x44, 0x500fef48] {
            assert!(!starts_menu_art(0x77264c28fdc0921b, state, Some((1920, 1080, true))));
        }
    }
}
