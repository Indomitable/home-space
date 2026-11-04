//! Android's certificate checks run in the JVM, so `rustls-platform-verifier` needs a handle on
//! it before the first HTTPS request.

use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::jni_mangle;
use jni::objects::{JClass, JObject};

/// `PlatformVerifier.init(context)` in the Android app.
#[jni_mangle("app.homespace.android.PlatformVerifier")]
pub fn init<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    context: JObject<'caller>,
) {
    unowned_env
        .with_env(|env| rustls_platform_verifier::android::init_with_env(env, context))
        .resolve::<ThrowRuntimeExAndDefault>();
}
