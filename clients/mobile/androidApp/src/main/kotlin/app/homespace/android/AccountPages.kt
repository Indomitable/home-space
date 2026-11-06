package app.homespace.android

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp

/** The screens the header's menu leads to. */
enum class AccountPage(val title: String) {
    PROFILE("Profile"),
    SETTINGS("Settings"),
}

/** The folder browser and the screens on top of it, which end with the session. */
@Composable
fun SignedInScreen(session: Session.SignedIn, model: SessionViewModel, modifier: Modifier = Modifier) {
    var page by rememberSaveable { mutableStateOf<AccountPage?>(null) }
    // Keeps the scroll position of the list for the way back.
    val saved = rememberSaveableStateHolder()
    when (val current = page) {
        null -> saved.SaveableStateProvider("browse") {
            BrowseScreen(session, model, onOpenPage = { page = it }, modifier = modifier)
        }
        else -> AccountPageScreen(current, onBack = { page = null }, modifier = modifier)
    }
}

/** Empty for now. */
@Composable
private fun AccountPageScreen(page: AccountPage, onBack: () -> Unit, modifier: Modifier = Modifier) {
    BackHandler(onBack = onBack)
    Column(modifier = modifier.fillMaxSize().padding(16.dp)) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            IconButton(onClick = onBack) {
                Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = "Back")
            }
            Text(page.title, style = MaterialTheme.typography.titleLarge)
        }
    }
}
