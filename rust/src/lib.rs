mod error;
mod render;
mod render_option;

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

#[macro_export]
macro_rules! load_ptr {
    ( $env:ident, $type:ident, $ptr:ident, $fail_rtn:expr ) => {{
        let ptr = $ptr as *mut $type;
        let option_mut_ref = unsafe { ptr.as_mut() };
        let Some(mut_ref) = option_mut_ref else {
            let _ = $env.throw_new(
                "java/lang/NullPointerException",
                concat!(stringify!($type), " cannot be null"),
            );
            return $fail_rtn;
        };
        mut_ref
    }};
    ( $env:ident, $type:ident, $ptr:ident ) => {{
        let ptr = $ptr as *mut $type;
        let option_mut_ref = unsafe { ptr.as_mut() };
        let Some(mut_ref) = option_mut_ref else {
            let _ = $env.throw_new(
                "java/lang/NullPointerException",
                concat!(stringify!($type), " cannot be null"),
            );
            return Default::default();
        };
        mut_ref
    }};
}

#[macro_export]
macro_rules! catch_panic {
    ( $x:expr, $on_fail:expr ) => {
        match std::panic::catch_unwind(|| {
            $x
        }) {
            Ok(r) => r,
            Err(_) => {
                $on_fail
            }
        }
    };
}


#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsNew(env: JNIEnv, _class: JClass, resources_dir: JString) -> jlong {
    let resources_dir: String = trythrow!(env, env.get_string(resources_dir)).into();
    let opt = RenderOptions::new(resources_dir);
    Box::into_raw(Box::new(opt)) as jlong
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsLoadSystemFonts(env: JNIEnv, _class: JClass, ptr: jlong) {
    let opt = load_ptr!(env, RenderOptions, ptr);
    opt.load_system_fonts();
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsLoadFont(env: JNIEnv, _class: JClass, ptr: jlong, path: JString) {
    let opt = load_ptr!(env, RenderOptions, ptr);
    let path: String = trythrow!(env, env.get_string(path)).into();
    opt.try_load_font(&path);
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsLoadFontsDir(env: JNIEnv, _class: JClass, ptr: jlong, path: JString) {
    let opt = load_ptr!(env, RenderOptions, ptr);
    let path: String = trythrow!(env, env.get_string(path)).into();
    opt.load_fonts_dir(&path);
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsSetShapeRendering(env: JNIEnv, _class: JClass, ptr: jlong, render_type: jint) {
    let opt = load_ptr!(env, RenderOptions, ptr);

    let t = match render_type {
        0 => ShapeRendering::OptimizeSpeed,
        1 => ShapeRendering::CrispEdges,
        2 => ShapeRendering::GeometricPrecision,
        _ => return,
    };

    opt.shape_rendering = t;
}
#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsSetTextRendering(env: JNIEnv, _class: JClass, ptr: jlong, render_type: jint) {
    let opt = load_ptr!(env, RenderOptions, ptr);

    let t = match render_type {
        0 => TextRendering::OptimizeSpeed,
        1 => TextRendering::OptimizeLegibility,
        2 => TextRendering::GeometricPrecision,
        _ => return,
    };

    opt.text_rendering = t;
}
#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsSetImageRendering(env: JNIEnv, _class: JClass, ptr: jlong, render_type: jint) {
    let opt = load_ptr!(env, RenderOptions, ptr);

    let t = match render_type {
        0 => ImageRendering::OptimizeQuality,
        1 => ImageRendering::OptimizeSpeed,
        _ => return,
    };

    opt.image_rendering = t;
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn RenderOptionsDestroy(_env: JNIEnv, _class: JClass, ptr: jlong) {
    destory!(RenderOptions, ptr);
}

#[jni_fn("me.aloic.ResvgJNI")]
pub fn convertToPNG(
    env: JNIEnv,
    _class: JClass,
    options_ptr: jlong,
    svg_data: JString, // SVG 数据（强制必须UTF-8）
    scale: jfloat,
) -> jbyteArray {
    catch_panic!(
        {
            let scale = if scale <= 0f32 { 1.0 } else { scale };
            let opt = load_ptr!(env, RenderOptions, options_ptr, std::ptr::null_mut());
            let svg_content: String = trythrow!(env, env.get_string(svg_data), std::ptr::null_mut()).into();
            let png_data = trythrow!(env, render::render(&svg_content, &opt.get_options(), scale), std::ptr::null_mut());
            trythrow!(env, env.byte_array_from_slice(&png_data), std::ptr::null_mut())
        },
        {
            let _ = env.throw("Unexpected Error");
            std::ptr::null_mut()
        }
    )
}
