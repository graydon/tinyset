//! A set that is compact in size.

use std;

use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

use tinyset::HasInvalid;

enum SearchResult {
    Present(usize),
    Empty(usize),
    /// The element is not present, but there is someone richer than
    /// us we could steal from!
    Richer(usize),
}

/// A set implemented of usize elements
#[derive(Debug,Clone)]
pub struct USizeSet {
    v: Data,
}

#[derive(Debug, Clone)]
enum Data {
    Su8(u8, [u8; 22]),
    Vu8(u8, Box<[u8]>),
    Su16(u16, [u16; 11]),
    Vu16(u16, Box<[u16]>),
}
impl Data {
    fn new() -> Data {
        Data::Su8(0, [u8::invalid(); 22])
    }
    fn with_max_cap(max: usize, cap: usize) -> Data {
        if max < u8::invalid() as usize {
            if cap <= 22 {
                Data::Su8(0, [u8::invalid(); 22])
            } else {
                Data::Vu8(0, vec![u8::invalid(); (cap*11/10).next_power_of_two()]
                          .into_boxed_slice())
            }
        } else if max < u16::invalid() as usize {
            if cap <= 11 {
                Data::Su16(0, [u16::invalid(); 11])
            } else {
                Data::Vu16(0, vec![u16::invalid(); (cap*11/10).next_power_of_two()]
                           .into_boxed_slice())
            }
        } else {
            unimplemented!()
        }
    }
}

fn capacity_to_rawcapacity(cap: usize) -> usize {
    (cap*11/10).next_power_of_two()
}

