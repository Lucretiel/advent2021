use std::{
    cell::UnsafeCell,
    cmp,
    collections::{hash_map, HashMap},
    hash::Hash,
    iter::FusedIterator,
    mem, ops,
    str::FromStr,
    sync::atomic::{self, AtomicBool},
};

use enum_map::MaybeUninit;
use num::Num;
use rayon::prelude::*;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
enum State<T, const N: usize> {
    Begin,
    Buffered([T; N]),
    Done,
}

impl<T, const N: usize> State<T, N> {
    fn take(&mut self) -> Self {
        mem::replace(self, State::Done)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Windows<I: Iterator, const N: usize> {
    iter: I,
    state: State<I::Item, N>,
}

impl<I: Iterator, const N: usize> Iterator for Windows<I, N>
where
    I::Item: Clone,
{
    type Item = [I::Item; N];

    fn next(&mut self) -> Option<Self::Item> {
        let buffer = match self.state.take() {
            State::Begin => brownstone::try_build_iter(&mut self.iter)?,
            State::Buffered(buffer) => buffer,
            State::Done => return None,
        };

        if let Some(next) = self.iter.next() {
            self.state = State::Buffered(brownstone::build_iter(
                buffer[1..].iter().cloned().chain(Some(next)),
            ))
        }

        Some(buffer)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.state {
            State::Begin => {
                let (min, max) = self.iter.size_hint();
                (
                    min.saturating_sub(N - 1),
                    max.map(|max| max.saturating_sub(N - 1)),
                )
            }
            State::Buffered(_) => {
                let (min, max) = self.iter.size_hint();
                (
                    min.saturating_add(1),
                    max.and_then(|max| max.checked_add(1)),
                )
            }
            State::Done => (0, Some(0)),
        }
    }
}

impl<I: Iterator, const N: usize> FusedIterator for Windows<I, N> where I::Item: Clone {}

impl<I: ExactSizeIterator, const N: usize> ExactSizeIterator for Windows<I, N>
where
    I::Item: Clone,
{
    fn len(&self) -> usize {
        match self.state {
            State::Begin => self.iter.len().saturating_sub(N - 1),
            State::Buffered(_) => self.iter.len() + 1,
            State::Done => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Chunks<I, const N: usize> {
    iter: I,
}

impl<I: Iterator, const N: usize> Iterator for Chunks<I, N> {
    type Item = [I::Item; N];

    fn next(&mut self) -> Option<Self::Item> {
        brownstone::try_build_iter(&mut self.iter)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iter.size_hint();

        (
            min.checked_div(N).unwrap_or(usize::MAX),
            max.and_then(|max| max.checked_div(N)),
        )
    }
}

impl<I: FusedIterator, const N: usize> FusedIterator for Chunks<I, N> {}

impl<I: DoubleEndedIterator + ExactSizeIterator, const N: usize> DoubleEndedIterator
    for Chunks<I, N>
{
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.iter.len() % N != 0 {
            let _ = self.iter.next_back();
        }

        brownstone::try_build_iter(self.iter.by_ref().rev()).map(|mut item| {
            item.reverse();
            item
        })
    }
}

#[derive(Debug)]
pub struct UseOksAdapter<'a, I, E> {
    iter: I,
    error: &'a mut Result<(), E>,
}

impl<I: Iterator<Item = Result<T, E>>, T, E> Iterator for UseOksAdapter<'_, I, E> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.error.as_ref().ok()?;
        self.iter
            .next()?
            .map_err(|err| {
                *self.error = Err(err);
            })
            .ok()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match *self.error {
            Err(_) => (0, Some(0)),
            Ok(()) => {
                let (_, max) = self.iter.size_hint();
                (0, max)
            }
        }
    }
}

impl<I, T, E> FusedIterator for UseOksAdapter<'_, I, E>
where
    I: Iterator<Item = Result<T, E>>,
    I: FusedIterator,
{
}

pub trait IterExt: Iterator + Sized {
    fn streaming_windows<const N: usize>(self) -> Windows<Self, N>
    where
        Self::Item: Clone,
    {
        Windows {
            iter: self,
            state: State::Begin,
        }
    }

