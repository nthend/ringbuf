use super::*;

use std::cell::{Cell};
use std::thread;
use std::io::{Read, Write};
use std::time::{Duration};

fn head_tail<T>(rb: &RingBuffer<T>) -> (usize, usize) {
    (rb.head.load(Ordering::SeqCst), rb.tail.load(Ordering::SeqCst))
}

#[test]
fn capacity() {
    let cap = 13;
    let buf = RingBuffer::<i32>::new(cap);
    assert_eq!(buf.capacity(), cap);
}

#[test]
fn split_capacity() {
    let cap = 13;
    let buf = RingBuffer::<i32>::new(cap);
    let (prod, cons) = buf.split();
    
    assert_eq!(prod.capacity(), cap);
    assert_eq!(cons.capacity(), cap);
}

#[test]
fn split_threads() {
    let buf = RingBuffer::<i32>::new(10);
    let (prod, cons) = buf.split();
    
    let pjh = thread::spawn(move || {
        let _ = prod;
    });

    let cjh = thread::spawn(move || {
        let _ = cons;
    });

    pjh.join().unwrap();
    cjh.join().unwrap();
}

#[test]
fn push() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, _) = buf.split();
    

    assert_eq!(head_tail(&prod.rb), (0, 0));

    assert_matches!(prod.push(123), Ok(()));
    assert_eq!(head_tail(&prod.rb), (0, 1));

    assert_matches!(prod.push(234), Ok(()));
    assert_eq!(head_tail(&prod.rb), (0, 2));

    assert_matches!(prod.push(345), Err((PushError::Full, 345)));
    assert_eq!(head_tail(&prod.rb), (0, 2));
}

#[test]
fn pop_empty() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (_, mut cons) = buf.split();


    assert_eq!(head_tail(&cons.rb), (0, 0));

    assert_eq!(cons.pop(), Err(PopError::Empty));
    assert_eq!(head_tail(&cons.rb), (0, 0));
}

#[test]
fn push_pop_one() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vcap = cap + 1;
    let values = [12, 34, 56, 78, 90];
    assert_eq!(head_tail(&cons.rb), (0, 0));

    for (i, v) in values.iter().enumerate() {
        assert_matches!(prod.push(*v), Ok(()));
        assert_eq!(head_tail(&cons.rb), (i % vcap, (i + 1) % vcap));

        match cons.pop() {
            Ok(w) => assert_eq!(w, *v),
            other => panic!(other),
        }
        assert_eq!(head_tail(&cons.rb), ((i + 1) % vcap, (i + 1) % vcap));

        assert_eq!(cons.pop(), Err(PopError::Empty));
        assert_eq!(head_tail(&cons.rb), ((i + 1) % vcap, (i + 1) % vcap));
    }
}

#[test]
fn push_pop_all() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vcap = cap + 1;
    let values = [(12, 34, 13), (56, 78, 57), (90, 10, 91)];
    assert_eq!(head_tail(&cons.rb), (0, 0));

    for (i, v) in values.iter().enumerate() {
        assert_matches!(prod.push(v.0), Ok(()));
        assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 1) % vcap));

        assert_matches!(prod.push(v.1), Ok(()));
        assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 2) % vcap));

        match prod.push(v.2) {
            Err((PushError::Full, w)) => assert_eq!(w, v.2),
            other => panic!(other),
        }
        assert_eq!(head_tail(&cons.rb), (cap*i % vcap, (cap*i + 2) % vcap));


        match cons.pop() {
            Ok(w) => assert_eq!(w, v.0),
            other => panic!(other),
        }
        assert_eq!(head_tail(&cons.rb), ((cap*i + 1) % vcap, (cap*i + 2) % vcap));

        match cons.pop() {
            Ok(w) => assert_eq!(w, v.1),
            other => panic!(other),
        }
        assert_eq!(head_tail(&cons.rb), ((cap*i + 2) % vcap, (cap*i + 2) % vcap));

        assert_eq!(cons.pop(), Err(PopError::Empty));
        assert_eq!(head_tail(&cons.rb), ((cap*i + 2) % vcap, (cap*i + 2) % vcap));
    }
}

