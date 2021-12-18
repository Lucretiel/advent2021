use std::collections::HashMap;

use anyhow::{bail, Context};
use itertools::{Itertools, MinMaxResult};
use nom::{
    character::complete::{line_ending, multispace0, multispace1, satisfy},
    combinator::success,
    sequence::pair,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::collect_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

use crate::library::{Counter, IterExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Chem {
    name: char,
}

fn parse_chem(input: &str) -> IResult<&str, Chem, ErrorTree<&str>> {
    satisfy(|c: char| c.is_ascii_uppercase())
        .map(|name| Chem { name })
        .parse(input)
}

fn parse_polymer(input: &str) -> IResult<&str, Polymer, ErrorTree<&str>> {
    collect_separated_terminated(parse_chem, success(()), line_ending)
        .map(|chems: Vec<Chem>| chems.into_iter().collect())
        .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rule {
    pattern: (Chem, Chem),
    insert: Chem,
}

fn parse_rule(input: &str) -> IResult<&str, Rule, ErrorTree<&str>> {
    pair(parse_chem, parse_chem)
        .context("pattern")
        .terminated(tag(" -> "))
        .and(parse_chem.context("insert"))
        .map(|(pattern, insert)| Rule { pattern, insert })
        .parse(input)
}

fn parse_rule_list<T: Extend<Rule> + Default>(input: &str) -> IResult<&str, T, ErrorTree<&str>> {
    collect_separated_terminated(parse_rule, line_ending, multispace0.all_consuming()).parse(input)
}

#[derive(Debug, Default, Clone)]
struct RuleSet {
    rules: HashMap<(Chem, Chem), Chem>,
}

impl Extend<Rule> for RuleSet {
    fn extend<T: IntoIterator<Item = Rule>>(&mut self, iter: T) {
        self.rules
            .extend(iter.into_iter().map(|rule| (rule.pattern, rule.insert)))
    }
}

fn parse_problem(input: &str) -> IResult<&str, (Polymer, RuleSet), ErrorTree<&str>> {
    parse_polymer
        .context("template")
        .terminated(multispace1)
        .and(parse_rule_list.context("rule list"))
        .parse(input)
}

fn final_parse_problem(input: &str) -> Result<(Polymer, RuleSet), ErrorTree<Location>> {
    final_parser(parse_problem)(input)
}

#[derive(Debug, Clone, Default)]
struct Polymer {
    pairs: Counter<(Chem, Chem)>,
    counts: Counter<Chem>,
}

impl FromIterator<Chem> for Polymer {
    fn from_iter<T: IntoIterator<Item = Chem>>(iter: T) -> Self {
        let mut this = Self::default();
        let mut iter = iter.into_iter();

        let mut prev = match iter.next() {
            Some(chem) => chem,
            None => return this,
        };

        this.counts.add_one(prev);

        iter.for_each(|next| {
            this.counts.add_one(next);
            this.pairs.add_one((prev, next));
            prev = next;
        });

        this
    }
}

impl Polymer {
    fn apply_rules(self, rules: &RuleSet) -> anyhow::Result<Self> {
        let pairs = self.pairs;
        let mut counts = self.counts;

        pairs
            .iter_counts()
            .map(|(&(a, b), count)| {
                rules
                    .rules
                    .get(&(a, b))
                    .with_context(|| format!("no matching rule {:?}", (a, b)))
                    .map(|&new| ((a, new, b), count))
            })
            .use_oks(move |insertions| {
                let mut pairs = Counter::new();

                insertions.for_each(|((a, new, b), count)| {
                    counts.add(new, count);
                    pairs.add((a, new), count);
                    pairs.add((new, b), count);
                });

                Polymer { pairs, counts }
            })
    }
}

fn solve(input: &str, count: usize) -> anyhow::Result<usize> {
    let (chem, rules) = final_parse_problem(input).context("parse error")?;

    let final_chem = (0..count).try_fold(chem, |chem, step| {
        chem.apply_rules(&rules)
            .with_context(|| format!("failure at step {}", step + 1))
    })?;

    let minmax = final_chem
        .counts
        .iter_counts()
        .map(|(_, count)| count)
        .minmax();

    Ok(match minmax {
        MinMaxResult::NoElements => bail!("No chemicals!"),
        MinMaxResult::OneElement(_) => 0,
        MinMaxResult::MinMax(min, max) => max - min,
    })
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    solve(input, 10)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    solve(input, 40)
}
