use peg::RuleResult;

use super::*;
use crate::nodes::*;

peg::parser!(
    pub grammar program() for str {
        // WHITESPACE
        rule _() =  quiet!{___+}
        rule __() = quiet!{ ____()+ }
        rule ___() = [' ' | '\n' | '\t'] / comment()
        rule ____() = [' ' | '\t']

        rule comment() = "//" #{|i, p| {
            let yunk = i[p..].chars().take_while(|c| *c != '\n').count();

            RuleResult::Matched(p + yunk, ())
        }}

        pub rule program() -> Program =  _? items:item()* { Program(items) }

        rule block() -> Block =
            open:TP(Token::BraceO) _?
            stmts:stmt()* _?
            close:T(Token::BraceC) _?
        { Block { open, stmts, close } }
        / expected!("Block")

        ////////////////////
        //////////////////// ITEMS
        ////////////////////

        rule item() -> Item = item_fn()

        rule item_fn() -> Item =
            unsafe_:(unsafe_:TP(Token::Unsafe) _ { unsafe_ })?
            fn_:TP(Token::Fn) _
            name:ident() _?
            args:fn_args() _?
            ret:fn_ret()? _?
            body:block() _?
        { Item::Function(ItemFunction { fn_, name, args, ret, body }) }
        / expected!("Function")

        ////////////////////
        //////////////////// FUNCTION
        ////////////////////

        rule fn_args() -> FnArgs = ()
            open:T(Token::ParenO) _?
            args:fn_arg() ** (_? "," _?) _?
            close:T(Token::ParenC)
        { FnArgs { open, args, close } }

        rule fn_arg() -> FnArg = ()
            name:ident() _? T(Token::Colon) _? ty:ident()
        { FnArg { name, ty } }

        rule fn_ret() -> Spanned<Ident> = T(Token::Arrow) _? ty:ident() { ty }

        ////////////////////
        //////////////////// STATEMENTS
        ////////////////////

        rule stmt() -> Statement = stmt_let() / stmt_loop() / stmt_while() / stmt_label() / stmt_expr()

        rule stmt_expr() -> Statement =
            e:block() _?
            semi:T(Token::Semi)? _?
        { Statement::Expr(StmtExpr { expr: Expr::Block(e), semi }) }
            /
            e:expr() semi:eol()
        { Statement::Expr(StmtExpr { expr: e, semi }) }

        rule stmt_label() -> Statement =
            squot:TP(Token::SQuot) _?
            name:ident() _?
            colon:T(Token::Colon) _?
        { Statement::Label(StmtLabel { squot, name, colon }) }
        / expected!("Label")

        rule stmt_let() -> Statement =
            let_:T(Token::Let) _
            name:ident() _?
            TP(Token::Equal) _?
            expr:expr() _?
            semi:T(Token::Semi)? _?
        { Statement::Let(StmtLet { let_, name, expr, semi }) }
        / expected!("Let")

        rule stmt_loop() -> Statement =
            loop_:T(Token::Loop) _?
            body:block() _?
        { Statement::Loop(StmtLoop { loop_, body }) }

        rule stmt_while() -> Statement =
            while_:T(Token::While) _?
            cond:expr() _?
            body:block() _?
        { Statement::While(StmtWhile { while_, cond, body }) }

        ////////////////////
        //////////////////// EXPRESSION
        ////////////////////

        rule expr() -> Expr = precedence!{
            name:ident() _? "=" _? y:@ { Expr::Assign(name, Box::from(y)) }
            --
            x:(@) _? "==" _? y:@ { Expr::Binary { kind: ExprBinary::Eq, lhs: x.into(), rhs: y.into() } }
            x:(@) _? "!=" _? y:@ { Expr::Binary { kind: ExprBinary::Ne, lhs: x.into(), rhs: y.into() } }
            x:(@) _? "<=" _? y:@ { Expr::Binary { kind: ExprBinary::Le, lhs: x.into(), rhs: y.into() } }
            x:(@) _? ">=" _? y:@ { Expr::Binary { kind: ExprBinary::Ge, lhs: x.into(), rhs: y.into() } }
            x:(@) _? ">" _? y:@ { Expr::Binary { kind: ExprBinary::Lt, lhs: x.into(), rhs: y.into() } }
            x:(@) _? "<" _? y:@ { Expr::Binary { kind: ExprBinary::Gt, lhs: x.into(), rhs: y.into() } }
            --
            c:expr_stom() _? "(" args:expr() ** (_? "," _?) ")" { Expr::Call(c.into(), args) }
            --
            x:(@) _? "+" _? y:@ { Expr::Binary { kind: ExprBinary::Add, lhs: x.into(), rhs: y.into() } }
            x:(@) _? "-" _? y:@ { Expr::Binary { kind: ExprBinary::Sub, lhs: x.into(), rhs: y.into() } }
            --
            e:expr_stom() { e }
        }
        / expected!("Expression")

        rule expr_stom() -> Expr =
            n:number() { Expr::Number(n) }
            / i:ident() { Expr::Ident(i) }
            / b:block() { Expr::Block(b) }
            / TP(Token::SQuot) _? name:ident() { Expr::Label(name) }
            / quiet!{"("} _? e:expr() _? ")" { e }

        rule number() -> Spanned<i32> =
            start:position!() n:$(quiet!{ ['-']? ['0'..='9']+ })
        {? n.parse().or(Err("i32")).map(|v| Span::new(start, n.len()).of(v)) }
            / expected!("Number")

        ////////////////////
        //////////////////// UTILS
        ////////////////////

        rule eol() -> Option<Spanned<Token>> =
            __? semi:TP(Token::Semi) _? { Some(semi) }
            / __? "\n" { None }

        // Token peek
        rule TP(token: Token) -> Spanned<Token> = tk:#{|input, pos| token::TP(token, input, pos)} { tk }

        // Token expect
        rule T(token: Token) -> Spanned<Token> = tk:#{|input, pos| token::T(token, input, pos)} {? tk }

        rule ident() -> Spanned<Ident> = tk:#{|input, pos| token::ident(input, pos)} {? tk }

    }
);

