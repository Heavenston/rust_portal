use super::*;
use std::{ assert_matches::assert_matches, sync::{ atomic::{ AtomicUsize, Ordering }, Arc } };

#[test]
fn test_dyn_vec_i32() {
    let mut dyn_vec = DynVec::new::<i32>();
    assert_eq!(dyn_vec.metadata().type_id, std::any::TypeId::of::<i32>());

    let mut view = dyn_vec.typed_mut::<i32>().unwrap();

    view.push(10_i32);
    view.push(20_i32);
    view.push(30_i32);
    assert_eq!(view.len(), 3);

    let val = view.get(1).unwrap();
    assert_eq!(*val, 20);

    let mut_val = view.get_mut(1).unwrap();
    *mut_val = 25;
    let val_after_mut = view.get(1).unwrap();
    assert_eq!(*val_after_mut, 25);

    view.set(0, 5).unwrap();
    assert_eq!(*view.get(0).unwrap(), 5);
    assert_eq!(view.len(), 3);
    let removed = view.swap_remove(0).unwrap();
    assert_eq!(removed, 5);
    assert_eq!(view.len(), 2);
    assert_eq!(*view.get(0).unwrap(), 30);
    assert_eq!(*view.get(1).unwrap(), 25);
}

#[test]
fn test_type_mismatch_push() {
    let mut dyn_vec = DynVec::new::<i32>();
    let res = dyn_vec.push(Box::new("hello".to_string()));
    assert!(res.is_err());
}

#[test]
fn test_growth_and_bounds() {
    let mut v = DynVec::new::<u64>();
    {
        let mut tv = v.typed_mut::<u64>().unwrap();
        for i in 0..100 { tv.push(i as u64); }
        assert_eq!(tv.len(), 100);
        assert_eq!(*tv.get(50).unwrap(), 50);
    }
    let removed = {
        let mut tv = v.typed_mut::<u64>().unwrap();
        tv.swap_remove(10).unwrap()
    };
    assert_eq!(removed, 10);
    assert_eq!(v.len(), 99);
    let tv = v.typed::<u64>().unwrap();
    assert!(tv.get(999).is_none());

    assert!(!v.is_empty());
}

#[test]
fn test_capacity_reserve_clear_pop_shrink() {
    let mut v = DynVec::with_capacity(DynVecMetadata::new::<i32>(), 0);
    assert!(v.is_empty());
    assert_eq!(v.capacity(), 0);

    v.reserve(10);
    assert!(v.capacity() >= 10);

    {
        let mut tv = v.typed_mut::<i32>().unwrap();
        tv.push(1_i32);
        tv.push(2_i32);
        tv.push(3_i32);
        assert_eq!(tv.len(), 3);
        assert_eq!(tv.pop().unwrap(), 3);
    }
    assert_eq!(v.len(), 2);
    v.clear();
    assert!(v.is_empty());
    let cap = v.capacity();
    {
        let mut tv = v.typed_mut::<i32>().unwrap();
        tv.push(4_i32);
    }
    assert_eq!(v.capacity(), cap);
    v.clear();
    v.shrink_to_fit();
    assert_eq!(v.capacity(), 0);
}

#[derive(Debug)]
struct DropProbe { hits: Arc<AtomicUsize> }
impl Drop for DropProbe { fn drop(&mut self) { self.hits.fetch_add(1, Ordering::SeqCst); } }

#[test]
fn test_drop_paths() {
    let hits = Arc::new(AtomicUsize::new(0));
    let mut v = DynVec::new::<DropProbe>();
    {
        let mut tv = v.typed_mut::<DropProbe>().unwrap();
        for _ in 0..5 { tv.push(DropProbe { hits: hits.clone() }); }
        tv.set(2, DropProbe { hits: hits.clone() }).unwrap();
        let _r: DropProbe = tv.swap_remove(1).unwrap();
        drop(_r);
        tv.clear();
    }
    assert_eq!(hits.load(Ordering::SeqCst), 6);
    drop(v);
    assert_eq!(hits.load(Ordering::SeqCst), 6);
}

