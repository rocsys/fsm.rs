extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use async_trait::async_trait;


use fsm::*;

// events
#[derive(Clone, PartialEq, Debug)]
pub struct Play;
impl FsmEvent for Play { }

#[derive(Clone, PartialEq, Debug)]
pub struct EndPause;
impl FsmEvent for EndPause {}

#[derive(Clone, PartialEq, Debug)]
pub struct Stop;
impl FsmEvent for Stop {}

#[derive(Clone, PartialEq, Debug)]
pub struct Pause;
impl FsmEvent for Pause {}

#[derive(Clone, PartialEq, Debug)]
pub struct OpenClose;
impl FsmEvent for OpenClose {}

#[derive(Clone, PartialEq, Debug)]
pub struct CdDetected { name: String }
impl FsmEvent for CdDetected {}

// states


#[derive(Clone, Default)]
pub struct Empty;
#[async_trait]
impl FsmState<Player> for Empty {
	async fn on_entry(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_empty_entry_counter += 1;
    }
	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_empty_exit_counter += 1;
    }
}

#[derive(Clone, Default)]
pub struct Open;
#[async_trait]
impl FsmState<Player> for Open {
	async fn on_entry(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_open_entry_counter += 1;
    }
	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_open_exit_counter += 1;
    }
}

#[derive(Clone, Default)]
pub struct Stopped;
#[async_trait]
impl FsmState<Player> for Stopped {
    async fn on_entry(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_stopped_entry_counter += 1;
    }
	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_stopped_exit_counter += 1;
    }
}

#[derive(Clone, Default)]
pub struct Paused;
#[async_trait]
impl FsmState<Player> for Paused {
    async fn on_entry(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_paused_entry_counter += 1;
    }
	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.action_paused_exit_counter += 1;
    }
}



// Submachine entry/exit
#[async_trait]
impl FsmState<Player> for Playing {
    async fn on_entry(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.playing_fsm_entry_counter += 1;
    }

	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Player>) {
        event_context.context.playing_fsm_exit_counter += 1;
    }
}

impl FsmStateFactory<PlayerContext> for Playing {
    fn new_state(parent_context: &PlayerContext) -> Self {
        Playing::new(Default::default())
    }
}

// actions


pub struct StartPlayback;
impl FsmAction<Player, Stopped, Playing> for StartPlayback {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Stopped, target_state: &mut Playing) {
        println!("StartPlayback");
        event_context.context.start_playback_counter += 1;
	}
}

pub struct OpenDrawer;
impl FsmAction<Player, Empty, Open> for OpenDrawer {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Empty, target_state: &mut Open) {
        println!("OpenDrawer");
	}
}
impl FsmAction<Player, Stopped, Open> for OpenDrawer {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Stopped, target_state: &mut Open) {
        println!("OpenDrawer");
	}
}

pub struct CloseDrawer;
impl FsmAction<Player, Open, Empty> for CloseDrawer {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Open, target_state: &mut Empty) {
        println!("CloseDrawer");
	}
}

pub struct StoreCdInfo;
impl FsmAction<Player, Empty, Stopped> for StoreCdInfo {
    fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Empty, target_state: &mut Stopped) {
        match event_context.event {
            &PlayerEvents::CdDetected(CdDetected { name: ref name }) => {
                println!("StoreCdInfo: name = {}", name);
            },
            _ => { panic!("Mismatched event!"); }
        }
	}
}

pub struct StopPlayback;
impl FsmAction<Player, Playing, Stopped> for StopPlayback {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Playing, target_state: &mut Stopped) {
        println!("StopPlayback");
	}
}
impl FsmAction<Player, Paused, Stopped> for StopPlayback {
	fn action(eevent_context: &mut EventContext<'_, Player>, source_state: &mut Paused, target_state: &mut Stopped) {
        println!("StopPlayback");
	}
}

pub struct PausePlayback;
impl FsmAction<Player, Playing, Paused> for PausePlayback {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Playing, target_state: &mut Paused) {
        println!("PausePlayback");
	}
}

pub struct ResumePlayback;
impl FsmAction<Player, Paused, Playing> for ResumePlayback {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Paused, target_state: &mut Playing) {
        println!("ResumePlayback");
	}
}

pub struct StopAndOpen;
impl FsmAction<Player, Playing, Open> for StopAndOpen {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Playing, target_state: &mut Open) {
        println!("StopAndOpen");
	}
}
impl FsmAction<Player, Paused, Open> for StopAndOpen {
	fn action(event_context: &mut EventContext<'_, Player>, source_state: &mut Paused, target_state: &mut Open) {
        println!("StopAndOpen");
	}
}

pub struct StoppedAgain;
impl FsmActionSelf<Player, Stopped> for StoppedAgain {
	fn action(event_context: &mut EventContext<'_, Player>, state: &mut Stopped) {
        println!("StoppedAgain");
	}
}


