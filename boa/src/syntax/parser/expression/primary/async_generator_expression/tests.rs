use crate::{
    syntax::{
        ast::{
            node::{AsyncGeneratorExpr, Declaration, DeclarationList, Return, StatementList},
            Const,
        },
        parser::tests::check_parser,
    },
    Interner,
};
use boa_interner::Sym;

///checks async generator expression parsing

#[test]
fn check_async_generator_expr() {
    let mut interner = Interner::default();
    check_parser(
        "const add = async function*(){
            return 1;
        };
        ",
        vec![DeclarationList::Const(
            vec![Declaration::new_with_identifier(
                interner.get_or_intern_static("add"),
                Some(
                    AsyncGeneratorExpr::new::<_, _, StatementList>(
                        None,
                        vec![],
                        vec![Return::new::<_, _, Option<Sym>>(Const::from(1), None).into()].into(),
                    )
                    .into(),
                ),
            )]
            .into(),
        )
        .into()],
        &mut interner,
    );
}

#[test]
fn check_nested_async_generator_expr() {
    let mut interner = Interner::default();
    check_parser(
        "const a = async function*() {
            const b = async function*() {
                return 1;
            };
        };
        ",
        vec![DeclarationList::Const(
            vec![Declaration::new_with_identifier(
                interner.get_or_intern_static("a"),
                Some(
                    AsyncGeneratorExpr::new::<_, _, StatementList>(
                        None,
                        vec![],
                        vec![DeclarationList::Const(
                            vec![Declaration::new_with_identifier(
                                interner.get_or_intern_static("b"),
                                Some(
                                    AsyncGeneratorExpr::new::<_, _, StatementList>(
                                        None,
                                        vec![],
                                        vec![Return::new::<_, _, Option<Sym>>(
                                            Const::from(1),
                                            None,
                                        )
                                        .into()]
                                        .into(),
                                    )
                                    .into(),
                                ),
                            )]
                            .into(),
                        )
                        .into()]
                        .into(),
                    )
                    .into(),
                ),
            )]
            .into(),
        )
        .into()],
        &mut interner,
    );
}
