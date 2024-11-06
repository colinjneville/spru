use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse2, spanned::Spanned, Arm, Item, LitInt, TypePath};

pub(crate) fn payload_variant_attr(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let item: Item = parse2(item)?;
    let (ident, generics) = match &item {
        Item::Enum(item) => (&item.ident, &item.generics),
        Item::Struct(item) => (&item.ident, &item.generics),
        Item::Union(item) => (&item.ident, &item.generics),
        _ => return Err(syn::Error::new(attr.span(), "payload_variant can only be applied to enums, structs, and unions.")),
    };

    let (impl_generics, type_generics, where_generics) = generics.split_for_impl();

    let attr: Arm = parse2(attr)?;
    let slot = attr.pat;
    let slot_span = slot.span();
    // TODO prohibit suffixes (e.g. `8u32`)
    let slot: LitInt = parse2(quote!(#slot))?;

    let variant = attr.body;
    let variant_span = variant.span();
    let variant: TypePath = parse2(quote!(#variant))?;
    
    let slot_impl = quote_spanned! { slot_span =>
        impl #impl_generics spru_message::payload::Slot<spru_message::payload::slot::Marker<#slot>> for #ident #type_generics
        #where_generics {
            type Variant = #variant;
        }
    };

    let variant_impl = quote_spanned! { variant_span => 
        impl #impl_generics spru_message::payload::Variant<#variant> for #ident #type_generics
        #where_generics {
            type Marker = spru_message::payload::slot::Marker<#slot>;
        }
    };

    let is_self_crate = std::env::var("CARGO_CRATE_NAME").unwrap() == "spru_message";
    let extern_crate_self = if is_self_crate {
        Some(quote!(extern crate self as spru_message;))
    } else {
        None
    };

    Ok(quote! {
        #item
        
        const _: () = {
            #extern_crate_self
            #slot_impl
            #variant_impl
        };
    })
}