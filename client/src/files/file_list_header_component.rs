use yew::prelude::*;

#[function_component(FileListHeader)]
pub fn file_list_header_component() -> Html {
    html!{
        <div class="file-list-header">
            <div>
            </div>
            <div>
                {"Name"}
            </div>
            <div>
                {"Size"}
            </div>
            <div>
                {"Last Modified"}
            </div>
        </div>
    }
}
