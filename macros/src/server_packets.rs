use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use crate::client_packets::ParsedPacketsArray;

pub fn create_server_packets_impl(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as ParsedPacketsArray);

    let structs = parsed.0.iter().map(|v| {
        let opcode = &v.id;
        let name = &v.name;

        let attrs = v.attrs.iter().map(|v| {
            let name = &v.name;
            let colon_token = &v.colon_token1;
            let ty = &v.ty;

            quote!(pub #name #colon_token #ty)
        });

        let attrs_write = v.attrs.iter().map(|v| {
            let name = &v.name;
            let ty = &v.ty;
            let endianness = &v.endianness;

            quote!(
                <#ty as OrderedWrite<#endianness>>::write(&self.#name, &mut buf)?
            )
        });

        quote! {
            #[allow(unused)]
            pub struct #name {
                #(
                    #attrs
                ),*
            }

            impl #name {
                fn to_bytes_inner_body(&self) -> Result<Vec<u8>, ::std::io::Error> {
                    let mut buf = Vec::new();
                    #(#attrs_write;)*
                    Ok(buf)
                }

                pub fn to_bytes(&self, session_key: Option<[u8; 40]>, encrypt_data: &mut (usize, u8)) -> Vec<u8> {
                    let inner_buf = self.to_bytes_inner_body().unwrap();
                    let mut outer_buf = Vec::new();
                    debug_assert!(inner_buf.len() <= u16::MAX as usize - 4);
                    outer_buf.write_u16::<BigEndian>(inner_buf.len() as u16 + 2).unwrap();
                    outer_buf.write_u16::<LittleEndian>(#opcode).unwrap();
                    if let Some(session_key) = session_key {
                        for b in &mut outer_buf {
                            let enc = (*b ^ session_key[encrypt_data.0]).wrapping_add(encrypt_data.1);
                            encrypt_data.0 = (encrypt_data.0 + 1) % session_key.len();
                            *b = enc;
                            encrypt_data.1 = enc;
                        }
                    }
                    outer_buf.write_all(&inner_buf).unwrap();
                    outer_buf
                }
            }
        }
    });

    let output = quote! {
        #(
            #structs
        )*
    };

    output.into()
}
