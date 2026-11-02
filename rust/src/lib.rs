mod error;
mod render;
mod render_option;

use jni::objects::AsJArrayRaw;
use render_option::RenderOptions;

use jni::sys::jbyteArray;
use jni::JNIEnv;
use jni::{
    objects::{JClass, JString},
    sys::{jfloat, jint, jlong},
};

use jni_fn::jni_fn;
use resvg::usvg::{ImageRendering, ShapeRendering, TextRendering};

#[macro_export]
macro_rules! trythrow {
    ( $env:ident, $x:expr, $fail_rtn:expr ) => {
        match $x {
            Ok(r) => r,
            Err(e) => {
                let _ = $env.throw(format!("{:?}", e));
                return $fail_rtn;
            }
        }
    };
    ( $env:ident, $x:expr ) => {
        match $x {
            Ok(r) => r,
            Err(e) => {
                let _ = $env.throw(format!("{:?}", e));
                return Default::default();
            }
        }
    };
}

#[macro_export]
macro_rules! destory {
    ( $type:ident, $ptr:ident ) => {
        unsafe {
            {
                let _boxed_ref = Box::from_raw($ptr as *mut $type);
                drop(_boxed_ref);
            }
        }
    };
}

/// Shared borrow of the native object. Callers mutate through its lock.
/// A mutable borrow here would alias across overlapping renders.
#[macro_export]
macro_rules! load_ptr {
    ( $env:ident, $type:ident, $ptr:ident, $fail_rtn:expr ) => {{
        let ptr = $ptr as *const $type;
        let option_ref = unsafe { ptr.as_ref() };
        let Some(shared) = option_ref else {
            let _ = $env.throw_new(
                "java/lang/NullPointerException",
                concat!(stringify!($type), " cannot be null"),
            );
            return $fail_rtn;
        };
        shared
    }};
    ( $env:ident, $type:ident, $ptr:ident ) => {{
        let ptr = $ptr as *const $type;
        let option_ref = unsafe { ptr.as_ref() };
        let Some(shared) = option_ref else {
            let _ = $env.throw_new(
                "java/lang/NullPointerException",
                concat!(stringify!($type), " cannot be null"),
            );
            return Default::default();
        };
        shared
    }};
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsNew(mut env: JNIEnv, _class: JClass, resources_dir: JString) -> jlong {
    let resources_dir: String = trythrow!(env, env.get_string(&resources_dir)).into();
    let opt = RenderOptions::new(resources_dir);
    Box::into_raw(Box::new(opt)) as jlong
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsLoadSystemFonts(mut env: JNIEnv, _class: JClass, ptr: jlong) {
    let opt = load_ptr!(env, RenderOptions, ptr);
    opt.load_system_fonts();
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsLoadFont(mut env: JNIEnv, _class: JClass, ptr: jlong, path: JString) {
    let opt = load_ptr!(env, RenderOptions, ptr);
    let path: String = trythrow!(env, env.get_string(&path)).into();
    opt.try_load_font(&path);
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsLoadFontsDir(mut env: JNIEnv, _class: JClass, ptr: jlong, path: JString) {
    let opt = load_ptr!(env, RenderOptions, ptr);
    let path: String = trythrow!(env, env.get_string(&path)).into();
    opt.load_fonts_dir(&path);
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsSetShapeRendering(mut env: JNIEnv, _class: JClass, ptr: jlong, render_type: jint) {
    let opt = load_ptr!(env, RenderOptions, ptr);

    let t = match render_type {
        0 => ShapeRendering::OptimizeSpeed,
        1 => ShapeRendering::CrispEdges,
        2 => ShapeRendering::GeometricPrecision,
        _ => return,
    };

    opt.set_shape_rendering(t);
}
#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsSetTextRendering(mut env: JNIEnv, _class: JClass, ptr: jlong, render_type: jint) {
    let opt = load_ptr!(env, RenderOptions, ptr);

    let t = match render_type {
        0 => TextRendering::OptimizeSpeed,
        1 => TextRendering::OptimizeLegibility,
        2 => TextRendering::GeometricPrecision,
        _ => return,
    };

    opt.set_text_rendering(t);
}
#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsSetImageRendering(mut env: JNIEnv, _class: JClass, ptr: jlong, render_type: jint) {
    let opt = load_ptr!(env, RenderOptions, ptr);

    let t = match render_type {
        0 => ImageRendering::OptimizeQuality,
        1 => ImageRendering::OptimizeSpeed,
        _ => return,
    };

    opt.set_image_rendering(t);
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsDestroy(_env: JNIEnv, _class: JClass, ptr: jlong) {
    destory!(RenderOptions, ptr);
}

fn convert(
    mut env: JNIEnv,
    options_ptr: jlong,
    svg_data: JString,
    scale: jfloat,
    render_type: render::RenderType,
) -> jbyteArray {
    let mut envc = unsafe { env.unsafe_clone() };
    match std::panic::catch_unwind(move || {
        let scale = if scale <= 0f32 { 1.0 } else { scale };
        let opt = load_ptr!(env, RenderOptions, options_ptr, std::ptr::null_mut());
        let svg_content: String =
            trythrow!(env, env.get_string(&svg_data), std::ptr::null_mut()).into();
        let encoded = trythrow!(
            env,
            opt.with_options(|usvg_opt| {
                render::render(&svg_content, usvg_opt, scale, render_type)
            }),
            std::ptr::null_mut()
        );
        trythrow!(
            env,
            env.byte_array_from_slice(&encoded)
                .map(|d| d.as_jarray_raw()),
            std::ptr::null_mut()
        )
    }) {
        Ok(r) => r,
        Err(_) => {
            let _ = envc.throw("Unexpected Error");
            std::ptr::null_mut()
        }
    }
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn convertToPNG(
    env: JNIEnv,
    _class: JClass,
    options_ptr: jlong,
    svg_data: JString,
    scale: jfloat,
) -> jbyteArray {
    convert(env, options_ptr, svg_data, scale, render::RenderType::Png)
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn convertToJPG(
    env: JNIEnv,
    _class: JClass,
    options_ptr: jlong,
    svg_data: JString,
    scale: jfloat,
    quality: jfloat,
    background_rgb: jint,
) -> jbyteArray {
    let params = render::JpegParams::new(
        render::jpeg_quality_from_f32(quality),
        render::background_from_rgb(background_rgb as u32),
    );
    convert(
        env,
        options_ptr,
        svg_data,
        scale,
        render::RenderType::Jpeg(params),
    )
}
