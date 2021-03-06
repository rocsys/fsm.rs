extern crate quote;

use crate::fsm_def::*;
use crate::viz::*;

use quote::*;

use itertools::Itertools;

pub fn build_state_store(fsm: &FsmDescription) -> quote::Tokens {
    let fsm_name = fsm.get_fsm_ty();
    let impl_suffix = fsm.get_impl_suffix();
    let states_store_ty = fsm.get_states_store_ty();
    let context_ty = &fsm.context_ty;

    let mut retr = quote::Tokens::new();

    let mut f = quote::Tokens::new();
    let mut n = quote::Tokens::new();
    for state in &fsm.get_all_states() {
        let field_name = FsmDescription::to_state_field_name(&state);
        f.append(quote! { #field_name: FsmArc<#state>,  }.as_str());
        n.append(quote! { #field_name: std::sync::Arc::new(tokio::sync::RwLock::new(#state::new_state(context))), }.as_str());

        retr.append(quote! {
            impl #impl_suffix FsmRetrieveState<#state> for #fsm_name {
                fn get_state(&self) -> FsmArc<#state> {
                    std::sync::Arc::clone(&self.states.#field_name)
                }
            }

            impl #impl_suffix FsmRetrieveState<#state> for #states_store_ty {
                fn get_state(&self) -> FsmArc<#state> {
                    std::sync::Arc::clone(&self.#field_name)
                }
            }
        }.as_str());
    }

    for sub in fsm.get_submachine_types() {
        let field_name = FsmDescription::to_state_sub_started_field_name(sub);
        f.append(quote! { #field_name: bool, }.as_str());
        n.append(quote! { #field_name: false, }.as_str());
    }

    let q = quote! {
        #[derive(Debug)]
        pub struct #states_store_ty {
            #f
        }

        impl #states_store_ty {
            pub fn new<'a>(context: &FsmArc<#context_ty>) -> #states_store_ty {
                #states_store_ty {
                    #n
                }
            }
        }

        #retr
    };

    q
}



pub fn build_enums(fsm: &FsmDescription) -> quote::Tokens {
    let fsm_name = fsm.get_fsm_ty();
    let impl_suffix = fsm.get_impl_suffix();
    let events_ty = fsm.get_events_ty();
    let states_ty = fsm.get_states_ty();

    // events
    let all_transitions = fsm.get_all_transitions();
    let events = all_transitions.iter().map(|ref x| &x.event).unique_by(|x| *x);

    let mut events_types = quote::Tokens::new();
    let mut event_traits = quote::Tokens::new();
    for event in events {
        let mut t = quote::Tokens::new();
        event.to_tokens(&mut t);
        if t.as_str() == "NoEvent" || t.as_str() == "FsmErrorEvent" { continue; }

        events_types.append(quote! { #event(#event), }.as_str());
        event_traits.append(quote! {
            impl From<#event> for #events_ty {
                fn from(ev: #event) -> Self {
                    #events_ty::#event(ev)
                }
            }
        }.as_str());
    }
    events_types.append(quote! { FsmErrorEvent(FsmErrorEvent), }.as_str());
    events_types.append(quote! { NoEvent(NoEvent) }.as_str());

    let mut derive_events = quote::Tokens::new();
    if fsm.copyable_events {
        derive_events.append("#[derive(Copy, Clone)]");
    }

    // states
    let mut state_types = quote::Tokens::new();
    let mut state_display = quote::Tokens::new();
    let mut state_name = quote::Tokens::new();
    let mut state_is_submachine = quote::Tokens::new();

    for state in &fsm.get_all_states() {
        state_types.append(quote! { #state(FsmArc<#state>), }.as_str());
        state_display.append(quote! { #states_ty::#state(_) => { write!(f, stringify!(#state)) } }.as_str());

        if fsm.is_submachine(&state) {
            state_name.append(quote! { #states_ty::#state(s) => {
                let sub = s.read().await;
                let sub = sub.get_current_state().await;

                format!("{}::{}", stringify!(#state), sub.name().await)
             } }.as_str());
            state_is_submachine.append(quote! { #states_ty::#state(_) => { true } }.as_str());
        } else {
            state_name.append(quote! { #states_ty::#state(_) => { format!(stringify!(#state)) } }.as_str());
            state_is_submachine.append(quote! { #states_ty::#state(_) => { false } }.as_str());
        }
    }

    quote! {
        // Events
        #[derive(Debug)]
        #derive_events
        pub enum #events_ty {
            #events_types
        }

        impl #impl_suffix FsmEvents<#fsm_name> for #events_ty {
            fn new_no_event() -> Self {
                #events_ty::NoEvent(NoEvent)
            }

            fn new_error_event(error: FsmTransitionError) -> Self {
                #events_ty::FsmErrorEvent(FsmErrorEvent(error))
            }
        }

        #event_traits

        // States
        #[derive(Clone, Debug)]
        pub enum #states_ty {
            #state_types
        }

        impl std::fmt::Display for #states_ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #state_display
                }
            }
        }

        #[async_trait]
        impl FsmRetrieveStateName for #states_ty {
            async fn name(&self) -> String {
                match self {
                    #state_name
                }
            }
        }
    }
}

pub fn build_state_transitions(fsm: &FsmDescription) -> quote::Tokens {
    let fsm_ty = fsm.get_fsm_ty();
    let events_ty = fsm.get_events_ty();
    let states_ty = fsm.get_states_ty();
    let error_state_ty = fsm.get_error_state_ty();

    // states

    let mut event_dispatch = quote::Tokens::new();
    let mut interrupted_states = quote::Tokens::new();

    let error_handling = if let Some(error_state) = error_state_ty {
        let error_state_name = ty_to_string(&error_state);
        quote! {
            if let Err(error) = result {
                let event = <Self::E as FsmEvents<Self>>::new_error_event(error);
                self.process_event(event)
                    .await
                    .expect(format!("Error state '{}' generated an error itself!", #error_state_name).as_str());
            }
         }
    } else {
        quote! { result.unwrap(); }
    };

    for region in &fsm.regions {
        let mut q = quote::Tokens::new();

        for state in &region.get_all_states() {
            let t: Vec<_> = region.transitions.iter().filter(|&x| &x.source_state == state).collect();

            if t.len() == 0 { continue; }

            let mut tq = quote::Tokens::new();

            for transition in t {

                let event = &transition.event;
                let target_state = &transition.target_state;
                let action = &transition.action;


                let source_state_field = FsmDescription::to_state_field_name(&state);
                let target_state_field = FsmDescription::to_state_field_name(&target_state);

                let action_call = if transition.has_same_states() {
                    quote! {
                        <#action as FsmActionSelf<#fsm_ty, #state>>::action(
                            &event_ctx,
                            &*self.states.#source_state_field.read().await
                        ).await;
                    }
                } else {
                    quote! {
                        <#action as FsmAction<#fsm_ty, #state, #target_state>>::action(
                            &event_ctx,
                            &*self.states.#source_state_field.read().await,
                            &*self.states.#target_state_field.read().await
                        ).await;
                    }
                };

                let mut sub_init = quote! { };
                if fsm.is_submachine(&target_state) {
                    let is_shallow = fsm.shallow_history_events.iter().find(|ref x| &x.event_ty == event && &x.target_state_ty == target_state).is_some();

                    if is_shallow == false {
                        sub_init = quote! {
                            {
                                self.states.#target_state_field.read().await.start().await;
                                just_called_start = true;
                            }
                        };
                    }
                }

                let mut sub_state_exit = quote! {};
                let mut sub_state_entry = quote! {};

                if fsm.is_submachine(&state) {
                    sub_state_exit = quote! {
                        {
                            let state = self.states.#source_state_field.read().await;
                            let s = state.get_current_state().await;

                            if let Err(error) = state.call_on_exit(s).await {
                                break Err(error);
                            }
                        }
                    };
                }


                if fsm.is_submachine(&target_state) {
                    sub_state_entry = quote! {
                        {
                            let state = self.states.#target_state_field.read().await;
                            let s = state.get_current_state().await;

                            if let Err(error) = state.call_on_entry(s).await {
                                break Err(error);
                            }
                        }
                    };
                }


                let mut state_exit = quote! {
                    self.inspection.on_state_exit(&current_state, &event_ctx).await;

                    {
                        let state = self.states.#source_state_field.read().await;
                        if let Err(error) = state.on_exit(&mut event_ctx).await {
                            break Err(error);
                        }
                    }
                };

                let mut state_entry = quote! {
                    self.inspection.on_state_entry(
                        &#states_ty::#target_state(std::sync::Arc::clone(&self.states.#target_state_field)),
                        &event_ctx
                    ).await;

                    {
                        let state = self.states.#target_state_field.read().await;
                        if let Err(error) = state.on_entry(&mut event_ctx).await {
                            break Err(error);
                        }
                    }
                };

                if transition.transition_type == TransitionType::Internal {
                    state_exit = quote! {};
                    state_entry = quote! {};
                }

                let guard = if let Some(ref guard_ty) = transition.guard {
                    quote! {
                        if #guard_ty::guard(&event_ctx, &self.states)
                    }
                } else {
                    quote! {}
                };

                let state_set = if fsm.has_multiple_regions() {
                    let mut q = quote! { self.state. };
                    q.append(&region.id.to_string());
                    q
                } else {
                    quote! { self.state }
                };


                let s = quote! {
                    (&#states_ty::#state(_), &#events_ty::#event(_)) #guard => {
                        let result: FsmTransitionResult<()> = loop {
                            self.inspection.on_transition(
                                &current_state,
                                &#states_ty::#target_state(std::sync::Arc::clone(&self.states.#target_state_field)),
                                &event_ctx
                            ).await;

                            #sub_state_exit
                            #state_exit

                            self.inspection.on_action(&current_state, &event_ctx).await;

                            {
                                #action_call
                            }

                            {
                                let mut state_ = #state_set.write().await;
                                *state_ = #states_ty::#target_state(std::sync::Arc::clone(&self.states.#target_state_field));
                            }

                            event_ctx.current_state = self.get_current_state().await;

                            #state_entry

                            let mut just_called_start = false;
                            #sub_init
                            if just_called_start == false {
                                #sub_state_entry
                            }

                            break Ok(())
                        };

                        #error_handling

                        Ok(())
                    },
                };

                tq.append(s.as_str());
            }

            q.append(tq.as_str());
        }

        let (region_state_field, result) = if fsm.has_multiple_regions() {
            let mut q = quote! { self.state. };
            q.append(&region.id.to_string());

            let mut r = quote::Tokens::new();
            r.append(&format!("r{}", region.id));
            (q, r)
        } else {
            (quote! { self.state }, quote! { res })
        };

        event_dispatch.append(quote! {

            let current_state = fsm_read_state(&#region_state_field).await;
            let #result = match (&current_state, &event) {
                #q
                (_, _) => Err(FsmError::NoTransition)
            };

        }.as_str());


        for interrupted_state in &region.interrupt_states {
            let s_ty = &interrupted_state.interrupt_state_ty;

            let mut m = quote::Tokens::new();
            for e_ty in &interrupted_state.resume_event_ty {
                m.append(quote! {
                    (#states_ty::#s_ty(_), &#events_ty::#e_ty(_)) => {
                        whitelisted_event = true;
                    },
                }.as_str());
            }

            interrupted_states.append(quote! {
                match (fsm_read_state(&#region_state_field).await, &event) {
                    #m
                    (#states_ty::#s_ty(_), _) => {
                        is_interrupted = true;
                    },
                    (_, _) => ()
                }
            }.as_str());
        }


    }

    let mut return_result = quote! {
        let mut res = None;
    };
    if fsm.has_multiple_regions() {
        #[allow(unused_assignments)]
        let mut r = quote::Tokens::new();

        for region in &fsm.regions {
            let mut q = quote! { self.state. };
            q.append(&region.id.to_string());

            r = quote::Tokens::new();
            r.append(&format!("r{}", region.id));

            return_result.append(quote! {
                if #r == Err(FsmError::NoTransition) {
                    self.inspection.on_no_transition(&fsm_read_state(&#q).await, &event_ctx).await;
                }
                if res.is_none() && #r.is_ok() {
                    res = Some(#r);
                }
                if res.is_none() && !#r.is_ok() && #r != Err(FsmError::NoTransition) {
                    res = Some(#r);
                }
            }.as_str());
        }

        return_result.append(quote! {
            let res = res.unwrap_or(Err(FsmError::NoTransition));
        }.as_str());
    } else {
        return_result = quote! {
            if res == Err(FsmError::NoTransition) {
                self.inspection.on_no_transition(&fsm_read_state(&self.state).await, &event_ctx).await;
            }
        }
    }

    let f = quote! {
        async fn process_event(&self, event: #events_ty) -> Result<(), FsmError> {
            if self.execute_queue_pre {
                self.execute_queued_events().await;
            }

            let res = {
                let mut event_ctx = EventContext {
                    event: &event,
                    queue: std::sync::Arc::clone(&self.queue),
                    context: std::sync::Arc::clone(&self.context),
                    current_state: self.get_current_state().await
                };

                {
                    let mut is_interrupted = false;
                    let mut whitelisted_event = false;
                    #interrupted_states
                    if is_interrupted && whitelisted_event == false {
                        return Err(FsmError::Interrupted);
                    }
                }

                #event_dispatch

                #return_result

                res
            };

            if self.execute_queue_post {
                self.execute_queued_events().await;
            }

            return res;
        }
    };


    f
}


pub fn build_main_struct(fsm: &FsmDescription) -> quote::Tokens {
    let fsm_ty = fsm.get_fsm_ty();
    let fsm_ty_inline = fsm.get_fsm_ty_inline();
    let impl_suffix = fsm.get_impl_suffix();
    let events_ty = fsm.get_events_ty();
    let states_ty = fsm.get_states_ty();
    let current_state_ty = fsm.get_current_state_ty();
    let states_store_ty = fsm.get_states_store_ty();
    let error_state_ty = fsm.get_error_state_ty();
    let inspection_ty = fsm.get_inspection_ty();
    let ctx = &fsm.context_ty;

    let transitions = build_state_transitions(fsm);

    let mut start = quote! {
        {
            let mut state = self.state.write().await;
            *state = Self::new_initial_state(&self.states);
        }

        let mut event = <Self::E as FsmEvents<Self>>::new_no_event();
    };

    let start_error_handling = if error_state_ty.is_some() {
        quote! {
            event = <Self::E as FsmEvents<Self>>::new_error_event(error);
         }
    } else {
        quote! { panic!("The on_entry action of the initial state returned an error: {:?}", error); }
    };

    for region in &fsm.regions {
        let initial_state = &region.initial_state_ty;
        let initial_state_field = FsmDescription::to_state_field_name(initial_state);

        let sub_start = if fsm.is_submachine(&initial_state) {
            quote! { self.states.#initial_state_field.read().await.start().await; }
        } else {
            quote! { }
        };

        start.append(quote! {
            {
                let event_ctx = EventContext {
                    event: &event,
                    queue: std::sync::Arc::clone(&self.queue),
                    context: std::sync::Arc::clone(&self.context),
                    current_state: self.get_current_state().await
                };

                self.inspection.on_state_entry(
                    &#states_ty::#initial_state(std::sync::Arc::clone(&self.states.#initial_state_field)),
                    &event_ctx
                ).await;

                {
                    let state = self.states.#initial_state_field.read().await;
                    if let Err(error) = state.on_entry(&event_ctx).await {
                        #start_error_handling
                    }
                }

                #sub_start
            }
        }.as_str());
    }

    start.append(quote! {
        match self.process_event(event).await {
            // The initial state receives a NoEvent which results in a NoTransition error. Hence, we ignore it
            Err(FsmError::NoTransition) | Ok(_) => (),
            Err(e) => panic!("Unknown error happens during starting the state machine: {:?}", e),
        }

        self.process_anonymous_transitions().await.ok();
    }.as_str());



    let mut stop = quote! {};
    if fsm.has_multiple_regions() {
        stop.append(quote!{
            let s = self.get_current_state().await;
        }.as_str());
        for region in &fsm.regions {
            let mut q = Tokens::new();
            q.append(&format!("s.{}", region.id));
            stop.append(quote! {
                // Ignore errors on exit
                let _ = self.call_on_exit(#q).await;
            }.as_str());
        }
    } else {
        stop = quote! {
            {
                let s = self.get_current_state().await;

                // Ignore errors on exit
                let _ = self.call_on_exit(s).await;
            }
        };
    }

    let sub_on_handlers = build_on_handlers(fsm);

    let initial_state = {
        let st: Vec<_> = fsm.regions.iter().map(|x| {
            let mut t = quote! { #states_ty:: };
            x.initial_state_ty.to_tokens(&mut t);
            t.append(quote! { (states.get_state()) });
            t
        }).collect();

        quote! {
            ( #(#st),* )
        }
    };

    let viz = build_viz(&fsm);




    /*
            complete_js.push_str(&format!("\nvar cy = init_cy_fsm();\n"));
            complete_js.push_str(&Self::viz_cytoscape_fsm(""));
            complete_js.push_str(&format!("\n f.run_layout(); \n"));
            */
    let main_struct_docs = match () {
        #[cfg(not(feature = "viz_docs"))]
        () => quote! {},
        #[cfg(feature = "viz_docs")]
        () =>
        {
            let js_file = &format!("fsm_viz_{}.js", ty_to_string(&fsm_ty_inline));
            let viz_html_file = &format!("fsm_viz_{}.html", ty_to_string(&fsm_ty_inline));
            let js_file_js = {
                let mut q = quote! {};

                {
                    let mut str_to_js = |key: &str, val: &str| {
                        let b: Vec<_> = val.bytes().map(|x| x.to_string()).collect();
                        q.append(&format!("\n/// var {} = String.fromCharCode({}); \n", key, b.join(", ")));
                    };

                    str_to_js("viz_js", js_file);
                    str_to_js("viz_html_full", viz_html_file);
                    str_to_js("lib_js", "viz_fsm.js");
                    str_to_js("t_scr", "text/javascript");
                    str_to_js("t_body", "body");
                    str_to_js("t_head", "head");
                    str_to_js("t_script", "script");
                }

                q
            };
            let q = quote! {
                /// A generated state machine.
                ///
                /// <span>
                /// <script src="https://cdnjs.cloudflare.com/ajax/libs/jquery/3.2.1/jquery.min.js"></script>
                /// <script src="https://cdnjs.cloudflare.com/ajax/libs/cytoscape/3.1.0/cytoscape.js"></script>
                /// <div id="cy" style="width: 100%; height: 500px;"></div>
                /// <script type="text/javascript">
                /// $(function() {
                ///    var crate_doc_root = window.rootPath + window.currentCrate + String.fromCharCode(47);
                #js_file_js
                ///    window.viz_fsm_inline = true;
                ///    window.viz_html_full = crate_doc_root + viz_html_full;
                ///    // fsm data
                ///    var s = document.createElement(t_script);
                ///    s.type = t_scr;
                ///    s.src = crate_doc_root + viz_js;
                ///    document.getElementsByTagName(t_head)[0].appendChild(s);
                ///    // library
                ///    var s = document.createElement(t_script);
                ///    s.type = t_scr;
                ///    s.src = crate_doc_root + lib_js;
                ///    document.getElementsByTagName(t_head)[0].appendChild(s);
                /// });
                /// </script>
                /// </span>
                ///
                /// <a href="#" id="viz_fullscreen_link" target="_blank">Fullscreen FSM visualization</a>
            };
            q
        }
    };

    let error_state = if let Some(error_state_name) = &error_state_ty {
        quote! { Some(#states_ty::#error_state_name(states.get_state())) }
    } else {
        quote! { None }
    };

    quote! {
        #main_struct_docs
        #[derive(Debug)]
        pub struct #fsm_ty {
	        state: FsmArc<#current_state_ty>,
            states: #states_store_ty,
	        context: FsmArc<#ctx>,
            queue: FsmArc<dyn FsmEventQueue<#fsm_ty>>,
            inspection: #inspection_ty,
            error_state: Option<#states_ty>,

            pub execute_queue_pre: bool,
            pub execute_queue_post: bool
        }

        #[async_trait]
        impl #impl_suffix Fsm for #fsm_ty {
            type E = #events_ty;
            type S = #states_ty;
            type SS = #states_store_ty;
            type C = #ctx;
            type CS = #current_state_ty;

            fn new(context: &FsmArc<Self::C>) -> Self {
                let states = #states_store_ty::new(&context);
                let state = Self::new_initial_state(&states);
                let error_state = #error_state;

                #fsm_ty_inline {
                    state: std::sync::Arc::new(tokio::sync::RwLock::new(state)),
                    states,
                    inspection: <#inspection_ty>::new_from_context(&context),
                    error_state,
                    context: std::sync::Arc::clone(context),
                    queue: std::sync::Arc::new(tokio::sync::RwLock::new(FsmEventQueueVec::new())),

                    execute_queue_pre: true,
                    execute_queue_post: true
                }
            }

            async fn start(&self) {
                #start
            }

	        async fn stop(&self) {
                #stop
            }

            fn get_queue(&self) -> &FsmArc<dyn FsmEventQueue<Self>> {
                &self.queue
            }

            async fn get_current_state(&self) -> #current_state_ty {
                fsm_read_state(&self.state).await
            }

            fn get_states(&self) -> &#states_store_ty {
                &self.states
            }

            #sub_on_handlers
            #transitions
        }

        impl #impl_suffix #fsm_ty {
            fn new_initial_state(states: &#states_store_ty) -> #current_state_ty {
               #initial_state
            }

            pub fn get_context(&self) -> &FsmArc<#ctx> {
                &self.context
            }

            pub fn module_path() -> &'static str {
                 module_path!()
            }

            #viz
        }
    }
}

pub fn build_on_handlers(fsm: &FsmDescription) -> quote::Tokens {
    let events_ty = fsm.get_events_ty();
    let states_ty = fsm.get_states_ty();

    let mut on_entry = quote::Tokens::new();
    let mut on_exit = quote::Tokens::new();

    for state in &fsm.get_all_states() {

        if fsm.is_submachine(&state) { continue; }

        let f = FsmDescription::to_state_field_name(&state);

        on_entry.append(quote!{
            #states_ty::#state(_) => {
                self.inspection.on_state_entry(&state, &event_ctx).await;
                self.states.#f.read().await.on_entry(&event_ctx).await
            },
        }.as_str());

        on_exit.append(quote!{
            #states_ty::#state(_) => {
                self.inspection.on_state_exit(&state, &event_ctx).await;
                self.states.#f.read().await.on_exit(&event_ctx).await
            },
        }.as_str());
    }

    quote! {
        async fn call_on_entry(&self, state: #states_ty) -> FsmTransitionResult<()> {
            let no = #events_ty::new_no_event();
            let event_ctx = EventContext {
                event: &no,
                queue: std::sync::Arc::clone(&self.queue),
                context: std::sync::Arc::clone(&self.context),
                current_state: self.get_current_state().await
            };

            match state {
                #on_entry
                _ => Ok(())
            }
        }

        async fn call_on_exit(&self, state: #states_ty) -> FsmTransitionResult<()> {
            let no = #events_ty::new_no_event();
            let event_ctx = EventContext {
                event: &no,
                queue: std::sync::Arc::clone(&self.queue),
                context: std::sync::Arc::clone(&self.context),
                current_state: self.get_current_state().await
            };

            match state {
                #on_exit
                _ => Ok(())
            }
        }
    }
}