#[test]
fn test_typed_views() {
    let mut v = DynVec::new::<i32>();
    {
        let mut tv = v.typed_mut::<i32>().unwrap();
        tv.push(1);
        tv.push(2);
        tv.push(3);
    }
    let view = v.typed::<i32>().unwrap();
    assert_eq!(view.len(), 3);
    assert_eq!(*view.get(1).unwrap(), 2);
    assert_eq!(view[0], 1);
    assert_eq!(view.iter().copied().sum::<i32>(), 1 + 2 + 3);
    let mut view_mut = v.typed_mut::<i32>().unwrap();
    assert_eq!(*view_mut.get_mut(2).unwrap(), 3);
    view_mut.set(1, 20).unwrap();
    view_mut.push(4);
    assert_eq!(view_mut.swap_remove(0).unwrap(), 1);
    assert_eq!(view_mut.pop().unwrap(), 3);
    for x in view_mut.iter_mut() { *x *= 2; }
    let view2 = v.typed::<i32>().unwrap();
    assert_eq!(view2.as_slice(), &[8, 40]);
}

#[test]
fn test_zst_unit_typed() {
    let mut v = DynVec::new::<()>();
    {
        let mut tv = v.typed_mut::<()>().unwrap();
        for _ in 0..10 { tv.push(()); }
        assert_eq!(tv.len(), 10);
        // Indexing and iteration should work
        assert_eq!(tv.as_slice().len(), 10);
        // swap_remove keeps len consistent
        tv.swap_remove(3).unwrap();
        assert_eq!(tv.len(), 9);
        // pop returns Some(()) until empty
        assert!(tv.pop().is_some());
    }
    // After scope, v still valid
    assert!(v.len() <= 9);
}

#[test]
fn test_zst_unit_untyped() {
    let mut v = DynVec::new::<()>();
    for _ in 0..5 { let _ = v.push(Box::new(())); }
    assert_eq!(v.len(), 5);
    // get returns &dyn Any; downcast_ref::<()>() works
    assert!(v.get(0).unwrap().as_any().is::<()>());
    assert_matches!(v.get(0).unwrap().as_typed::<()>(), Ok(&()));
    // set with ZST keeps len
    let _ = v.set(2, Box::new(()));
    assert_eq!(v.len(), 5);
    // swap_remove returns OwnedDynVecValue; convert to Box<dyn Any>
    let b = v.swap_remove(1).unwrap().into_boxed_any();
    assert!(b.downcast::<()>().is_ok());
    assert_eq!(v.len(), 4);
    // pop to empty
    while v.pop().is_some() {}
    assert!(v.is_empty());
}

#[test]
fn test_zst_custom() {
    #[derive(Copy, Clone, Debug)]
    struct Z;
    let mut v = DynVec::new::<Z>();
    {
        let mut tv = v.typed_mut::<Z>().unwrap();
        for _ in 0..16 { tv.push(Z); }
        assert_eq!(tv.len(), 16);
        // swap remove a few
        tv.swap_remove(0).unwrap();
        tv.swap_remove(5.min(tv.len()-1)).unwrap();
        assert!(tv.pop().is_some());
    }
    assert!(v.len() <= 14);
    // Untyped operations work too
    let _ = v.push(Box::new(Z));
    assert!(v.get(0).unwrap().as_any().is::<Z>());
    assert_matches!(v.get(0).unwrap().as_typed::<Z>(), Ok(&Z));
}

#[test]
fn test_owned_guard_push_into_basic() {
    let mut src = DynVec::new::<i32>();
    let mut dst = DynVec::new::<i32>();
    {
        let mut tv = src.typed_mut::<i32>().unwrap();
        tv.extend([1, 2, 3]);
    }
    // remove middle element and push into dst
    src.swap_remove(1).unwrap().push_into(&mut dst).unwrap();
    // After guard drop, src should have [1, 3] in some order (swap removal brings last into idx)
    let s = src.typed::<i32>().unwrap();
    assert_eq!(s.as_slice(), &[1, 3]);
    let d = dst.typed::<i32>().unwrap();
    assert_eq!(d.as_slice(), &[2]);
}

