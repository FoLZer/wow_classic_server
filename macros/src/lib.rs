mod client_packets;
mod record;
mod server_packets;
mod tracked;

use proc_macro::TokenStream;

use crate::{
    client_packets::create_client_packets_impl, record::record_impl,
    server_packets::create_server_packets_impl, tracked::tracked_impl,
};

#[proc_macro]
pub fn create_client_packets(input: TokenStream) -> TokenStream {
    create_client_packets_impl(input)
}

#[proc_macro]
pub fn create_server_packets(input: TokenStream) -> TokenStream {
    create_server_packets_impl(input)
}

#[proc_macro_attribute]
pub fn tracked(attr: TokenStream, item: TokenStream) -> TokenStream {
    tracked_impl(attr, item)
}

#[proc_macro_derive(Record)]
pub fn record(input: TokenStream) -> TokenStream {
    record_impl(input)
}
