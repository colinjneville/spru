
vacro_parser::define! { _Dummy: 
    #(_dummy: Scriptable {
        State: state #(state: ScriptableState),
        Ty: ty #(ty: ScriptableType),
    })
}

impl Scriptable {
    pub fn details(&self) -> &ScriptableDetails {
        match self {
            Scriptable::State { state } => &state.details,
            Scriptable::Ty { ty } => &ty.details,
        }
    }
}

impl quote::ToTokens for Scriptable {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let t = match self {
            Self::State { state } => quote::quote! { state #state },
            Self::Ty { ty } => quote::quote! { ty #ty },
        };
        tokens.extend(t);
    }
}

vacro_parser::define! { pub ScriptableState:
    #(details: ScriptableDetails)
    #(members*[,]: StateMemberKind {
        MemberGet: get [ #(get: Get) ],
        MemberSet: set [ #(set: Set) ],
        MemberCreate: create [ #(create: Create) ],
        MemberMethod: method [ #(method: Method) ],
        MemberFunction: function [ #(function: Function) ],
    } )
}

impl From<Get> for StateMemberKind {
    fn from(get: Get) -> Self {
        Self::MemberGet { get }
    }
}

impl From<Set> for StateMemberKind {
    fn from(set: Set) -> Self {
        Self::MemberSet { set }
    }
}

impl From<Create> for StateMemberKind {
    fn from(create: Create) -> Self {
        Self::MemberCreate { create }
    }
}

impl From<Method> for StateMemberKind {
    fn from(method: Method) -> Self {
        Self::MemberMethod { method }
    }
}

impl From<Function> for StateMemberKind {
    fn from(function: Function) -> Self {
        Self::MemberFunction { function }
    }
}

impl quote::ToTokens for ScriptableState {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            details,
            members,
        } = self;

        tokens.extend(quote::quote! {
            #details
            #members
        });
    }
}

impl quote::ToTokens for StateMemberKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let t = match self {
            Self::MemberGet { get } => quote::quote! { get [ #get ] },
            Self::MemberSet { set } => quote::quote! { set [ #set ] },
            Self::MemberCreate { create } => quote::quote! { create [ #create ] },
            Self::MemberMethod { method } => quote::quote! { method [ #method ] },
            Self::MemberFunction { function } => quote::quote! { function [ #function ] },
        };
        tokens.extend(t);
    }
}

vacro_parser::define! { pub ScriptableType:
    #(details: ScriptableDetails)
    #(members*[,]: TypeMemberKind {
        MemberGet: get [ #(get: Get) ],
        MemberMethod: method [ #(method: Method) ],
        MemberFunction: function [ #(function: Function) ],
    })
}

impl From<Get> for TypeMemberKind {
    fn from(get: Get) -> Self {
        Self::MemberGet { get }
    }
}

impl From<Method> for TypeMemberKind {
    fn from(method: Method) -> Self {
        Self::MemberMethod { method }
    }
}

impl From<Function> for TypeMemberKind {
    fn from(function: Function) -> Self {
        Self::MemberFunction { function }
    }
}

impl quote::ToTokens for ScriptableType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            details,
            members,
        } = self;

        tokens.extend(quote::quote! {
            #details
            #members
        });
    }
}

impl quote::ToTokens for TypeMemberKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let t = match self {
            Self::MemberGet { get } => quote::quote! { get [ #get ] },
            Self::MemberMethod { method } => quote::quote! { method [ #method ] },
            Self::MemberFunction { function } => quote::quote! { function [ #function ] },
        };
        tokens.extend(t);
    }
}

vacro_parser::define! { pub ScriptableDetails:
    #(self_type: syn::Type)
    [ #(options: ScriptableOptions) ]
    [ #(generics: syn::Generics) ]
    [ #(where_clause: syn::WhereClause) ]
}

vacro_parser::define! { pub ScriptableOptions:
    #(include?: ScriptableOptionInclude)
    #(partial?: ScriptableOptionPartial)
    #(derive?: ScriptableOptionDerive)
}

impl quote::ToTokens for ScriptableOptions {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            include,
            partial,
            derive,
        } = self;

        tokens.extend(include.to_token_stream());
        tokens.extend(partial.to_token_stream());
        tokens.extend(derive.to_token_stream());
    }
}

impl Default for ScriptableOptions {
    fn default() -> Self {
        Self { 
            include: Default::default(), 
            partial: Default::default(), 
            derive: Default::default(),
        }
    }
}

vacro_parser::define! { pub ScriptableOptionInclude:
    include = [ #(include*[,]: syn::Ident) ],
}

impl quote::ToTokens for ScriptableOptionInclude {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            include,
        } = self;

        tokens.extend(quote::quote! {
            include = [ #include ],
        });
    }
}

vacro_parser::define! { pub ScriptableOptionPartial:
    partial = #(partial: syn::Ident),
}

impl quote::ToTokens for ScriptableOptionPartial {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            partial,
        } = self;

        tokens.extend(quote::quote! {
            partial = #partial,
        });
    }
}

vacro_parser::define! { pub ScriptableOptionDerive:
    derive = [ #(derive*[,]: ScriptableOptionDeriveKind {
        Eq: #{Eq},
    }) ],
}

impl Clone for ScriptableOptionDeriveKind {
    fn clone(&self) -> Self {
        match self {
            Self::Eq => Self::Eq,
        }
    }
}

impl ScriptableOptionDeriveKind {
    pub fn op_name(&self) -> &'static str {
        match self {
            ScriptableOptionDeriveKind::Eq => "eq",
        }
    }
}

impl quote::ToTokens for ScriptableOptionDerive {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            derive,
        } = self;

        tokens.extend(quote::quote! {
            derive = [ #derive ],
        });
    }
}

impl quote::ToTokens for ScriptableOptionDeriveKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ts = match self {
            ScriptableOptionDeriveKind::Eq => quote::quote! { Eq },
        };

        tokens.extend(ts);
    }
}


impl quote::ToTokens for ScriptableDetails {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            self_type,
            options,
            generics,
            where_clause,
        } = self;

        // `WhereClause::parse` requires `where` token, but `WhereClause::to_tokens` omits it if there are not predicates...
        let where_clause = if where_clause.predicates.is_empty() {
            quote::quote! { where }
        } else {
            quote::quote! { #where_clause }
        };

        tokens.extend(quote::quote! {
            #self_type
            [ #options ]
            [ #generics ]
            [ #where_clause ]
        });
    }
}

vacro_parser::define! { pub Get:
    #(name: syn::LitStr)
    [ #(ty: syn::Type) ]
    #(kind: GetKind {
        Fn: #{fn} #(ident: syn::Ident),
        Field: #(ident: syn::Ident),
    })
}

impl quote::ToTokens for Get {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            ty,
            kind,
        } = self;

        tokens.extend(quote::quote! {
            #name
            [ #ty ]
            #kind
        });
    }
}

impl quote::ToTokens for GetKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let t = match self {
            Self::Fn { ident } => quote::quote! { fn #ident },
            Self::Field { ident } => quote::quote! { #ident },
        };
        tokens.extend(t);
    }
}

vacro_parser::define! { pub Set:
    #(name: syn::LitStr)
    [ #(ty: syn::Type) ]
    #(kind: SetKind {
        Fn: #{fn} #(ident: syn::Ident) -> #(ret: syn::Type),
        Field: #(ident: syn::Ident),
    })
}

impl quote::ToTokens for Set {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            ty,
            kind,
        } = self;

        tokens.extend(quote::quote! {
            #name
            [ #ty ]
            #kind
        });
    }
}

impl quote::ToTokens for SetKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let t = match self {
            Self::Fn { ident, ret } => quote::quote! { fn #ident -> #ret },
            Self::Field { ident } => quote::quote! { #ident },
        };
        tokens.extend(t);
    }
}

vacro_parser::define! { pub Method:
    #(name: syn::LitStr)
    [ #(ident: syn::Ident) ]
    [ #(ret: syn::Type) ]
    // Not used for TypeMethod
    [ #(actions*[,]: syn::Type) ]
    [ #(params*[,]: syn::FnArg) ]
}

impl quote::ToTokens for Method {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            ident,
            ret,
            actions,
            params,
        } = self;

        tokens.extend(quote::quote! {
            #name
            [ #ident ]
            [ #ret ]
            [ #actions ]
            [ #params ]
        });
    }
}

vacro_parser::define! { pub Create:
    #(name: syn::LitStr)
    [ #(ident: syn::Ident) ]
    [ #(action: syn::Type) ]
    [ #(params*[,]: syn::FnArg) ]
}

impl quote::ToTokens for Create {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            ident,
            action,
            params,
        } = self;

        tokens.extend(quote::quote! {
            #name
            [ #ident ]
            [ #action ]
            [ #params ]
        });
    }
}

vacro_parser::define! { pub Function:
    #(name: syn::LitStr)
    [ #(ident: syn::Ident) ]
    [ #(ret: syn::Type) ]
    [ #(params*[,]: syn::FnArg) ]
}

impl quote::ToTokens for Function {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            ident,
            ret,
            params,
        } = self;

        tokens.extend(quote::quote! {
            #name
            [ #ident ]
            [ #ret ]
            [ #params ]
        });
    }
}