    fn streaming_chunks<const N: usize>(self) -> Chunks<Self, N> {
        Chunks { iter: self }
    }

    fn use_oks<T, U, E, F>(self, body: F) -> Result<U, E>
    where
        Self: Iterator<Item = Result<T, E>>,
        F: for<'a> FnOnce(UseOksAdapter<'a, Self, E>) -> U,
    {
        let mut err = Ok(());

        let value = body(UseOksAdapter {
            iter: self,
            error: &mut err,
        });

        err.map(|()| value)
    }
}

impl<I: Iterator> IterExt for I {}

#[cfg(test)]
mod iter_ext_tests {
    use super::*;

    #[test]
    fn test_streaming_windows() {
        assert!((0..6).streaming_windows().map(|[a, b, c]| [a, b, c]).eq([
            [0, 1, 2],
            [1, 2, 3],
            [2, 3, 4],
            [3, 4, 5],
        ]))
    }

    #[test]
    fn test_streaming_size_hint() {
        let mut windows = (0..6).streaming_windows();

        assert_eq!(windows.size_hint(), (4, Some(4)));
        assert_eq!(windows.next(), Some([0, 1, 2]));

        assert_eq!(windows.size_hint(), (3, Some(3)));
        assert_eq!(windows.next(), Some([1, 2, 3]));

        assert_eq!(windows.size_hint(), (2, Some(2)));
        assert_eq!(windows.next(), Some([2, 3, 4]));

        assert_eq!(windows.size_hint(), (1, Some(1)));
        assert_eq!(windows.next(), Some([3, 4, 5]));

        assert_eq!(windows.size_hint(), (0, Some(0)));
        assert_eq!(windows.next(), None);
    }

    #[test]
    fn test_streaming_size_hint_inexact() {
        let mut windows = (0..6).streaming_windows().filter(|_| true);

        assert_eq!(windows.size_hint(), (0, Some(4)));
        assert_eq!(windows.next(), Some([0, 1, 2]));

        assert_eq!(windows.size_hint(), (0, Some(3)));
        assert_eq!(windows.next(), Some([1, 2, 3]));

        assert_eq!(windows.size_hint(), (0, Some(2)));
        assert_eq!(windows.next(), Some([2, 3, 4]));

        assert_eq!(windows.size_hint(), (0, Some(1)));
        assert_eq!(windows.next(), Some([3, 4, 5]));

        assert_eq!(windows.size_hint(), (0, Some(0)));
        assert_eq!(windows.next(), None);
    }
}

pub trait StrExt {
    fn parse_radix<N: Num>(&self, radix: u32) -> Result<N, N::FromStrRadixErr>;
}

impl StrExt for str {
    fn parse_radix<N: Num>(&self, radix: u32) -> Result<N, N::FromStrRadixErr> {
        N::from_str_radix(self, radix)
    }
}

#[derive(Debug, Clone, Error)]
#[error("failed to parse token {token:?} at index {index}")]
pub struct ParseListError<E> {
    token: String,
    index: usize,

    #[source]
    error: E,
}

