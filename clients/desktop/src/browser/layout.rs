use iced::{
    Alignment, Element, Length, Padding,
    alignment::Horizontal,
    widget::{
        Row, button, center, column, container, mouse_area, opaque, row, space, stack, text,
        text_input,
    },
};

use client_core::{labels::HOME, listing::collapsed_crumbs};
use protocol::Node;

use super::{
    Menu, Message, Page, State, anchor::Anchor, context_menu, context_menu::Placement, dialogs,
    grid, search, sync_queue, syncing, toolbar,
};
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    theme,
};

const PAGE_PADDING: f32 = 16.0;
const SPACING: f32 = 12.0;
const GRID_SPACING: f32 = 4.0;
const USERNAME_SIZE: f32 = 18.0;
/// Lines the username up with the text of the buttons below it.
const USERNAME_INDENT: f32 = 10.0;
const SEARCH_WIDTH: f32 = 280.0;
const BUTTON_PADDING: f32 = 10.0;
const LABEL_SPACING: f32 = 6.0;
const CHAR_WIDTH: f32 = 8.5;
/// Room kept for the sync indicator, whose label changes with the sync state.
const INDICATOR_WIDTH: f32 = 180.0;
const SIGN_OUT: &str = "Sign out";
const SEARCH_PLACEHOLDER: &str = "Search files and folders";
const DROP_PADDING: f32 = 32.0;
const TRASH: &str = "Recycle bin";
const SYNC_QUEUE: &str = "Sync queue";
const DEVICES: &str = "Devices";
const CRUMB_SEPARATOR: &str = "›";
const CRUMB_OVERFLOW: &str = "…";

impl State {
    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.folder.selection.selected(&self.folder.listing.nodes);
        let page = container(self.page(&selected))
            .padding(PAGE_PADDING)
            .width(Length::Fill)
            .height(Length::Fill);

