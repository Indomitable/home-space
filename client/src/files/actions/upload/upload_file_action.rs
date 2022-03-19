use yew::prelude::*;

use crate::modal::modal_dialog::{ModalDialog, ModalDialogHeader};
use super::upload_file_component::FileUpload;

#[derive(Properties, PartialEq)]
pub(crate) struct UploadFileActionProps {
    pub parent_id: i64,
    pub supports_open_dialog: bool,
    pub close_action_list: Callback<()>
}

pub(crate) enum UploadFileActionMessages {
    OpenModalDialog,
    CloseModalDialog
}

pub(crate) struct UploadFileAction {
    is_upload_file_modal_open: bool,
}

impl Component for UploadFileAction {
    type Message = UploadFileActionMessages;
    type Properties = UploadFileActionProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            is_upload_file_modal_open: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            UploadFileActionMessages::OpenModalDialog => {
                self.is_upload_file_modal_open = true;
                true
            },
            UploadFileActionMessages::CloseModalDialog => {
                self.is_upload_file_modal_open = false;
                ctx.props().close_action_list.emit(());
                true
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onclick = ctx.link().callback(|_| UploadFileActionMessages::OpenModalDialog);

        let action = html! {
            <a {onclick}>
                <span class="icon-outlined">{"upload_file"}</span>
                <span>{"Upload folder or files"}</span>
            </a>
        };

        let on_files_uploaded = ctx.link().callback(|_| UploadFileActionMessages::CloseModalDialog);

        if self.is_upload_file_modal_open {
            let modal_dilog_header = ModalDialogHeader::Text("Select data to upload.".to_owned());
            let on_backdrop_click = ctx.link().callback(|_| UploadFileActionMessages::CloseModalDialog );

            let UploadFileActionProps { parent_id, supports_open_dialog, .. } = ctx.props();
            
            return html!{
                <>
                    {action}
                    <ModalDialog header={modal_dilog_header} use_backdrop={Some(true)} on_backdrop_click={Some(on_backdrop_click)}>
                        <FileUpload parent_id={parent_id} supports_open_dialog={*supports_open_dialog} {on_files_uploaded} />
                    </ModalDialog>
                </>
            };
        }
        action
    }
}