pub fn parse_input_iter<'a, T, C>(
    input: impl IntoIterator<Item = &'a str>,
) -> Result<C, ParseListError<T::Err>>
where
    T: FromStr,
    C: FromIterator<T>,
{
    input
        .into_iter()
        .enumerate()
        .map(|(index, token)| {
            token.parse().map_err(|error| ParseListError {
                token: token.to_string(),
                index,
                error,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counter<T: Eq + Hash> {
    counts: HashMap<T, usize>,
}

impl<T: Eq + Hash> Default for Counter<T> {
    fn default() -> Self {
        Self {
            counts: Default::default(),
        }
    }
}

impl<T: Eq + Hash> Counter<T> {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn add(&mut self, value: T, additional: usize) {
        if additional > 0 {
            self.counts
                .entry(value)
                .and_modify(|value| *value += additional)
                .or_insert(additional);
        }
    }

    pub fn add_one(&mut self, value: T) {
        self.add(value, 1)
    }

    pub fn iter_counts(
        &self,
    ) -> impl Iterator<Item = (&T, usize)> + Clone + FusedIterator + ExactSizeIterator {
        self.counts.iter().map(|(item, &count)| (item, count))
    }

    pub fn merge(self, other: Self) -> Self {
        let (mut receiver, sender) = match self.counts.len().cmp(&other.counts.len()) {
            cmp::Ordering::Less => (other, self),
            cmp::Ordering::Equal => match self.counts.capacity() >= other.counts.capacity() {
                true => (self, other),
                false => (other, self),
            },
            cmp::Ordering::Greater => (self, other),
        };

        receiver.extend(sender);
        receiver
    }
}

impl<T: Eq + Hash + Sync> Counter<T> {
    pub fn par_iter_counts(&self) -> impl ParallelIterator<Item = (&T, usize)> {
        self.counts.par_iter().map(|(item, &count)| (item, count))
    }
}

impl<T: Eq + Hash> Extend<T> for Counter<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().map(|item| (item, 1)))
    }
}

impl<T: Eq + Hash> Extend<(T, usize)> for Counter<T> {
    fn extend<I: IntoIterator<Item = (T, usize)>>(&mut self, iter: I) {
        iter.into_iter()
            .for_each(|(item, count)| self.add(item, count))
    }
}

impl<T: Eq + Hash> ops::Add<Self> for Counter<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.merge(rhs)
    }
}

impl<T: Eq + Hash> ops::AddAssign<Self> for Counter<T> {
    fn add_assign(&mut self, rhs: Self) {
        *self = mem::take(self).merge(rhs);
    }
}

impl<T: Eq + Hash + Send> ParallelExtend<T> for Counter<T> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: rayon::iter::IntoParallelIterator<Item = T>,
    {
        self.par_extend(par_iter.into_par_iter().map(|item| (item, 1)))
    }
}

impl<T: Eq + Hash + Send> ParallelExtend<(T, usize)> for Counter<T> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: rayon::iter::IntoParallelIterator<Item = (T, usize)>,
    {
        let this = AtomicCell::new(mem::take(self));

        *self = par_iter
            .into_par_iter()
            .fold(
                || this.take().unwrap_or_default(),
                |mut counter, (item, count)| {
                    counter.add(item, count);
                    counter
                },
            )
            .reduce(Counter::new, Counter::merge)
    }
}

impl<T: Eq + Hash + Send> FromParallelIterator<T> for Counter<T> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = T>,
    {
        par_iter.into_par_iter().map(|item| (item, 1)).collect()
    }
}

impl<T: Eq + Hash + Send> FromParallelIterator<(T, usize)> for Counter<T> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = (T, usize)>,
    {
        par_iter
            .into_par_iter()
            .fold(Counter::new, |mut counter, (item, count)| {
                counter.add(item, count);
                counter
            })
            .reduce(Counter::new, Counter::merge)
    }
}

impl<T: Eq + Hash, U> FromIterator<U> for Counter<T>
where
    Self: Extend<U>,
{
    fn from_iter<I: IntoIterator<Item = U>>(iter: I) -> Self {
        let mut this = Self::new();
        this.extend(iter);
        this
    }
}

impl<T: Eq + Hash> IntoIterator for Counter<T> {
    type Item = (T, usize);

    type IntoIter = hash_map::IntoIter<T, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.counts.into_iter()
    }
}

struct AtomicCell<T> {
    inhabited: AtomicBool,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> AtomicCell<T> {
    fn empty() -> Self {
        Self {
            inhabited: AtomicBool::new(false),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    fn new(value: T) -> Self {
        Self {
            inhabited: AtomicBool::new(true),
            value: UnsafeCell::new(MaybeUninit::new(value)),
        }
    }

    fn take(&self) -> Option<T> {
        match self.inhabited.swap(false, atomic::Ordering::Relaxed) {
            false => None,
            true => Some(unsafe { self.value.get().as_ref().unwrap().as_ptr().read() }),
        }
    }
}

impl<T> Default for AtomicCell<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> From<T> for AtomicCell<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

unsafe impl<T: Send> Sync for AtomicCell<T> {}