        let mut layers: Vec<Element<'_, Message>> = vec![page.into()];
        if !self.transfers.is_empty() {
            layers.push(
                container(self.transfers.view().map(Message::Transfer))
                    .padding(PAGE_PADDING)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_right(Length::Fill)
                    .align_bottom(Length::Fill)
                    .into(),
            );
        }
        if let Some(menu) = self.menu {
            layers.extend(self.menu_layers(menu, &selected));
        }
        if self.syncing.settings {
            layers.push(modal(
                syncing::settings(self.syncing.status(), self.syncing.notice.as_deref()),
                Message::Sync(syncing::Message::Settings(false)),
            ));
        }
        if let Some(confirm) = &self.syncing.confirm {
            layers.push(modal(
                syncing::confirm(confirm),
                Message::Sync(syncing::Message::Confirm(false)),
            ));
        }
        if let Some(dialog) = &self.dialog {
            layers.push(modal(
                dialog.view().map(Message::Dialog),
                Message::Dialog(dialogs::Message::Cancel),
            ));
        }
        if self.dropping {
            layers.push(self.drop_overlay());
        }
        stack(layers).into()
    }

    /// The header and whatever the browser shows below it: search results, the recycle bin or
    /// the folder.
    fn page<'a>(&'a self, selected: &[&'a Node]) -> Element<'a, Message> {
        let header = self.header();
        if self.search.is_active() {
            column![header, self.search.view()].spacing(SPACING).into()
        } else {
            match &self.page {
                Page::Trash(trash) => column![
                    header,
                    trail(crumb(TRASH, Message::OpenTrash)),
                    trash.view().map(Message::Trash)
                ]
                .spacing(SPACING)
                .into(),
                Page::SyncQueue => column![
                    header,
                    trail(crumb(SYNC_QUEUE, Message::OpenSyncQueue)),
                    sync_queue::view(self.syncing.status())
                ]
                .spacing(SPACING)
                .into(),
                Page::Devices(devices) => column![
                    header,
                    trail(crumb(DEVICES, Message::OpenDevices)),
                    devices.view().map(Message::Devices)
                ]
                .spacing(SPACING)
                .into(),
                Page::Folder => self.folder_page(header, selected),
            }
        }
    }

    fn header(&self) -> Element<'_, Message> {
        let can_go_back = self.can_go_back();
        let (show_username, labelled) = self.header_fit();
        let sign_out = if labelled {
            row![Icon::SignOut.view(ICON_SIZE, Tint::Text), text(SIGN_OUT)]
                .spacing(LABEL_SPACING)
                .align_y(Alignment::Center)
        } else {
            row![Icon::SignOut.view(ICON_SIZE, Tint::Text)]
        };
        let sign_out = button(sign_out)
            .style(theme::tool)
            .on_press(Message::SignOut)
            .into();
        let mut header = row![].spacing(SPACING).align_y(Alignment::Center);
        if show_username {
            header = header.push(
                container(text(self.me.username.as_str()).size(USERNAME_SIZE))
                    .padding(Padding::ZERO.left(USERNAME_INDENT)),
            );
        }
        header
            .push(space::horizontal())
            .push(
                button(Icon::Back.view(
                    ICON_SIZE,
                    if can_go_back {
                        Tint::Text
                    } else {
                        Tint::Disabled
                    },
                ))
                .style(theme::tool)
                .on_press_maybe(can_go_back.then_some(Message::Back)),
            )
            .push(syncing::indicator(self.syncing.status()))
            .push(self.search_box())
            .push(if labelled {
                sign_out
            } else {
                toolbar::tip(sign_out, SIGN_OUT, "")
            })
            .into()
    }

    /// Whether the header has room for the username and for the label of Sign out. The
    /// username goes first, then the label.
    fn header_fit(&self) -> (bool, bool) {
        let width = self.window.width - PAGE_PADDING * 2.0;
        let button = |label_width: f32| BUTTON_PADDING * 2.0 + ICON_SIZE + label_width;
        let label = LABEL_SPACING + SIGN_OUT.chars().count() as f32 * CHAR_WIDTH;
        let username = USERNAME_INDENT
            + self.me.username.chars().count() as f32 * CHAR_WIDTH * USERNAME_SIZE / 14.0;
        // Back, the indicator, the search box, Sign out and the spacer, with a gap between each.
        let rest = button(0.0) + INDICATOR_WIDTH + SEARCH_WIDTH.min(width / 2.0) + SPACING * 4.0;
        let fits = |used: f32| rest + used <= width;
        let labelled = fits(button(label));
        (
            labelled && fits(button(label) + username + SPACING),
            labelled,
        )
    }

    fn folder_page<'a>(
        &'a self,
        header: Element<'a, Message>,
        selected: &[&'a Node],
    ) -> Element<'a, Message> {
        let mut page = column![
            header,
            self.breadcrumbs(),
            toolbar::view(
                selected,
                self.me.lost_and_found_id,
                !self.in_lost_and_found(),
                self.folder.id,
                self.window.width - PAGE_PADDING * 2.0,
            )
        ]
        .spacing(SPACING);
        if let Some(error) = &self.folder.error {
            page = page.push(container(text(error.as_str())).style(theme::error));
        }
        let grid = column![
            grid::header(
                self.folder.selection.coverage(&self.folder.listing.nodes),
                self.folder.sort
            ),
            grid::rows(
                &self.folder.listing.nodes,
                self.folder.id,
                &self.folder.selection,
                self.syncing.status()
            ),
        ]
        .spacing(GRID_SPACING);
        match &self.details {
            Some(details) => page.push(
                row![
                    container(grid).width(Length::Fill),
                    details.view(
                        self.syncing
                            .status()
                            .location(Some(details.node_id()))
                            .map(syncing::display_path)
                    )
                ]
                .spacing(SPACING),
            ),
            None => page.push(grid),
        }
        .into()
    }

    /// The click-away backdrop and the popup menu.
    fn menu_layers<'a>(&'a self, menu: Menu, selected: &[&'a Node]) -> [Element<'a, Message>; 2] {
        let (placement, panel) = match menu {
            Menu::Actions(placement) => (
                placement,
                context_menu::view(
                    selected,
                    self.me.lost_and_found_id,
                    self.syncing.entries(selected),
                ),
            ),
            Menu::Crumbs(placement) => (
                placement,
                context_menu::folders(self.folder.hidden_crumbs()),
            ),
        };
        [
            mouse_area(space().width(Length::Fill).height(Length::Fill))
                .on_press(Message::CloseMenu)
                .on_right_press(Message::CloseMenu)
                .into(),
            context_menu::place(panel, placement),
        ]
    }

    fn drop_overlay(&self) -> Element<'_, Message> {
        let target = self
            .folder
            .listing
            .path
            .last()
            .map_or(HOME, |folder| folder.name.as_str());
        let label = container(text(format!("Drop files to upload to {target}")))
            .padding(DROP_PADDING)
            .style(theme::panel);
        center(label).style(theme::backdrop).into()
    }

    fn search_box(&self) -> Element<'_, Message> {
        let input = text_input(SEARCH_PLACEHOLDER, self.search.query())
            .on_input(|query| Message::Search(search::Message::Changed(query)))
            // Half the page at most, so the username and Sign out keep their room.
            .width(SEARCH_WIDTH.min((self.window.width - PAGE_PADDING * 2.0) / 2.0));
        let mut bar = row![input].spacing(GRID_SPACING).align_y(Alignment::Center);
        if !self.search.query().is_empty() {
            bar = bar.push(
                button(Icon::Close.view(ICON_SIZE, Tint::Muted))
                    .style(theme::tool)
                    .on_press(Message::Search(search::Message::Clear)),
            );
        }
        bar.into()
    }

    fn breadcrumbs(&self) -> Element<'_, Message> {
        let hidden = collapsed_crumbs(self.folder.listing.path.len()).unwrap_or_default();
        let crumbs = self
            .folder
            .listing
            .path
            .iter()
            .enumerate()
            .filter_map(|(index, folder)| {
                if index == hidden.start && !hidden.is_empty() {
                    Some(overflow_crumb())
                } else if hidden.contains(&index) {
                    None
                } else {
                    Some(crumb(folder.name.as_str(), Message::Jump(Some(folder.id))))
                }
            })
            .flatten();
        let mut trail = trail(crumbs);
        if self.folder.loading {
            trail = trail.push(space::horizontal()).push(text("Loading…"));
        }
        trail.into()
    }
}

