//! `ColorFix` — port verbatim `windows/src/types/ColorFix.cpp`.
//!
//! Oklab color nudging (`GetPerceivableColor`): khi fg/bg quá gần nhau
//! (ΔEOK² < `minSquaredDistance`), nudge L của `color` đủ xa khỏi
//! `reference` để distinguishable. Used bởi `RenderSettings::
//! get_attribute_colors` / `get_attribute_underline_color` khi
//! `RenderModeFlags::ALWAYS_DISTINGUISHABLE` / `INDEXED_DISTINGUISHABLE`
//! bật (port WT `Feature_AdjustIndistinguishableText` — retail build
//! always-on; ta không implement runtime feature flag probe, mặc định
//! enabled, gating bằng `RenderModeFlags`).
//!
//! Credit: OKlab transform của Björn Ottosson
//! (<https://bottosson.github.io/posts/oklab/>) — public domain / MIT.
//! `cbrt_est` mirror WT fast cube-root estimator (max err 6.7e-4 trong
//! [0,1]).
//!
//! Encoding: input/output color là packed ABGR (`a<<24 | b<<16 | g<<8 | r`),
//! layout của `settings::Color`. Giống COLORREF ở 3 byte thấp
//! (`r | g<<8 | b<<16`); byte cao = alpha (ABGR) vs 0 (COLORREF).
//! Port `colorrefToLinear` đọc `(c) & 0xff` = R, `(c>>8)&0xff` = G,
//! `(c>>16)&0xff` = B — không đổi.

#![allow(clippy::excessive_precision)]
#![allow(clippy::doc_lazy_continuation)]

