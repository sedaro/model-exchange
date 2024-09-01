use std::time::Duration;

#[derive(Debug)]
pub enum NodeCommands {
    /// Signals a Node to start
    Start,
    /// Signals a Node to stop
    Stop,
    /// Signals a Node that the exchange has changed its SedaroML representation (on disk).  This signal is not sent if the translation round did not change the node.
    Changed,
    /// Signals a Node that the exchange has completed a translation round
    Done,
}

#[derive(Debug)]
pub enum NodeResponses {
    /// Signal to acknowledge that a Node has started
    Started,
    /// Signal to acknowledge that a Node has stopped
    Stopped,
    // Signals the Exchange that the Node has completed all side-effects to a `Changed` command (includes Duration to complete all side-effects)
    Done(Duration),
}