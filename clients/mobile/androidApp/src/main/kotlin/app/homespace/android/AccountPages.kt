package app.homespace.android

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
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
    TRANSFERS("Transfers"),
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
        else -> AccountPageScreen(current, session, model, onBack = { page = null }, modifier = modifier)
    }
}

@Composable
private fun AccountPageScreen(
    page: AccountPage,
    session: Session.SignedIn,
    model: SessionViewModel,
    onBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var showSync by rememberSaveable { mutableStateOf(false) }
    val back = { if (showSync) showSync = false else onBack() }
    BackHandler(onBack = back)
    Column(modifier = modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            IconButton(onClick = back) {
                Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = "Back")
            }
            Text(if (showSync) "Sync" else page.title, style = MaterialTheme.typography.titleLarge)
        }
        when {
            showSync -> SyncSettingsScreen(session, model)
            page == AccountPage.TRANSFERS -> TransfersScreen(session, model) { showSync = true }
            page == AccountPage.SETTINGS -> ListItem(
                headlineContent = { Text("Sync") },
                supportingContent = { Text("Wi-Fi, background and folders that upload") },
                modifier = Modifier.clickable { showSync = true },
            )
        }
    }
}
