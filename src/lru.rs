use std::sync::Arc;
use std::sync::Weak;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Error;
use std::io::ErrorKind;

struct LruEntry<T> {
    key: String,
    data: T,
    next: Option<Arc<RefCell<LruEntry<T>>>>,
    prev: Option<Weak<RefCell<LruEntry<T>>>>
}

/*impl<'a, T> LruEntry<'a, T> {
    
    fn new(key: &'a str, 
           data: &'a T, 
           next: Option<Box<&'a LruEntry<'a, T>>>, 
           prev: Option<&'a LruEntry<'a, T>>) -> LruEntry<'a, T> {
        LruEntry {
            key: key,
            data: data,
            next: next,
            prev: prev
        }
    }
}*/

struct LruCache<T> {
    cache: HashMap<String, Arc<RefCell<LruEntry<T>>>>,
    front: Option<Arc<RefCell<LruEntry<T>>>>,
    back: Option<Arc<RefCell<LruEntry<T>>>>
}

impl<T> LruCache<T> {
    
    fn new() -> LruCache<T> {
        let _cache = HashMap::new();
        LruCache {
            cache: _cache,
            front: None,
            back: None
        }
    }

    fn insert(&mut self, key: String, data: T) {
        /*let mut entry = Arc::new(LruEntry {
            key: key,
            data: data, 
            next: None,
            prev: None});*/
        
        let entry: Arc<RefCell<LruEntry<T>>>;
        match self.back {
            None => {
                entry = Arc::new(RefCell::new(LruEntry {
                    key: key.clone(),
                    data: data, 
                    next: None,
                    prev: None}));
                self.front = Some(entry.clone());
            },
            Some(ref _back_ptr) => {
                entry = Arc::new(RefCell::new(LruEntry {
                    key: key.clone(),
                    data: data, 
                    next: None,
                    prev: Some(Arc::downgrade(&_back_ptr))}));
                (**_back_ptr).borrow_mut().next = Some(entry.clone());
            }
        }

        self.back = Some(entry.clone());
        self.cache.insert(key, entry.clone());
    }

    fn remove(&mut self, key: String) -> Result<Arc<RefCell<LruEntry<T>>>, Error> {
        match self.cache.remove(&key) {
            None => {
                Err(Error::new(ErrorKind::Other, "Not found"))
            },
            Some(entry) => {
                match (*entry).borrow_mut().prev {
                    None => {
                        // first element in the list.
                        //self.front = Some((*entry).borrow().next);
                        match self.front {
                            None => {},
                            Some(ref _front) => {
                                match (**_front).borrow_mut().next {
                                    None => {
                                        // front was only elem in list set front to None
                                        self.front = None;
                                    },
                                    Some(ref _second) => {
                                        // the element after the remove one is not the first
                                        self.front = Some(_second.clone());
                                        (**_second).borrow_mut().prev = None;
                                    }
                                }
                            }
                        }
                        Ok(entry)
                    },
                    Some(ref _prev) => {
                        // not the first element.
                        //Arc::get_mut(_prev.upgrade()).next = entry.next;

                        /*match _prev.upgrade() {
                            None => ()
                        }*/

                        match (*entry).borrow_mut().next {
                            None => {
                                //(*_prev.upgrade()).next = None;
                                match _prev.upgrade() {
                                    None => {/* broken */},
                                    Some(ref _prev_arc) => {
                                        (*_prev_arc).borrow_mut().next = None;
                                    }
                                }
                            },
                            Some(ref _next) => {
                                (*_next).borrow_mut().prev = Some(_prev.clone()); 
                                //(*_prev).next = Some(_next.clone());
                                match _prev.upgrade() {
                                    None => {/* broken */}
                                    Some(ref _prev_arc) => {
                                        (*_prev_arc).borrow_mut().next = Some(_next.clone());
                                    }
                                }
                            }
                        }
                        Ok(entry)
                    }
                }
            }
        }
    }

}
