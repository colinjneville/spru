use std::{cell::Cell, rc::Rc, sync::Arc};

#[derive(Debug, Clone)]
struct PendingReadyCount(Rc<Cell<usize>>);

impl PendingReadyCount {
    pub fn new() -> Self {
        Self(Rc::new(Cell::new(1)))
    }

    pub fn increment(&self) -> Result<(), ()> {
        let mut value = self.0.get();
        if value != 0 {
            value += 1;
            self.0.set(value);

            Ok(())
        } else {
            Err(())
        }
    }

    pub fn decrement(&self) -> Result<bool, ()> {
        let mut value = self.0.get();
        if value != 0 {
            value -= 1;
            self.0.set(value);

            Ok(value == 0)
        } else {
            Err(())
        }
    }
}

#[derive(Debug)]
pub struct ReadyStatus {
    local_ready: bool,
    pending_ready: PendingReadyCount,
    all_ready_event: Arc<event_listener::Event>,
}

impl Drop for ReadyStatus {
    fn drop(&mut self) {
        if !self.local_ready {
            if self.pending_ready.decrement() == Ok(true) {
                self.notify();
            }
        }
    }
}

impl ReadyStatus {
    pub fn new(all_ready_event: Arc<event_listener::Event>) -> Self {
        Self {
            local_ready: false,
            pending_ready: PendingReadyCount::new(),
            all_ready_event,
        }
    }

    fn notify(&self) {
        self.all_ready_event.notify(usize::MAX);
    }

    pub fn try_clone(&self) -> Result<Self, ()> {
        self.pending_ready.increment()?;

        Ok(Self { 
            local_ready: false, 
            pending_ready: self.pending_ready.clone(), 
            all_ready_event: self.all_ready_event.clone() 
        })
    }

    pub fn is_ready(&self) -> bool {
        self.local_ready
    }

    pub async fn set_ready(&mut self, ready: bool) -> Result<bool, ()> {
        if ready {
            Ok(self.ready().await)
        } else {
            self.unready().await
        }
    }

    pub async fn ready(&mut self) -> bool {
        if !self.local_ready {
            self.local_ready = true;

            if self.pending_ready.decrement().expect("Invalid ready count") {
                self.notify();
            }

            true
        } else {
            false
        }
    }

    pub async fn unready(&mut self) -> Result<bool, ()> {
        if self.local_ready {
            self.local_ready = false;

            self.pending_ready.increment()?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}