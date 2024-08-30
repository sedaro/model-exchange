use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct QueuedSet<G> {
  queue: Vec<G>,
}
impl<G: std::cmp::PartialEq> QueuedSet<G> {
  pub fn new() -> QueuedSet<G> {
    QueuedSet { queue: vec![] }
  }
  pub fn enqueue(&mut self, change_iden: G) {
    if !self.queue.contains(&change_iden) { self.queue.push(change_iden) }
  }
  pub fn dequeue(&mut self) -> Option<G> {
    if self.queue.len() > 0 { Some(self.queue.remove(0)) }
    else { None }
  }
  pub fn peek(&self) -> Option<&G> {
    if self.queue.len() > 0 { Some(&self.queue[0]) }
    else { None }
  }
}

pub type ChangeQueue = Arc<Mutex<QueuedSet<String>>>;