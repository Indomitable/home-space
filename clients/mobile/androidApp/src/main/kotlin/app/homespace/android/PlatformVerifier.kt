package app.homespace.android

import android.content.Context

/**
 * Gives the Rust HTTP client's certificate verifier access to Android's trust store. Call it
 * before the first HTTPS request (`clients/mobile-core/src/android.rs`).
 */
object PlatformVerifier {
    init {
        // JNA opens the library for the bindings, but only this registers its JNI functions.
        System.loadLibrary("homespace_core")
    }

    @JvmStatic
    external fun init(context: Context)
}
