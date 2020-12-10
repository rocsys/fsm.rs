extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use async_trait::async_trait;


use fsm::*;


// events

#[derive(Clone, PartialEq, Default, Debug)]
pub struct EventA;
impl FsmEvent for EventA {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct EventA2;
impl FsmEvent for EventA2 {}


#[derive(Clone, PartialEq, Default, Debug)]
pub struct EventB;
impl FsmEvent for EventB {}


#[derive(Clone, PartialEq, Default, Debug)]
pub struct ErrorDetected;
impl FsmEvent for ErrorDetected {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct ErrorFixed;
impl FsmEvent for ErrorFixed {}


// states

#[derive(Clone, PartialEq, Default)]
pub struct InitialA;
#[async_trait]
impl<'a> FsmState<Ortho<'a>> for InitialA { }

#[derive(Clone, PartialEq, Default)]
pub struct InitialB;
#[async_trait]
impl<'a> FsmState<Ortho<'a>> for InitialB { }


#[derive(Clone, PartialEq, Default)]
pub struct StateA;
#[async_trait]
impl<'a> FsmState<Ortho<'a>> for StateA { }

#[derive(Clone, PartialEq, Default)]
pub struct StateB;
#[async_trait]
impl<'a> FsmState<Ortho<'a>> for StateB { }

#[derive(Clone, PartialEq, Default)]
pub struct FixedC;
#[async_trait]
impl<'a> FsmState<Ortho<'a>> for FixedC { }



#[derive(Clone, PartialEq, Default)]
pub struct AllOk;
#[async_trait]
impl<'a> FsmState<Ortho<'a>> for AllOk { }

#[derive(Clone, PartialEq, Default)]
pub struct ErrorMode;
#[async_trait]
impl<'a> FsmState<Ortho<'a>> for ErrorMode { }

#[allow(dead_code)]
pub struct OrthoContext<'a> {
    id: &'a str
}


#[derive(Fsm)]
#[allow(dead_code)]
struct OrthoDefinition<'a>(
    InitialState<Ortho<'a>, (InitialA, InitialB, FixedC, AllOk)>,
	ContextType<OrthoContext<'a>>,


    Transition        < Ortho<'a>,  InitialA,  EventA,   StateA,   NoAction>,
    Transition        < Ortho<'a>,  StateA,    EventA2,  InitialA, NoAction>,

    Transition        < Ortho<'a>,  InitialB,  EventB,   StateB, NoAction>,

    Transition        < Ortho<'a>,  AllOk,     ErrorDetected, ErrorMode, NoAction >,
	Transition        < Ortho<'a>,  ErrorMode, ErrorFixed,    AllOk,     NoAction >,

    // In case the current state is "ErrorMode", every other event other than "ErrorFixed" is blocked.
    InterruptState    < Ortho<'a>,  ErrorMode, ErrorFixed >
);


#[cfg(test)]
#[tokio::test]
async fn test_orthogonal() {

    let id = "fsm_a";
    let ctx = OrthoContext {
        id: &id
    };
	let mut fsm = Ortho::new(ctx);

	fsm.start().await;

    assert_eq!((OrthoStates::InitialA, OrthoStates::InitialB, OrthoStates::FixedC, OrthoStates::AllOk), fsm.get_current_state());

    fsm.process_event(OrthoEvents::EventA(EventA)).await.unwrap();
    assert_eq!((OrthoStates::StateA, OrthoStates::InitialB, OrthoStates::FixedC, OrthoStates::AllOk), fsm.get_current_state());

    fsm.process_event(OrthoEvents::EventB(EventB)).await.unwrap();
    assert_eq!((OrthoStates::StateA, OrthoStates::StateB, OrthoStates::FixedC, OrthoStates::AllOk), fsm.get_current_state());


    fsm.process_event(OrthoEvents::ErrorDetected(ErrorDetected)).await.unwrap();
    assert_eq!((OrthoStates::StateA, OrthoStates::StateB, OrthoStates::FixedC, OrthoStates::ErrorMode), fsm.get_current_state());

    assert_eq!(fsm.process_event(OrthoEvents::EventA2(EventA2)).await, Err(FsmError::Interrupted));

    fsm.process_event(OrthoEvents::ErrorFixed(ErrorFixed)).await.unwrap();
    assert_eq!((OrthoStates::StateA, OrthoStates::StateB, OrthoStates::FixedC, OrthoStates::AllOk), fsm.get_current_state());



}