#[test]
fn test_owned_guard_set_into_overwrite() {
    let mut src = DynVec::new::<i32>();
    let mut dst = DynVec::new::<i32>();
    {
        let mut tv = src.typed_mut::<i32>().unwrap();
        tv.extend([10, 20, 30]);
    }
    {
        let mut dv = dst.typed_mut::<i32>().unwrap();
        dv.extend([100, 200]);
    }
    // Move first element (10) and overwrite index 1 of dst
    src.swap_remove(0).unwrap().set_into(&mut dst, 1).unwrap();
    let s = src.typed::<i32>().unwrap();
    assert_eq!(s.as_slice(), &[30, 20]);
    {
        let d = dst.typed::<i32>().unwrap();
        assert_eq!(d.as_slice(), &[100, 10]);
    }
    // Overwrite at index 0 with next value (30)
    src.swap_remove(0).unwrap().set_into(&mut dst, 0).unwrap();
    {
        let s2 = src.typed::<i32>().unwrap();
        assert_eq!(s2.as_slice(), &[20]);
    }
    {
        let d2 = dst.typed::<i32>().unwrap();
        assert_eq!(d2.as_slice(), &[30, 10]);
    }

    // Overwrite again at index 1 with last remaining value (20)
    src.swap_remove(0).unwrap().set_into(&mut dst, 1).unwrap();
    let d3 = dst.typed::<i32>().unwrap();
    assert_eq!(d3.as_slice(), &[30, 20]);
}

#[test]
fn test_push_into_type_mismatch_returns_err() {
    let mut a = DynVec::new::<i32>();
    let mut b = DynVec::new::<u64>();
    {
        let mut tv = a.typed_mut::<i32>().unwrap();
        tv.push(42);
    }
    let res = a.swap_remove(0).unwrap().push_into(&mut b);
    assert!(res.is_err());
}

#[test]
fn test_into_typed_and_drop_semantics() {
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    struct DropProbe { hits: Arc<AtomicUsize> }
    impl Drop for DropProbe { fn drop(&mut self) { self.hits.fetch_add(1, Ordering::SeqCst); } }

    // Consumed path (into_typed): element should NOT be dropped by guard, only by dropping returned value
    let hits = Arc::new(AtomicUsize::new(0));
    let mut v = DynVec::new::<DropProbe>();
    {
        let mut tv = v.typed_mut::<DropProbe>().unwrap();
        tv.push(DropProbe { hits: hits.clone() });
    }
    let p: DropProbe = v.swap_remove(0).unwrap().into_typed::<DropProbe>().unwrap();
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    drop(p);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(v.len(), 0);

    // Not consumed path: dropping guard should drop the element
    let mut v2 = DynVec::new::<DropProbe>();
    {
        let mut tv = v2.typed_mut::<DropProbe>().unwrap();
        tv.push(DropProbe { hits: hits.clone() });
    }
    let _guard = v2.swap_remove(0).unwrap();
    drop(_guard);
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    assert_eq!(v2.len(), 0);
}

#[test]
fn test_zst_set_into_and_push_into() {
    #[derive(Copy, Clone, Debug)]
    struct Z;
    let mut a = DynVec::new::<Z>();
    let mut b = DynVec::new::<Z>();
    {
        let mut tv = a.typed_mut::<Z>().unwrap();
        for _ in 0..3 { tv.push(Z); }
    }
    a.swap_remove(1).unwrap().push_into(&mut b).unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 1);
    // Overwrite at index 0; length does not change
    a.swap_remove(0).unwrap().set_into(&mut b, 0).unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    // Overwrite again at index 0 with last remaining value
    a.swap_remove(0).unwrap().set_into(&mut b, 0).unwrap();
    assert_eq!(a.len(), 0);
    assert_eq!(b.len(), 1);
}

#[test]
fn test_pop_returns_guard() {
    let mut v = DynVec::new::<i32>();
    v.typed_mut::<i32>().unwrap().extend([7, 8]);
    let g = v.pop().unwrap();
    // Move into another vec
    let mut dst = DynVec::new::<i32>();
    g.push_into(&mut dst).unwrap();
    // src len decremented on drop
    assert_eq!(v.len(), 1);
    assert_eq!(dst.typed::<i32>().unwrap().as_slice(), &[8]);
}

