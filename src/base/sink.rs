use base::Sink;

use std::ptr;
use std::marker::PhantomData;


impl<T> Sink<T> for Vec<T> {
    #[inline] fn consume(&mut self, x: T) {
        self.push(x);
    }
}



#[derive(new)]
pub struct UncheckedVecSink<T> {
    pub output: Vec<T>
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
    #[inline] fn consume(&mut self, x: &'a T) {
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
    #[inline] fn consume(&mut self, x: &'a T) {
        self.sink.consume(x.clone());
    }
}


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
