// Reference theme for the Material 3 redesign (docs/designs/mobile/android).
// Not compiled — copy into clients/mobile/androidApp when implementing.
// Brand scheme: SchemeFidelity from seed #2563EB (material-color-utilities 0.3.0),
// so primaryContainer is exactly the web accent.

package app.homespace.android.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

val BrandLight: ColorScheme = lightColorScheme(
    primary = Color(0xFF004AC6), onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFF2563EB), onPrimaryContainer = Color(0xFFEEEFFF),
    secondary = Color(0xFF495C95), onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFDBE1FF), onSecondaryContainer = Color(0xFF00184B),
    tertiary = Color(0xFF943700), onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFFFDBCD), onTertiaryContainer = Color(0xFF360F00),
    error = Color(0xFFBA1A1A), onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFFDAD6), onErrorContainer = Color(0xFF93000A),
    background = Color(0xFFFAF8FF), onBackground = Color(0xFF191B23),
    surface = Color(0xFFFAF8FF), onSurface = Color(0xFF191B23),
    surfaceVariant = Color(0xFFDFE1F4), onSurfaceVariant = Color(0xFF434655),
    surfaceContainerLowest = Color(0xFFFFFFFF), surfaceContainerLow = Color(0xFFF3F3FE),
    surfaceContainer = Color(0xFFEDEDF9), surfaceContainerHigh = Color(0xFFE7E7F3),
    surfaceContainerHighest = Color(0xFFE1E2ED),
    outline = Color(0xFF737686), outlineVariant = Color(0xFFC3C6D7),
    inverseSurface = Color(0xFF2E3039), inverseOnSurface = Color(0xFFF0F0FB),
    inversePrimary = Color(0xFFB4C5FF), scrim = Color(0xFF000000),
)

val BrandDark: ColorScheme = darkColorScheme(
    primary = Color(0xFFB4C5FF), onPrimary = Color(0xFF002A78),
    primaryContainer = Color(0xFF2563EB), onPrimaryContainer = Color(0xFFEEEFFF),
    secondary = Color(0xFFB4C5FF), onSecondary = Color(0xFF182D63),
    secondaryContainer = Color(0xFF33467E), onSecondaryContainer = Color(0xFFDBE1FF),
    tertiary = Color(0xFFFFB596), onTertiary = Color(0xFF581E00),
    tertiaryContainer = Color(0xFF7A2C00), onTertiaryContainer = Color(0xFFFFDBCD),
    error = Color(0xFFFFB4AB), onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A), onErrorContainer = Color(0xFFFFDAD6),
    background = Color(0xFF11131B), onBackground = Color(0xFFE1E2ED),
    surface = Color(0xFF11131B), onSurface = Color(0xFFE1E2ED),
    surfaceVariant = Color(0xFF434655), onSurfaceVariant = Color(0xFFC3C6D7),
    surfaceContainerLowest = Color(0xFF0C0E16), surfaceContainerLow = Color(0xFF191B23),
    surfaceContainer = Color(0xFF1D1F27), surfaceContainerHigh = Color(0xFF282A32),
    surfaceContainerHighest = Color(0xFF32343D),
    outline = Color(0xFF8D90A0), outlineVariant = Color(0xFF434655),
    inverseSurface = Color(0xFFE1E2ED), inverseOnSurface = Color(0xFF2E3039),
    inversePrimary = Color(0xFF0053DB), scrim = Color(0xFF000000),
)

/** Colours Material 3 has no role for. `success` only ever sits beside the words "Up to date" / "Done". */
@Immutable
data class ExtendedColors(
    val success: Color,
    val successContainer: Color,
    val onSuccessContainer: Color,
)

private val ExtendedLight = ExtendedColors(Color(0xFF15803D), Color(0xFFC8F2D3), Color(0xFF00391B))
private val ExtendedDark = ExtendedColors(Color(0xFF5FD39A), Color(0xFF00522A), Color(0xFFC8F2D3))

val LocalExtendedColors = staticCompositionLocalOf { ExtendedLight }

@Composable
fun HomeSpaceTheme(
    dark: Boolean = isSystemInDarkTheme(),
    /** The "Use system colours" setting; only honoured on Android 12+. */
    useSystemColours: Boolean = false,
    content: @Composable () -> Unit,
) {
    val context = LocalContext.current
    val scheme = when {
        useSystemColours && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        dark -> BrandDark
        else -> BrandLight
    }
    androidx.compose.runtime.CompositionLocalProvider(
        LocalExtendedColors provides if (dark) ExtendedDark else ExtendedLight,
    ) {
        // Typography and shapes: the Material 3 defaults (Roboto, baseline type scale).
        MaterialTheme(colorScheme = scheme, content = content)
    }
}
