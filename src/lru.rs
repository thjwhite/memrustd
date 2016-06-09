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

    fn get(&mut self, key: String) -> Result<Arc<RefCell<LruEntry<T>>>, Error> {
        //let entry = self.cache.get(&key);
        let value: Arc<RefCell<LruEntry<T>>> = match self.cache.get(&key) {
            None => {
                return Err(Error::new(ErrorKind::Other, "Not found"));
            }
            Some(value) => {
                value.clone()
            }
        };
        
        self.close_gap(value.clone());
        (*value).borrow_mut().prev = None;
        match self.front {
            None => {
                /* How did we get here? */
            }
            Some(ref _front) => {
                value.borrow_mut().next = Some(_front.clone());
                _front.borrow_mut().prev = Some(Arc::downgrade(&value));
            }
        }
        self.front = Some(value.clone());
        Ok(value.clone())

    }

    fn insert(&mut self, key: String, data: T) {
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

    fn close_gap(&mut self, entry: Arc<RefCell<LruEntry<T>>>) {
        match (*entry).borrow_mut().prev {
            None => {
                let mut new_front = None;
                match self.front {
                    None => {},
                    Some(ref _front) => {
                        match (**_front).borrow_mut().next {
                            None => {},
                            Some(ref _second) => {
                                // the element after the remove one is not the first
                                new_front = Some(_second.clone());
                                (**_second).borrow_mut().prev = None;
                            }
                        }
                    }
                }
                self.front = new_front;
            },
            Some(ref _prev) => {
                match (*entry).borrow_mut().next {
                    None => {
                        match _prev.upgrade() {
                            None => {/* broken */},
                            Some(ref _prev_arc) => {
                                (*_prev_arc).borrow_mut().next = None;
                            }
                        }
                    },
                    Some(ref _next) => {
                        (*_next).borrow_mut().prev = Some(_prev.clone()); 
                        match _prev.upgrade() {
                            None => {/* broken */}
                            Some(ref _prev_arc) => {
                                (*_prev_arc).borrow_mut().next = Some(_next.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    fn remove(&mut self, key: String) -> Result<Arc<RefCell<LruEntry<T>>>, Error> {
        match self.cache.remove(&key) {
            None => {
                Err(Error::new(ErrorKind::Other, "Not found"))
            },
            Some(entry) => {
                self.close_gap(entry.clone());
                Ok(entry)
            }
        }
    }

}