impl USizeSet {
    /// Creates an empty set..
    pub fn default() -> USizeSet {
        Self::with_capacity(0)
    }
    /// Creates an empty set..
    pub fn new() -> USizeSet {
        USizeSet::with_capacity(0)
    }
    /// Creates an empty set with the specified capacity.
    pub fn with_capacity(cap: usize) -> USizeSet {
        let nextcap = capacity_to_rawcapacity(cap);
        if cap <= 22 {
            USizeSet { v: Data::new() }
        } else if cap < u8::invalid() as usize {
            USizeSet { v: Data::Vu8( 0, vec![u8::invalid(); nextcap].into_boxed_slice()) }
        } else {
            USizeSet {
                v: Data::Vu16(0, vec![u16::invalid(); nextcap].into_boxed_slice()),
            }
        }
    }
    /// Creates an empty set with the specified capacity.
    pub fn with_max_and_capacity(max: usize, cap: usize) -> USizeSet {
        USizeSet { v: Data::with_max_cap(max, cap) }
    }
    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        match &self.v {
            &Data::Su8(sz,_) => sz as usize,
            &Data::Vu8(sz,_) => sz as usize,
            &Data::Su16(sz,_) => sz as usize,
            &Data::Vu16(sz,_) => sz as usize,
        }
    }
    /// Reserves capacity for at least `additional` more elements to be
    /// inserted in the set. The collection may reserve more space
    /// to avoid frequent reallocations.
    pub fn reserve(&mut self, additional: usize) {
        match self.v {
            Data::Su8(sz, v) if sz as usize + additional > 22 => {
                self.v = Data::Vu8(0, vec![u8::invalid();
                                           ((sz as usize+additional)*11/10).next_power_of_two()]
                                   .into_boxed_slice());
                for i in 0..sz as usize {
                    self.insert_unchecked(v[i] as usize);
                }
            },
            Data::Su8(_,_) => (),
            _ => unimplemented!(),
        }
    }
    /// Reserves capacity for at least `additional` more elements to
    /// be inserted in the set, with maximum value of `max`. The
    /// collection may reserve more space to avoid frequent
    /// reallocations.
    pub fn reserve_with_max(&mut self, max: usize, additional: usize) {
        match self.v {
            Data::Su8(sz, v) if max >= 255 => {
                let mut n = Self::with_max_and_capacity(max, sz as usize + additional);
                for i in 0..sz as usize {
                    n.insert_unchecked(v[i] as usize);
                }
                *self = n;
            },
            Data::Su8(sz, v) if sz as usize + additional > 22 => {
                self.v = Data::Vu8(0, vec![u8::invalid();
                                           ((sz as usize+additional)*11/10).next_power_of_two()]
                                   .into_boxed_slice());
                for i in 0..sz as usize {
                    self.insert_unchecked(v[i] as usize);
                }
            },
            Data::Su8(_,_) => (),
            _ => unimplemented!(),
        }
    }
    fn max_and_cap(&self) -> (usize, usize) {
        match self.v {
            Data::Su8(_, ref v) => (u8::invalid() as usize - 1, v.len()),
            Data::Vu8(_, ref v) => (u8::invalid() as usize - 1, v.len()*10/11),
            Data::Su16(_, ref v) => (u8::invalid() as usize - 1, v.len()),
            Data::Vu16(_, ref v) => (u8::invalid() as usize - 1, v.len()*10/11),
        }
    }
    /// Adds a value to the set.
    ///
    /// If the set did not have this value present, `true` is returned.
    ///
    /// If the set did have this value present, `false` is returned.
    pub fn insert(&mut self, elem: usize) -> bool {
        self.reserve_with_max(elem, 1);
        self.insert_unchecked(elem)
    }
    fn insert_unchecked(&mut self, value: usize) -> bool {
        match self.v {
            Data::Su8(ref mut sz, ref mut v) => {
                let value = value as u8;
                for &x in v.iter().take(*sz as usize) {
                    if x == value {
                        return false;
                    }
                }
                v[*sz as usize] = value;
                *sz += 1;
                true
            },
            Data::Vu8(ref mut sz, ref mut v) => {
                let mut value = value as u8;
                match search(v, value) {
                    SearchResult::Present(_) => false,
                    SearchResult::Empty(i) => {
                        v[i] = value;
                        *sz += 1;
                        true
                    },
                    SearchResult::Richer(i) => {
                        *sz += 1;
                        std::mem::swap(&mut v[i], &mut value);
                        steal(v, i, value);
                        true
                    },
                }
            },
            _ => unimplemented!(),
        }
    }
    /// Returns true if the set contains a value.
    pub fn contains(&self, value: &usize) -> bool {
        let value = *value;
        match self.v {
            Data::Su8(sz, ref v) => {
                if value >= u8::invalid() as usize {
                    return false;
                }
                let value = value as u8;
                for &x in v.iter().take(sz as usize) {
                    if x == value {
                        return true;
                    }
                }
                false
            },
            Data::Vu8(_, ref v) => {
                if value >= u8::invalid() as usize {
                    return false;
                }
                let value = value as u8;
                match search(v, value) {
                    SearchResult::Present(_) => true,
                    SearchResult::Empty(_) => false,
                    SearchResult::Richer(_) => false,
                }
            },
            _ => unimplemented!(),
        }
    }
    /// Removes an element, and returns true if that element was present.
    pub fn remove(&mut self, value: &usize) -> bool {
        let value = *value;
        match self.v {
            Data::Su8(ref mut sz, ref mut v) => {
                if value >= u8::invalid() as usize {
                    return false;
                }
                let value = value as u8;
                let mut i = None;
                for (j, &x) in v.iter().enumerate().take(*sz as usize) {
                    if x == value {
                        i = Some(j);
                        break;
                    }
                }
                return if let Some(i) = i {
                    v[i] = v[*sz as usize -1];
                    *sz -= 1;
                    true
                } else {
                    false
                };
            },
            Data::Vu8(ref mut sz, ref mut v) => {
                if value >= u8::invalid() as usize {
                    return false;
                }
                let value = value as u8;
                match search(v, value) {
                    SearchResult::Present(mut i) => {
                        *sz -= 1;
                        let mask = v.len() - 1;
                        let invalid = u8::invalid();
                        loop {
                            let iplus1 = (i+1) & mask;
                            if v[iplus1] == invalid ||
                                (v[iplus1].hash_usize().wrapping_sub(iplus1) & mask) == 0
                            {
                                v[i] = invalid;
                                return true;
                            }
                            v[i] = v[iplus1];
                            i = iplus1;
                        }
                    },
                    SearchResult::Empty(_) => false,
                    SearchResult::Richer(_) => false,
                }
            },
            _ => unimplemented!(),
        }
    }
    // /// Returns an iterator over the set.
    // pub fn iter(&self) -> Iter {
    //     Iter {
    //         slice: self.v.sl(),
    //         nleft: self.len(),
    //     }
    // }
    // /// Clears the set, returning all elements in an iterator.
    // pub fn drain(&mut self) -> IntoIter {
    //     let set = std::mem::replace(self, USizeSet::new());
    //     let sz = set.len();
    //     IntoIter { set: set, nleft: sz }
    // }
}

// /// An iterator for `USizeSet`.
// pub struct Iter<'a> {
//     slice: &'a [usize],
//     nleft: usize,
// }