/// sRGB → linear RGB LUT (8-bit, 256 entry). WT `ColorFix.cpp:30-45` dùng
/// hex-float literals (`0x1.3e4568p-12f`); Rust stable **chưa** support
/// hex float literal (nightly `hex_float_literal` only), nên convert sang
/// decimal f32 đầy đủ precision round-trip (giá trị bit-identical với C++).
pub static SRGB_TO_RGB_LUT: [f32; 256] = [
    0.0,
    0.0003035269910469651,
    0.0006070539820939302,
    0.0009105809731408954,
    0.0012141079641878605,
    0.0015176349552348256,
    0.0018211619462817907,
    0.002124688820913434,
    0.002428215928375721,
    0.002731743035838008,
    0.0030352699104696512,
    0.003346535610035062,
    0.0036765069235116243,
    0.004024717025458813,
    0.0043914420530200005,
    0.004776953253895044,
    0.005181516986340284,
    0.005605391692370176,
    0.006048832554370165,
    0.006512091029435396,
    0.006995410192757845,
    0.007499031722545624,
    0.008023192174732685,
    0.00856812484562397,
    0.009134056977927685,
    0.009721217676997185,
    0.01032982300966978,
    0.010960093699395657,
    0.011612244881689548,
    0.01228648703545332,
    0.012983030639588833,
    0.013702080585062504,
    0.014443843625485897,
    0.015208514407277107,
    0.015996292233467102,
    0.016807375475764275,
    0.017641952261328697,
    0.01850021816790104,
    0.019382361322641373,
    0.020288562402129173,
    0.021219009533524513,
    0.022173883393406868,
    0.023153364658355713,
    0.024157630279660225,
    0.02518685720860958,
    0.026241222396492958,
    0.027320891618728638,
    0.0284260381013155,
    0.02955683320760727,
    0.030713440850377083,
    0.03189602866768837,
    0.03310476243495941,
    0.03433980792760849,
    0.03560131415724754,
    0.036889445036649704,
    0.038204364478588104,
    0.039546236395835876,
    0.040915198624134064,
    0.0423114113509655,
    0.04373502731323242,
    0.045186202973127365,
    0.04666508361697197,
    0.04817182198166847,
    0.04970656335353851,
    0.051269467920064926,
    0.05286065489053726,
    0.054480280727148056,
    0.05612849444150925,
    0.05780543386936188,
    0.05951124057173729,
    0.06124607101082802,
    0.06301002949476242,
    0.06480327993631363,
    0.06662595272064209,
    0.06847818195819855,
    0.07036010921001434,
    0.07227186113595963,
    0.07421357929706573,
    0.07618539035320282,
    0.07818742841482162,
    0.0802198275923729,
    0.08228271454572678,
    0.08437621593475342,
    0.08650046586990356,
    0.08865560591220856,
    0.09084173291921616,
    0.0930589810013771,
    0.09530748426914215,
    0.09758736193180084,
    0.09989874064922333,
    0.10224174708127975,
    0.10461649298667908,
    0.10702311247587204,
    0.1094617173075676,
    0.1119324341416359,
    0.11443538218736649,
    0.11697067320346832,
    0.11953843384981155,
    0.12213879823684692,
    0.1247718408703804,
    0.12743769586086273,
    0.13013648986816406,
    0.13286833465099335,
    0.13563334941864014,
    0.1384316235780716,
    0.14126330614089966,
    0.1441284865140915,
    0.14702728390693665,
    0.14995980262756348,
    0.15292616188526154,
    0.15592646598815918,
    0.15896086394786835,
    0.16202940046787262,
    0.16513222455978394,
    0.16826939582824707,
    0.17144109308719635,
    0.17464739084243774,
    0.1778884083032608,
    0.1811642348766327,
    0.18447498977184296,
    0.18782076239585876,
    0.19120167195796967,
    0.19461780786514282,
    0.19806930422782898,
    0.2015562355518341,
    0.20507870614528656,
    0.20863685011863708,
    0.21223072707653046,
    0.2158605307340622,
    0.2195262312889099,
    0.22322797775268555,
    0.22696588933467865,
    0.23074007034301758,
    0.23455065488815308,
    0.23839765787124634,
    0.24228119850158691,
    0.24620139598846436,
    0.2501583695411682,
    0.25415217876434326,
    0.25818291306495667,
    0.26225072145462036,
    0.2663556635379791,
    0.2704978585243225,
    0.2746773660182953,
    0.2788943350315094,
    0.28314879536628723,
    0.2874408960342407,
    0.29177069664001465,
    0.29613831639289856,
    0.3005438446998596,
    0.304987370967865,
    0.30946895480155945,
    0.31398874521255493,
    0.3185468316078186,
    0.32314324378967285,
    0.3277781307697296,
    0.33245158195495605,
    0.33716365694999695,
    0.34191444516181946,
    0.3467040956020355,
    0.3515326976776123,
    0.3564002513885498,
    0.3613068759441376,
    0.36625269055366516,
    0.37123778462409973,
    0.37626221776008606,
    0.3813261091709137,
    0.3864295184612274,
    0.3915725648403168,
    0.39675530791282654,
    0.40197786688804626,
    0.4072403013706207,
    0.4125427007675171,
    0.4178851544857025,
    0.42326775193214417,
    0.4286905527114868,
    0.4341537058353424,
    0.4396572411060333,
    0.44520124793052673,
    0.4507858455181122,
    0.45641106367111206,
    0.4620770514011383,
    0.4677838385105133,
    0.4735315442085266,
    0.479320228099823,
    0.48514997959136963,
    0.49102088809013367,
    0.4969330430030823,
    0.5028865933418274,
    0.5088814496994019,
    0.51491779088974,
    0.5209956765174866,
    0.5271152257919312,
    0.5332764983177185,
    0.5394796133041382,
    0.5457245707511902,
    0.5520114898681641,
    0.5583404898643494,
    0.5647116303443909,
    0.5711249113082886,
    0.5775805115699768,
    0.5840784907341003,
    0.590618908405304,
    0.5972018837928772,
    0.6038274168968201,
    0.6104956269264221,
    0.6172066330909729,
    0.6239604353904724,
    0.630757212638855,
    0.6375969648361206,
    0.6444797515869141,
    0.6514056921005249,
    0.6583748459815979,
    0.6653873324394226,
    0.6724432110786438,
    0.6795425415039062,
    0.6866854429244995,
    0.6938719153404236,
    0.7011020183563232,
    0.7083759307861328,
    0.7156936526298523,
    0.7230552434921265,
    0.7304608821868896,
    0.7379105687141418,
    0.7454043626785278,
    0.7529423236846924,
    0.7605246305465698,
    0.7681512832641602,
    0.7758223414421082,
    0.7835379242897034,
    0.7912980318069458,
    0.7991028428077698,
    0.8069523572921753,
    0.8148466944694519,
    0.8227858543395996,
    0.830769956111908,
    0.8387991189956665,
    0.8468732833862305,
    0.854992687702179,
    0.8631572723388672,
    0.8713672161102295,
    0.8796223402023315,
    0.8879231810569763,
    0.8962693810462952,
    0.9046613574028015,
    0.9130986928939819,
    0.921582043170929,
    0.9301108717918396,
    0.9386858940124512,
    0.9473065733909607,
    0.9559735059738159,
    0.9646862745285034,
    0.973445475101471,
    0.9822505712509155,
    0.9911022186279297,
    1.0,
];

