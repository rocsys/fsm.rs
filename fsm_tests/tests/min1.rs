extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use async_trait::async_trait;

use fsm::*;

#[derive(Clone, PartialEq, Default)]
pub struct StaticA;

#[async_trait]
impl FsmState<FsmMinOne> for StaticA {

}

#[derive(Fsm)]
struct FsmMinOneDefinition(
	InitialState<FsmMinOne, StaticA>
);


#[cfg(test)]
#[tokio::test]
async fn test_fsm_min1() {
    let mut fsm = FsmMinOne::new(());
    fsm.start().await;
    assert_eq!(FsmMinOneStates::StaticA, fsm.get_current_state());
}