// impl<'a, T: 'a+HasInvalid> Iterator for Iter<'a, T> {
//     type Item = &'a T;
//     fn next(&mut self) -> Option<&'a T> {
//         if self.nleft == 0 {
//             None
//         } else {
//             assert!(self.slice.len() >= self.nleft as usize);
//             while self.slice[0] == T::invalid() {
//                 self.slice = self.slice.split_first().unwrap().1;
//             }
//             let val = &self.slice[0];
//             self.slice = self.slice.split_first().unwrap().1;
//             self.nleft -= 1;
//             Some(val)
//         }
//     }
//     fn size_hint(&self) -> (usize, Option<usize>) {
//         (self.nleft, Some(self.nleft))
//     }
// }

// impl IntoIterator for &USizeSet {
//     type Item = &T;
//     type IntoIter = Iter;

//     fn into_iter(self) -> Iter {
//         self.iter()
//     }
// }

// /// An iterator for `USizeSet`.
// pub struct IntoIter {
//     set: USizeSet,
//     nleft: usize,
// }

// impl Iterator for IntoIter {
//     type Item = usize;
//     fn next(&mut self) -> Option<&usize> {
//         if self.nleft == 0 {
//             None
//         } else {
//             self.nleft -= 1;
//             let mut i = self.nleft;
//             loop {
//                 let val = std::mem::replace(&mut self.set.v.mu()[i], T::invalid());
//                 if val != T::invalid() {
//                     return Some(val);
//                 }
//                 i -= 1;
//             }
//         }
//     }
//     fn size_hint(&self) -> (usize, Option<usize>) {
//         (self.nleft, Some(self.nleft))
//     }
// }


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use rand::{XorShiftRng, SeedableRng, Rand};
    #[test]
    fn it_works() {
        let mut ss: USizeSet<usize> = USizeSet::new();
        println!("inserting 5");
        ss.insert(5);
        println!("contains 5");
        assert!(ss.contains(&5));
        println!("contains 4");
        assert!(!ss.contains(&4));
        println!("inserting 3");
        ss.insert(3);
        println!("now {:?}", &ss);
        assert!(ss.contains(&3));
        assert!(ss.contains(&5));
        assert_eq!(ss.len(), 2);
        for num in ss.iter() {
            println!("num is {}", num);
            assert!(ss.contains(num));
        }
        assert!(!ss.remove(&2));
        assert!(ss.remove(&3));
        assert!(!ss.contains(&3));
        assert_eq!(ss.len(), 1);
    }
    #[test]
    fn size_unwasted() {
        println!("small size: {}", std::mem::size_of::<USizeSet<usize>>());
        println!(" hash size: {}", std::mem::size_of::<HashSet<usize>>());
        assert!(std::mem::size_of::<USizeSet>() <=
                2*std::mem::size_of::<HashSet<usize>>());
        assert!(std::mem::size_of::<USizeSet>() <= 24);
    }

    macro_rules! initialize {
        ($set: ident, $item: ident, $num: expr) => {{
            let mut rng = XorShiftRng::from_seed([$num as u32,$num as u32,3,4]);
            let mut set = $set::<$item>::new();
            let mut refset = HashSet::<$item>::new();
            if $num > 0 {
                while set.len() < $num {
                    let ins = $item::rand(&mut rng) % (2*$num as $item);
                    let rem = $item::rand(&mut rng) % (2*$num as $item);
                    set.insert(ins);
                    if !set.contains(&ins) {
                        println!("oops insert");
                    }
                    set.remove(&rem);
                    if set.contains(&rem) {
                        println!("oops remove");
                    }
                    refset.insert(ins);
                    refset.remove(&rem);
                    println!("inserting {}, removing {} => {}", ins, rem, set.len());
                    println!("set: {:?}", set);
                    println!("refset: {:?}", refset);
                    let mut fails = false;
                    for i in 0..255 {
                        fails = fails || set.contains(&i) != refset.contains(&i);
                    }
                    if fails {
                        for i in 0..255 {
                            println!("i {}", i);
                            assert_eq!(set.contains(&i), refset.contains(&i));
                        }
                    }
                }
            }
            set
        }};
    }

    #[test]
    fn random_inserts_and_removals_u8() {
        for sz in 0..50 {
            println!("\nUSizeSet {}\n", sz);
            let myset = initialize!(USizeSet, u8, sz);
            println!("\nHashSet {}\n", sz);
            let refset = initialize!(HashSet, u8, sz);
            for i in 0..255 {
                assert_eq!(myset.contains(&i), refset.contains(&i));
            }
        }
    }

    #[test]
    fn random_inserts_and_removals_u16() {
        for sz in 0..20 {
            println!("\nUSizeSet {}\n", sz);
            let myset = initialize!(USizeSet, u16, sz);
            println!("\nHashSet {}\n", sz);
            let refset = initialize!(HashSet, u16, sz);
            for i in 0..50 {
                assert_eq!(myset.contains(&i), refset.contains(&i));
            }
        }
    }

    #[test]
    fn test_matches_u8() {
        let mut steps: Vec<Result<u8,u8>> = vec![Err(8), Ok(0), Ok(16), Ok(1), Ok(8)];
        let mut set = USizeSet::<u8>::new();
        let mut refset = HashSet::<u8>::new();
        loop {
            match steps.pop() {
                Some(Ok(v)) => {
                    println!("\ninserting {}", v);
                    set.insert(v); refset.insert(v);
                },
                Some(Err(v)) => {
                    println!("\nremoving {}", v);
                    set.remove(&v); refset.remove(&v);
                },
                None => return,
            }
            println!("set: {:?}", set);
            println!("refset: {:?}", refset);
            assert_eq!(set.len(), refset.len());
            for i in 0..255 {
                if set.contains(&i) != refset.contains(&i) {
                    println!("trouble at {}", i);
                    assert_eq!(set.contains(&i), refset.contains(&i));
                }
            }
        }
    }

    #[cfg(test)]
    quickcheck! {
        fn prop_matches_u8(steps: Vec<Result<u8,u8>>) -> bool {
            let mut steps = steps;
            let mut set = USizeSet::<u8>::new();
            let mut refset = HashSet::<u8>::new();
            loop {
                match steps.pop() {
                    Some(Ok(v)) => {
                        set.insert(v); refset.insert(v);
                    },
                    Some(Err(v)) => {
                        set.remove(&v); refset.remove(&v);
                    },
                    None => return true,
                }
                if set.len() != refset.len() { return false; }
                for i in 0..255 {
                    if set.contains(&i) != refset.contains(&i) { return false; }
                }
            }
        }
    }

    #[cfg(test)]
    quickcheck! {
        fn prop_matches_usize(steps: Vec<Result<usize,usize>>) -> bool {
            let mut steps = steps;
            let mut set = USizeSet::new();
            let mut refset = HashSet::<usize>::new();
            loop {
                match steps.pop() {
                    Some(Ok(v)) => {
                        set.insert(v); refset.insert(v);
                    },
                    Some(Err(v)) => {
                        set.remove(&v); refset.remove(&v);
                    },
                    None => return true,
                }
                if set.len() != refset.len() { return false; }
                for i in 0..2550 {
                    if set.contains(&i) != refset.contains(&i) { return false; }
                }
            }
        }
    }
}