#[derive(Default, Debug)]
pub struct PlayerContext {
    action_empty_entry_counter: usize,
    action_empty_exit_counter: usize,

    action_open_entry_counter: usize,
    action_open_exit_counter: usize,

    action_stopped_entry_counter: usize,
    action_stopped_exit_counter: usize,

    action_paused_entry_counter: usize,
    action_paused_exit_counter: usize,

    start_playback_counter: usize,

    playing_fsm_entry_counter: usize,
    playing_fsm_exit_counter: usize
}


#[derive(Fsm)]
struct PlayerDefinition(
	InitialState<Player, Empty>,
	ContextType<PlayerContext>,

    SubMachine<Playing>,
    ShallowHistory<Player, EndPause, Playing>,

    Transition<Player, Stopped,     Play,       Playing,    StartPlayback>,
    Transition<Player, Stopped,     OpenClose,  Open,       OpenDrawer>,
    TransitionSelf<Player, Stopped,     Stop,               StoppedAgain>,

    Transition<Player, Open,        OpenClose,  Empty,      CloseDrawer>,

    Transition<Player, Empty,       OpenClose,  Open,       OpenDrawer>,
    Transition<Player, Empty,       CdDetected, Stopped,    StoreCdInfo>,

    // playing transitions
    Transition<Player,  Playing,    Stop,       Stopped,    StopPlayback>,
    Transition<Player,  Playing,    Pause,      Paused,     PausePlayback>,
    Transition<Player,  Playing,    OpenClose,  Open,       StopAndOpen>,

    Transition<Player, Paused,      EndPause,   Playing,    ResumePlayback>,
    Transition<Player, Paused,      Stop,       Stopped,    StopPlayback>,
    Transition<Player, Paused,      OpenClose,  Open,       StopAndOpen>,
);


// Playing FSM

// events
#[derive(Clone, PartialEq, Debug)]
pub struct NextSong;
impl FsmEvent for NextSong { }

#[derive(Clone, PartialEq, Debug)]
pub struct PreviousSong;
impl FsmEvent for PreviousSong { }


// states

