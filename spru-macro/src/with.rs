use proc_macro2::TokenStream;
use quote::TokenStreamExt as _;

struct Input {
    interactor: syn::Ident,
    _arrow: syn::Token![=>],
    stmts: Vec<syn::Stmt>,
}

impl syn::parse::Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let interactor = input.parse()?;
        let _arrow = input.parse()?;

        use syn::parse::Parser as _;

        fn parse_stream(
            interactor: &syn::Ident,
            input: syn::parse::ParseStream,
        ) -> syn::Result<TokenStream> {
            let stream_parser = |input: syn::parse::ParseStream| parse_stream(interactor, input);

            let mut output = TokenStream::new();

            while !input.is_empty() {
                let is_tilde = input.peek(syn::Token![~]);
                let mut tt = input
                    .parse::<proc_macro2::TokenTree>()
                    .expect("Non-empty ParseStream must have a TokenTree");

                if is_tilde {
                    let tilde_span = tt.span();

                    let lookahead = input.lookahead1();
                    if lookahead.peek(syn::token::Bracket) {
                        let content;
                        let _bracket = syn::bracketed!(content in input);

                        let stream = stream_parser(&content)?;

                        output.append_all(quote::quote_spanned! { tilde_span =>
                            #interactor.get(#stream)
                        });
                    } else {
                        return Err(lookahead.error());
                    }
                } else {
                    if let proc_macro2::TokenTree::Group(group) = tt {
                        let stream = stream_parser.parse2(group.stream())?;
                        tt = proc_macro2::TokenTree::Group(proc_macro2::Group::new(
                            group.delimiter(),
                            stream,
                        ));
                    }

                    output.append(tt);
                }
            }

            Ok(output)
        }

        let stmts = parse_stream(&interactor, input)?;

        let stmts = syn::Block::parse_within.parse2(stmts)?;

        Ok(Self {
            interactor,
            _arrow,
            stmts,
        })
    }
}

pub(crate) fn fn_with(input: TokenStream) -> syn::Result<TokenStream> {
    let Input {
        interactor: _interactor,
        _arrow,
        stmts,
    } = syn::parse2(input)?;

    Ok(quote::quote! { #(#stmts)* })
}

#[cfg(test)]
mod test {
    #[test]
    fn parse() {
        let expected = quote::quote! {
            let a = interactor.get(root.asdf)?;
            let b = interactor.get(a.b[3])?;
            array[interactor.get(b.c)?]
        };

        let actual = super::fn_with(quote::quote! {
            interactor =>
            let a = ~[root.asdf]?;
            let b = ~[a.b[3]]?;
            array[~[b.c]?]
        })
        .unwrap();

        assert_eq! {
            expected.to_string(),
            actual.to_string(),
        };
    }
}
