use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn record_impl(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = ast.ident;
    let syn::Data::Struct(data) = ast.data else {
        panic!("This derive macro should only be used on structs")
    };
    let mut v = Vec::new();
    for field in data.fields {
        let name = field.ident.unwrap();
        let ty = field.ty;
        v.push(quote! {
            #name: <#ty as dbc_reader::structs::Record>::from_reader(reader, _cstring_reader)?
        })
    }
    quote! {
        impl dbc_reader::structs::Record for #name {
            fn from_reader<R: std::io::Read>(reader: &mut R, _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>) -> Result<Self, std::io::Error> where Self: Sized {
                Ok(Self {
                    #(#v),*
                })
            }
        }
    }
    .into()
}