fn search<T: HasInvalid>(v: &[T], elem: T) -> SearchResult {
    let h = elem.hash_usize();
    let invalid = T::invalid();
    let mut dist = 0;
    let mask = v.len() - 1;
    loop {
        let i = h+dist & mask;
        if v[i] == invalid {
            return SearchResult::Empty(i);
        } else if v[i] == elem {
            return SearchResult::Present(i);
        }
        // the following is a bit contorted, to compute distance
        // when wrapped.
        let his_dist = i.wrapping_sub(v[i].hash_usize()) & mask;
        if his_dist < dist {
            return SearchResult::Richer(i);
        }
        dist += 1;
        assert!(dist <= v.len());
    }
}

fn search_from<T: HasInvalid>(v: &[T], i_start: usize, elem: T) -> SearchResult {
    let h = elem.hash_usize();
    let mask = v.len() - 1;
    let invalid = T::invalid();
    let mut dist = i_start.wrapping_sub(h) & mask;
    loop {
        let i = h+dist & mask;
        if v[i] == invalid {
            return SearchResult::Empty(i);
        } else if v[i] == elem {
            return SearchResult::Present(i);
        }
        // the following is a bit contorted, to compute distance
        // when wrapped.
        let his_dist = i.wrapping_sub(v[i].hash_usize()) & mask;
        if his_dist < dist {
            return SearchResult::Richer(i);
        }
        dist += 1;
        assert!(dist <= v.len());
    }
}

fn steal<T: HasInvalid>(v: &mut [T], mut i: usize, mut elem: T) {
    loop {
        match search_from(v, i, elem) {
            SearchResult::Present(_) => return,
            SearchResult::Empty(i) => {
                v[i] = elem;
                return;
            },
            SearchResult::Richer(inew) => {
                std::mem::swap(&mut elem, &mut v[inew]);
                i = inew;
            },
        }
    }
}