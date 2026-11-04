// Plugins are declared here once so every module shares the same classloader and versions.
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.compose.compiler) apply false
}
