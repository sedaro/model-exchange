use crate::change_queue::ChangeQueue;
pub trait Watcher {
  fn identifier(&self) -> String;
  fn change_queue(&self) -> Option<ChangeQueue>;
  fn bind(&mut self, change_queue: ChangeQueue);
  fn trigger(&self) {
    match self.change_queue() {
      None => panic!("Watcher not bound to a change queue"),
      Some(queue) => {
        queue.lock().unwrap().enqueue(self.identifier());
      },
    }
  }
}