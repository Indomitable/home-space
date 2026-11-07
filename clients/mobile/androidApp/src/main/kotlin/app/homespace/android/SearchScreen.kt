package app.homespace.android

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SearchBar
import androidx.compose.material3.SearchBarDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import uniffi.homespace_core.CoreException
import uniffi.homespace_core.Entry
import uniffi.homespace_core.EntryKind
import uniffi.homespace_core.SearchResult

/** How long typing must pause before the query is sent, as in the desktop client. */
private const val DEBOUNCE_MILLIS = 250L

/** Searches all of the user's files and folders by name, wherever they are. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun SearchScreen(
    session: Session.SignedIn,
    model: SessionViewModel,
    onClose: () -> Unit,
    onOpenFolder: (Entry) -> Unit,
    onAction: (Entry, RowAction) -> Unit,
) {
    var query by rememberSaveable { mutableStateOf("") }
    // The last answer stays up until the next one comes, so the list doesn't flicker.
    var results by remember { mutableStateOf<List<SearchResult>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }

    // A new query cancels this, so a late answer can't replace a newer one.
    LaunchedEffect(session.client, query, model.reloadKey) {
        error = null
        if (query.isBlank()) {
            results = null
            return@LaunchedEffect
        }
        delay(DEBOUNCE_MILLIS)
        try {
            results = session.client.search(query.trim())
        } catch (_: CoreException.Unauthorized) {
            model.onSessionEnded()
        } catch (e: CoreException) {
            error = e.fileMessage()
        }
    }

    SearchBar(
        inputField = {
            SearchBarDefaults.InputField(
                query = query,
                onQueryChange = { query = it },
                onSearch = {},
                expanded = true,
                onExpandedChange = { if (!it) onClose() },
                placeholder = { Text("Search files and folders") },
                modifier = Modifier.focusRequester(focus),
                leadingIcon = {
                    IconButton(onClick = onClose) {
                        Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = "Back")
                    }
                },
                trailingIcon = {
                    if (query.isNotEmpty()) {
                        IconButton(onClick = { query = "" }) {
                            Icon(painterResource(R.drawable.ic_close), contentDescription = "Clear search")
                        }
                    }
                },
            )
        },
        expanded = true,
        onExpandedChange = { if (!it) onClose() },
    ) {
        val found = results.orEmpty()
        val notice = when {
            error != null -> error
            query.isBlank() || results == null -> null
            found.isEmpty() -> "No results for “${query.trim()}”"
            found.size == 1 -> "1 result"
            else -> "${found.size} results"
        }
        notice?.let {
            Text(
                it,
                style = MaterialTheme.typography.labelLarge,
                color = if (error != null) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.fillMaxWidth().padding(16.dp),
            )
        }
        LazyColumn(modifier = Modifier.navigationBarsPadding().imePadding()) {
            items(found, key = { it.entry.id }) { result ->
                val entry = result.entry
                EntryRow(
                    entry = entry,
                    selecting = false,
                    isSelected = false,
                    progress = model.downloads[entry.id],
                    canDelete = entry.id != session.user.lostAndFoundId,
                    onClick = {
                        if (entry.kind == EntryKind.FOLDER) {
                            onOpenFolder(entry)
                        } else {
                            model.openFile(session, entry)
                        }
                    },
                    onLongClick = {},
                    onAction = { onAction(entry, it) },
                    location = result.location,
                    highlight = query.trim(),
                )
            }
        }
    }
}
