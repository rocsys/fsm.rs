
extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use std::sync::Arc;

use async_trait::async_trait;
use assert_matches::assert_matches;

use fsm::*;

#[derive(Clone, PartialEq, Debug)]
pub struct GoToProcess;
impl FsmEvent for GoToProcess {}

#[derive(Clone, PartialEq, Debug)]
pub struct GoToRecovered;
impl FsmEvent for GoToRecovered {}

#[derive(Clone, PartialEq, Debug)]
pub struct GoToChild;
impl FsmEvent for GoToChild {}

#[derive(Clone, PartialEq, Debug)]
pub struct GoToBrokenChild;
impl FsmEvent for GoToBrokenChild {}

// ------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Initial;

#[derive(Debug, Clone, Default)]
pub struct InitialWithFailure;

#[derive(Debug, Clone, Default)]
pub struct ProcessWithFailure;

#[derive(Debug, Clone, Default)]
pub struct Error;

#[derive(Debug, Clone, Default)]
pub struct Recovered;

// ------------------------------------------

trait WithFailure {
  fn fail(&self) -> FsmTransitionResult<()> {
    Err(FsmTransitionError(
      Arc::new(std::io::Error::last_os_error().into())
    ))
  }
}

impl WithFailure for InitialWithFailure {}

impl WithFailure for ProcessWithFailure {}

// ------------------------------------------

#[async_trait]
impl FsmState<Broken> for InitialWithFailure {
  async fn on_entry(&self, _: &EventContext<'_, Broken>) -> FsmTransitionResult<()> {
    self.fail()
  }
}

#[async_trait]
impl FsmState<Broken> for Error {}

// ------------------------------------------

#[async_trait]
impl FsmState<Parent> for Initial {}

#[async_trait]
impl FsmState<Parent> for ProcessWithFailure {
  async fn on_entry(&self, _: &EventContext<'_, Parent>) -> FsmTransitionResult<()> {
    self.fail()
  }
}

#[async_trait]
impl FsmState<Parent> for Error {}

#[async_trait]
impl FsmState<Parent> for Recovered {}

#[async_trait]
impl FsmState<Parent> for Child {}

#[async_trait]
impl FsmState<Parent> for BrokenChild {}

// ------------------------------------------

#[async_trait]
impl FsmState<BrokenChild> for InitialWithFailure {
  async fn on_entry(&self, _: &EventContext<'_, BrokenChild>) -> FsmTransitionResult<()> {
    self.fail()
  }
}

#[async_trait]
impl FsmState<BrokenChild> for Error {}

// --------------------------------------------

#[async_trait]
impl FsmState<Child> for Initial {}

#[async_trait]
impl FsmState<Child> for ProcessWithFailure {
  async fn on_entry(&self, _: &EventContext<'_, Child>) -> FsmTransitionResult<()> {
    self.fail()
  }
}

#[async_trait]
impl FsmState<Child> for Error {}

// --------------------------------------------

#[async_trait]
impl FsmStateFactory<()> for Child {
  fn new_state(parent_context: &FsmArc<()>) -> Self {
    Child::new(parent_context)
  }
}

#[async_trait]
impl FsmStateFactory<()> for BrokenChild {
  fn new_state(parent_context: &FsmArc<()>) -> Self {
    BrokenChild::new(parent_context)
  }
}

// --------------------------------------------

#[derive(Fsm)]
#[allow(dead_code)]
struct BrokenDefinition(
	InitialState<Broken, InitialWithFailure>,
  ErrorState<Broken, Error>,
);

#[derive(Fsm)]
#[allow(dead_code)]
struct ParentDefinition(
	InitialState<Parent, Initial>,
  ErrorState<Parent, Error>,

  SubMachine<Child>,
  SubMachine<BrokenChild>,

  Transition<Parent, Initial, GoToProcess, ProcessWithFailure, NoAction>,
  Transition<Parent, Error, GoToRecovered, Recovered, NoAction>,

  Transition<Parent, Initial, GoToChild, Child, NoAction>,
  Transition<Parent, Initial, GoToBrokenChild, BrokenChild, NoAction>,
);

#[derive(Fsm)]
#[allow(dead_code)]
struct BrokenChildDefinition(
	InitialState<BrokenChild, InitialWithFailure>,
  ErrorState<BrokenChild, Error>,
);

#[derive(Fsm)]
#[allow(dead_code)]
struct ChildDefinition(
	InitialState<Child, Initial>,
  ErrorState<Child, Error>,

  Transition<Child, Initial, GoToProcess, ProcessWithFailure, NoAction>,
);

#[cfg(test)]
#[tokio::test]
async fn test_error_in_initial_state() {
	let fsm = Broken::new(&Default::default());

	assert_matches!(fsm.get_current_state().await, BrokenStates::InitialWithFailure(_));

	fsm.start().await;

	assert_matches!(fsm.get_current_state().await, BrokenStates::Error(_));
}

#[cfg(test)]
#[tokio::test]
async fn test_error_during_processing_event() {
	let fsm = Parent::new(&Default::default());

	assert_matches!(fsm.get_current_state().await, ParentStates::Initial(_));

	fsm.start().await;

	assert_matches!(fsm.get_current_state().await, ParentStates::Initial(_));

  fsm.process_event(ParentEvents::GoToProcess(GoToProcess)).await.unwrap();

  assert_matches!(fsm.get_current_state().await, ParentStates::Error(_));
}

#[cfg(test)]
#[tokio::test]
async fn test_error_recovery() {
	let fsm = Parent::new(&Default::default());

	fsm.start().await;

  fsm.process_event(ParentEvents::GoToProcess(GoToProcess)).await.unwrap();

  assert_matches!(fsm.get_current_state().await, ParentStates::Error(_));

  fsm.process_event(ParentEvents::GoToRecovered(GoToRecovered)).await.unwrap();

  assert_matches!(fsm.get_current_state().await, ParentStates::Recovered(_));
}

#[cfg(test)]
#[tokio::test]
async fn test_error_in_initial_state_of_child() {
	let parent = Parent::new(&Default::default());

	parent.start().await;

  parent.process_event(ParentEvents::GoToBrokenChild(GoToBrokenChild)).await.unwrap();

  assert_matches!(parent.get_current_state().await, ParentStates::BrokenChild(_));

  {
    let child: FsmArc<BrokenChild> = parent.get_state();
    let child = child.read().await;

    assert_matches!(child.get_current_state().await, BrokenChildStates::Error(_));
  }
}

#[cfg(test)]
#[tokio::test]
async fn test_error_during_processing_event_in_child() {
	let parent = Parent::new(&Default::default());

	parent.start().await;

  parent.process_event(ParentEvents::GoToChild(GoToChild)).await.unwrap();

  assert_matches!(parent.get_current_state().await, ParentStates::Child(_));

  {
    let child: FsmArc<Child> = parent.get_state();
    let child = child.read().await;

    assert_matches!(child.get_current_state().await, ChildStates::Initial(_));

    child.process_event(ChildEvents::GoToProcess(GoToProcess)).await.unwrap();

    assert_matches!(child.get_current_state().await, ChildStates::Error(_));
  }
}