mod token {
    use super::*;

    #[inline(always)]
    fn token(tk: Token, input: &str, pos: usize) -> Result<(usize, Spanned<Token>), &'static str> {
        let token = tk.as_str();

        let span = Span {
            start: pos,
            len: token.len(),
        };

        if input.get(pos..span.end()).is_none_or(|peek| peek != token) {
            return Err(token);
        }

        // Check if is not part of a bigger word
        if !tk.is_punctuation()
            && input
                .get(span.end()..span.end() + 1)
                .and_then(|peek| peek.chars().next())
                .is_some_and(|c| c.is_ascii_alphanumeric())
        {
            return Err(token);
        }

        Ok((span.end(), span.of(tk)))
    }

    #[allow(non_snake_case)]
    pub fn TP(tk: Token, input: &str, pos: usize) -> RuleResult<Spanned<Token>> {
        match token(tk, input, pos) {
            Ok(res) => RuleResult::Matched(res.0, res.1),
            Err(_) => RuleResult::Failed,
        }
    }

    #[allow(non_snake_case)]
    pub fn T(
        tk: Token,
        input: &str,
        pos: usize,
    ) -> RuleResult<Result<Spanned<Token>, &'static str>> {
        match token(tk, input, pos) {
            Ok(res) => RuleResult::Matched(res.0, Ok(res.1)),
            Err(err) => RuleResult::Matched(pos, Err(err)),
        }
    }

    pub fn ident(input: &str, pos: usize) -> RuleResult<Result<Spanned<Ident>, &'static str>> {
        let ident = input[pos..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

        // first char whould be alphabetic
        if ident
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphabetic())
        {
            return RuleResult::Matched(pos, Err("Identifier"));
        }

        if Token::is_reverved(&ident) {
            return RuleResult::Matched(pos, Err("Identifier"));
        }

        let span = Span {
            start: pos,
            len: ident.len(),
        };

        RuleResult::Matched(span.end(), Ok(span.of(Ident(ident))))
    }
}