fn modal<'a>(content: Element<'a, Message>, on_dismiss: Message) -> Element<'a, Message> {
    opaque(mouse_area(center(opaque(content)).style(theme::backdrop)).on_press(on_dismiss))
}

/// The breadcrumb trail: Home, then `crumbs`.
fn trail<'a>(crumbs: impl IntoIterator<Item = Element<'a, Message>>) -> Row<'a, Message> {
    row![
        button(text(HOME))
            .style(theme::tool)
            .on_press(Message::Jump(None))
    ]
    .extend(crumbs)
    .spacing(GRID_SPACING)
    .align_y(Alignment::Center)
}

/// The crumb standing for the hidden folders, whose menu hangs from its left edge.
fn overflow_crumb<'a>() -> [Element<'a, Message>; 2] {
    let button = button(text(CRUMB_OVERFLOW))
        .style(theme::tool)
        // Never sent: the anchor takes the press, and only needs the button to look enabled.
        .on_press(Message::CloseMenu);
    [
        text(CRUMB_SEPARATOR).into(),
        Anchor::new(button)
            .on_press(|bounds| Message::CrumbsMenu(Placement::hanging(bounds, Horizontal::Left)))
            .into(),
    ]
}

/// A separator followed by a link-styled crumb.
fn crumb(label: &str, message: Message) -> [Element<'_, Message>; 2] {
    [
        text(CRUMB_SEPARATOR).into(),
        button(text(label))
            .style(theme::tool)
            .on_press(message)
            .into(),
    ]
}
