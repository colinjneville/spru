#[proc_macro]
pub fn unavailable(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    cfg_select! {
        feature = "report-unavailable" => {
            // TODO
            proc_macro::TokenStream::new()
        }
        _ => proc_macro::TokenStream::new()
    }
}