/// `saturate(f)` — port `ColorFix.cpp:51-54`. Clamp [0,1].
#[inline]
fn saturate(f: f32) -> f32 {
    f.clamp(0.0, 1.0)
}

/// `cbrtf_est(a)` — port `ColorFix.cpp:58-83`. Fast cube-root estimator:
/// FP32 bit-hack (`u/3 + 709921077`) + 1 round Newton. Max err < 6.7e-4
/// for `a` in [0,1]. Mirror WT determinism (không `f32::cbrt` libm).
#[inline]
fn cbrt_est(a: f32) -> f32 {
    // `f32::to_bits`/`from_bits` thay `union FP32` (Rust idiomatic,
    // không UB). Bit-hack giữ nguyên: u/3 + 709921077.
    let mut u = a.to_bits();
    u = u.wrapping_div(3).wrapping_add(709_921_077);
    let x = f32::from_bits(u);
    // 1 round Newton: (1/3) * (a/(x²) + 2x).
    (1.0 / 3.0) * (a / (x * x) + (x + x))
}

/// OKlab color space — port verbatim `ColorFix.cpp:88-137`.
mod oklab {
    /// OKlab L/a/b.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Lab {
        pub l: f32,
        pub a: f32,
        pub b: f32,
    }

    /// Linear sRGB.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Rgb {
        pub r: f32,
        pub g: f32,
        pub b: f32,
    }

    /// `linear_srgb_to_oklab` (`ColorFix.cpp:104-119`). Coeff verbatim.
    #[inline]
    pub fn linear_srgb_to_oklab(c: Rgb) -> Lab {
        let l = 0.4122214708f32 * c.r + 0.5363325363f32 * c.g + 0.0514459929f32 * c.b;
        let m = 0.2119034982f32 * c.r + 0.6806995451f32 * c.g + 0.1073969566f32 * c.b;
        let s = 0.0883024619f32 * c.r + 0.2817188376f32 * c.g + 0.6299787005f32 * c.b;

        let l_ = super::cbrt_est(l);
        let m_ = super::cbrt_est(m);
        let s_ = super::cbrt_est(s);

        Lab {
            l: 0.2104542553f32 * l_ + 0.7936177850f32 * m_ - 0.0040720468f32 * s_,
            a: 1.9779984951f32 * l_ - 2.4285922050f32 * m_ + 0.4505937099f32 * s_,
            b: 0.0259040371f32 * l_ + 0.7827717662f32 * m_ - 0.8086757660f32 * s_,
        }
    }

    /// `oklab_to_linear_srgb` (`ColorFix.cpp:121-136`). Coeff verbatim.
    #[inline]
    pub fn oklab_to_linear_srgb(c: Lab) -> Rgb {
        let l_ = c.l + 0.3963377774f32 * c.a + 0.2158037573f32 * c.b;
        let m_ = c.l - 0.1055613458f32 * c.a - 0.0638541728f32 * c.b;
        let s_ = c.l - 0.0894841775f32 * c.a - 1.2914855480f32 * c.b;

        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        Rgb {
            r: 4.0767416621f32 * l - 3.3077115913f32 * m + 0.2309699292f32 * s,
            g: -1.2684380046f32 * l + 2.6097574011f32 * m - 0.3413193965f32 * s,
            b: -0.0041960863f32 * l - 0.7034186147f32 * m + 1.7076147010f32 * s,
        }
    }
}

/// `colorrefToLinear(c)` — port `ColorFix.cpp:143-149`. ABGR layout: low
/// byte = R, `>>8` = G, `>>16` = B (giống COLORREF).
#[inline]
fn colorref_to_linear(c: u32) -> oklab::Rgb {
    // `c >> 0` mirror verbatim WT C++ `(c) & 0xff` (R channel) — giữ symmetry
    // với `>>8`/`>>16`; clippy `identity_op` allow local.
    #[allow(clippy::identity_op)]
    let r = SRGB_TO_RGB_LUT[((c >> 0) & 0xff) as usize];
    let g = SRGB_TO_RGB_LUT[((c >> 8) & 0xff) as usize];
    let b = SRGB_TO_RGB_LUT[((c >> 16) & 0xff) as usize];
    oklab::Rgb { r, g, b }
}