#[test]
fn producer_full() {
    let buf = RingBuffer::<i32>::new(1);
    let (mut prod, _) = buf.split();

    assert!(!prod.is_full());

    assert_matches!(prod.push(123), Ok(()));
    assert!(prod.is_full());
}

#[test]
fn consumer_empty() {
    let buf = RingBuffer::<i32>::new(1);
    let (mut prod, cons) = buf.split();


    assert_eq!(head_tail(&cons.rb), (0, 0));
    assert!(cons.is_empty());

    assert_matches!(prod.push(123), Ok(()));
    assert!(!cons.is_empty());
}

#[derive(Debug)]
struct Dropper<'a> {
    cnt: &'a Cell<i32>,
}

impl<'a> Dropper<'a> {
    fn new(c: &'a Cell<i32>) -> Self {
        Self { cnt: c }
    }
}

impl<'a> Drop for Dropper<'a> {
    fn drop(&mut self) {
        self.cnt.set(self.cnt.get() + 1);
    }
}

#[test]
fn drop() {
    let (ca, cb) = (Cell::new(0), Cell::new(0));
    let (da, db) = (Dropper::new(&ca), Dropper::new(&cb));

    let cap = 3;
    let buf = RingBuffer::new(cap);

    {
        let (mut prod, mut cons) = buf.split();

        assert_eq!((ca.get(), cb.get()), (0, 0));

        prod.push(da).unwrap();
        assert_eq!((ca.get(), cb.get()), (0, 0));

        prod.push(db).unwrap();
        assert_eq!((ca.get(), cb.get()), (0, 0));

        cons.pop().unwrap();
        assert_eq!((ca.get(), cb.get()), (1, 0));
    }
    
    assert_eq!((ca.get(), cb.get()), (1, 1));
}

#[test]
fn push_access() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (123, 456);
    let push_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = vs_20.0;
        left[1] = vs_20.1;
        Ok((2, ()))
    };

    assert_eq!(unsafe { prod.push_access(push_fn_20) }.unwrap().unwrap(), (2, ()));

    assert_eq!(cons.pop().unwrap(), vs_20.0);
    assert_eq!(cons.pop().unwrap(), vs_20.1);
    assert_matches!(cons.pop(), Err(PopError::Empty));

    let vs_11 = (123, 456);
    let push_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        left[0] = vs_11.0;
        right[0] = vs_11.1;
        Ok((2, ()))
    };

    assert_eq!(unsafe { prod.push_access(push_fn_11) }.unwrap().unwrap(), (2, ()));

    assert_eq!(cons.pop().unwrap(), vs_11.0);
    assert_eq!(cons.pop().unwrap(), vs_11.1);
    assert_matches!(cons.pop(), Err(PopError::Empty));
}

/*
/// This test doesn't compile.
/// And that's good :)
#[test]
fn push_access_oref() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, _) = buf.split();

    let mut ovar = 123;
    let mut oref = &mut 123;
    let push_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        left[0] = 456;
        oref = &mut left[0];
        Ok((1, ()))
    };

    assert_eq!(unsafe {
        prod.push_access(push_fn_20)
    }.unwrap().unwrap(), (1, ()));

    assert_eq!(*oref, 456);
}
*/

#[test]
fn pop_access_full() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (_, mut cons) = buf.split();

    let dummy_fn = |_l: &mut [i32], _r: &mut [i32]| -> Result<(usize, ()), ()> {
        if true {
            Ok((0, ()))
        } else {
            Err(())
        }
    };
    assert_matches!(unsafe { cons.pop_access(dummy_fn) }, Err(PopAccessError::Empty));
}

#[test]
fn pop_access_empty() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (_, mut cons) = buf.split();

    let dummy_fn = |_l: &mut [i32], _r: &mut [i32]| -> Result<(usize, ()), ()> {
        if true {
            Ok((0, ()))
        } else {
            Err(())
        }
    };
    assert_matches!(unsafe { cons.pop_access(dummy_fn) }, Err(PopAccessError::Empty));
}

