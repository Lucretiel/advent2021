use std::{iter::FusedIterator, mem};

use num::Num;

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
