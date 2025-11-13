use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{
    AngleBracketedGenericArguments, DeriveInput, GenericArgument, Ident, Path, PathArguments,
    PathSegment, Type, TypePath, parse_macro_input,
    punctuated::Punctuated,
    token::{Gt, Lt},
};

pub fn tracked_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as DeriveInput);
    let syn::Data::Struct(ref mut data) = ast.data else {
        panic!("This derive macro should only be used on structs")
    };

    let mut field_names = Vec::new();
    for field in &mut data.fields {
        field_names.push(field.ident.clone().unwrap());

        field.ty = Type::Path(TypePath {
            qself: None,
            path: Path {
                leading_colon: None,
                segments: Punctuated::from_iter([
                    PathSegment {
                        ident: Ident::new("crate", Span::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident::new("tracked_field", Span::call_site()),
                        arguments: PathArguments::None,
                    },
                    PathSegment {
                        ident: Ident::new("TrackedField", Span::call_site()),
                        arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: Lt(Span::call_site()),
                            args: Punctuated::from_iter([GenericArgument::Type(field.ty.clone())]),
                            gt_token: Gt(Span::call_site()),
                        }),
                    },
                ]),
            },
        });
    }

    let name = ast.ident.clone();
    let s = ast.into_token_stream();

    quote! {
        #s

        impl crate::tracked_field::ClientUpdatable for #name {
            fn write_update_block(
                &self,
                mask_bits: &mut bit_vec::BitVec<u32>,
                values_buf: &mut Vec<u32>,
            ) {
                #(
                    self.#field_names.write(mask_bits, values_buf);
                )*
            }

            fn write_full_update_block(
                &self,
                mask_bits: &mut bit_vec::BitVec<u32>,
                values_buf: &mut Vec<u32>,
            ) {
                #(
                    self.#field_names.write_forced(mask_bits, values_buf);
                )*
            }
        }
    }
    .into()
}
