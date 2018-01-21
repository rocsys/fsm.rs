extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

extern crate serde;
#[macro_use]
extern crate serde_derive;

use fsm::*;

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct Event1 { v: usize }
impl FsmEvent for Event1 {}

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct EventGo;
impl FsmEvent for EventGo {}

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct EventRestart;
impl FsmEvent for EventRestart {}

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct EventStart;
impl FsmEvent for EventStart {}

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct EventStop;
impl FsmEvent for EventStop {}

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct StateA { a: usize }
impl FsmState<FsmMinTwo> for StateA { }

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct StateB { b: usize }
impl FsmState<FsmMinTwo> for StateB { }

#[derive(Clone, PartialEq, Default, Debug, Serialize)]
pub struct StateC { c: usize }
impl FsmState<FsmMinTwo> for StateC { }

#[derive(Fsm)]
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
#[test]
fn test_fsm_min2() {
    let mut fsm = FsmMinTwo::new(()).unwrap();
    fsm.start();
    assert_eq!(FsmMinTwoStates::StateA, fsm.get_current_state());

    fsm.process_event(FsmMinTwoEvents::EventStart(EventStart)).unwrap();
    assert_eq!(FsmMinTwoStates::StateB, fsm.get_current_state());
}