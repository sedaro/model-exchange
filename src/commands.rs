use std::time::Duration;
use crate::model::sedaroml::ModelDiff;

#[derive(Debug)]
pub enum NodeCommands {
  /// Signals a Node to start
  Start,
  /// Signals a Node to stop
  Stop,
  /// Signals a Node that the exchange has changed its SedaroML representation (on disk).  This signal is not sent if the translation round did not change the node.
  Changed(ModelDiff),
  /// Signals a Node that the exchange has completed a translation round
  Done,
  /// Signals to a Node to fix its conflict via a particular resolution strategy
  ResolveConflict(ConflictResolutions),
}

#[derive(Debug)]
pub enum NodeResponses {
  /// Signal to acknowledge that a Node has started successfully and is running
  Started,
  /// Signal to indicate existence of model conflicts in the node that need to be resolved
  Conflict(ModelDiff),
  /// Signal to acknowledge that a Node has stopped
  Stopped,
  // Signals the Exchange that the Node has completed all side-effects to a `Changed` command (includes Duration to complete all side-effects)
  Done(Duration),
  // Signals the Exchange that the Node has completed all side-effects to a `ResolveConflict` command (includes Duration to complete all side-effects)
  ConflictResolved(Duration),
}

#[derive(Debug)]
pub enum ConflictResolutions {
  /// Signals to keep the current representation and reconcile the source document/model
  KeepRep,
  /// Signals to update the current representation from the source document/model
  UpdateRep,
}