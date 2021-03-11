
extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use std::sync::Arc;

use async_trait::async_trait;

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

#[derive(Clone, Default)]
struct Initial;

#[derive(Clone, Default)]
struct InitialWithFailure;

#[derive(Clone, Default)]
struct ProcessWithFailure;

#[derive(Clone, Default)]
struct Error;

#[derive(Clone, Default)]
struct Recovered;

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

	assert_eq!(fsm.get_current_state().await, BrokenStates::InitialWithFailure);

	fsm.start().await;

	assert_eq!(fsm.get_current_state().await, BrokenStates::Error);
}

#[cfg(test)]
#[tokio::test]
async fn test_error_during_processing_event() {
	let fsm = Parent::new(&Default::default());

	assert_eq!(fsm.get_current_state().await, ParentStates::Initial);

	fsm.start().await;

	assert_eq!(fsm.get_current_state().await, ParentStates::Initial);

  fsm.process_event(ParentEvents::GoToProcess(GoToProcess)).await.unwrap();

  assert_eq!(fsm.get_current_state().await, ParentStates::Error);
}

#[cfg(test)]
#[tokio::test]
async fn test_error_recovery() {
	let fsm = Parent::new(&Default::default());

	fsm.start().await;

  fsm.process_event(ParentEvents::GoToProcess(GoToProcess)).await.unwrap();

  assert_eq!(fsm.get_current_state().await, ParentStates::Error);

  fsm.process_event(ParentEvents::GoToRecovered(GoToRecovered)).await.unwrap();

  assert_eq!(fsm.get_current_state().await, ParentStates::Recovered);
}

#[cfg(test)]
#[tokio::test]
async fn test_error_in_initial_state_of_child() {
	let parent = Parent::new(&Default::default());

	parent.start().await;

  parent.process_event(ParentEvents::GoToBrokenChild(GoToBrokenChild)).await.unwrap();

  assert_eq!(parent.get_current_state().await, ParentStates::BrokenChild);

  {
    let child: &BrokenChild = parent.get_state();
    assert_eq!(BrokenChildStates::Error, child.get_current_state().await);
  }
}

#[cfg(test)]
#[tokio::test]
async fn test_error_during_processing_event_in_child() {
	let parent = Parent::new(&Default::default());

	parent.start().await;

  parent.process_event(ParentEvents::GoToChild(GoToChild)).await.unwrap();

  assert_eq!(parent.get_current_state().await, ParentStates::Child);

  {
    let child: &Child = parent.get_state();
    assert_eq!(ChildStates::Initial, child.get_current_state().await);

    child.process_event(ChildEvents::GoToProcess(GoToProcess)).await.unwrap();

    assert_eq!(ChildStates::Error, child.get_current_state().await);
  }
}