extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use async_trait::async_trait;

use fsm::*;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Event1 { v: usize }
impl FsmEvent for Event1 {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct EventGo;
impl FsmEvent for EventGo {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct EventRestart;
impl FsmEvent for EventRestart {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct EventStart;
impl FsmEvent for EventStart {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct EventStop;
impl FsmEvent for EventStop {}

#[derive(Clone, PartialEq, Default)]
pub struct StateA { a: usize }
#[async_trait]
impl FsmState<FsmMinTwo> for StateA { }

#[derive(Clone, PartialEq, Default)]
pub struct StateB { b: usize }
#[async_trait]
impl FsmState<FsmMinTwo> for StateB { }

#[derive(Clone, PartialEq, Default)]
pub struct StateC { c: usize }
#[async_trait]
impl FsmState<FsmMinTwo> for StateC { }

#[derive(Fsm)]
#[allow(dead_code)]
struct FsmMinTwoDefinition(
	InitialState<FsmMinTwo, StateA>,

    Transition         < FsmMinTwo,  StateA,        EventStart,        StateB,    NoAction >,
    Transition         < FsmMinTwo,  StateB,        EventStop,         StateA,    NoAction >,
    Transition         < FsmMinTwo,  StateB,        EventGo,           StateC,    NoAction >,

    Transition         < FsmMinTwo, (StateA, StateB, StateC),
                                                    EventRestart,      StateA,    NoAction >,

    TransitionInternal < FsmMinTwo,  StateA,        Event1,                       NoAction >,

    InterruptState     < FsmMinTwo,  StateB,        EventStop >
);

#[cfg(test)]
#[tokio::test]
async fn test_fsm_min2() {
    let fsm = FsmMinTwo::new(&Default::default());
    fsm.start().await;
    assert_eq!(FsmMinTwoStates::StateA, fsm.get_current_state().await);

    fsm.process_event(FsmMinTwoEvents::EventStart(EventStart)).await.unwrap();
    assert_eq!(FsmMinTwoStates::StateB, fsm.get_current_state().await);
}