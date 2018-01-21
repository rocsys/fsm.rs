extern crate quote;
extern crate syn;

use fsm_def::*;
use viz::*;

use quote::*;

use itertools::Itertools;

pub fn build_state_store(fsm: &FsmDescription) -> quote::Tokens {
    let fsm_name = fsm.get_fsm_ty();
    let impl_suffix = fsm.get_impl_suffix();
    let states_ty = fsm.get_states_ty();
    let states_store_ty = fsm.get_states_store_ty();

    let mut retr = quote::Tokens::new();

    let mut f = quote::Tokens::new();
    let mut n = quote::Tokens::new();
    for state in &fsm.get_all_states() {
        if fsm.is_submachine(&state) {
            continue;
        }

        let field_name = FsmDescription::to_state_field_name(&state);
        f.append(quote! { #field_name: #state,  }.as_str());
        n.append(quote! { #field_name: #state::new_state(context), }.as_str());

        retr.append(quote! {
            impl #impl_suffix FsmRetrieveState<#state> for #fsm_name {
                fn get_state(&self) -> &#state {
                    &self.states.#field_name
                }

                fn get_state_mut(&mut self) -> &mut #state {
                    &mut self.states.#field_name
                }
            }

            impl FsmRetrieveState<#state> for #states_store_ty {
                fn get_state(&self) -> &#state {
                    &self.#field_name
                }

                fn get_state_mut(&mut self) -> &mut #state {
                    &mut self.#field_name
                }
            }
        }.as_str());
    }

    let fsm_runtime_ty_inline = fsm.get_fsm_runtime_ty_inline();
    let (runtime_impl_generics, runtime_ty_generics, runtime_where_clause) = fsm.runtime_generics.split_for_impl();

    for sub in fsm.get_submachine_types() {
        {
            let field_name = FsmDescription::to_state_sub_started_field_name(sub);
            f.append(quote! { #field_name: bool, }.as_str());
            n.append(quote! { #field_name: false, }.as_str());
        }

        {
            let field_name = format!("fsm_sub_{}", syn_to_string(&sub).to_lowercase());
            let field_name = syn::parse_type(&field_name).unwrap();

            retr.append(quote! {
                impl #runtime_impl_generics FsmRetrieveState<#sub> for #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
                    fn get_state(&self) -> &#sub {
                        &self.#field_name.fsm
                    }

                    fn get_state_mut(&mut self) -> &mut #sub {
                        &mut self.#field_name.fsm
                    }
                }
            });
        }
    }

    let q = quote! {
        pub struct #states_store_ty {
            #f
        }

        impl #states_store_ty {
            pub fn new<C>(context: &C) -> #states_store_ty {
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
    let events_ref_ty = fsm.get_events_ref_ty();
    let event_kind_ty = fsm.get_event_kind_ty();
    let actions_ty = fsm.get_actions_ty();
    let states_ty = fsm.get_states_ty();
    let history_ty = fsm.get_history_ty();

    // events
    let all_transitions = fsm.get_all_transitions();
    //let events: Vec<_> = all_transitions.iter().map(|ref x| &x.event).unique_by(|x| *x).collect();

    let mut events_types = quote::Tokens::new();
    let mut events_ref_types = quote! {};
    let mut event_traits = quote::Tokens::new();
    let mut event_kinds = quote! {};
    {
        let events ={
            let mut events = fsm.get_all_events();
            let ty = syn::parse_type("NoEvent").unwrap();
            if !events.contains(&ty) {
                events.push(ty);
            }
            events
        };

        for event in events {
            event_kinds.append(quote! { #event, } );
            events_types.append(quote! { #event(#event), });
            events_ref_types.append(quote! {
                #event(&'a #event),
            });

            event_traits.append(quote! {
                impl From<#event> for #events_ty {
                    fn from(ev: #event) -> Self {
                        #events_ty::#event(ev)
                    }
                }

                impl<'a> From<&'a #event> for #events_ref_ty <'a> {
                    fn from(ev: &'a #event) -> Self {
                        #events_ref_ty::#event(ev)
                    }
                }
            }.as_str());
        }
    }

    let mut derive_events = quote::Tokens::new();
    if fsm.copyable_events {
        derive_events.append("#[derive(Copy, Clone)]");
    }

    // states
    let mut state_types = quote::Tokens::new();

    for state in &fsm.get_all_states() {
        state_types.append(quote! { #state, }.as_str());
    }
    
    quote! {
        // Events
        #[derive(Debug)]
        #derive_events
        pub enum #events_ty {
            #events_types
        }
        impl FsmEvents for #events_ty {
            fn new_no_event() -> Self {
                #events_ty::NoEvent(NoEvent)
            }
        }

        #[derive(Debug)]
        #derive_events
        pub enum #events_ref_ty <'a> {
            #events_ref_types
        }

        impl<'a> FsmEventsRef for #events_ref_ty <'a> {

        }

        #event_traits

        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        pub enum #event_kind_ty {
            #event_kinds
        }

        // States
        #[derive(PartialEq, Eq, Copy, Clone, Debug)]
        pub enum #states_ty {
            #state_types
        }
    }
}

