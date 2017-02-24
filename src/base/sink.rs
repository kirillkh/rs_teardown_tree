use base::Sink;

use std::ptr;
use std::marker::PhantomData;


impl<T> Sink<T> for Vec<T> {
    #[inline(always)] fn consume(&mut self, x: T) {
        self.push(x);
    }
}



#[derive(new)]
pub struct UncheckedVecSink<T> {
    output: Vec<T>
}

impl<T> Sink<T> for UncheckedVecSink<T> {
    #[inline(always)] fn consume(&mut self, item: T) {
        consume_unchecked(&mut self.output, item);
    }
}



#[derive(new)]
pub struct UncheckedVecRefSink<'a, T: 'a> {
    pub output: &'a mut Vec<T>
}

impl<'a, T: 'a> Sink<T> for UncheckedVecRefSink<'a, T> {
    #[inline(always)] fn consume(&mut self, item: T) {
        consume_unchecked(&mut self.output, item);
    }
}



pub struct RefCopyingSink<T: Copy, S: Sink<T>> {
    pub sink: S,
    _ph: PhantomData<T>
}

impl<T, S> RefCopyingSink<T, S>
    where T: Copy, S: Sink<T>
{
    pub fn new(sink: S) -> Self {
        RefCopyingSink { sink:sink, _ph: PhantomData }
    }
}

impl<'a, T, S> Sink<&'a T> for RefCopyingSink<T, S>
    where T: Copy+'a, S: Sink<T>
{
    #[inline(always)] fn consume(&mut self, x: &'a T) {
        self.sink.consume(*x);
    }
}



pub struct RefCloningSink<T: Clone, S: Sink<T>> {
    pub sink: S,
    _ph: PhantomData<T>
}

impl<T, S> RefCloningSink<T, S>
    where T: Clone, S: Sink<T>
{
    pub fn new(sink: S) -> Self {
        RefCloningSink { sink:sink, _ph: PhantomData }
    }
}

impl<'a, T, S> Sink<&'a T> for RefCloningSink<T, S>
    where T: Clone+'a, S: Sink<T>
{
    #[inline(always)] fn consume(&mut self, x: &'a T) {
        self.sink.consume(x.clone());
    }
}



pub struct SinkAdapter<T, S: Sink<T>> {
    sink: S,
    _ph: PhantomData<T>
}

impl<T, S: Sink<T>> SinkAdapter<T, S> {
    #[inline]
    pub fn new(sink: S) -> Self {
        SinkAdapter { sink: sink, _ph: PhantomData }
    }
}

impl<T, S: Sink<T>> Sink<(T, ())> for SinkAdapter<T, S> {
    #[inline(always)]
    fn consume(&mut self, entry: (T, ())) {
        self.sink.consume(entry.0)
    }
}



pub struct RefSinkAdapter<'a, T: 'a, S: Sink<&'a T>> {
    sink: S,
    _ph: PhantomData<&'a T>
}

impl<'a, T: 'a, S: Sink<&'a T>> RefSinkAdapter<'a, T, S> {
    #[inline]
    pub fn new(sink: S) -> Self {
        RefSinkAdapter { sink: sink, _ph: PhantomData }
    }
}

impl<'a, T: 'a, S: Sink<&'a T>> Sink<&'a (T, ())> for RefSinkAdapter<'a, T, S> {
    #[inline]
    fn consume(&mut self, entry: &'a (T, ())) {
        self.sink.consume(&entry.0)
    }
}



pub struct CountingSink<T, S: Sink<T>> {
    sink: S,
    count: usize,
    _ph: PhantomData<T>
}

impl<T, S: Sink<T>> CountingSink<T, S> {
    #[inline]
    pub fn new(sink: S) -> Self {
        CountingSink { sink: sink, count: 0, _ph: PhantomData }
    }

    pub fn count(&self) -> usize {
        self.count
    }
}

impl<T, S: Sink<T>> Sink<T> for CountingSink<T, S> {
    #[inline]
    fn consume(&mut self, entry: T) {
        self.count += 1;
        self.sink.consume(entry)
    }
}



// The caller must make sure output.len() < output.capacity().
#[inline(always)]
pub fn consume_unchecked<T>(output: &mut Vec<T>, item: T) {
    unsafe {
        let len = output.len();
        debug_assert!(len < output.capacity());
        output.set_len(len + 1);
        let p = output.get_unchecked_mut(len);

        ptr::write(p, item);
    }
}