#[test]
fn pop_access() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();


    let vs_20 = (123, 456);

    assert_matches!(prod.push(vs_20.0), Ok(()));
    assert_matches!(prod.push(vs_20.1), Ok(()));
    assert_matches!(prod.push(0), Err((PushError::Full, 0)));

    let pop_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0], vs_20.0);
        assert_eq!(left[1], vs_20.1);
        Ok((2, ()))
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_20) }.unwrap().unwrap(), (2, ()));


    let vs_11 = (123, 456);
    
    assert_matches!(prod.push(vs_11.0), Ok(()));
    assert_matches!(prod.push(vs_11.1), Ok(()));
    assert_matches!(prod.push(0), Err((PushError::Full, 0)));
    
    let pop_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_eq!(left[0], vs_11.0);
        assert_eq!(right[0], vs_11.1);
        Ok((2, ()))
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_11) }.unwrap().unwrap(), (2, ()));

}

#[test]
fn push_access_return() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let push_fn_3 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        Ok((3, ()))
    };

    assert_matches!(unsafe { prod.push_access(push_fn_3) }, Err(PushAccessError::BadLen)
    );

    let push_fn_err = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), i32> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        Err(123)
    };

    assert_matches!(unsafe { prod.push_access(push_fn_err) }, Ok(Err(123))
    );

    let push_fn_0 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        Ok((0, ()))
    };

    assert_matches!(unsafe { prod.push_access(push_fn_0) }, Ok(Ok((0, ())))
    );

    let push_fn_1 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = 12;
        Ok((1, ()))
    };

    assert_matches!(unsafe { prod.push_access(push_fn_1) }, Ok(Ok((1, ())))
    );

    let push_fn_2 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
        left[0] = 34;
        Ok((1, ()))
    };

    assert_matches!(unsafe { prod.push_access(push_fn_2) }, Ok(Ok((1, ())))
    );

    assert_eq!(cons.pop().unwrap(), 12);
    assert_eq!(cons.pop().unwrap(), 34);
    assert_matches!(cons.pop(), Err(PopError::Empty));
}

#[test]
fn pop_access_return() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    assert_matches!(prod.push(12), Ok(()));
    assert_matches!(prod.push(34), Ok(()));
    assert_matches!(prod.push(0), Err((PushError::Full, 0)));

    let pop_fn_3 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        Ok((3, ()))
    };

    assert_matches!(unsafe { cons.pop_access(pop_fn_3) }, Err(PopAccessError::BadLen)
    );

    let pop_fn_err = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), i32> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        Err(123)
    };

    assert_matches!(unsafe { cons.pop_access(pop_fn_err) }, Ok(Err(123))
    );

    let pop_fn_0 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        Ok((0, ()))
    };

    assert_matches!(unsafe { cons.pop_access(pop_fn_0) }, Ok(Ok((0, ())))
    );

    let pop_fn_1 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0], 12);
        Ok((1, ()))
    };

    assert_matches!(unsafe { cons.pop_access(pop_fn_1) }, Ok(Ok((1, ())))
    );

    let pop_fn_2 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0], 34);
        Ok((1, ()))
    };

    assert_matches!(unsafe { cons.pop_access(pop_fn_2) }, Ok(Ok((1, ())))
    );
}