pub fn build_state_timeout_timers_entry(fsm: &FsmDescription, state: &syn::Ty) -> Tokens {
    let mut q = quote!{};

    let state_field = fsm.to_state_field_access(&state);

    let start_timeout_timers = fsm.timeout_timers.iter().filter(|t| &t.state == state);
    for timer in start_timeout_timers {
        let n = timer.id as usize;
        q.append(quote! {
            if self.timeout_timers[#n].is_some() {
                panic!("This timer wasn't properly disposed!");
            }

            self.timeout_timers[#n] = #state_field.timeout_on_entry(&mut event_ctx);
            if let Some(ref timer) = self.timeout_timers[#n] {                                
                self.timers.create_timeout_timer(TimerId(#n as u32), timer.timeout);
            }
        });
    }

    q
}

pub fn build_state_timeout_timers_exit(fsm: &FsmDescription, state: &syn::Ty) -> Tokens {
    let mut q = quote!{};

    let stop_timeout_timers = fsm.timeout_timers.iter().filter(|t| &t.state == state);
    for timer in stop_timeout_timers {
        let n = timer.id as usize;
        q.append(quote! {
            if let Some(mut timer) = self.timeout_timers[#n].take() {
                if timer.cancel_on_state_exit {
                    // this timer hasn't timed out. announce its cancellation to the outside API
                    self.timers.cancel_timer(TimerId(#n as u32));
                }
            }
        });
    }

    q
}

pub fn build_event_state_transitions(fsm: &FsmDescription, event: &syn::Ty) -> quote::Tokens {

    let fsm_ty = fsm.get_fsm_ty();
    let fsm_ty_inline = fsm.get_fsm_ty_inline();
    let impl_suffix = fsm.get_impl_suffix();
    let fsm_where_ty = fsm.get_fsm_where_ty();
    let events_ty = fsm.get_events_ty();
    let events_ref_ty = fsm.get_events_ref_ty();
    let event_kind_ty = fsm.get_event_kind_ty();
    let states_ty = fsm.get_states_ty();
    let actions_ty = fsm.get_actions_ty();
    let history_ty = fsm.get_history_ty();
    let context_ty = &fsm.context_ty;

    let fsm_runtime_ty_inline = fsm.get_fsm_runtime_ty_inline();
    let (runtime_impl_generics, runtime_ty_generics, runtime_where_clause) = fsm.runtime_generics.split_for_impl();

    // states
    let mut event_dispatch = quote::Tokens::new();
    let mut interrupted_states = quote::Tokens::new();

    let mut transitions_count = 0;

    for region in &fsm.regions {
        let region_id = region.id;

        let mut q = quote!{ };

        for state in &region.get_all_states() {
            let t: Vec<_> = region.transitions.iter().filter(|&x| &x.source_state == state && &x.event == event).collect();

            if t.len() == 0 { continue; }

            let mut tq = quote!{};

            for transition in t {
                let transition_id = transition.id;
                let transition_id = quote! { TransitionId::Table(#transition_id) };

                transitions_count += 1;

                let event = &transition.event;
                let target_state = &transition.target_state;
                let action = &transition.action;
                let action_str = syn_to_string(&action);


                let source_state_field = fsm.to_state_field_access(&state);
                let target_state_field = fsm.to_state_field_access(&target_state);

                let action_call = if transition.has_same_states() {
                    quote! {
                        <#action as FsmActionSelf<#fsm_ty, #state, #event>>::action(&event, &mut event_ctx, &mut #source_state_field);
                        
                        self.inspection.on_action(#transition_id, & #action_str,
                                                  &#states_ty::#state, & #source_state_field,
                                                  &#states_ty::#state, & #source_state_field,
                                                  &event_ctx);
                    }
                } else {
                    quote! {
                        <#action as FsmAction<#fsm_ty, #state, #event, #target_state>>::action(&event, &mut event_ctx, &mut #source_state_field, &mut #target_state_field);

                        self.inspection.on_action(#transition_id, & #action_str,
                                                  &#states_ty::#state, & #source_state_field,
                                                  &#states_ty::#target_state, & #target_state_field,
                                                  &event_ctx);
                    }
                };

                let mut sub_init = quote! { };
                if fsm.is_submachine(&target_state) {
                    let f = FsmDescription::to_state_sub_started_field_name(&target_state);

                    let is_shallow = fsm.shallow_history_events.iter().find(|ref x| &x.event_ty == event && &x.target_state_ty == target_state).is_some();

                    if is_shallow == false {
                        let tf = fsm.to_sub_runtime(&target_state);
                        sub_init = quote! {
                            {
                                #tf.start();
                                self.fsm.states.#f = true;
                                just_called_start = true;
                            }
                        };
                    }
                }

                let mut sub_state_exit = quote! {};
                let mut sub_state_entry = quote! {};

                if fsm.is_submachine(&state) {
                    let tf = fsm.to_sub_runtime(&state);
                    sub_state_exit = quote! {
                        {                            
                            let s = #source_state_field.get_current_state();
                            #tf.call_on_exit(s);
                        }
                    };
                }

                
                if fsm.is_submachine(&target_state) {
                    let tf = fsm.to_sub_runtime(&target_state);
                    sub_state_entry = quote! {
                        {
                            let s = #target_state_field.get_current_state();
                            #tf.call_on_entry(s);
                        }
                    };
                }


                let mut state_exit = quote! {                    
                    #source_state_field.on_exit(&mut event_ctx);
                    self.inspection.on_state_exit(#transition_id, &current_state, & #source_state_field, &event_ctx);
                };

                let mut state_entry = quote! {                    
                    #target_state_field.on_entry(&mut event_ctx);
                    self.inspection.on_state_entry(#transition_id, &#states_ty::#target_state, & #target_state_field, &event_ctx);
                };
                
                if transition.transition_type == TransitionType::Internal {
                    state_exit = quote! {};
                    state_entry = quote! {};
                } else {
                    // is there a timeout timer for this state?
                    state_entry.append(build_state_timeout_timers_entry(fsm, target_state));
                    state_exit.append(build_state_timeout_timers_exit(fsm, state));
                }

                let guard = if let Some(ref guard_ty) = transition.guard {
                    quote! {
                        if <#guard_ty>::guard(&event, &event_ctx, &self.fsm.states)
                    }
                } else {
                    quote! {}
                };

                let state_set = if fsm.has_multiple_regions() { 
                    let mut q = quote! { self.fsm.state. };
                    q.append(&region.id.to_string());
                    q
                } else {
                    quote! { self.fsm.state }
                };

                
                let s = quote! {
                    #states_ty::#state #guard => {

                        self.inspection.on_transition_start(#transition_id, &current_state, &#states_ty::#target_state, &event_ctx);
                        
                        #state_exit
                        #sub_state_exit
                        
                        #action_call
                        //self.inspection.on_action(#transition_id, & #action_str, &event_ctx);
                        

                        let mut just_called_start = false;
                        #sub_init

                        #state_entry
                        if just_called_start == false {
                            #sub_state_entry
                        }

                        self.inspection.on_transition_finish(#transition_id, &current_state, &#states_ty::#target_state, &event_ctx);
                        #state_set = #states_ty::#target_state;
                        
                        Ok(())
                    },
                };
                
                tq.append(s);
            }

            q.append(tq);
        }

        let (region_state_field, result) = if fsm.has_multiple_regions() { 
            let mut q = quote! { self.fsm.state. };
            q.append(&region.id.to_string());

            let mut r = quote::Tokens::new();
            r.append(&format!("r{}", region.id));
            (q, r)            
        } else {
            (quote! { self.fsm.state }, quote! { res })
        };

        event_dispatch.append(quote! {

            let #result = {
                let event_ref: #events_ref_ty = (&event).into();
                let mut event_ctx = EventContext {
                    queue: &mut self.queue,
                    context: &mut self.fsm.context,
                    region: #region_id
                };

                let current_state = #region_state_field;
                
                match (current_state) {
                    #q
                    _ => {
                        Err(FsmError::NoTransition)
                    }
                }
            };

        });
       
        for interrupted_state in &region.interrupt_states {
            let mut m = quote!{};

            let s_ty = &interrupted_state.interrupt_state_ty;

            let whitelisted_event = interrupted_state.resume_event_ty.contains(event);            
            if whitelisted_event {
                m.append(quote! {
                    #states_ty::#s_ty => {
                        whitelisted_event = true;
                    },
                });
            } else {
                m.append(quote! {
                    #states_ty::#s_ty => {
                        is_interrupted = true;
                    },
                });
            }

            interrupted_states.append(quote! {
                match #region_state_field {
                    #m
                    _ => ()
                }
            }.as_str());
        }
    }

    let mut return_result = quote! {
        let mut res = None;
    };
    if fsm.has_multiple_regions() {                 
        let mut r = quote::Tokens::new();

        for region in &fsm.regions {
            let mut q = quote! { self.fsm.state. };
            q.append(&region.id.to_string());

            r = quote::Tokens::new();
            r.append(&format!("r{}", region.id));
            
            return_result.append(quote! {
                if #r == Err(FsmError::NoTransition) {
                    //self.inspection.on_no_transition(&#q, &event_ctx);
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
        });
    } else {
        return_result = quote! {
            if res == Err(FsmError::NoTransition) {
                //self.inspection.on_no_transition(&self.fsm.state, &event_ctx);
            }
        }
    }

    let f = quote! {
        impl #runtime_impl_generics FsmProcessor<#fsm_ty, #event> for #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
            fn process_event(&mut self, event: #event) -> Result<(), FsmError> {
                if self.execute_queue_pre {
                    self.execute_queued_events();
                }

                {                      
                    self.inspection.on_process_event(&self.fsm.get_current_state(), #event_kind_ty::#event, &event);
                }

                let res = {
                                        
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
                    self.execute_queued_events();
                }

                return res;
            }
        }
    };

    //panic!("f: {:?}", f);


    f
}



pub fn build_main_struct(fsm: &FsmDescription) -> quote::Tokens {

    let fsm_ty = fsm.get_fsm_ty();
    let fsm_viz_ty = fsm.get_fsm_viz_ty();
    let fsm_ty_inline = fsm.get_fsm_ty_inline();
    let impl_suffix = fsm.get_impl_suffix();
    let fsm_where_ty = fsm.get_fsm_where_ty();
    let events_ty = fsm.get_events_ty();
    let events_ref_ty = fsm.get_events_ref_ty();
    let event_kind_ty = fsm.get_event_kind_ty();
    let states_ty = fsm.get_states_ty();
    let current_state_ty = fsm.get_current_state_ty();
    let states_store_ty = fsm.get_states_store_ty();
    let actions_ty = fsm.get_actions_ty();
    let history_ty = fsm.get_history_ty();
    let ctx = &fsm.context_ty;
    
    let event_transitions = {
        let mut t = quote! {};

        let all_events = fsm.get_all_events();
        for event in all_events {
            t.append(build_event_state_transitions(fsm, &event));
        }

        t
    };

    let tagged_event_process = {
        let mut t = quote! {};

        let all_events = fsm.get_all_events();
        for event in all_events {
            t.append(quote! {
                #events_ty::#event(ev) => {
                    return self.process_event(ev);
                }
            });
        }

        t
    };

    let mut start = quote! {
        self.fsm.state = <#fsm_ty>::new_initial_state();
        let no = #events_ty::new_no_event();
    };

    for region in &fsm.regions {
        let initial_state = &region.initial_state_ty;
        let initial_state_field = FsmDescription::to_state_field_name(initial_state);
        let get_current_region_state = fsm.get_current_region_state(region.id);
        let region_id = region.id;

        start.append("{");
        start.append(quote! {                        
            let mut event_ctx = EventContext {
                queue: &mut self.queue,
                context: &mut self.fsm.context,
                region: #region_id
            };
            self.fsm.states.#initial_state_field.on_entry(&mut event_ctx);
            let current_region_state = self.fsm.state #get_current_region_state;
            self.inspection.on_state_entry(TransitionId::Start, &current_region_state, &self.fsm.states.#initial_state_field, &event_ctx);
        });

        start.append(build_state_timeout_timers_entry(fsm, initial_state));

        start.append("}");
    }

    start.append(quote! {
        self.process_tagged_event(no);
        self.process_anonymous_transitions();
        if self.execute_queue_post {
            self.execute_queued_events();
        }
    }.as_str());

    

    let mut stop = quote! {};
    if fsm.has_multiple_regions() {
        stop.append(quote!{
            let s = self.fsm.get_current_state();
        }.as_str());
        for region in &fsm.regions {
            let mut q = Tokens::new();
            q.append(&format!("s.{}", region.id));
            stop.append(quote! {
                self.call_on_exit(#q);
            }.as_str());
        }        
    } else {        
        stop = quote! {
            {
                let s = self.fsm.get_current_state();
                self.call_on_exit(s);
            }
        };
    }
    
    let sub_on_handlers = build_on_handlers(fsm);

    let initial_state = {
        let st: Vec<_> = fsm.regions.iter().map(|x| {
            let mut t = quote! { #states_ty:: };            
            x.initial_state_ty.to_tokens(&mut t);
            t
        }).collect();

        quote! {
            ( #(#st),* )
        }
    };
    
    let viz = build_viz(&fsm);

    let main_struct_docs = match () {
        #[cfg(not(feature = "viz_docs"))]
        () => quote! {},
        #[cfg(feature = "viz_docs")]
        () => 
        {
            let js_file = &format!("fsm_viz_{}.js", syn_to_string(&fsm_ty_inline));
            let viz_html_file = &format!("fsm_viz_{}.html", syn_to_string(&fsm_ty_inline));            
            let js_file_js = {
                let mut q = quote! {};

                {
                    let mut str_to_js = |key: &str, val: &str| {
                        let b: Vec<_> = val.bytes().map(|x| x.to_string()).collect();
                        q.append(&format!("\n/// var {} = String.fromCharCode({}); \n", key, b.connect(", ")));
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
            let mut q = quote! {
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

    let fsm_runtime_ty_inline = fsm.get_fsm_runtime_ty_inline();
    let (runtime_impl_generics, runtime_ty_generics, runtime_where_clause) = fsm.runtime_generics.split_for_impl();

    let runtime_struct_where_clause = {
        use syn::*;

        let mut w1 = runtime_where_clause.clone();

        for ty_param in &fsm.runtime_generics.ty_params {
            w1.predicates.push(WherePredicate::BoundPredicate(WhereBoundPredicate {
                bound_lifetimes: vec![],
                bounded_ty: parse_type(&syn_to_string(&ty_param.ident)).unwrap(),
                bounds: ty_param.bounds.clone()
            }));
        }
        

        w1
    };

    let mut fsm_sub_inits = vec![];

    let runtime_additional_fields = {
        let mut q = quote! {};

        for sub in fsm.get_submachine_types() {
            let field_name = format!("fsm_sub_{}", syn_to_string(&sub).to_lowercase());
            let field_name = syn::parse_type(&field_name).unwrap();

            let sub_runtime_type = {
                // FI: FsmInspection, FT: FsmTimers
                let n = format!("{}Runtime < FI, FT >", syn_to_string(&sub));
                syn::parse_type(&n).unwrap()
            };

            q.append(quote! {
                #field_name: #sub_runtime_type,
            });

            let sub_runtime_type = {
                let n = format!("{}", syn_to_string(&sub));
                syn::parse_type(&n).unwrap()
            };

            fsm_sub_inits.push(quote! {
                #field_name: <#sub_runtime_type>::new_custom(
                    Default::default() /* todo: create context from parent? */,
                    inspection.clone(),
                    timers.clone()
                )?,
            });
        }


        if fsm.timeout_timers.len() > 0 {
            let l = fsm.timeout_timers.len();
            q.append(quote! {
                timeout_timers: [Option<TimerSettings>; #l ],
            });
        }

        q
    };

    let new_runtime_fsm_null = {
        let g = fsm.get_fsm_runtime_generics(&[
            ("FI", &syn::parse_type(&"FsmInspectNull").unwrap()),
            ("FT", &syn::parse_type(&"FsmTimersNull").unwrap())
        ]);

        
        let fsm_sub_inits = {
            let mut q = quote!{};
            for f in &fsm_sub_inits {
                q.append(f);
            }
            q
        };

        let fsm_additional_field_inits = {
            let mut q = quote!{};

            if fsm.timeout_timers.len() > 0 {
                q.append(quote! {
                    timeout_timers: Default::default(),
                });
            }

            q
        };


        let g_custom = fsm.get_fsm_runtime_generics(&[]);

        let timers_required = {
            if fsm.has_timers() {
                quote! {
                    if !<FT>::implemented() {
                        return Err(FsmError::TimersImplementationRequired);
                    }
                }
            } else {
                quote! {}
            }
        };

        let fi_bounds = {
            let mut q = vec![];
            q.push(quote! { FsmInspect<#fsm_ty> });

            for sub in fsm.get_submachine_types() {
                q.push(quote! { FsmInspect<#sub> });
            }

            quote! { #(#q)+* }
        };
        

        quote! {
            impl #impl_suffix #fsm_ty #fsm_where_ty {
                pub fn new(context: #ctx) -> Result<#g, FsmError> {
                    Self::new_custom(context, FsmInspectNull, FsmTimersNull)
                }

                pub fn new_custom<FI: #fi_bounds, FT: FsmTimers>(context: #ctx, inspection: FI, timers: FT) -> Result<#g_custom, FsmError> {
                    let queue = FsmEventQueueVec::new();
                    let fsm = Self::new_fsm(context);
                    
                    #timers_required

                    //inspection.init(&fsm);

                    let runtime = #fsm_runtime_ty_inline {                        
                        fsm: fsm,

                        #fsm_sub_inits
                        #fsm_additional_field_inits

                        execute_queue_pre: true,
                        execute_queue_post: true,

                        inspection: inspection,
                        timers: timers,
                        queue: queue
                    };                    

                    Ok(runtime)
                }
            }
        }
    };

    let sub_event_processor = {
        let mut q = quote! {};

        for sub in fsm.get_submachine_types() {
            let field_name = format!("fsm_sub_{}", syn_to_string(&sub).to_lowercase());
            let field_name = syn::parse_type(&field_name).unwrap();

            let ev = syn::parse_type(&format!("{}Events", syn_to_string(&sub))).unwrap();

            q.append(quote! {
                impl #runtime_impl_generics FsmProcessor<#fsm_ty, #ev> for #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
                    fn process_event(&mut self, event: PlayingEvents) -> Result<(), FsmError> {
                        self.#field_name.process_tagged_event(event)
                    }
                }
            });
        }

        q
    };


    let timer_events_processor = {
        let mut timer_timeouts = quote!{};
        for timer in &fsm.timeout_timers {
            let id = timer.id as u32;
            let event = &timer.event_on_timeout;

            timer_timeouts.append(quote! {
                #id => {
                    let timer = self.timeout_timers[#id as usize].take();

                    let event: #event = Default::default();                    
                    self.process_event(event)?;                    
                }
            })
        }

        quote! {
            pub fn process_timer_event(&mut self, timer_event: &FsmTimerEvent) -> Result<(), FsmError> {
                match timer_event {
                    &FsmTimerEvent::TimedOut(ref timer) => {
                        match timer.timer_id.0 {
                            #timer_timeouts 
                            _ => {
                                return Err(FsmError::UnknownTimerId);
                            }
                        }
                    }
                }

                Ok(())
            }
        }
    };

    let fsm_info = ::codegen_info::build_fsm_info(&fsm);

    quote! {

        pub struct #fsm_runtime_ty_inline #runtime_ty_generics #runtime_struct_where_clause {
            inspection: FI,
            timers: FT,
            queue: FsmEventQueueVec<#fsm_ty>,
            
            fsm: #fsm_ty,

            #runtime_additional_fields

            pub execute_queue_pre: bool,
            pub execute_queue_post: bool
        }
        
        impl #runtime_impl_generics FsmFrontend<#fsm_ty> for #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
            fn start(&mut self) {
                #start
            }

	        fn stop(&mut self) {
                #stop
            }

            fn get_queue(&self) -> &FsmEventQueue<#fsm_ty> {
                &self.queue
            }

            fn get_queue_mut(&mut self) -> &mut FsmEventQueue<#fsm_ty> {
                &mut self.queue
            }

            fn process_tagged_event(&mut self, event: #events_ty) -> Result<(), FsmError> {
                match event {
                    #tagged_event_process
                    #events_ty::NoEvent(_) => {
                        return Err(FsmError::NoTransition);
                    }
                }
            }            
        }

        impl #runtime_impl_generics #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
            #sub_on_handlers

            pub fn get_timers(&self) -> &FT {
                &self.timers
            }

            pub fn get_timers_mut(&mut self) -> &mut FT {
                &mut self.timers
            }

            #timer_events_processor
        }


        #main_struct_docs
        pub struct #fsm_ty_inline #impl_suffix #fsm_where_ty {
	        state: #current_state_ty,
            states: #states_store_ty,
	        context: #ctx
        }

        impl #impl_suffix Fsm for #fsm_ty #fsm_where_ty {
            type E = #events_ty;
            type EventKind = #event_kind_ty;
            //type EventRef = #events_ref_ty;
            type S = #states_ty;
            type SS = #states_store_ty;
            type C = #ctx;
            type CS = #current_state_ty;
            type RegionState = #states_ty;

            fn get_current_state(&self) -> #current_state_ty {
                self.state
            }

            fn get_states(&self) -> &#states_store_ty {
                &self.states
            }

	        fn get_states_mut(&mut self) -> &mut #states_store_ty {
                &mut self.states
            }
        }

        #event_transitions

        impl #impl_suffix #fsm_ty #fsm_where_ty {
            pub fn new_fsm(context: #ctx) -> Self {
                #fsm_ty_inline {
                    state: Self::new_initial_state(),
                    states: #states_store_ty::new(&context),
                    context: context                    
                }
            }
        }

        #new_runtime_fsm_null

        impl #impl_suffix #fsm_ty #fsm_where_ty {            
            fn new_initial_state() -> #current_state_ty {
                #initial_state
            }
            
            pub fn get_context(&self) -> &#ctx {
                &self.context
            }

            pub fn get_context_mut(&mut self) -> &mut #ctx {
                &mut self.context
            }                        
        }

        impl #runtime_impl_generics FsmProcessor<#fsm_ty, #events_ty> for #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
            fn process_event(&mut self, event: #events_ty) -> Result<(), FsmError> {
                self.process_tagged_event(event.into())
            }
        }

        #sub_event_processor

        impl #runtime_impl_generics ::std::ops::Deref for #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
            type Target = #fsm_ty;

            fn deref(&self) -> & #fsm_ty {
                &self.fsm
            }
        }

        impl #runtime_impl_generics ::std::ops::DerefMut for #fsm_runtime_ty_inline #runtime_ty_generics #runtime_where_clause {
            fn deref_mut(&mut self) -> &mut #fsm_ty {
                &mut self.fsm
            }
        }

        pub struct #fsm_viz_ty;
        impl #fsm_viz_ty {
            pub fn module_path() -> &'static str {
                 module_path!()
            }

            #viz
        }

        #fsm_info
    }
}

