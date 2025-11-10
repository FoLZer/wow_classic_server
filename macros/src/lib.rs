mod client_packets;
mod server_packets;

use proc_macro::TokenStream;

use crate::{
    client_packets::create_client_packets_impl, server_packets::create_server_packets_impl,
};

#[proc_macro]
pub fn create_client_packets(input: TokenStream) -> TokenStream {
    create_client_packets_impl(input)
}

#[proc_macro]
pub fn create_server_packets(input: TokenStream) -> TokenStream {
    create_server_packets_impl(input)
}