#[derive(Clone, PartialEq, Default)]
pub struct Song1;
#[async_trait]
impl FsmState<Playing> for Song1 {
    async fn on_entry(&mut self, event_context: &mut EventContext<'_, Playing>) {
        println!("Starting Song 1");
        event_context.context.song1_entry_counter += 1;
    }
	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Playing>) {
        println!("Finishing Song 1");
        event_context.context.song1_exit_counter += 1;
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct Song2;
#[async_trait]
impl FsmState<Playing> for Song2 {
    async fn on_entry(&mut self, event_context: &mut EventContext<'_, Playing>) {
        println!("Starting Song 2");
        event_context.context.song2_entry_counter += 1;
    }
	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Playing>) {
        println!("Finishing Song 2");
        event_context.context.song2_exit_counter += 1;
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct Song3;
#[async_trait]
impl FsmState<Playing> for Song3 {
    async fn on_entry(&mut self, event_context: &mut EventContext<'_, Playing>) {
        println!("Starting Song 3");
    }
	async fn on_exit(&mut self, event_context: &mut EventContext<'_, Playing>) {
        println!("Finishing Song 3");
    }
}



// Actions
pub struct StartNextSong;
impl FsmAction<Playing, Song1, Song2> for StartNextSong {
	fn action(event_context: &mut EventContext<'_, Playing>, source_state: &mut Song1, target_state: &mut Song2) {
        println!("Playing::StartNextSong");
	}
}
impl FsmAction<Playing, Song2, Song3> for StartNextSong {
	fn action(event_context: &mut EventContext<'_, Playing>, source_state: &mut Song2, target_state: &mut Song3) {
        println!("Playing::StartNextSong");
	}
}

pub struct StartPrevSong;
impl FsmAction<Playing, Song2, Song1> for StartPrevSong {
	fn action(event_context: &mut EventContext<'_, Playing>, source_state: &mut Song2, target_state: &mut Song1) {
        println!("Playing::StartPrevSong");
	}
}
impl FsmAction<Playing, Song3, Song2> for StartPrevSong {
	fn action(event_context: &mut EventContext<'_, Playing>, source_state: &mut Song3, target_state: &mut Song2) {
        println!("Playing::StartPrevSong");
	}
}



#[derive(Default)]
pub struct PlayingContext {
    song1_entry_counter: usize,
    song1_exit_counter: usize,

    song2_entry_counter: usize,
    song2_exit_counter: usize,
}

#[derive(Fsm)]
struct PlayingDefinition(
	InitialState<Playing, Song1>,
    ContextType<PlayingContext>,

    Transition<Playing, Song1,  NextSong,       Song2,  StartNextSong>,
    Transition<Playing, Song2,  PreviousSong,   Song1,  StartPrevSong>,

    Transition<Playing, Song2,  NextSong,       Song3,  StartNextSong>,
    Transition<Playing, Song3,  PreviousSong,   Song2,  StartPrevSong>
);


#[cfg(test)]
#[tokio::test]
async fn test_player() {

    let mut p = Player::new(Default::default());

	p.start().await;
    assert_eq!(1, p.get_context().action_empty_entry_counter);

    p.process_event(PlayerEvents::OpenClose(OpenClose)).await.unwrap();
    assert_eq!(PlayerStates::Open, p.get_current_state());
    assert_eq!(1, p.get_context().action_empty_exit_counter);
    assert_eq!(1, p.get_context().action_open_entry_counter);


    p.process_event(PlayerEvents::OpenClose(OpenClose)).await.unwrap();
    assert_eq!(PlayerStates::Empty, p.get_current_state());
    assert_eq!(2, p.get_context().action_empty_entry_counter);
    assert_eq!(1, p.get_context().action_open_exit_counter);

    p.process_event(PlayerEvents::CdDetected(CdDetected { name: "louie, louie".to_string() })).await.unwrap();
    assert_eq!(PlayerStates::Stopped, p.get_current_state());
    assert_eq!(1, p.get_context().action_stopped_entry_counter);
    assert_eq!(2, p.get_context().action_empty_exit_counter);


    p.process_event(PlayerEvents::Play(Play)).await.unwrap();
    assert_eq!(PlayerStates::Playing, p.get_current_state());

    assert_eq!(PlayingStates::Song1, p.states.playing.get_current_state());
    assert_eq!(1, p.states.playing.get_context().song1_entry_counter);

    assert_eq!(1, p.get_context().action_stopped_exit_counter);
    assert_eq!(1, p.get_context().playing_fsm_entry_counter);
    assert_eq!(1, p.get_context().start_playback_counter);


    {
        let sub: &mut Playing = p.get_state_mut();
        sub.process_event(PlayingEvents::NextSong(NextSong)).await.unwrap();
        assert_eq!(PlayingStates::Song2, sub.get_current_state());
        assert_eq!(1, sub.get_context().song1_exit_counter);
        assert_eq!(1, sub.get_context().song2_entry_counter);
        assert_eq!(0, sub.get_context().song2_exit_counter);
    }
    assert_eq!(PlayerStates::Playing, p.get_current_state());
    assert_eq!(0, p.get_context().playing_fsm_exit_counter);


    p.process_event(PlayerEvents::Pause(Pause)).await.unwrap();
    assert_eq!(PlayerStates::Paused, p.get_current_state());
    assert_eq!(1, p.get_context().action_paused_entry_counter);
    assert_eq!(1, p.get_context().playing_fsm_exit_counter);
    {
        let sub: &Playing = p.get_state();
        assert_eq!(1, sub.get_context().song2_entry_counter);
        assert_eq!(1, sub.get_context().song2_exit_counter);
    }

    p.process_event(PlayerEvents::EndPause(EndPause)).await.unwrap();
    {
        let sub: &Playing = p.get_state();
        assert_eq!(PlayingStates::Song2, sub.get_current_state());
        assert_eq!(2, sub.get_context().song2_entry_counter);
    }
    assert_eq!(PlayerStates::Playing, p.get_current_state());
    assert_eq!(1, p.get_context().action_paused_exit_counter);
    assert_eq!(2, p.get_context().playing_fsm_entry_counter);

    p.process_event(PlayerEvents::Pause(Pause)).await.unwrap();
    assert_eq!(PlayerStates::Paused, p.get_current_state());
    assert_eq!(2, p.get_context().playing_fsm_exit_counter);
    assert_eq!(2, p.get_context().action_paused_entry_counter);

    p.process_event(PlayerEvents::Stop(Stop)).await.unwrap();
    assert_eq!(PlayerStates::Stopped, p.get_current_state());
    assert_eq!(2, p.get_context().action_paused_exit_counter);
    assert_eq!(2, p.get_context().action_stopped_entry_counter);

    p.process_event(PlayerEvents::Stop(Stop)).await.unwrap();
    assert_eq!(PlayerStates::Stopped, p.get_current_state());
    assert_eq!(2, p.get_context().action_stopped_exit_counter);
    assert_eq!(3, p.get_context().action_stopped_entry_counter);


    p.process_event(PlayerEvents::Play(Play)).await.unwrap();
    assert_eq!(PlayerStates::Playing, p.get_current_state());
    {
        let sub: &Playing = p.get_state();
        assert_eq!(PlayingStates::Song1, sub.get_current_state());
        assert_eq!(2, sub.get_context().song1_entry_counter);
    }

}