#[test]
fn push_pop_access() {
    let cap = 2;
    let buf = RingBuffer::<i32>::new(cap);
    let (mut prod, mut cons) = buf.split();

    let vs_20 = (123, 456);
    let push_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        left[0] = vs_20.0;
        left[1] = vs_20.1;
        Ok((2, ()))
    };

    assert_eq!(unsafe { prod.push_access(push_fn_20) }.unwrap().unwrap(), (2, ()));

    let pop_fn_20 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 0);
        assert_eq!(left[0], vs_20.0);
        assert_eq!(left[1], vs_20.1);
        Ok((2, ()))
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_20) }.unwrap().unwrap(), (2, ()));


    let vs_11 = (123, 456);
    let push_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        left[0] = vs_11.0;
        right[0] = vs_11.1;
        Ok((2, ()))
    };

    assert_eq!(unsafe { prod.push_access(push_fn_11) }.unwrap().unwrap(), (2, ()));

    let pop_fn_11 = |left: &mut [i32], right: &mut [i32]| -> Result<(usize, ()), ()> {
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_eq!(left[0], vs_11.0);
        assert_eq!(right[0], vs_11.1);
        Ok((2, ()))
    };

    assert_eq!(unsafe { cons.pop_access(pop_fn_11) }.unwrap().unwrap(), (2, ()));
}

#[test]
fn push_pop_access_message() {
    let buf = RingBuffer::<u8>::new(7);
    let (mut prod, mut cons) = buf.split();

    let smsg = "The quick brown fox jumps over the lazy dog";
    
    let pjh = thread::spawn(move || {
        let mut bytes = smsg.as_bytes();
        while bytes.len() > 0 {
            let push_fn = |left: &mut [u8], right: &mut [u8]| -> Result<(usize, ()),()> {
                let n = bytes.read(left).unwrap();
                let m = bytes.read(right).unwrap();
                Ok((n + m, ()))
            };
            match unsafe { prod.push_access(push_fn) } {
                Ok(res) => match res {
                    Ok((_n, ())) => (),
                    Err(()) => unreachable!(),
                },
                Err(e) => match e {
                    PushAccessError::Full => thread::sleep(Duration::from_millis(1)),
                    PushAccessError::BadLen => unreachable!(),
                }
            }
        }
        loop {
            match prod.push(0) {
                Ok(()) => break,
                Err((PushError::Full, _)) => thread::sleep(Duration::from_millis(1)),
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        loop {
            let pop_fn = |left: &mut [u8], right: &mut [u8]| -> Result<(usize, ()),()> {
                let n = bytes.write(left).unwrap();
                let m = bytes.write(right).unwrap();
                Ok((n + m, ()))
            };
            match unsafe { cons.pop_access(pop_fn) } {
                Ok(res) => match res {
                    Ok((_n, ())) => (),
                    Err(()) => unreachable!(),
                },
                Err(e) => match e {
                    PopAccessError::Empty => {
                        if bytes.ends_with(&[0]) {
                            break;
                        } else {
                            thread::sleep(Duration::from_millis(1));
                        }
                    },
                    PopAccessError::BadLen => unreachable!(),
                }
            }
        }

        assert_eq!(bytes.pop().unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}

#[test]
fn push_pop_slice_message() {
    let buf = RingBuffer::<u8>::new(7);
    let (mut prod, mut cons) = buf.split();

    let smsg = "The quick brown fox jumps over the lazy dog";
    
    let pjh = thread::spawn(move || {
        let mut bytes = smsg.as_bytes();
        while bytes.len() > 0 {
            match prod.push_slice(bytes) {
                Ok(n) => bytes = &bytes[n..bytes.len()],
                Err(PushError::Full) => thread::sleep(Duration::from_millis(1)),
            }
        }
        loop {
            match prod.push(0) {
                Ok(()) => break,
                Err((PushError::Full, _)) => thread::sleep(Duration::from_millis(1)),
            }
        }
    });

    let cjh = thread::spawn(move || {
        let mut bytes = Vec::<u8>::new();
        let mut buffer = [0; 5];
        loop {
            match cons.pop_slice(&mut buffer) {
                Ok(n) => bytes.extend_from_slice(&buffer[0..n]),
                Err(PopError::Empty) => {
                    if bytes.ends_with(&[0]) {
                        break;
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }
        }

        assert_eq!(bytes.pop().unwrap(), 0);
        String::from_utf8(bytes).unwrap()
    });

    pjh.join().unwrap();
    let rmsg = cjh.join().unwrap();

    assert_eq!(smsg, rmsg);
}