pub fn build_on_handlers(fsm: &FsmDescription) -> quote::Tokens {
    
    //let fsm_ty = fsm.get_fsm_ty();
    let events_ty = fsm.get_events_ty();
    let events_ref_ty = fsm.get_events_ref_ty();
    let states_ty = fsm.get_states_ty();

    let mut on_entry = quote::Tokens::new();
    let mut on_exit = quote::Tokens::new();


    for region in &fsm.regions {
        let region_id = region.id;
        for state in &region.get_all_states() {
            if fsm.is_submachine(&state) { continue; }
            let f = FsmDescription::to_state_field_name(&state);

            let event_ctx = quote! {
                let mut event_ctx = EventContext {
                    queue: &mut self.queue,
                    context: &mut self.fsm.context,                    
                    region: #region_id
                };
            };

            on_entry.append(quote!{
                #states_ty::#state => {
                    #event_ctx
                    self.fsm.states.#f.on_entry(&mut event_ctx);
                    self.inspection.on_state_entry(TransitionId::Start, &state, &self.fsm.states.#f, &event_ctx);
                },
            });

            on_exit.append(quote!{
                #states_ty::#state => {
                    #event_ctx
                    self.fsm.states.#f.on_exit(&mut event_ctx);
                    self.inspection.on_state_exit(TransitionId::Stop, &state, &self.fsm.states.#f, &event_ctx);
                },
            });
        }
    }

    quote! {
        pub fn call_on_entry(&mut self, state: #states_ty) {
            let no = #events_ty::new_no_event();            
            match state {
                #on_entry
                _ => ()
            }
        }

        pub fn call_on_exit(&mut self, state: #states_ty) {
            let no = #events_ty::new_no_event();
            match state {
                #on_exit
                _ => ()
            }
        }
    }
}