/// `linearToColorref(c)` — port `ColorFix.cpp:153-162`. Linear → sRGB
/// companding (gamma 2.4) + `lrintf` rounding (half-to-even). Rust
/// `round_ties_even()` ≡ `lrintf` (C99 round-half-to-even).
#[inline]
fn linear_to_colorref(c: oklab::Rgb) -> u32 {
    let r = saturate(c.r);
    let g = saturate(c.g);
    let b = saturate(c.b);
    let r = srgb_compand(r) * 255.0;
    let g = srgb_compand(g) * 255.0;
    let b = srgb_compand(b) * 255.0;
    let ri = r.round_ties_even() as i32 as u32;
    let gi = g.round_ties_even() as i32 as u32;
    let bi = b.round_ties_even() as i32 as u32;
    ri | (gi << 8) | (bi << 16)
}

#[inline]
fn srgb_compand(linear: f32) -> f32 {
    if linear > 0.0031308 {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    } else {
        12.92 * linear
    }
}

/// `GetPerceivableColor(color, reference, minSquaredDistance)` — port
/// verbatim `ColorFix.cpp:167-210`. ΔEOK² = dl² + da² + db²; nếu
/// `≥ minSquaredDistance` → return `color` (giữ alpha byte). Else nudge
/// `colorOklab.l = referenceOklab.l ± deltaL` (sign theo color-vs-ref L),
/// clamp [0,1] (flip sign nếu oos), convert lại sRGB, preserve alpha.
pub fn get_perceivable_color(color: u32, reference: u32, min_squared_distance: f32) -> u32 {
    let reference_oklab = oklab::linear_srgb_to_oklab(colorref_to_linear(reference));
    let color_oklab = oklab::linear_srgb_to_oklab(colorref_to_linear(color));

    let dl = reference_oklab.l - color_oklab.l;
    let da = reference_oklab.a - color_oklab.a;
    let db = reference_oklab.b - color_oklab.b;
    let dl2 = dl * dl;
    let da2 = da * da;
    let db2 = db * db;

    let distance = dl2 + da2 + db2;
    if distance >= min_squared_distance {
        return color;
    }

    let mut delta_l = (min_squared_distance - da2 - db2).sqrt();
    if color_oklab.l < reference_oklab.l {
        delta_l = -delta_l;
    }

    let mut new_l = reference_oklab.l + delta_l;
    if new_l < 0.0 || new_l > 1.0 {
        new_l = reference_oklab.l - delta_l;
    }
    let new_color = oklab::Lab {
        l: new_l,
        a: color_oklab.a,
        b: color_oklab.b,
    };
    linear_to_colorref(oklab::oklab_to_linear_srgb(new_color)) | (color & 0xff000000)
}

/// `AdjustLightness(color, delta)` — port `ColorFix.cpp:212-217`. OKlab L +=
/// delta, clamp [0,1], preserve alpha.
pub fn adjust_lightness(color: u32, delta: f32) -> u32 {
    let mut lab = oklab::linear_srgb_to_oklab(colorref_to_linear(color));
    lab.l = saturate(lab.l + delta);
    linear_to_colorref(oklab::oklab_to_linear_srgb(lab)) | (color & 0xff000000)
}