#[test]
fn test_extend_typed_and_untyped() {
    // Typed view extend
    let mut v = DynVec::new::<i32>();
    {
        let mut tv = v.typed_mut::<i32>().unwrap();
        // trait method (in scope via prelude)
        tv.extend([1, 2, 3]);
        // trait method
        std::iter::Extend::extend(&mut tv, [4, 5]);
        // extend from references requires Clone
        let buf = vec![6_i32, 7_i32];
        tv.extend(buf.iter());
    }
    let tv = v.typed::<i32>().unwrap();
    assert_eq!(tv.as_slice(), &[1, 2, 3, 4, 5, 6, 7]);

    // Untyped extend via Box<dyn Any>
    let mut u = DynVec::new::<String>();
    let items = ["a", "bb", "ccc"].into_iter().map(|s| Box::new(s.to_string()) as Box<dyn Any>);
    u.extend(items);
    // trait method
    let items2 = ["dddd", "eeeee"].into_iter().map(|s| Box::new(s.to_string()) as Box<dyn Any>);
    std::iter::Extend::extend(&mut u, items2);
    let uv = u.typed::<String>().unwrap();
    assert_eq!(
        uv.as_slice(),
        &["a".to_string(), "bb".to_string(), "ccc".to_string(), "dddd".to_string(), "eeeee".to_string()]
    );
}

#[test]
fn test_collect_into_dynvec() {
    let v: DynVec = [10_i64, 20, 30].into_iter().collect();
    assert_eq!(v.metadata().type_id, TypeId::of::<i64>());
    let t = v.typed::<i64>().unwrap();
    assert_eq!(t.as_slice(), &[10, 20, 30]);
}

#[test]
fn test_push_default_empty() {
    #[derive(Debug, PartialEq, Eq)]
    struct MyType {
        value: String,
    }

    impl Default for MyType {
        fn default() -> Self {
            Self { value: "Feur".into() }
        }
    }

    let mut v: DynVec = DynVec::new::<MyType>();
    assert_matches!(v.push_default(), Ok(()));
    assert_eq!(v.typed::<MyType>().unwrap().as_slice(), &[MyType {
        value: "Feur".into(),
    }]);
}

#[test]
fn test_push_default_non_empty() {
    let mut v: DynVec = [10_i64, 20, 30].into_iter().collect();
    assert_matches!(v.push_default(), Ok(()));
    assert_eq!(v.typed::<i64>().unwrap().as_slice(), &[10, 20, 30, 0]);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_push_default_zst() {
    static DEFAULT_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static DROP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct MyZST;

    impl Drop for MyZST {
        fn drop(&mut self) {
            // println!("{}", std::backtrace::Backtrace::force_capture());
            DROP_COUNTER.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl Default for MyZST {
        fn default() -> Self {
            DEFAULT_COUNTER.fetch_add(1, Ordering::Relaxed);
            MyZST
        }
    }

    let mut v: DynVec = [MyZST, MyZST].into_iter().collect();

    assert_eq!(DEFAULT_COUNTER.load(Ordering::Relaxed), 0);
    assert_eq!(DROP_COUNTER.load(Ordering::Relaxed), 0);

    assert_matches!(v.push_default(), Ok(()));

    assert_eq!(DEFAULT_COUNTER.load(Ordering::Relaxed), 1);
    assert_eq!(DROP_COUNTER.load(Ordering::Relaxed), 0);

    assert_eq!(v.typed::<MyZST>().unwrap().as_slice().len(), 3);

    assert_matches!(v.push_default(), Ok(()));

    assert_eq!(DEFAULT_COUNTER.load(Ordering::Relaxed), 2);
    assert_eq!(DROP_COUNTER.load(Ordering::Relaxed), 0);

    v.pop();

    assert_eq!(DEFAULT_COUNTER.load(Ordering::Relaxed), 2);
    assert_eq!(DROP_COUNTER.load(Ordering::Relaxed), 1);

    v.pop();

    assert_eq!(DEFAULT_COUNTER.load(Ordering::Relaxed), 2);
    assert_eq!(DROP_COUNTER.load(Ordering::Relaxed), 2);

    drop(v);

    assert_eq!(DEFAULT_COUNTER.load(Ordering::Relaxed), 2);
    assert_eq!(DROP_COUNTER.load(Ordering::Relaxed), 4);
}
