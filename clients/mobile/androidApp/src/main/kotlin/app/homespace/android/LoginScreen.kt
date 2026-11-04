package app.homespace.android

import android.content.ActivityNotFoundException
import androidx.browser.customtabs.CustomTabsIntent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.core.net.toUri

/** Shown while a saved session is being tried, and when trying it failed. */
@Composable
fun ResumingScreen(session: Session.Resuming, model: SessionViewModel, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        if (session.error == null) {
            CircularProgressIndicator()
            Text("Resuming your session…")
        } else {
            Text("Could not resume your session", style = MaterialTheme.typography.titleMedium)
            Text(session.error, color = MaterialTheme.colorScheme.error)
            Button(onClick = model::retryResume) { Text("Retry") }
            TextButton(onClick = model::signOut) { Text("Sign out") }
        }
    }
}

@Composable
fun LoginScreen(model: SessionViewModel, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .imePadding()
            .verticalScroll(rememberScrollState())
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Home Space", style = MaterialTheme.typography.headlineMedium)
        Text("Sign in to your files", style = MaterialTheme.typography.bodyLarge)

        val pending = model.pendingSso
        if (pending != null) {
            Text("Signing in with ${pending.name}…")
            Text(
                "Finish in the browser. If you closed it, cancel and try again.",
                style = MaterialTheme.typography.bodySmall,
            )
            OutlinedButton(onClick = model::cancelSso) { Text("Cancel") }
            return@Column
        }

        OutlinedTextField(
            value = model.server,
            onValueChange = model::onServerChanged,
            label = { Text("Server") },
            singleLine = true,
            enabled = !model.busy,
            keyboardOptions = KeyboardOptions(
                capitalization = KeyboardCapitalization.None,
                autoCorrectEnabled = false,
                keyboardType = KeyboardType.Uri,
                imeAction = ImeAction.Next,
            ),
            modifier = Modifier.fillMaxWidth(),
        )

        when (val probe = model.probe) {
            Probe.Checking -> Text("Contacting the server…")
            is Probe.Failed -> {
                Text(probe.message, color = MaterialTheme.colorScheme.error)
                OutlinedButton(onClick = model::probeNow) { Text("Retry") }
            }
            is Probe.Ready -> {
                probe.options.ssoName?.let { name -> SsoButton(name, model) }
                if (probe.options.ssoName != null && probe.options.password) {
                    Text("or", modifier = Modifier.align(Alignment.CenterHorizontally))
                }
                if (probe.options.password) PasswordForm(model)
            }
        }

        model.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    }
}

@Composable
private fun SsoButton(name: String, model: SessionViewModel) {
    val context = LocalContext.current
    Button(
        enabled = !model.busy,
        modifier = Modifier.fillMaxWidth(),
        onClick = {
            val url = model.beginSso(name) ?: return@Button
            try {
                CustomTabsIntent.Builder().build().launchUrl(context, url.toUri())
            } catch (_: ActivityNotFoundException) {
                model.onBrowserFailed()
            }
        },
    ) { Text("Sign in with $name") }
}

@Composable
private fun PasswordForm(model: SessionViewModel) {
    val canSubmit = !model.busy && model.username.isNotBlank() && model.password.isNotEmpty()
    OutlinedTextField(
        value = model.username,
        onValueChange = { model.username = it },
        label = { Text("Username") },
        singleLine = true,
        enabled = !model.busy,
        keyboardOptions = KeyboardOptions(
            capitalization = KeyboardCapitalization.None,
            autoCorrectEnabled = false,
            imeAction = ImeAction.Next,
        ),
        modifier = Modifier
            .fillMaxWidth()
            .semantics { contentType = ContentType.Username },
    )
    OutlinedTextField(
        value = model.password,
        onValueChange = { model.password = it },
        label = { Text("Password") },
        singleLine = true,
        enabled = !model.busy,
        visualTransformation = PasswordVisualTransformation(),
        keyboardOptions = KeyboardOptions(
            keyboardType = KeyboardType.Password,
            imeAction = ImeAction.Go,
        ),
        keyboardActions = KeyboardActions(onGo = {
            if (canSubmit) model.signInWithPassword()
        }),
        modifier = Modifier
            .fillMaxWidth()
            .semantics { contentType = ContentType.Password },
    )
    Button(
        enabled = canSubmit,
        onClick = model::signInWithPassword,
        modifier = Modifier.fillMaxWidth(),
    ) { Text(if (model.busy) "Signing in…" else "Sign in") }
}