/// `GetLightness(color)` — port `ColorFix.cpp:219-222`. OKlab L of `color`.
pub fn get_lightness(color: u32) -> f32 {
    oklab::linear_srgb_to_oklab(colorref_to_linear(color)).l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_lut_length_256() {
        assert_eq!(SRGB_TO_RGB_LUT.len(), 256);
        assert!((SRGB_TO_RGB_LUT[0] - 0.0).abs() < 1e-6);
        assert!((SRGB_TO_RGB_LUT[255] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cbrt_est_within_6e4_of_std_cbrt() {
        // WT comment: max err < 6.7e-4 cho a in [0,1].
        for &a in &[0.0f32, 0.1, 0.25, 0.5, 0.9, 1.0] {
            let est = cbrt_est(a);
            let truth = a.cbrt();
            assert!(
                (est - truth).abs() < 6.7e-4,
                "cbrt_est({a}) = {est}, std cbrt = {truth}, err = {}",
                (est - truth).abs()
            );
        }
    }

    #[test]
    fn test_linear_srgb_oklab_roundtrip() {
        // Tolerance ±2 channel do roundtrip dùng `cbrt_est` (fast cube-root
        // estimator, không `f32::cbrt`) — sai số ~6.7e-4 trên mỗi cube-root
        // amplify qua oklab fwd/inv matrix → channel các màu saturated sRGB
        // (như pure green) lệch tới ~6 (e.g. green R 0 → 6). Mirror WT C++
        // `ColorFix.cpp` cùng estimator, cùng sai số — đây không bug.
        let colors: [u32; 5] = [
            0xFF000000, // black
            0xFFFFFFFF, // white
            0xFF0000FF, // red (ABGR: byte0=R=0xFF, byte1=G=0, byte2=B=0, byte3=A=0xFF).
            0xFF00FF00, // green (byte1=G=0xFF)
            0xFFFFFF00, // blue (byte2=B=0xFF)
        ];
        for abgr in colors {
            let lin = colorref_to_linear(abgr);
            let lab = oklab::linear_srgb_to_oklab(lin);
            let back_lin = oklab::oklab_to_linear_srgb(lab);
            let back_abgr = linear_to_colorref(back_lin) | (abgr & 0xff000000);
            let r_orig = (abgr & 0xff) as i32;
            let g_orig = ((abgr >> 8) & 0xff) as i32;
            let b_orig = ((abgr >> 16) & 0xff) as i32;
            let r_back = (back_abgr & 0xff) as i32;
            let g_back = ((back_abgr >> 8) & 0xff) as i32;
            let b_back = ((back_abgr >> 16) & 0xff) as i32;
            assert!((r_orig - r_back).abs() <= 7, "R {r_orig} -> {r_back}");
            assert!((g_orig - g_back).abs() <= 7, "G {g_orig} -> {g_back}");
            assert!((b_orig - b_back).abs() <= 7, "B {b_orig} -> {b_back}");
        }
    }

    #[test]
    fn test_get_lightness_black_white_red() {
        // black → 0; white → 1; red (R=0xFF, ABGR 0xFF0000FF) → ~0.5
        let l_black = get_lightness(0xFF000000);
        let l_white = get_lightness(0xFFFFFFFF);
        let l_red = get_lightness(0xFF0000FF);
        assert!(l_black.abs() < 0.02, "black L = {l_black}");
        assert!((l_white - 1.0).abs() < 0.02, "white L = {l_white}");
        // OKlab L của pure red ≈ 0.6279 (Ottosson دستور); tolerance 0.05.
        assert!((l_red - 0.6279).abs() < 0.05, "red L = {l_red}");
    }

    #[test]
    fn test_get_perceivable_color_far_colors_pass_through() {
        // red (0xFF0000FF) vs green (0xFF00FF00): ΔEOK² ≫ 0.25.
        let color = 0xFF0000FFu32;
        let reference = 0xFF00FF00u32;
        let out = get_perceivable_color(color, reference, 0.25);
        assert_eq!(out, color, "far colors pass through unchanged");
    }

    #[test]
    fn test_get_perceivable_color_identical_colors_nudges() {
        let color = 0xFF555555u32;
        let reference = 0xFF555555u32;
        let out = get_perceivable_color(color, reference, 0.25);
        assert_ne!(out, reference, "identical colors must be nudged");
        // alpha byte preserved
        assert_eq!(out & 0xff000000, 0xff000000);
    }

    #[test]
    fn test_get_perceivable_color_preserves_alpha_byte() {
        let color = 0x80123456u32; // alpha 0x80
        let reference = 0xFF000000u32;
        let out = get_perceivable_color(color, reference, 0.25);
        assert_eq!(out & 0xff000000, 0x80000000, "alpha byte preserved");
    }

    #[test]
    fn test_get_perceivable_color_darker_than_ref_darkens() {
        // color darker than ref → sign -deltaL → result L < ref L.
        let color = 0xFF080808u32; // very dark
        let reference = 0xFF808080u32; // mid
        let out = get_perceivable_color(color, reference, 0.25);
        let l_out = get_lightness(out);
        let l_ref = get_lightness(reference);
        assert!(
            l_out < l_ref,
            "darker color should darken further: l_out={l_out} vs l_ref={l_ref}"
        );
    }
}
