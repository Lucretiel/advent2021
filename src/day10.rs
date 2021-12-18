use std::iter;

use itertools::Itertools;
use nom::{
    branch::alt,
    character::complete::char,
    combinator::{eof, success},
    multi::fold_many0,
    IResult, Parser,
};
use nom_supreme::{
    error::{BaseErrorKind, ErrorTree, StackContext},
    final_parser::final_parser,
    multi::collect_separated_terminated,
    ParserExt,
};

fn chunk_parser<'a>(start: char, end: char) -> impl Parser<&'a str, (), ErrorTree<&'a str>> {
    fold_many0(parse_chunk, || (), |(), ()| ())
        .terminated(char(end).context("end"))
        .cut()
        .preceded_by(char(start))
}

fn parse_chunk(input: &str) -> IResult<&str, (), ErrorTree<&str>> {
    alt((
        chunk_parser('(', ')').context("p"),
        chunk_parser('{', '}').context("c"),
        chunk_parser('[', ']').context("s"),
        chunk_parser('<', '>').context("a"),
    ))
    .parse(input)
}

fn parse_line(input: &str) -> IResult<&str, (), ErrorTree<&str>> {
    collect_separated_terminated(parse_chunk, success(()), eof).parse(input)
}

fn final_parse_line(input: &str) -> Result<(), ErrorTree<&str>> {
    final_parser(parse_line)(input)
}

#[derive(Debug, Clone, Copy)]
struct ContextView<'a, I> {
    context: &'a [(I, StackContext)],
    parent: Option<&'a ContextView<'a, I>>,
}

impl<'a, I> ContextView<'a, I> {
    fn empty() -> Self {
        Self {
            context: &[],
            parent: None,
        }
    }

    pub fn contains_context(&self, context: &str) -> bool {
        self.context.iter().any(|(_, c)| match *c {
            StackContext::Context(c) => context == c,
            _ => false,
        }) || self
            .parent
            .map(|parent| parent.contains_context(context))
            .unwrap_or(false)
    }

    pub fn iter_context_frames(&self) -> impl Iterator<Item = &'a [(I, StackContext)]> + '_ {
        let mut this = Some(self);

        iter::from_fn(move || {
            let &ContextView {
                parent: next,
                context: frame,
            } = this?;
            this = next;
            Some(frame)
        })
    }

    /// Iterate all contexts, in reverse order
    pub fn iter(&self) -> impl Iterator<Item = (&'a I, &'a StackContext)> + '_ {
        self.iter_context_frames()
            .flat_map(|frame| frame.iter().map(|(loc, ctx)| (loc, ctx)).rev())
    }
}

fn visit_error<I>(
    err: &ErrorTree<I>,
    visitor: &mut dyn for<'a> FnMut(&'a I, &'a BaseErrorKind, ContextView<'a, I>),
) {
    match err {
        ErrorTree::Base { location, kind } => visitor(location, kind, ContextView::empty()),
        ErrorTree::Stack { base, contexts } => visit_error(base, &mut |location, kind, ctx| {
            let ctx = ContextView {
                context: contexts,
                parent: Some(&ctx),
            };

            visitor(location, kind, ctx);
        }),
        ErrorTree::Alt(branches) => branches.iter().for_each(|err| visit_error(err, visitor)),
    }
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    Ok(input
        .lines()
        .map(|line| match final_parse_line(line) {
            Ok(()) => {
                eprintln!("Unexpectedly good line");
                0
            }
            Err(err) => {
                let mut score = 0;

                visit_error(&err, &mut |tail, _, ctx| {
                    if !ctx.contains_context("end") {
                        return;
                    }

                    if let Some(c) = tail.chars().next() {
                        score = match c {
                            ')' => 3,
                            ']' => 57,
                            '}' => 1197,
                            '>' => 25137,
                            _ => 0,
                        };
                    }
                });

                score
            }
        })
        .sum())
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    let mut scores = input
        .lines()
        .filter_map(|line| match final_parse_line(line) {
            Ok(()) => {
                eprintln!("Unexpectedly good line");
                None
            }
            Err(err) => {
                let mut score: Option<i64> = None;

                visit_error(&err, &mut |tail, _, ctx| {
                    if !ctx.contains_context("end") {
                        return;
                    }

                    if tail.is_empty() {
                        score = Some(
                            ctx.iter()
                                .filter_map(|(_, ctx)| match ctx {
                                    StackContext::Context("p") => Some(1),
                                    StackContext::Context("s") => Some(2),
                                    StackContext::Context("c") => Some(3),
                                    StackContext::Context("a") => Some(4),
                                    _ => None,
                                })
                                .enumerate()
                                .map(|(idx, digit)| digit * 5i64.pow(idx as u32))
                                .sum(),
                        )
                    }
                });

                score
            }
        })
        .collect_vec();

    scores.sort_unstable();

    Ok(scores[scores.len() / 2])
}
