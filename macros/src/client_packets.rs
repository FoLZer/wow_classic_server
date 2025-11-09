use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, LitInt, Token, braced, parse_macro_input, punctuated::Punctuated};

struct ParsedInputPacket {
    name: Ident,
    id: LitInt,
    brace_token: syn::token::Brace,
    attrs: Punctuated<ParsedPacketAttribute, Token![,]>,
}

impl syn::parse::Parse for ParsedInputPacket {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            name: input.parse()?,
            id: input.parse()?,
            brace_token: braced!(content in input),
            attrs: content.parse_terminated(ParsedPacketAttribute::parse, Token![,])?,
        })
    }
}

struct ParsedPacketAttribute {
    name: Ident,
    colon_token1: Token![:],
    ty: syn::Type,
    colon_token2: Token![:],
    endianness: syn::Type,
}

impl syn::parse::Parse for ParsedPacketAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            colon_token1: input.parse()?,
            ty: input.parse()?,
            colon_token2: input.parse()?,
            endianness: input.parse()?,
        })
    }
}

struct ParsedPacketsArray(Punctuated<ParsedInputPacket, Token![,]>);

impl syn::parse::Parse for ParsedPacketsArray {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self(
            input.parse_terminated(ParsedInputPacket::parse, Token![,])?,
        ))
    }
}

pub fn create_client_packets_impl(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as ParsedPacketsArray);

    let generated_enum_values = parsed.0.iter().map(|v| {
        let name = &v.name;
        quote!(#name(#name))
    });

    let structs = parsed.0.iter().map(|v| {
        let name = &v.name;

        let attrs = v.attrs.iter().map(|v| {
            let name = &v.name;
            let colon_token = &v.colon_token1;
            let ty = &v.ty;

            quote!(#name #colon_token #ty)
        });

        let attrs_read = v.attrs.iter().map(|v| {
            let name = &v.name;
            let ty = &v.ty;
            let endianness = &v.endianness;

            quote!(
                #name: <#ty as OrderedRead<#endianness>>::from_reader(cursor)?
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
                fn from_reader(cursor: &mut ::std::io::Cursor<&[u8]>) -> ::std::result::Result<Self, ::std::io::Error> {
                    Ok(Self {
                        #(#attrs_read),*
                    })
                }
            }
        }
    });

    let decode_function_match_statements = parsed.0.iter().map(|v| {
        let name = &v.name;
        let opcode = &v.id;
        quote!(#opcode => Ok(ClientPacket::#name(#name::from_reader(cursor).map_err(ParseError::Io)?)))
    });

    let output = quote! {
        #(
            #structs
        )*

        pub enum ClientPacket {
            #(#generated_enum_values),*
        }

        fn decode_packet_inner(cursor: &mut ::std::io::Cursor<&[u8]>, opcode: u32) -> ::std::result::Result<ClientPacket, ParseError> {
            match opcode {
                #(#decode_function_match_statements,)*
                _ => {
                    return Err(ParseError::InvalidOpcode(opcode));
                }
            }
        }
    };

    output.into()
}
