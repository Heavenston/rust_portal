
use std::iter::Peekable;
use proc_macro::{ Delimiter, Group, TokenStream, TokenTree, Literal };

pub fn eval_expr_lit(items: &mut Peekable<impl Iterator<Item = TokenTree>>) -> bool {
    match items.next() {
        Some(TokenTree::Literal(lit)) => {
            match format!("{lit}").as_str() {
                "0" => false, "1" => true,
                _ => panic!("Expected 0 or 1 got {lit}"),
            }
        },
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
            eval_expr(&mut g.stream().into_iter().peekable())
        },

        _ => panic!("Expected literal"),
    }
}
pub fn eval_expr_not(items: &mut Peekable<impl Iterator<Item = TokenTree>>) -> bool {
    if items.next_if(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char() == '!')).is_some() {
        !eval_expr_not(items)
    }
    else {
        eval_expr_lit(items)
    }
}
pub fn eval_expr_binop(items: &mut Peekable<impl Iterator<Item = TokenTree>>) -> bool {
    let lhs = eval_expr_not(items);

    if items.next_if(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '=')).is_some() {
        let rhs = eval_expr_not(items);
        return lhs == rhs;
    }

    lhs
}
pub fn eval_expr_and(items: &mut Peekable<impl Iterator<Item = TokenTree>>) -> bool {
    let mut val = eval_expr_binop(items);

    if items.next_if(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '&')).is_some() {
        val = val && eval_expr_and(items);
    }

    val
}
pub fn eval_expr_or(items: &mut Peekable<impl Iterator<Item = TokenTree>>) -> bool {
    let mut val = eval_expr_and(items);

    if items.next_if(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '|')).is_some() {
        val = val || eval_expr_or(items);
    }

    val
}
pub fn eval_expr(items: &mut Peekable<impl Iterator<Item = TokenTree>>) -> bool {
    eval_expr_or(items)
}

pub fn optional_enum_fields(item: TokenStream) -> TokenStream {
    let mut iter = item.into_iter();
    let mut result = TokenStream::new();

    while let Some(token) = iter.next() {
        match token {
            TokenTree::Ident(ident) if format!("{ident}") == "__optional" => {
                let Some(TokenTree::Group(sg)) = iter.next()
                else { panic!("Expected parenteses after __optional") };
                let mut stream = optional_enum_fields(sg.stream()).into_iter()
                    .peekable();
                if eval_expr(&mut stream) {
                    result.extend(stream);
                }
            },

            TokenTree::Ident(ident) if format!("{ident}") == "__eval" => {
                let Some(TokenTree::Group(sg)) = iter.next()
                else { panic!("Expected parenteses after __eval") };
                let mut stream = optional_enum_fields(sg.stream()).into_iter()
                    .peekable();
                result.extend([TokenTree::Literal(Literal::u8_unsuffixed(eval_expr(&mut stream) as u8))]);
            },

            TokenTree::Group(g) => {
                result.extend([TokenTree::Group({
                    let mut ng = Group::new(g.delimiter(), optional_enum_fields(g.stream()));
                    ng.set_span(g.span());
                    ng
                })]);
            },

            token => result.extend([token]),
        }
    }

    result
}
